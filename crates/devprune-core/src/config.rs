use std::path::PathBuf;

use crate::constants::{APP_NAME, ITEMS_DIRNAME, MANIFEST_FILENAME, TRASH_DIRNAME};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub trash_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub items_dir: PathBuf,
    pub log_path: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Option<Self> {
        let data_dir = dirs::data_dir()?.join(APP_NAME);
        let config_dir = dirs::config_dir()?.join(APP_NAME);
        Some(Self::from_dirs(data_dir, config_dir))
    }

    pub fn with_base(base: PathBuf) -> Self {
        let data_dir = base.join("data");
        let config_dir = base.join("config");
        Self::from_dirs(data_dir, config_dir)
    }

    fn from_dirs(data_dir: PathBuf, config_dir: PathBuf) -> Self {
        let trash_dir = data_dir.join(TRASH_DIRNAME);
        let manifest_path = data_dir.join(MANIFEST_FILENAME);
        let items_dir = trash_dir.join(ITEMS_DIRNAME);
        let log_path = data_dir.join(format!("{}.log", APP_NAME));

        Self {
            data_dir,
            config_dir,
            trash_dir,
            manifest_path,
            items_dir,
            log_path,
        }
    }

    /// Returns paths that should be skipped during scanning, combining
    /// platform-specific system paths with the app's own trash directory.
    pub fn skip_paths(&self) -> Vec<PathBuf> {
        let platform_paths: &[&str] = if cfg!(target_os = "macos") {
            crate::constants::MACOS_SKIP_PATHS
        } else {
            crate::constants::LINUX_SKIP_PATHS
        };

        let mut paths: Vec<PathBuf> = platform_paths
            .iter()
            .map(|s| PathBuf::from(s))
            .collect();

        paths.push(self.trash_dir.clone());
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn with_base_produces_correct_structure() {
        let tmp = tempdir().unwrap();
        let base = tmp.path().to_path_buf();
        let paths = AppPaths::with_base(base.clone());

        assert_eq!(paths.data_dir, base.join("data"));
        assert_eq!(paths.config_dir, base.join("config"));
        assert_eq!(paths.trash_dir, base.join("data").join(TRASH_DIRNAME));
        assert_eq!(
            paths.manifest_path,
            base.join("data").join(MANIFEST_FILENAME)
        );
        assert_eq!(
            paths.items_dir,
            base.join("data").join(TRASH_DIRNAME).join(ITEMS_DIRNAME)
        );
    }

    #[test]
    fn skip_paths_includes_trash_dir() {
        let tmp = tempdir().unwrap();
        let paths = AppPaths::with_base(tmp.path().to_path_buf());
        let skip = paths.skip_paths();
        assert!(skip.contains(&paths.trash_dir));
    }

    #[test]
    fn resolve_returns_some() {
        // This test may fail in a sandboxed environment with no home dir,
        // but on a normal system dirs::data_dir() should succeed.
        let result = AppPaths::resolve();
        assert!(result.is_some());
    }
}
