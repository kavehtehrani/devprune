use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DevpruneError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("trash operation failed: {message}")]
    Trash { message: String },

    #[error("restore conflict: {path} already exists at destination")]
    RestoreConflict { path: PathBuf },

    #[error("insufficient space: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("cross-device move not supported: {from} -> {to}")]
    CrossDeviceMove { from: PathBuf, to: PathBuf },

    #[error("rule configuration error: {message}")]
    RuleConfig { message: String },

    #[error("manifest corrupted: {message}")]
    ManifestCorrupted { message: String },
}

pub type Result<T> = std::result::Result<T, DevpruneError>;

/// A non-fatal error encountered during scanning. The scan continues
/// but the error is recorded so the user can inspect it afterwards.
#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scan error at {}: {}", self.path.display(), self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let err = DevpruneError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        let msg = err.to_string();
        assert!(msg.contains("I/O error"));
    }

    #[test]
    fn permission_denied_display() {
        let err = DevpruneError::PermissionDenied {
            path: PathBuf::from("/etc/shadow"),
        };
        assert!(err.to_string().contains("permission denied"));
        assert!(err.to_string().contains("/etc/shadow"));
    }

    #[test]
    fn path_not_found_display() {
        let err = DevpruneError::PathNotFound {
            path: PathBuf::from("/nonexistent"),
        };
        assert!(err.to_string().contains("path not found"));
        assert!(err.to_string().contains("/nonexistent"));
    }

    #[test]
    fn trash_error_display() {
        let err = DevpruneError::Trash {
            message: "disk full".to_string(),
        };
        assert!(err.to_string().contains("trash operation failed"));
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn restore_conflict_display() {
        let err = DevpruneError::RestoreConflict {
            path: PathBuf::from("/home/user/project"),
        };
        assert!(err.to_string().contains("restore conflict"));
    }

    #[test]
    fn insufficient_space_display() {
        let err = DevpruneError::InsufficientSpace {
            needed: 1024,
            available: 512,
        };
        let msg = err.to_string();
        assert!(msg.contains("insufficient space"));
        assert!(msg.contains("1024"));
        assert!(msg.contains("512"));
    }

    #[test]
    fn cross_device_move_display() {
        let err = DevpruneError::CrossDeviceMove {
            from: PathBuf::from("/home/user/file"),
            to: PathBuf::from("/mnt/backup/file"),
        };
        assert!(err.to_string().contains("cross-device move"));
    }

    #[test]
    fn rule_config_display() {
        let err = DevpruneError::RuleConfig {
            message: "invalid glob".to_string(),
        };
        assert!(err.to_string().contains("rule configuration error"));
        assert!(err.to_string().contains("invalid glob"));
    }

    #[test]
    fn manifest_corrupted_display() {
        let err = DevpruneError::ManifestCorrupted {
            message: "unexpected EOF".to_string(),
        };
        assert!(err.to_string().contains("manifest corrupted"));
        assert!(err.to_string().contains("unexpected EOF"));
    }

    #[test]
    fn scan_error_display() {
        let err = ScanError {
            path: PathBuf::from("/some/path"),
            message: "no permission".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/some/path"));
        assert!(msg.contains("no permission"));
    }
}
