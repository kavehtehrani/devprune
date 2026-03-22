use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ScanError;
use crate::rules::types::{Category, SafetyLevel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub id: Uuid,
    pub path: PathBuf,
    pub rule_id: String,
    pub rule_name: String,
    /// Human-readable description of the rule, typically including restore
    /// instructions (e.g. "Regenerate with `npm install`").
    pub rule_description: String,
    pub category: Category,
    pub safety: SafetyLevel,
    pub size: Option<u64>,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanEvent {
    Found(ArtifactInfo),
    SizeUpdate { id: Uuid, size: u64 },
    Progress(ProgressInfo),
    Error(ScanError),
    Complete(ScanSummary),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    pub dirs_visited: u64,
    pub artifacts_found: u64,
    pub total_size_found: u64,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_artifacts: u64,
    pub total_size: u64,
    pub duration: Duration,
    pub errors: Vec<ScanError>,
    pub dirs_visited: u64,
}

/// Configuration for a single scan run. `paths` defaults to `["."]` so
/// callers can do `ScanConfig::default()` for a sensible starting point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub paths: Vec<PathBuf>,
    pub max_depth: Option<u32>,
    pub follow_symlinks: bool,
    pub include_categories: Vec<Category>,
    pub exclude_categories: Vec<Category>,
    pub min_size_bytes: Option<u64>,
    pub skip_paths: Vec<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            max_depth: None,
            follow_symlinks: false,
            include_categories: vec![],
            exclude_categories: vec![],
            min_size_bytes: None,
            skip_paths: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionSummary {
    pub count: usize,
    pub total_size: u64,
}

// ScanError needs Serialize/Deserialize for ScanEvent to derive them.
impl Serialize for ScanError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ScanError", 2)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("message", &self.message)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ScanError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            path: PathBuf,
            message: String,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(ScanError {
            path: h.path,
            message: h.message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_config_default_paths() {
        let cfg = ScanConfig::default();
        assert_eq!(cfg.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn scan_config_default_no_filters() {
        let cfg = ScanConfig::default();
        assert!(cfg.include_categories.is_empty());
        assert!(cfg.exclude_categories.is_empty());
        assert!(cfg.skip_paths.is_empty());
        assert!(cfg.max_depth.is_none());
        assert!(cfg.min_size_bytes.is_none());
        assert!(!cfg.follow_symlinks);
    }

    #[test]
    fn artifact_info_serialization_roundtrip() {
        let artifact = ArtifactInfo {
            id: Uuid::new_v4(),
            path: PathBuf::from("/home/user/project/node_modules"),
            rule_id: "npm-node-modules".to_string(),
            rule_name: "npm node_modules".to_string(),
            rule_description: "Regenerate with `npm install`.".to_string(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            size: Some(102_400),
            last_modified: Some(Utc::now()),
            is_directory: true,
        };

        let json = serde_json::to_string(&artifact).unwrap();
        let decoded: ArtifactInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, artifact.id);
        assert_eq!(decoded.path, artifact.path);
        assert_eq!(decoded.category, artifact.category);
        assert_eq!(decoded.safety, artifact.safety);
        assert_eq!(decoded.size, artifact.size);
        assert_eq!(decoded.is_directory, artifact.is_directory);
    }

    #[test]
    fn scan_event_found_roundtrip() {
        let artifact = ArtifactInfo {
            id: Uuid::new_v4(),
            path: PathBuf::from("/tmp/target"),
            rule_id: "cargo-target".to_string(),
            rule_name: "Cargo target".to_string(),
            rule_description: "Regenerate with `cargo build`.".to_string(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            size: None,
            last_modified: None,
            is_directory: true,
        };
        let event = ScanEvent::Found(artifact);
        let json = serde_json::to_string(&event).unwrap();
        let decoded: ScanEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ScanEvent::Found(_)));
    }

    #[test]
    fn selection_summary_fields() {
        let summary = SelectionSummary {
            count: 5,
            total_size: 2048,
        };
        assert_eq!(summary.count, 5);
        assert_eq!(summary.total_size, 2048);
    }
}
