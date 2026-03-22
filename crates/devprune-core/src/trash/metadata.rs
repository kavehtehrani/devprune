use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{CONTENT_DIRNAME, MANIFEST_VERSION, METADATA_FILENAME};
use crate::error::{DevpruneError, Result};
use crate::rules::types::Category;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntryMetadata {
    pub id: Uuid,
    pub original_path: PathBuf,
    pub trashed_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub rule_id: String,
    pub category: Category,
    pub original_permissions: u32,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashManifest {
    pub version: u32,
    pub entries: Vec<TrashManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashManifestEntry {
    pub id: Uuid,
    pub original_path: PathBuf,
    pub trashed_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub rule_id: String,
    pub category: Category,
}

impl From<&TrashEntryMetadata> for TrashManifestEntry {
    fn from(m: &TrashEntryMetadata) -> Self {
        Self {
            id: m.id,
            original_path: m.original_path.clone(),
            trashed_at: m.trashed_at,
            size_bytes: m.size_bytes,
            rule_id: m.rule_id.clone(),
            category: m.category,
        }
    }
}

impl TrashManifest {
    pub fn new() -> Self {
        Self {
            version: MANIFEST_VERSION,
            entries: Vec::new(),
        }
    }

    /// Serialize to JSON, write to a temp file, fsync, then rename into place.
    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| DevpruneError::Trash {
            message: format!("failed to serialize manifest: {e}"),
        })?;

        let parent = path.parent().ok_or_else(|| DevpruneError::Trash {
            message: format!("manifest path has no parent: {}", path.display()),
        })?;

        // Write to a temp file in the same directory so the rename is atomic.
        let tmp_path = parent.join(format!(".manifest.tmp.{}", Uuid::new_v4()));
        fs::write(&tmp_path, &json)?;

        // fsync the file contents.
        {
            let file = fs::OpenOptions::new().write(true).open(&tmp_path)?;
            file.sync_all()?;
        }

        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Read and parse the manifest. Returns an empty manifest when the file
    /// does not exist.
    pub fn read_from(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => {
                serde_json::from_str(&contents).map_err(|e| DevpruneError::ManifestCorrupted {
                    message: format!("failed to parse manifest at {}: {e}", path.display()),
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Scan all UUID directories under `items_dir`, read each `metadata.json`,
    /// and build a fresh manifest from what is actually on disk.
    ///
    /// Orphaned metadata (no content dir) is cleaned up. Content without
    /// metadata is logged as a warning.
    pub fn rebuild_from_items(items_dir: &Path) -> Result<Self> {
        let mut entries = Vec::new();

        // items_dir may not exist yet (empty trash).
        if !items_dir.exists() {
            return Ok(Self::new());
        }

        for dir_entry in fs::read_dir(items_dir)? {
            let dir_entry = dir_entry?;
            let item_dir = dir_entry.path();

            if !item_dir.is_dir() {
                continue;
            }

            let metadata_path = item_dir.join(METADATA_FILENAME);
            let content_path = item_dir.join(CONTENT_DIRNAME);

            let has_metadata = metadata_path.exists();
            let has_content = content_path.exists();

            match (has_metadata, has_content) {
                (true, true) => {
                    let raw = fs::read_to_string(&metadata_path)?;
                    let meta: TrashEntryMetadata =
                        serde_json::from_str(&raw).map_err(|e| DevpruneError::ManifestCorrupted {
                            message: format!(
                                "failed to parse metadata at {}: {e}",
                                metadata_path.display()
                            ),
                        })?;
                    entries.push(TrashManifestEntry::from(&meta));
                }
                (true, false) => {
                    // Orphaned metadata with no content: clean it up.
                    log::warn!(
                        "trash: orphaned metadata (no content) at {}; removing",
                        item_dir.display()
                    );
                    fs::remove_dir_all(&item_dir)?;
                }
                (false, true) => {
                    log::warn!(
                        "trash: content directory without metadata at {}; skipping",
                        item_dir.display()
                    );
                }
                (false, false) => {
                    // Empty item dir; ignore.
                }
            }
        }

        // Sort by trashed_at for a stable manifest order.
        entries.sort_by_key(|e| e.trashed_at);

        Ok(Self {
            version: MANIFEST_VERSION,
            entries,
        })
    }
}

impl Default for TrashManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_metadata(id: Uuid, original_path: PathBuf) -> TrashEntryMetadata {
        TrashEntryMetadata {
            id,
            original_path,
            trashed_at: Utc::now(),
            size_bytes: 1024,
            rule_id: "cargo-target".to_string(),
            category: Category::BuildOutput,
            original_permissions: 0o755,
            hostname: "testhost".to_string(),
        }
    }

    #[test]
    fn metadata_serialization_roundtrip() {
        let id = Uuid::new_v4();
        let meta = make_metadata(id, PathBuf::from("/home/user/project/target"));

        let json = serde_json::to_string(&meta).unwrap();
        let decoded: TrashEntryMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.id, meta.id);
        assert_eq!(decoded.original_path, meta.original_path);
        assert_eq!(decoded.size_bytes, meta.size_bytes);
        assert_eq!(decoded.rule_id, meta.rule_id);
        assert_eq!(decoded.original_permissions, meta.original_permissions);
        assert_eq!(decoded.hostname, meta.hostname);
    }

    #[test]
    fn manifest_atomic_write_and_read() {
        let tmp = tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");

        let id = Uuid::new_v4();
        let meta = make_metadata(id, PathBuf::from("/home/user/project/target"));

        let mut manifest = TrashManifest::new();
        manifest.entries.push(TrashManifestEntry::from(&meta));

        manifest.write_atomic(&manifest_path).unwrap();

        let loaded = TrashManifest::read_from(&manifest_path).unwrap();
        assert_eq!(loaded.version, MANIFEST_VERSION);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].id, id);
    }

    #[test]
    fn manifest_read_nonexistent_returns_empty() {
        let tmp = tempdir().unwrap();
        let manifest_path = tmp.path().join("nonexistent.json");

        let manifest = TrashManifest::read_from(&manifest_path).unwrap();
        assert_eq!(manifest.version, MANIFEST_VERSION);
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn manifest_rebuild_from_items() {
        let tmp = tempdir().unwrap();
        let items_dir = tmp.path().join("items");
        fs::create_dir_all(&items_dir).unwrap();

        // Create two valid items.
        for _ in 0..2 {
            let id = Uuid::new_v4();
            let item_dir = items_dir.join(id.to_string());
            fs::create_dir_all(&item_dir).unwrap();

            let content_dir = item_dir.join(CONTENT_DIRNAME);
            fs::create_dir_all(&content_dir).unwrap();

            let meta = make_metadata(id, PathBuf::from("/some/path"));
            let json = serde_json::to_string(&meta).unwrap();
            fs::write(item_dir.join(METADATA_FILENAME), json).unwrap();
        }

        let manifest = TrashManifest::rebuild_from_items(&items_dir).unwrap();
        assert_eq!(manifest.entries.len(), 2);
    }

    #[test]
    fn manifest_rebuild_cleans_orphaned_metadata() {
        let tmp = tempdir().unwrap();
        let items_dir = tmp.path().join("items");
        fs::create_dir_all(&items_dir).unwrap();

        // Item with metadata but no content dir.
        let id = Uuid::new_v4();
        let item_dir = items_dir.join(id.to_string());
        fs::create_dir_all(&item_dir).unwrap();

        let meta = make_metadata(id, PathBuf::from("/some/path"));
        let json = serde_json::to_string(&meta).unwrap();
        fs::write(item_dir.join(METADATA_FILENAME), json).unwrap();

        let manifest = TrashManifest::rebuild_from_items(&items_dir).unwrap();

        // The orphaned item should have been removed and not appear in the manifest.
        assert!(manifest.entries.is_empty());
        assert!(!item_dir.exists());
    }
}
