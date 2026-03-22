use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::config::AppPaths;
use crate::constants::{CONTENT_DIRNAME, EXDEV_ERROR_CODE, METADATA_FILENAME};
use crate::error::{DevpruneError, Result};
use crate::rules::types::Category;
use crate::trash::metadata::{TrashEntryMetadata, TrashManifest, TrashManifestEntry};

/// Manages the devprune trash directory: trashing, restoring, and purging items.
pub struct TrashManager {
    app_paths: AppPaths,
}

impl TrashManager {
    /// Create a new `TrashManager`, ensuring the required directories exist.
    pub fn new(app_paths: AppPaths) -> Result<Self> {
        fs::create_dir_all(&app_paths.items_dir)?;
        Ok(Self { app_paths })
    }

    /// Move `source` into trash and return the assigned UUID.
    ///
    /// The item directory layout is:
    ///   `<items_dir>/<uuid>/metadata.json`
    ///   `<items_dir>/<uuid>/content/`   ← the actual data
    pub fn trash_item(
        &self,
        source: &Path,
        size_bytes: u64,
        rule_id: &str,
        category: Category,
    ) -> Result<Uuid> {
        if !source.exists() {
            return Err(DevpruneError::PathNotFound {
                path: source.to_path_buf(),
            });
        }

        let id = Uuid::new_v4();
        let item_dir = self.item_dir(id);
        let content_path = item_dir.join(CONTENT_DIRNAME);

        fs::create_dir_all(&item_dir)?;

        let permissions = read_permissions(source)?;
        let hostname = current_hostname();

        let metadata = TrashEntryMetadata {
            id,
            original_path: source.to_path_buf(),
            trashed_at: Utc::now(),
            size_bytes,
            rule_id: rule_id.to_string(),
            category,
            original_permissions: permissions,
            hostname,
        };

        // Write metadata first so we can detect a partial trash on rebuild.
        let meta_json = serde_json::to_string_pretty(&metadata).map_err(|e| {
            DevpruneError::Trash {
                message: format!("failed to serialize metadata: {e}"),
            }
        })?;
        fs::write(item_dir.join(METADATA_FILENAME), meta_json)?;

        // Move the content.
        move_path(source, &content_path)?;

        self.rebuild_manifest()?;
        Ok(id)
    }

    /// Restore a trashed item to its original location.
    ///
    /// Returns the path it was restored to.
    pub fn restore_item(&self, id: Uuid) -> Result<PathBuf> {
        let item_dir = self.item_dir(id);
        let metadata_path = item_dir.join(METADATA_FILENAME);
        let content_path = item_dir.join(CONTENT_DIRNAME);

        let raw = fs::read_to_string(&metadata_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DevpruneError::PathNotFound {
                    path: metadata_path.clone(),
                }
            } else {
                e.into()
            }
        })?;

        let metadata: TrashEntryMetadata =
            serde_json::from_str(&raw).map_err(|e| DevpruneError::ManifestCorrupted {
                message: format!("failed to parse metadata for {id}: {e}"),
            })?;

        let dest = &metadata.original_path;

        if dest.exists() {
            return Err(DevpruneError::RestoreConflict { path: dest.clone() });
        }

        // Ensure the parent directory exists.
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        move_path(&content_path, dest)?;

        // Clean up the item directory after a successful restore.
        fs::remove_dir_all(&item_dir)?;

        self.rebuild_manifest()?;
        Ok(dest.clone())
    }

    /// Permanently remove a trashed item.
    pub fn purge_item(&self, id: Uuid) -> Result<()> {
        let item_dir = self.item_dir(id);
        if item_dir.exists() {
            fs::remove_dir_all(&item_dir)?;
        }
        self.rebuild_manifest()?;
        Ok(())
    }

    /// Purge all items older than `days` days. Returns the IDs that were purged.
    pub fn purge_older_than(&self, days: u64) -> Result<Vec<Uuid>> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let entries = self.list_items()?;

        let to_purge: Vec<Uuid> = entries
            .iter()
            .filter(|e| e.trashed_at < cutoff)
            .map(|e| e.id)
            .collect();

        for id in &to_purge {
            let item_dir = self.item_dir(*id);
            if item_dir.exists() {
                fs::remove_dir_all(&item_dir)?;
            }
        }

        // One manifest rebuild after all purges.
        self.rebuild_manifest()?;
        Ok(to_purge)
    }

    /// Return all entries currently in the manifest.
    pub fn list_items(&self) -> Result<Vec<TrashManifestEntry>> {
        let manifest = TrashManifest::read_from(&self.app_paths.manifest_path)?;
        Ok(manifest.entries)
    }

    // --- private helpers ---

    fn item_dir(&self, id: Uuid) -> PathBuf {
        self.app_paths.items_dir.join(id.to_string())
    }

    fn rebuild_manifest(&self) -> Result<()> {
        let manifest = TrashManifest::rebuild_from_items(&self.app_paths.items_dir)?;
        manifest.write_atomic(&self.app_paths.manifest_path)?;
        Ok(())
    }
}

