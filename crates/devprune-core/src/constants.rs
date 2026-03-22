pub const APP_NAME: &str = "devprune";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_TICK_RATE_MS: u64 = 100;
pub const DEFAULT_AUTO_PURGE_DAYS: u64 = 30;

pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const MANIFEST_VERSION: u32 = 1;
pub const METADATA_FILENAME: &str = "metadata.json";
pub const CONTENT_DIRNAME: &str = "content";
pub const TRASH_DIRNAME: &str = "trash";
pub const ITEMS_DIRNAME: &str = "items";

pub const LINUX_SKIP_PATHS: &[&str] = &["/proc", "/sys", "/dev", "/run", "/tmp", "/snap"];
pub const MACOS_SKIP_PATHS: &[&str] =
    &["/System", "/Library", "/Volumes", "/private/var/vm"];
pub const ALWAYS_SKIP_DIRS: &[&str] = &[".git"];

/// errno 18: cross-device rename (EXDEV). Used when detecting that `rename`
/// failed because source and destination are on different filesystems.
pub const EXDEV_ERROR_CODE: i32 = 18;

/// Threshold in bytes above which the confirm-delete dialog shows an extra
/// large-deletion warning (10 GiB).
pub const LARGE_DELETE_THRESHOLD_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_not_empty() {
        assert!(!APP_NAME.is_empty());
    }

    #[test]
    fn app_version_not_empty() {
        assert!(!APP_VERSION.is_empty());
    }

    #[test]
    fn manifest_filename_not_empty() {
        assert!(!MANIFEST_FILENAME.is_empty());
    }

    #[test]
    fn metadata_filename_not_empty() {
        assert!(!METADATA_FILENAME.is_empty());
    }

    #[test]
    fn content_dirname_not_empty() {
        assert!(!CONTENT_DIRNAME.is_empty());
    }

    #[test]
    fn trash_dirname_not_empty() {
        assert!(!TRASH_DIRNAME.is_empty());
    }

    #[test]
    fn items_dirname_not_empty() {
        assert!(!ITEMS_DIRNAME.is_empty());
    }

    #[test]
    fn linux_skip_paths_not_empty() {
        assert!(!LINUX_SKIP_PATHS.is_empty());
    }

    #[test]
    fn macos_skip_paths_not_empty() {
        assert!(!MACOS_SKIP_PATHS.is_empty());
    }

    #[test]
    fn always_skip_dirs_not_empty() {
        assert!(!ALWAYS_SKIP_DIRS.is_empty());
    }
}