/// Read Unix permissions mode from a path, returning 0 as a safe fallback on
/// platforms that don't support it.
fn read_permissions(path: &Path) -> Result<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(path)?;
        Ok(meta.permissions().mode())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(0)
    }
}

/// Return the machine hostname, falling back to "unknown" when it cannot be
/// determined.
fn current_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Move `src` to `dst`.
///
/// Tries `fs::rename` first. On a cross-device move (EXDEV, errno 18) it
/// falls back to a recursive copy followed by a size/count verification and
/// then deletion of the original.
fn move_path(src: &Path, dst: &Path) -> Result<()> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => copy_verify_delete(src, dst),
        Err(e) => Err(e.into()),
    }
}

fn is_cross_device(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(EXDEV_ERROR_CODE)
}

/// Recursively copy `src` to `dst`, verify the counts and total size match,
/// then remove `src`.
fn copy_verify_delete(src: &Path, dst: &Path) -> Result<()> {
    copy_recursive(src, dst)?;

    let (src_count, src_size) = count_and_size(src)?;
    let (dst_count, dst_size) = count_and_size(dst)?;

    if src_count != dst_count || src_size != dst_size {
        // Remove the incomplete destination before returning the error.
        let _ = fs::remove_dir_all(dst);
        return Err(DevpruneError::Trash {
            message: format!(
                "cross-device copy verification failed: src({src_count} files, {src_size}B) != dst({dst_count} files, {dst_size}B)"
            ),
        });
    }

    fs::remove_dir_all(src)?;
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let name = entry.file_name();
            copy_recursive(&entry.path(), &dst.join(&name))?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
    }
    Ok(())
}

/// Walk a path and return (file_count, total_bytes).
fn count_and_size(path: &Path) -> Result<(u64, u64)> {
    let mut count = 0u64;
    let mut total = 0u64;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let (c, s) = count_and_size(&entry.path())?;
            count += c;
            total += s;
        }
    } else {
        count = 1;
        total = fs::metadata(path)?.len();
    }

    Ok((count, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_manager(base: &Path) -> TrashManager {
        let app_paths = AppPaths::with_base(base.to_path_buf());
        TrashManager::new(app_paths).unwrap()
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn list_empty_trash() {
        let tmp = tempdir().unwrap();
        let manager = make_manager(tmp.path());
        let items = manager.list_items().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn trash_and_restore_cycle() {
        let tmp = tempdir().unwrap();
        let manager = make_manager(tmp.path());

        let src = tmp.path().join("project").join("target");
        write_file(&src.join("binary"), "compiled");

        let id = manager
            .trash_item(&src, 7, "cargo-target", Category::BuildOutput)
            .unwrap();

        // Source should be gone.
        assert!(!src.exists());

        // Item should appear in listing.
        let items = manager.list_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, id);

        // Restore it.
        let restored = manager.restore_item(id).unwrap();
        assert_eq!(restored, src);
        assert!(src.join("binary").exists());

        // Trash should now be empty.
        assert!(manager.list_items().unwrap().is_empty());
    }

    #[test]
    fn restore_conflict_when_path_exists() {
        let tmp = tempdir().unwrap();
        let manager = make_manager(tmp.path());

        let src = tmp.path().join("artifact");
        write_file(&src, "data");

        let id = manager
            .trash_item(&src, 4, "some-rule", Category::Cache)
            .unwrap();

        // Recreate the original path to cause a conflict.
        write_file(&src, "new data");

        let err = manager.restore_item(id).unwrap_err();
        assert!(matches!(err, DevpruneError::RestoreConflict { .. }));
    }

    #[test]
    fn purge_removes_permanently() {
        let tmp = tempdir().unwrap();
        let manager = make_manager(tmp.path());

        let src = tmp.path().join("artifact");
        write_file(&src, "data");

        let id = manager
            .trash_item(&src, 4, "some-rule", Category::Cache)
            .unwrap();

        manager.purge_item(id).unwrap();

        assert!(manager.list_items().unwrap().is_empty());
        // Trying to restore should fail (item dir gone).
        assert!(manager.restore_item(id).is_err());
    }

    #[test]
    fn restore_creates_parent_dir() {
        let tmp = tempdir().unwrap();
        let manager = make_manager(tmp.path());

        // Source lives in a deep tree that will not exist when we restore.
        let src = tmp.path().join("deep").join("nested").join("dir").join("file.txt");
        write_file(&src, "hello");

        let id = manager
            .trash_item(&src, 5, "some-rule", Category::Logs)
            .unwrap();

        // Remove the parent so restore has to recreate it.
        fs::remove_dir_all(tmp.path().join("deep")).unwrap();

        let restored = manager.restore_item(id).unwrap();
        assert!(restored.exists());
    }
}
