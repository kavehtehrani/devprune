# devprune Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust TUI tool that scans filesystems for developer artifacts and lets users safely delete them with recovery support.

**Architecture:** Two-crate Cargo workspace (devprune-core lib + devprune binary). Core lib handles scanning, rules, and trash with zero terminal deps. Binary provides both TUI (ratatui) and headless CLI modes via a single executable. Parallel scanning via `ignore` crate with channel-based communication to the TUI.

**Tech Stack:** Rust, ratatui, crossterm, ignore, rayon, clap, serde, serde_json, toml, chrono, thiserror, uuid, bytesize, dirs

**Spec:** `docs/superpowers/specs/2026-03-22-devprune-design.md`

---

## Phase 1: Foundation & Core Types

### Task 1: Workspace Scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/devprune-core/Cargo.toml`
- Create: `crates/devprune-core/src/lib.rs`
- Create: `crates/devprune/Cargo.toml`
- Create: `crates/devprune/src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize git repo**

```bash
cd /home/kaveh/Documents/code/devprune
git init
```

- [ ] **Step 2: Create workspace root Cargo.toml**

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
rust-version = "1.85"

[workspace.dependencies]
devprune-core = { path = "crates/devprune-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
bytesize = "2"
dirs = "6"
log = "0.4"
ignore = "0.4"
rayon = "1"
tempfile = "3"
```

- [ ] **Step 3: Create devprune-core/Cargo.toml**

```toml
[package]
name = "devprune-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
bytesize = { workspace = true }
dirs = { workspace = true }
log = { workspace = true }
ignore = { workspace = true }
rayon = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 4: Create devprune/Cargo.toml**

```toml
[package]
name = "devprune"
version.workspace = true
edition.workspace = true

[[bin]]
name = "devprune"
path = "src/main.rs"

[dependencies]
devprune-core = { workspace = true }
ratatui = "0.30"
crossterm = "0.29"
clap = { version = "4", features = ["derive"] }
log = { workspace = true }
env_logger = "0.11"
serde_json = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 5: Create minimal source files**

`crates/devprune-core/src/lib.rs`:
```rust
pub mod config;
pub mod constants;
pub mod error;
pub mod rules;
pub mod scanner;
pub mod trash;
pub mod types;
```

`crates/devprune/src/main.rs`:
```rust
fn main() {
    println!("devprune v0.1.0");
}
```

- [ ] **Step 6: Create .gitignore**

```
/target
.DS_Store
*.swp
*.swo
.superpowers/
```

- [ ] **Step 7: Verify workspace compiles**

Run: `cargo build`
Expected: Successful compilation (will have warnings about empty modules, that's fine)

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: initialize cargo workspace with devprune-core and devprune crates"
```

---

### Task 2: Constants and Configuration

**Files:**
- Create: `crates/devprune-core/src/constants.rs`
- Create: `crates/devprune-core/src/config.rs`

- [ ] **Step 1: Write test for constants**

Create `crates/devprune-core/src/constants.rs`:
```rust
pub const APP_NAME: &str = "devprune";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_TICK_RATE_MS: u64 = 100;
pub const DEFAULT_AUTO_PURGE_DAYS: u64 = 30;
pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const METADATA_FILENAME: &str = "metadata.json";
pub const CONTENT_DIRNAME: &str = "content";
pub const TRASH_DIRNAME: &str = "trash";
pub const ITEMS_DIRNAME: &str = "items";

/// Paths that should always be skipped during scanning on Linux.
pub const LINUX_SKIP_PATHS: &[&str] = &["/proc", "/sys", "/dev", "/run", "/tmp", "/snap"];

/// Paths that should always be skipped during scanning on macOS.
pub const MACOS_SKIP_PATHS: &[&str] = &["/System", "/Library", "/Volumes", "/private/var/vm"];

/// Directory names that should always be skipped (never descended into, never matched).
pub const ALWAYS_SKIP_DIRS: &[&str] = &[".git"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_not_empty() {
        assert!(!APP_NAME.is_empty());
        assert!(!LINUX_SKIP_PATHS.is_empty());
        assert!(!MACOS_SKIP_PATHS.is_empty());
        assert!(!ALWAYS_SKIP_DIRS.is_empty());
    }
}
```

- [ ] **Step 2: Write config module with tests**

Create `crates/devprune-core/src/config.rs`:
```rust
use std::path::PathBuf;

use crate::constants;

/// Resolved application paths using platform-appropriate directories.
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
    /// Resolve all application paths using the `dirs` crate.
    /// Returns None if platform data/config dirs cannot be determined.
    pub fn resolve() -> Option<Self> {
        let data_dir = dirs::data_dir()?.join(constants::APP_NAME);
        let config_dir = dirs::config_dir()?.join(constants::APP_NAME);
        let trash_dir = data_dir.join(constants::TRASH_DIRNAME);
        let manifest_path = trash_dir.join(constants::MANIFEST_FILENAME);
        let items_dir = trash_dir.join(constants::ITEMS_DIRNAME);
        let log_path = data_dir.join(format!("{}.log", constants::APP_NAME));

        Some(Self {
            data_dir,
            config_dir,
            trash_dir,
            manifest_path,
            items_dir,
            log_path,
        })
    }

    /// Create an AppPaths rooted at a custom base directory (for testing).
    pub fn with_base(base: PathBuf) -> Self {
        let data_dir = base.join("data");
        let config_dir = base.join("config");
        let trash_dir = data_dir.join(constants::TRASH_DIRNAME);
        let manifest_path = trash_dir.join(constants::MANIFEST_FILENAME);
        let items_dir = trash_dir.join(constants::ITEMS_DIRNAME);
        let log_path = data_dir.join(format!("{}.log", constants::APP_NAME));

        Self {
            data_dir,
            config_dir,
            trash_dir,
            manifest_path,
            items_dir,
            log_path,
        }
    }

    /// Returns the list of paths to skip during scanning for the current platform.
    pub fn skip_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();

        #[cfg(target_os = "linux")]
        {
            paths.extend(constants::LINUX_SKIP_PATHS.iter().map(PathBuf::from));
        }

        #[cfg(target_os = "macos")]
        {
            paths.extend(constants::MACOS_SKIP_PATHS.iter().map(PathBuf::from));
        }

        // Always skip the trash directory itself
        paths.push(self.trash_dir.clone());

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_base_creates_correct_structure() {
        let paths = AppPaths::with_base(PathBuf::from("/tmp/test-devprune"));
        assert_eq!(paths.data_dir, PathBuf::from("/tmp/test-devprune/data"));
        assert_eq!(paths.trash_dir, PathBuf::from("/tmp/test-devprune/data/trash"));
        assert_eq!(paths.items_dir, PathBuf::from("/tmp/test-devprune/data/trash/items"));
        assert_eq!(paths.manifest_path, PathBuf::from("/tmp/test-devprune/data/trash/manifest.json"));
        assert_eq!(paths.config_dir, PathBuf::from("/tmp/test-devprune/config"));
    }

    #[test]
    fn skip_paths_includes_trash_dir() {
        let paths = AppPaths::with_base(PathBuf::from("/tmp/test-devprune"));
        let skips = paths.skip_paths();
        assert!(skips.contains(&paths.trash_dir));
    }

    #[test]
    fn resolve_returns_some_on_normal_system() {
        // This should work on any Linux/macOS dev machine
        assert!(AppPaths::resolve().is_some());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p devprune-core`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/devprune-core/src/constants.rs crates/devprune-core/src/config.rs
git commit -m "feat: add constants and config modules with platform path resolution"
```

---

### Task 3: Error Types

**Files:**
- Create: `crates/devprune-core/src/error.rs`

- [ ] **Step 1: Write error types**

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DevpruneError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Trash operation failed: {message}")]
    Trash {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Restore conflict: {path} already exists")]
    RestoreConflict { path: PathBuf },

    #[error("Insufficient disk space: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("Cross-device move from {source} to {destination}")]
    CrossDeviceMove { source: PathBuf, destination: PathBuf },

    #[error("Rule configuration error: {message}")]
    RuleConfig { message: String },

    #[error("Manifest corrupted: {message}")]
    ManifestCorrupted { message: String },
}

/// Result type alias for devprune operations.
pub type Result<T> = std::result::Result<T, DevpruneError>;

/// Non-fatal scan error collected during scanning.
#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = DevpruneError::PermissionDenied {
            path: PathBuf::from("/restricted"),
        };
        assert!(err.to_string().contains("/restricted"));

        let err = DevpruneError::RestoreConflict {
            path: PathBuf::from("/home/user/project/node_modules"),
        };
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn scan_error_display() {
        let err = ScanError {
            path: PathBuf::from("/some/path"),
            message: "permission denied".to_string(),
        };
        assert_eq!(err.to_string(), "/some/path: permission denied");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune-core`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune-core/src/error.rs
git commit -m "feat: add error types with thiserror"
```

---

### Task 4: Core Types (ArtifactInfo, ScanEvent, etc.)

**Files:**
- Create: `crates/devprune-core/src/types.rs`

- [ ] **Step 1: Write core types with tests**

```rust
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ScanError;
use crate::rules::types::{Category, SafetyLevel};

/// Information about a discovered developer artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub id: Uuid,
    pub path: PathBuf,
    pub rule_id: String,
    pub rule_name: String,
    pub category: Category,
    pub safety: SafetyLevel,
    /// None while size is still being computed asynchronously.
    pub size: Option<u64>,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_directory: bool,
}

/// Events sent from scanner threads to the UI/collector.
#[derive(Debug)]
pub enum ScanEvent {
    /// A new artifact was discovered (size may be pending).
    Found(ArtifactInfo),
    /// Async size calculation completed for an artifact.
    SizeUpdate { id: Uuid, size: u64 },
    /// Periodic progress update.
    Progress(ProgressInfo),
    /// Non-fatal error encountered during scan.
    Error(ScanError),
    /// Scan is complete.
    Complete(ScanSummary),
}

/// Progress information sent periodically during scanning.
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub dirs_visited: u64,
    pub artifacts_found: u64,
    pub total_size_found: u64,
    pub elapsed: Duration,
}

/// Summary produced when a scan finishes.
#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub total_artifacts: usize,
    pub total_size: u64,
    pub duration: Duration,
    pub errors: Vec<ScanError>,
    pub dirs_visited: u64,
}

/// Configuration for a scan operation.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub paths: Vec<PathBuf>,
    pub threads: Option<usize>,
    pub max_depth: Option<usize>,
    pub cross_device: bool,
    pub min_size: Option<u64>,
    pub categories: Option<Vec<Category>>,
    pub safety_filter: Option<SafetyLevel>,
    pub exclude_patterns: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            threads: None,
            max_depth: None,
            cross_device: false,
            min_size: None,
            categories: None,
            safety_filter: None,
            exclude_patterns: Vec::new(),
        }
    }
}

/// Summary of what the user has selected for deletion.
#[derive(Debug, Clone, Default)]
pub struct SelectionSummary {
    pub count: usize,
    pub total_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_config_defaults() {
        let config = ScanConfig::default();
        assert_eq!(config.paths, vec![PathBuf::from(".")]);
        assert!(config.threads.is_none());
        assert!(!config.cross_device);
        assert!(config.exclude_patterns.is_empty());
    }

    #[test]
    fn artifact_info_serialization_roundtrip() {
        let artifact = ArtifactInfo {
            id: Uuid::new_v4(),
            path: PathBuf::from("/home/user/project/node_modules"),
            rule_id: "node_modules".to_string(),
            rule_name: "Node.js Dependencies".to_string(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            size: Some(356_515_840),
            last_modified: Some(Utc::now()),
            is_directory: true,
        };

        let json = serde_json::to_string(&artifact).unwrap();
        let deserialized: ArtifactInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, artifact.path);
        assert_eq!(deserialized.rule_id, artifact.rule_id);
        assert_eq!(deserialized.size, artifact.size);
    }

    #[test]
    fn selection_summary_default_is_empty() {
        let summary = SelectionSummary::default();
        assert_eq!(summary.count, 0);
        assert_eq!(summary.total_size, 0);
    }
}
```

Note: This depends on `rules::types` which we'll create next. Create a placeholder first.

- [ ] **Step 2: Create rule types (needed by types.rs)**

Create `crates/devprune-core/src/rules/mod.rs`:
```rust
pub mod types;
pub mod catalog;
pub mod parser;
```

Create `crates/devprune-core/src/rules/types.rs` (full implementation in Task 5).

Create placeholder `crates/devprune-core/src/rules/catalog.rs`:
```rust
// Populated in Task 6
```

Create placeholder `crates/devprune-core/src/rules/parser.rs`:
```rust
// Populated in Phase 4
```

- [ ] **Step 3: Create remaining module stubs**

Create `crates/devprune-core/src/scanner/mod.rs`:
```rust
pub mod walker;
pub mod filter;
```

Create placeholder `crates/devprune-core/src/scanner/walker.rs`:
```rust
// Populated in Task 7
```

Create placeholder `crates/devprune-core/src/scanner/filter.rs`:
```rust
// Populated in Task 7
```

Create `crates/devprune-core/src/trash/mod.rs`:
```rust
pub mod metadata;
pub mod storage;
```

Create placeholder `crates/devprune-core/src/trash/metadata.rs`:
```rust
// Populated in Phase 2
```

Create placeholder `crates/devprune-core/src/trash/storage.rs`:
```rust
// Populated in Phase 2
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p devprune-core`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/devprune-core/src/
git commit -m "feat: add core types, scan events, and module structure"
```

---

### Task 5: Rule Types

**Files:**
- Create: `crates/devprune-core/src/rules/types.rs`

- [ ] **Step 1: Write rule types with tests**

```rust
use serde::{Deserialize, Serialize};

/// Category grouping for developer artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Dependencies,
    BuildOutput,
    Cache,
    VirtualEnv,
    IdeArtifact,
    Coverage,
    Logs,
    CompiledGenerated,
    Misc,
}

impl Category {
    /// Human-readable display name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dependencies => "Dependencies",
            Self::BuildOutput => "Build Outputs",
            Self::Cache => "Caches",
            Self::VirtualEnv => "Virtual Environments",
            Self::IdeArtifact => "IDE Artifacts",
            Self::Coverage => "Coverage & Test",
            Self::Logs => "Logs",
            Self::CompiledGenerated => "Compiled/Generated",
            Self::Misc => "Miscellaneous",
        }
    }

    /// All category variants in display order.
    pub fn all() -> &'static [Category] {
        &[
            Self::Dependencies,
            Self::BuildOutput,
            Self::Cache,
            Self::VirtualEnv,
            Self::IdeArtifact,
            Self::Coverage,
            Self::Logs,
            Self::CompiledGenerated,
            Self::Misc,
        ]
    }
}

/// How confident we are that deleting this artifact is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SafetyLevel {
    /// Fully regenerable from source (node_modules, target/, __pycache__).
    Safe,
    /// Likely regenerable but may contain local state (.venv).
    Cautious,
    /// May contain user config or state (.vscode/, .idea/).
    Risky,
}

impl SafetyLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Cautious => "Cautious",
            Self::Risky => "Risky",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Safe => "Fully regenerable from source",
            Self::Cautious => "Likely regenerable but may have local state",
            Self::Risky => "May contain user config or state",
        }
    }
}

/// How to match filesystem entries against a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchCondition {
    /// Match a directory with this exact name.
    DirName(String),
    /// Match a directory whose name matches a glob (e.g., "cmake-build-*").
    DirGlob(String),
    /// Match a file with this exact name (e.g., ".eslintcache").
    FileName(String),
    /// Match files whose name matches a glob (e.g., "npm-debug.log*").
    FileGlob(String),
    /// Match files with this extension (e.g., "pyc").
    FileExtension(String),
}

/// A rule defining a type of developer artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier (bare name, no slashes). E.g., "node_modules".
    pub id: String,
    /// Human-readable name. E.g., "Node.js Dependencies".
    pub name: String,
    pub category: Category,
    pub safety: SafetyLevel,
    pub match_condition: MatchCondition,
    /// Sibling files that must exist in the parent directory for the rule to match.
    /// Empty means match unconditionally.
    pub context_markers: Vec<String>,
    /// Description shown in UI.
    pub description: String,
    /// Whether this rule is enabled by default.
    pub enabled: bool,
}

impl Rule {
    /// Returns true if this rule requires context markers to match.
    pub fn needs_context(&self) -> bool {
        !self.context_markers.is_empty()
    }

    /// Returns true if this rule matches directories (vs files).
    pub fn matches_directories(&self) -> bool {
        matches!(
            self.match_condition,
            MatchCondition::DirName(_) | MatchCondition::DirGlob(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_display_names() {
        assert_eq!(Category::Dependencies.display_name(), "Dependencies");
        assert_eq!(Category::BuildOutput.display_name(), "Build Outputs");
        assert_eq!(Category::VirtualEnv.display_name(), "Virtual Environments");
    }

    #[test]
    fn category_all_returns_all_variants() {
        let all = Category::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&Category::Dependencies));
        assert!(all.contains(&Category::Misc));
    }

    #[test]
    fn safety_level_ordering() {
        // Safe < Cautious < Risky
        assert!(SafetyLevel::Safe < SafetyLevel::Cautious);
        assert!(SafetyLevel::Cautious < SafetyLevel::Risky);
    }

    #[test]
    fn rule_needs_context() {
        let rule_with_context = Rule {
            id: "target".to_string(),
            name: "Rust target".to_string(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("target".to_string()),
            context_markers: vec!["Cargo.toml".to_string()],
            description: "Rust build output".to_string(),
            enabled: true,
        };
        assert!(rule_with_context.needs_context());

        let rule_without_context = Rule {
            id: "node_modules".to_string(),
            name: "Node modules".to_string(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("node_modules".to_string()),
            context_markers: vec![],
            description: "Node.js dependencies".to_string(),
            enabled: true,
        };
        assert!(!rule_without_context.needs_context());
    }

    #[test]
    fn rule_matches_directories() {
        let dir_rule = Rule {
            id: "test".to_string(),
            name: "Test".to_string(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("test".to_string()),
            context_markers: vec![],
            description: "test".to_string(),
            enabled: true,
        };
        assert!(dir_rule.matches_directories());

        let file_rule = Rule {
            id: "test".to_string(),
            name: "Test".to_string(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileName(".eslintcache".to_string()),
            context_markers: vec![],
            description: "test".to_string(),
            enabled: true,
        };
        assert!(!file_rule.matches_directories());
    }

    #[test]
    fn rule_serialization_roundtrip() {
        let rule = Rule {
            id: "node_modules".to_string(),
            name: "Node.js Dependencies".to_string(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("node_modules".to_string()),
            context_markers: vec![],
            description: "Node.js dependency directory".to_string(),
            enabled: true,
        };

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "node_modules");
        assert_eq!(deserialized.category, Category::Dependencies);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune-core -- rules::types`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune-core/src/rules/types.rs
git commit -m "feat: add rule types with Category, SafetyLevel, MatchCondition, and Rule"
```

---

### Task 6: Built-in Rule Catalog

**Files:**
- Create: `crates/devprune-core/src/rules/catalog.rs`

- [ ] **Step 1: Write the catalog with tests**

The catalog is a function returning `Vec<Rule>` with all built-in rules from the spec. This is a large file but straightforward -- each rule is a struct literal.

```rust
use super::types::*;

/// Returns the complete built-in rule catalog.
/// Each rule is compiled into the binary and used as the default set.
pub fn builtin_rules() -> Vec<Rule> {
    vec![
        // === Dependencies ===
        Rule {
            id: "node_modules".into(),
            name: "Node.js Dependencies".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("node_modules".into()),
            context_markers: vec![],
            description: "Node.js dependency directory. Regenerate with `npm install` or `yarn install`.".into(),
            enabled: true,
        },
        Rule {
            id: "bower_components".into(),
            name: "Bower Components".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("bower_components".into()),
            context_markers: vec![],
            description: "Bower dependency directory. Regenerate with `bower install`.".into(),
            enabled: true,
        },
        Rule {
            id: "vendor_php".into(),
            name: "PHP Composer Vendor".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("vendor".into()),
            context_markers: vec!["composer.json".into()],
            description: "PHP Composer dependencies. Regenerate with `composer install`.".into(),
            enabled: true,
        },
        Rule {
            id: "vendor_go".into(),
            name: "Go Vendor".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("vendor".into()),
            context_markers: vec!["go.mod".into()],
            description: "Go vendored dependencies. Regenerate with `go mod vendor`.".into(),
            enabled: true,
        },
        Rule {
            id: "bundle".into(),
            name: "Ruby Bundler".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".bundle".into()),
            context_markers: vec![],
            description: "Ruby Bundler cache. Regenerate with `bundle install`.".into(),
            enabled: true,
        },
        Rule {
            id: "pods".into(),
            name: "CocoaPods".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("Pods".into()),
            context_markers: vec!["Podfile".into()],
            description: "CocoaPods dependencies. Regenerate with `pod install`.".into(),
            enabled: true,
        },
        Rule {
            id: "pub_cache".into(),
            name: "Dart/Flutter Pub Cache".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".pub-cache".into()),
            context_markers: vec![],
            description: "Dart/Flutter package cache.".into(),
            enabled: true,
        },
        Rule {
            id: "elm_stuff".into(),
            name: "Elm Packages".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("elm-stuff".into()),
            context_markers: vec![],
            description: "Elm package artifacts. Regenerate with `elm make`.".into(),
            enabled: true,
        },
        Rule {
            id: "jspm_packages".into(),
            name: "jspm Packages".into(),
            category: Category::Dependencies,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("jspm_packages".into()),
            context_markers: vec![],
            description: "jspm package directory.".into(),
            enabled: true,
        },

        // === Build Outputs ===
        Rule {
            id: "target_rust".into(),
            name: "Rust Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("target".into()),
            context_markers: vec!["Cargo.toml".into()],
            description: "Rust/Cargo build output. Regenerate with `cargo build`.".into(),
            enabled: true,
        },
        Rule {
            id: "target_maven".into(),
            name: "Maven Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("target".into()),
            context_markers: vec!["pom.xml".into()],
            description: "Maven build output. Regenerate with `mvn compile`.".into(),
            enabled: true,
        },
        Rule {
            id: "target_sbt".into(),
            name: "sbt Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("target".into()),
            context_markers: vec!["build.sbt".into()],
            description: "Scala sbt build output. Regenerate with `sbt compile`.".into(),
            enabled: true,
        },
        Rule {
            id: "build_gradle".into(),
            name: "Gradle Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("build".into()),
            context_markers: vec!["build.gradle".into()],
            description: "Gradle build output. Regenerate with `gradle build`.".into(),
            enabled: true,
        },
        Rule {
            id: "build_cmake".into(),
            name: "CMake Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("build".into()),
            context_markers: vec!["CMakeLists.txt".into()],
            description: "CMake build directory. Regenerate with `cmake --build`.".into(),
            enabled: true,
        },
        Rule {
            id: "build_python".into(),
            name: "Python Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("build".into()),
            context_markers: vec!["setup.py".into()],
            description: "Python build output. Regenerate with `python -m build`.".into(),
            enabled: true,
        },
        Rule {
            id: "build_js".into(),
            name: "JS Build Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("build".into()),
            context_markers: vec!["package.json".into()],
            description: "JavaScript build output. Regenerate with your build script.".into(),
            enabled: true,
        },
        Rule {
            id: "dist_js".into(),
            name: "JS Distribution".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("dist".into()),
            context_markers: vec!["package.json".into()],
            description: "JavaScript bundler output. Regenerate with your build script.".into(),
            enabled: true,
        },
        Rule {
            id: "dist_python".into(),
            name: "Python Distribution".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("dist".into()),
            context_markers: vec!["setup.py".into(), "pyproject.toml".into()],
            description: "Python distribution output. Regenerate with `python -m build`.".into(),
            enabled: true,
        },
        Rule {
            id: "out_gradle".into(),
            name: "Gradle Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName("out".into()),
            context_markers: vec!["build.gradle".into()],
            description: "Gradle output directory.".into(),
            enabled: true,
        },
        Rule {
            id: "out_java".into(),
            name: "Java Output".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName("out".into()),
            context_markers: vec![".classpath".into()],
            description: "Java compiled output directory.".into(),
            enabled: true,
        },
        Rule {
            id: "elixir_build".into(),
            name: "Elixir Mix Build".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("_build".into()),
            context_markers: vec!["mix.exs".into()],
            description: "Elixir Mix build output. Regenerate with `mix compile`.".into(),
            enabled: true,
        },
        Rule {
            id: "swift_build".into(),
            name: "Swift Build".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".build".into()),
            context_markers: vec!["Package.swift".into()],
            description: "Swift Package Manager build output. Regenerate with `swift build`.".into(),
            enabled: true,
        },
        Rule {
            id: "cmake_build_dir".into(),
            name: "CLion CMake Build".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirGlob("cmake-build-*".into()),
            context_markers: vec![],
            description: "CLion CMake build directories.".into(),
            enabled: true,
        },

        // === Caches ===
        Rule {
            id: "pycache".into(),
            name: "Python Bytecode Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("__pycache__".into()),
            context_markers: vec![],
            description: "Python bytecode cache. Automatically regenerated.".into(),
            enabled: true,
        },
        Rule {
            id: "pytest_cache".into(),
            name: "pytest Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".pytest_cache".into()),
            context_markers: vec![],
            description: "pytest cache directory.".into(),
            enabled: true,
        },
        Rule {
            id: "mypy_cache".into(),
            name: "mypy Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".mypy_cache".into()),
            context_markers: vec![],
            description: "mypy type checker cache.".into(),
            enabled: true,
        },
        Rule {
            id: "ruff_cache".into(),
            name: "Ruff Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".ruff_cache".into()),
            context_markers: vec![],
            description: "Ruff linter cache.".into(),
            enabled: true,
        },
        Rule {
            id: "parcel_cache".into(),
            name: "Parcel Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".parcel-cache".into()),
            context_markers: vec![],
            description: "Parcel bundler cache.".into(),
            enabled: true,
        },
        Rule {
            id: "turbo".into(),
            name: "Turborepo Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".turbo".into()),
            context_markers: vec![],
            description: "Turborepo local cache.".into(),
            enabled: true,
        },
        Rule {
            id: "next_cache".into(),
            name: "Next.js Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".next".into()),
            context_markers: vec!["package.json".into()],
            description: "Next.js build cache. Regenerate with `next build`.".into(),
            enabled: true,
        },
        Rule {
            id: "nuxt_cache".into(),
            name: "Nuxt.js Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".nuxt".into()),
            context_markers: vec!["package.json".into()],
            description: "Nuxt.js build cache.".into(),
            enabled: true,
        },
        Rule {
            id: "angular_cache".into(),
            name: "Angular CLI Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".angular".into()),
            context_markers: vec![],
            description: "Angular CLI cache directory.".into(),
            enabled: true,
        },
        Rule {
            id: "svelte_kit".into(),
            name: "SvelteKit Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".svelte-kit".into()),
            context_markers: vec![],
            description: "SvelteKit build cache.".into(),
            enabled: true,
        },
        Rule {
            id: "gradle_caches".into(),
            name: "Gradle Caches".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".gradle".into()),
            context_markers: vec![],
            description: "Gradle cache directory.".into(),
            enabled: true,
        },
        Rule {
            id: "sass_cache".into(),
            name: "Sass Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".sass-cache".into()),
            context_markers: vec![],
            description: "Sass compiler cache.".into(),
            enabled: true,
        },
        Rule {
            id: "eslintcache".into(),
            name: "ESLint Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileName(".eslintcache".into()),
            context_markers: vec![],
            description: "ESLint cache file.".into(),
            enabled: true,
        },
        Rule {
            id: "stylelintcache".into(),
            name: "Stylelint Cache".into(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileName(".stylelintcache".into()),
            context_markers: vec![],
            description: "Stylelint cache file.".into(),
            enabled: true,
        },

        // === Virtual Environments ===
        Rule {
            id: "venv_dot".into(),
            name: "Python .venv".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName(".venv".into()),
            context_markers: vec!["requirements.txt".into(), "pyproject.toml".into(), "setup.py".into()],
            description: "Python virtual environment. Regenerate with `python -m venv .venv && pip install -r requirements.txt`.".into(),
            enabled: true,
        },
        Rule {
            id: "venv".into(),
            name: "Python venv".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName("venv".into()),
            context_markers: vec!["requirements.txt".into(), "pyproject.toml".into(), "setup.py".into()],
            description: "Python virtual environment. Regenerate with `python -m venv venv && pip install -r requirements.txt`.".into(),
            enabled: true,
        },
        Rule {
            id: "env_python".into(),
            name: "Python env".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Risky,
            match_condition: MatchCondition::DirName("env".into()),
            context_markers: vec!["requirements.txt".into(), "pyproject.toml".into(), "setup.py".into()],
            description: "Possibly a Python virtual environment. 'env' is a common generic directory name -- verify before deleting.".into(),
            enabled: true,
        },
        Rule {
            id: "conda".into(),
            name: "Conda Environment".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName(".conda".into()),
            context_markers: vec![],
            description: "Conda environment directory.".into(),
            enabled: true,
        },
        Rule {
            id: "tox".into(),
            name: "Tox Environments".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".tox".into()),
            context_markers: vec!["tox.ini".into()],
            description: "Tox test environments. Regenerate with `tox`.".into(),
            enabled: true,
        },
        Rule {
            id: "nox".into(),
            name: "Nox Environments".into(),
            category: Category::VirtualEnv,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".nox".into()),
            context_markers: vec!["noxfile.py".into()],
            description: "Nox test environments. Regenerate with `nox`.".into(),
            enabled: true,
        },

        // === IDE Artifacts ===
        Rule {
            id: "idea".into(),
            name: "JetBrains IDE".into(),
            category: Category::IdeArtifact,
            safety: SafetyLevel::Risky,
            match_condition: MatchCondition::DirName(".idea".into()),
            context_markers: vec![],
            description: "JetBrains IDE configuration. May contain project-specific settings.".into(),
            enabled: true,
        },
        Rule {
            id: "vscode".into(),
            name: "VS Code".into(),
            category: Category::IdeArtifact,
            safety: SafetyLevel::Risky,
            match_condition: MatchCondition::DirName(".vscode".into()),
            context_markers: vec![],
            description: "VS Code workspace settings. May contain project-specific configuration.".into(),
            enabled: true,
        },
        Rule {
            id: "vim_swap".into(),
            name: "Vim Swap Files".into(),
            category: Category::IdeArtifact,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("*.swp".into()),
            context_markers: vec![],
            description: "Vim swap files.".into(),
            enabled: true,
        },
        Rule {
            id: "vim_swap_old".into(),
            name: "Vim Old Swap Files".into(),
            category: Category::IdeArtifact,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("*.swo".into()),
            context_markers: vec![],
            description: "Vim old swap files.".into(),
            enabled: true,
        },
        Rule {
            id: "vs".into(),
            name: "Visual Studio".into(),
            category: Category::IdeArtifact,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName(".vs".into()),
            context_markers: vec![],
            description: "Visual Studio local settings directory.".into(),
            enabled: true,
        },

        // === Coverage & Test ===
        Rule {
            id: "coverage".into(),
            name: "Coverage Reports".into(),
            category: Category::Coverage,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("coverage".into()),
            context_markers: vec!["package.json".into(), "pytest.ini".into(), "setup.cfg".into(), ".coveragerc".into()],
            description: "Test coverage report directory.".into(),
            enabled: true,
        },
        Rule {
            id: "htmlcov".into(),
            name: "Python HTML Coverage".into(),
            category: Category::Coverage,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("htmlcov".into()),
            context_markers: vec![],
            description: "Python coverage.py HTML report.".into(),
            enabled: true,
        },
        Rule {
            id: "nyc_output".into(),
            name: "NYC/Istanbul Output".into(),
            category: Category::Coverage,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".nyc_output".into()),
            context_markers: vec![],
            description: "NYC/Istanbul code coverage output.".into(),
            enabled: true,
        },
        Rule {
            id: "coverage_file".into(),
            name: "Python .coverage File".into(),
            category: Category::Coverage,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileName(".coverage".into()),
            context_markers: vec![],
            description: "Python coverage.py data file.".into(),
            enabled: true,
        },

        // === Logs ===
        Rule {
            id: "npm_debug_log".into(),
            name: "npm Debug Log".into(),
            category: Category::Logs,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("npm-debug.log*".into()),
            context_markers: vec![],
            description: "npm debug log file.".into(),
            enabled: true,
        },
        Rule {
            id: "yarn_debug_log".into(),
            name: "Yarn Debug Log".into(),
            category: Category::Logs,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("yarn-debug.log*".into()),
            context_markers: vec![],
            description: "Yarn debug log file.".into(),
            enabled: true,
        },
        Rule {
            id: "yarn_error_log".into(),
            name: "Yarn Error Log".into(),
            category: Category::Logs,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("yarn-error.log*".into()),
            context_markers: vec![],
            description: "Yarn error log file.".into(),
            enabled: true,
        },

        // === Compiled/Generated ===
        Rule {
            id: "eggs".into(),
            name: "Python Eggs".into(),
            category: Category::CompiledGenerated,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".eggs".into()),
            context_markers: vec![],
            description: "Python egg build artifacts.".into(),
            enabled: true,
        },
        Rule {
            id: "egg_info".into(),
            name: "Python Egg Info".into(),
            category: Category::CompiledGenerated,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirGlob("*.egg-info".into()),
            context_markers: vec![],
            description: "Python egg metadata directory.".into(),
            enabled: true,
        },
        Rule {
            id: "tsbuildinfo".into(),
            name: "TypeScript Build Info".into(),
            category: Category::CompiledGenerated,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::FileGlob("*.tsbuildinfo".into()),
            context_markers: vec![],
            description: "TypeScript incremental build info file.".into(),
            enabled: true,
        },
        Rule {
            id: "terraform".into(),
            name: "Terraform Providers".into(),
            category: Category::CompiledGenerated,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName(".terraform".into()),
            context_markers: vec![],
            description: "Terraform provider plugins and modules. Regenerate with `terraform init`.".into(),
            enabled: true,
        },
        Rule {
            id: "serverless".into(),
            name: "Serverless Framework".into(),
            category: Category::CompiledGenerated,
            safety: SafetyLevel::Cautious,
            match_condition: MatchCondition::DirName(".serverless".into()),
            context_markers: vec![],
            description: "Serverless Framework build artifacts.".into(),
            enabled: true,
        },

        // === Misc ===
        Rule {
            id: "docusaurus".into(),
            name: "Docusaurus Cache".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".docusaurus".into()),
            context_markers: vec![],
            description: "Docusaurus build cache.".into(),
            enabled: true,
        },
        Rule {
            id: "expo".into(),
            name: "Expo Cache".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".expo".into()),
            context_markers: vec![],
            description: "Expo/React Native cache.".into(),
            enabled: true,
        },
        Rule {
            id: "meteor_local".into(),
            name: "Meteor Local".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("local".into()),
            context_markers: vec![".meteor".into()],
            description: "Meteor local build artifacts.".into(),
            enabled: true,
        },
        Rule {
            id: "stack_work".into(),
            name: "Haskell Stack".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".stack-work".into()),
            context_markers: vec![],
            description: "Haskell Stack build artifacts. Regenerate with `stack build`.".into(),
            enabled: true,
        },
        Rule {
            id: "cabal_sandbox".into(),
            name: "Haskell Cabal Sandbox".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".cabal-sandbox".into()),
            context_markers: vec![],
            description: "Haskell Cabal sandbox directory.".into(),
            enabled: true,
        },
        Rule {
            id: "cmake_deps".into(),
            name: "CMake FetchContent".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("_deps".into()),
            context_markers: vec![],
            description: "CMake FetchContent downloaded dependencies.".into(),
            enabled: true,
        },
        Rule {
            id: "zig_cache".into(),
            name: "Zig Cache".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("zig-cache".into()),
            context_markers: vec![],
            description: "Zig compiler cache.".into(),
            enabled: true,
        },
        Rule {
            id: "zig_out".into(),
            name: "Zig Output".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("zig-out".into()),
            context_markers: vec![],
            description: "Zig build output.".into(),
            enabled: true,
        },
        Rule {
            id: "zig_cache_dot".into(),
            name: "Zig Cache (dot)".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName(".zig-cache".into()),
            context_markers: vec![],
            description: "Zig compiler cache (dot-prefixed).".into(),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_not_empty() {
        let rules = builtin_rules();
        assert!(!rules.is_empty());
    }

    #[test]
    fn all_rule_ids_are_unique() {
        let rules = builtin_rules();
        let mut ids = HashSet::new();
        for rule in &rules {
            assert!(
                ids.insert(&rule.id),
                "Duplicate rule ID: {}",
                rule.id
            );
        }
    }

    #[test]
    fn all_rules_have_required_fields() {
        for rule in builtin_rules() {
            assert!(!rule.id.is_empty(), "Rule has empty id");
            assert!(!rule.name.is_empty(), "Rule {} has empty name", rule.id);
            assert!(!rule.description.is_empty(), "Rule {} has empty description", rule.id);
            // IDs should not contain slashes
            assert!(
                !rule.id.contains('/'),
                "Rule ID '{}' contains a slash",
                rule.id
            );
        }
    }

    #[test]
    fn all_categories_have_at_least_one_rule() {
        let rules = builtin_rules();
        for category in Category::all() {
            assert!(
                rules.iter().any(|r| r.category == *category),
                "No rules for category {:?}",
                category
            );
        }
    }

    #[test]
    fn context_marker_rules_have_markers() {
        let rules = builtin_rules();
        // Rules with generic names should have context markers
        let generic_names = ["build", "dist", "out", "vendor", "coverage", "env"];
        for rule in &rules {
            if let MatchCondition::DirName(ref name) = rule.match_condition {
                if generic_names.contains(&name.as_str()) {
                    assert!(
                        !rule.context_markers.is_empty(),
                        "Rule '{}' matches generic name '{}' but has no context markers",
                        rule.id,
                        name
                    );
                }
            }
        }
    }

    #[test]
    fn all_rules_are_enabled_by_default() {
        for rule in builtin_rules() {
            assert!(rule.enabled, "Rule '{}' is disabled by default", rule.id);
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune-core -- rules::catalog`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune-core/src/rules/catalog.rs
git commit -m "feat: add complete built-in rule catalog with 55+ detection rules"
```

---

## Phase 2: Scanner

### Task 7: Scanner Filter (Rule Matching Logic)

**Files:**
- Create: `crates/devprune-core/src/scanner/filter.rs`

- [ ] **Step 1: Write tests for filter logic**

```rust
use std::path::Path;

use crate::rules::types::{MatchCondition, Rule};

/// Check if a directory entry name matches a rule's match condition.
pub fn matches_entry_name(name: &str, condition: &MatchCondition) -> bool {
    match condition {
        MatchCondition::DirName(expected) => name == expected,
        MatchCondition::DirGlob(pattern) => {
            glob_match::glob_match(pattern, name)
        }
        MatchCondition::FileName(expected) => name == expected,
        MatchCondition::FileGlob(pattern) => {
            glob_match::glob_match(pattern, name)
        }
        MatchCondition::FileExtension(ext) => {
            Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == ext)
        }
    }
}

/// Check if context markers are satisfied: at least one marker file must exist
/// in the parent directory.
pub fn check_context_markers(parent: &Path, markers: &[String]) -> bool {
    if markers.is_empty() {
        return true;
    }
    markers.iter().any(|marker| parent.join(marker).exists())
}

/// Find the first matching rule for a directory entry.
/// `name` is the entry's file name, `parent` is the parent directory path.
/// `is_dir` indicates whether the entry is a directory.
pub fn find_matching_rule<'a>(
    name: &str,
    parent: &Path,
    is_dir: bool,
    rules: &'a [Rule],
) -> Option<&'a Rule> {
    rules.iter().find(|rule| {
        if !rule.enabled {
            return false;
        }

        // Check if the rule type matches the entry type
        let type_matches = if is_dir {
            rule.matches_directories()
        } else {
            !rule.matches_directories()
        };

        if !type_matches {
            return false;
        }

        // Check name match
        if !matches_entry_name(name, &rule.match_condition) {
            return false;
        }

        // Check context markers
        check_context_markers(parent, &rule.context_markers)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::types::*;
    use tempfile::TempDir;

    #[test]
    fn dir_name_match() {
        assert!(matches_entry_name("node_modules", &MatchCondition::DirName("node_modules".into())));
        assert!(!matches_entry_name("node_module", &MatchCondition::DirName("node_modules".into())));
    }

    #[test]
    fn dir_glob_match() {
        assert!(matches_entry_name("cmake-build-debug", &MatchCondition::DirGlob("cmake-build-*".into())));
        assert!(matches_entry_name("cmake-build-release", &MatchCondition::DirGlob("cmake-build-*".into())));
        assert!(!matches_entry_name("cmake-build", &MatchCondition::DirGlob("cmake-build-*".into())));
    }

    #[test]
    fn file_name_match() {
        assert!(matches_entry_name(".eslintcache", &MatchCondition::FileName(".eslintcache".into())));
        assert!(!matches_entry_name("eslintcache", &MatchCondition::FileName(".eslintcache".into())));
    }

    #[test]
    fn file_glob_match() {
        assert!(matches_entry_name("npm-debug.log", &MatchCondition::FileGlob("npm-debug.log*".into())));
        assert!(matches_entry_name("npm-debug.log.1", &MatchCondition::FileGlob("npm-debug.log*".into())));
    }

    #[test]
    fn file_extension_match() {
        assert!(matches_entry_name("file.pyc", &MatchCondition::FileExtension("pyc".into())));
        assert!(!matches_entry_name("file.py", &MatchCondition::FileExtension("pyc".into())));
    }

    #[test]
    fn context_markers_empty_always_matches() {
        assert!(check_context_markers(Path::new("/nonexistent"), &[]));
    }

    #[test]
    fn context_markers_with_existing_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();

        assert!(check_context_markers(
            tmp.path(),
            &["Cargo.toml".to_string()]
        ));
        assert!(!check_context_markers(
            tmp.path(),
            &["package.json".to_string()]
        ));
    }

    #[test]
    fn context_markers_any_match_suffices() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pom.xml"), "").unwrap();

        // Should match because pom.xml exists, even though Cargo.toml doesn't
        assert!(check_context_markers(
            tmp.path(),
            &["Cargo.toml".to_string(), "pom.xml".to_string()]
        ));
    }

    #[test]
    fn find_matching_rule_respects_dir_type() {
        let rules = vec![
            Rule {
                id: "node_modules".into(),
                name: "Node modules".into(),
                category: Category::Dependencies,
                safety: SafetyLevel::Safe,
                match_condition: MatchCondition::DirName("node_modules".into()),
                context_markers: vec![],
                description: "test".into(),
                enabled: true,
            },
            Rule {
                id: "eslintcache".into(),
                name: "ESLint cache".into(),
                category: Category::Cache,
                safety: SafetyLevel::Safe,
                match_condition: MatchCondition::FileName(".eslintcache".into()),
                context_markers: vec![],
                description: "test".into(),
                enabled: true,
            },
        ];

        // Directory should match dir rule
        let result = find_matching_rule("node_modules", Path::new("/tmp"), true, &rules);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "node_modules");

        // File should match file rule
        let result = find_matching_rule(".eslintcache", Path::new("/tmp"), false, &rules);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "eslintcache");

        // File named "node_modules" should NOT match dir rule
        let result = find_matching_rule("node_modules", Path::new("/tmp"), false, &rules);
        assert!(result.is_none());
    }

    #[test]
    fn find_matching_rule_respects_context() {
        let tmp = TempDir::new().unwrap();
        // No Cargo.toml present

        let rules = vec![Rule {
            id: "target_rust".into(),
            name: "Rust target".into(),
            category: Category::BuildOutput,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("target".into()),
            context_markers: vec!["Cargo.toml".into()],
            description: "test".into(),
            enabled: true,
        }];

        // Should not match without Cargo.toml
        let result = find_matching_rule("target", tmp.path(), true, &rules);
        assert!(result.is_none());

        // Create Cargo.toml
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let result = find_matching_rule("target", tmp.path(), true, &rules);
        assert!(result.is_some());
    }

    #[test]
    fn find_matching_rule_skips_disabled() {
        let rules = vec![Rule {
            id: "test".into(),
            name: "Test".into(),
            category: Category::Misc,
            safety: SafetyLevel::Safe,
            match_condition: MatchCondition::DirName("test_dir".into()),
            context_markers: vec![],
            description: "test".into(),
            enabled: false,
        }];

        let result = find_matching_rule("test_dir", Path::new("/tmp"), true, &rules);
        assert!(result.is_none());
    }
}
```

Note: We need to add `glob_match` crate to devprune-core's dependencies.

- [ ] **Step 2: Add glob_match dependency**

Add to `crates/devprune-core/Cargo.toml` under `[dependencies]`:
```toml
glob-match = "0.2"
```

And add to workspace `Cargo.toml` under `[workspace.dependencies]`:
```toml
glob-match = "0.2"
```

Update `crates/devprune-core/Cargo.toml` to use:
```toml
glob-match = { workspace = true }
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p devprune-core -- scanner::filter`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/devprune-core/src/scanner/filter.rs Cargo.toml crates/devprune-core/Cargo.toml
git commit -m "feat: add scanner filter with rule matching, context markers, and glob support"
```

---

### Task 8: Parallel Walker

**Files:**
- Create: `crates/devprune-core/src/scanner/walker.rs`
- Modify: `crates/devprune-core/src/scanner/mod.rs`

- [ ] **Step 1: Write the ScanCoordinator**

`crates/devprune-core/src/scanner/mod.rs`:
```rust
pub mod filter;
pub mod walker;

pub use walker::ScanCoordinator;
```

`crates/devprune-core/src/scanner/walker.rs`:
```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use ignore::WalkBuilder;
use uuid::Uuid;

use crate::config::AppPaths;
use crate::constants;
use crate::error::ScanError;
use crate::rules::types::Rule;
use crate::scanner::filter;
use crate::types::*;

/// Coordinates a parallel filesystem scan.
pub struct ScanCoordinator {
    config: ScanConfig,
    rules: Vec<Rule>,
    app_paths: AppPaths,
}

impl ScanCoordinator {
    pub fn new(config: ScanConfig, rules: Vec<Rule>, app_paths: AppPaths) -> Self {
        Self {
            config,
            rules,
            app_paths,
        }
    }

    /// Start the scan, returning a receiver for ScanEvents.
    /// The scan runs in background threads; events are sent as they are discovered.
    pub fn start(self) -> mpsc::Receiver<ScanEvent> {
        let (tx, rx) = mpsc::channel();
        let start_time = Instant::now();
        let dirs_visited = Arc::new(AtomicU64::new(0));
        let artifacts_found = Arc::new(AtomicU64::new(0));
        let total_size = Arc::new(AtomicU64::new(0));
        let scan_done = Arc::new(AtomicBool::new(false));

        let skip_paths = self.app_paths.skip_paths();
        let rules = Arc::new(self.rules);
        let config = self.config;

        // Spawn the walker in a dedicated thread
        let tx_clone = tx.clone();
        let dirs_visited_clone = dirs_visited.clone();
        let artifacts_found_clone = artifacts_found.clone();
        let total_size_clone = total_size.clone();
        let scan_done_clone = scan_done.clone();

        std::thread::spawn(move || {
            // Build the walker for each scan path
            for scan_path in &config.paths {
                let resolved = if scan_path.is_relative() {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(scan_path)
                } else {
                    scan_path.clone()
                };

                let mut builder = WalkBuilder::new(&resolved);
                builder
                    .hidden(false)          // Don't skip hidden directories
                    .git_ignore(false)       // We WANT to find gitignored artifacts
                    .git_global(false)
                    .git_exclude(false)
                    .follow_links(false)     // Don't follow symlinks
                    .same_file_system(!config.cross_device);

                if let Some(threads) = config.threads {
                    builder.threads(threads);
                }

                if let Some(max_depth) = config.max_depth {
                    builder.max_depth(Some(max_depth));
                }

                let skip_paths = skip_paths.clone();
                let rules = rules.clone();
                let tx = tx_clone.clone();
                let dirs_visited = dirs_visited_clone.clone();
                let artifacts_found = artifacts_found_clone.clone();
                let total_size = total_size_clone.clone();
                let exclude_patterns = config.exclude_patterns.clone();

                // Use the parallel walker
                builder.build_parallel().run(|| {
                    let skip_paths = skip_paths.clone();
                    let rules = rules.clone();
                    let tx = tx.clone();
                    let dirs_visited = dirs_visited.clone();
                    let artifacts_found = artifacts_found.clone();
                    let total_size = total_size.clone();
                    let exclude_patterns = exclude_patterns.clone();

                    Box::new(move |entry| {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(err) => {
                                let _ = tx.send(ScanEvent::Error(ScanError {
                                    path: PathBuf::from(""),
                                    message: err.to_string(),
                                }));
                                return ignore::WalkState::Continue;
                            }
                        };

                        let path = entry.path();

                        // Skip hardcoded paths
                        if skip_paths.iter().any(|s| path.starts_with(s)) {
                            return ignore::WalkState::Skip;
                        }

                        // Skip always-skipped directory names (e.g., .git)
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if entry.file_type().is_some_and(|ft| ft.is_dir())
                                && constants::ALWAYS_SKIP_DIRS.contains(&name)
                            {
                                return ignore::WalkState::Skip;
                            }

                            // Check exclude patterns
                            let path_str = path.to_string_lossy();
                            if exclude_patterns
                                .iter()
                                .any(|p| glob_match::glob_match(p, &path_str))
                            {
                                return ignore::WalkState::Skip;
                            }
                        }

                        let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());

                        if is_dir {
                            dirs_visited.fetch_add(1, Ordering::Relaxed);
                        }

                        // Get entry name
                        let name = match path.file_name().and_then(|n| n.to_str()) {
                            Some(n) => n,
                            None => return ignore::WalkState::Continue,
                        };

                        // Get parent directory
                        let parent = match path.parent() {
                            Some(p) => p,
                            None => return ignore::WalkState::Continue,
                        };

                        // Try to match against rules
                        if let Some(rule) = filter::find_matching_rule(name, parent, is_dir, &rules) {
                            let last_modified = entry
                                .metadata()
                                .ok()
                                .and_then(|m| m.modified().ok())
                                .map(DateTime::<Utc>::from);

                            let artifact = ArtifactInfo {
                                id: Uuid::new_v4(),
                                path: path.to_path_buf(),
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                category: rule.category,
                                safety: rule.safety,
                                size: None,
                                last_modified,
                                is_directory: is_dir,
                            };

                            artifacts_found.fetch_add(1, Ordering::Relaxed);
                            let artifact_id = artifact.id;
                            let artifact_path = artifact.path.clone();
                            let _ = tx.send(ScanEvent::Found(artifact));

                            // Spawn size calculation for directories
                            if is_dir {
                                let tx_size = tx.clone();
                                let total_size = total_size.clone();
                                rayon::spawn(move || {
                                    let size = compute_dir_size(&artifact_path);
                                    total_size.fetch_add(size, Ordering::Relaxed);
                                    let _ = tx_size.send(ScanEvent::SizeUpdate {
                                        id: artifact_id,
                                        size,
                                    });
                                });
                            }

                            // Skip subtree for matched directories
                            if is_dir {
                                return ignore::WalkState::Skip;
                            }
                        }

                        ignore::WalkState::Continue
                    })
                });
            }

            // Signal completion
            scan_done_clone.store(true, Ordering::SeqCst);
            let elapsed = start_time.elapsed();
            let _ = tx_clone.send(ScanEvent::Complete(ScanSummary {
                total_artifacts: artifacts_found.load(Ordering::Relaxed) as usize,
                total_size: total_size.load(Ordering::Relaxed),
                duration: elapsed,
                errors: vec![],
                dirs_visited: dirs_visited.load(Ordering::Relaxed),
            }));
        });

        rx
    }
}

/// Compute total size of a directory by walking all files.
fn compute_dir_size(path: &Path) -> u64 {
    let mut total: u64 = 0;
    let walker = ignore::WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .build();

    for entry in walker.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::catalog::builtin_rules;
    use tempfile::TempDir;

    fn create_test_project(tmp: &TempDir) {
        // Create a fake Node.js project
        let project = tmp.path().join("my-project");
        std::fs::create_dir_all(project.join("node_modules/some-package")).unwrap();
        std::fs::write(project.join("node_modules/some-package/index.js"), "module.exports = {}").unwrap();
        std::fs::write(project.join("package.json"), "{}").unwrap();

        // Create a fake Rust project
        let rust_project = tmp.path().join("rust-project");
        std::fs::create_dir_all(rust_project.join("target/debug")).unwrap();
        std::fs::write(rust_project.join("target/debug/binary"), "binary content").unwrap();
        std::fs::write(rust_project.join("Cargo.toml"), "[package]").unwrap();

        // Create a __pycache__
        let python_project = tmp.path().join("python-project");
        std::fs::create_dir_all(python_project.join("__pycache__")).unwrap();
        std::fs::write(python_project.join("__pycache__/module.cpython-312.pyc"), "bytecode").unwrap();

        // Create a non-matching directory (should NOT be flagged)
        std::fs::create_dir_all(tmp.path().join("regular-dir/src")).unwrap();
        std::fs::write(tmp.path().join("regular-dir/src/main.rs"), "fn main() {}").unwrap();
    }

    #[test]
    fn scanner_finds_node_modules() {
        let tmp = TempDir::new().unwrap();
        create_test_project(&tmp);

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let paths = AppPaths::with_base(tmp.path().join(".devprune-test"));
        let coordinator = ScanCoordinator::new(config, builtin_rules(), paths);
        let rx = coordinator.start();

        let mut found_artifacts: Vec<ArtifactInfo> = Vec::new();
        let mut completed = false;

        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Found(artifact) => found_artifacts.push(artifact),
                ScanEvent::Complete(_) => {
                    completed = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(completed);
        assert!(
            found_artifacts.iter().any(|a| a.rule_id == "node_modules"),
            "Should find node_modules"
        );
    }

    #[test]
    fn scanner_finds_rust_target_with_context() {
        let tmp = TempDir::new().unwrap();
        create_test_project(&tmp);

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let paths = AppPaths::with_base(tmp.path().join(".devprune-test"));
        let coordinator = ScanCoordinator::new(config, builtin_rules(), paths);
        let rx = coordinator.start();

        let mut found_artifacts: Vec<ArtifactInfo> = Vec::new();

        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Found(artifact) => found_artifacts.push(artifact),
                ScanEvent::Complete(_) => break,
                _ => {}
            }
        }

        assert!(
            found_artifacts.iter().any(|a| a.rule_id == "target_rust"),
            "Should find Rust target/ (Cargo.toml exists)"
        );
    }

    #[test]
    fn scanner_finds_pycache() {
        let tmp = TempDir::new().unwrap();
        create_test_project(&tmp);

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let paths = AppPaths::with_base(tmp.path().join(".devprune-test"));
        let coordinator = ScanCoordinator::new(config, builtin_rules(), paths);
        let rx = coordinator.start();

        let mut found_artifacts: Vec<ArtifactInfo> = Vec::new();

        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Found(artifact) => found_artifacts.push(artifact),
                ScanEvent::Complete(_) => break,
                _ => {}
            }
        }

        assert!(
            found_artifacts.iter().any(|a| a.rule_id == "pycache"),
            "Should find __pycache__"
        );
    }

    #[test]
    fn scanner_does_not_flag_regular_dirs() {
        let tmp = TempDir::new().unwrap();
        create_test_project(&tmp);

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let paths = AppPaths::with_base(tmp.path().join(".devprune-test"));
        let coordinator = ScanCoordinator::new(config, builtin_rules(), paths);
        let rx = coordinator.start();

        let mut found_paths: Vec<PathBuf> = Vec::new();

        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Found(artifact) => found_paths.push(artifact.path),
                ScanEvent::Complete(_) => break,
                _ => {}
            }
        }

        assert!(
            !found_paths.iter().any(|p| p.ends_with("regular-dir") || p.ends_with("src")),
            "Should not flag regular directories"
        );
    }

    #[test]
    fn scanner_skips_git_directories() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();
        std::fs::write(tmp.path().join(".git/HEAD"), "ref: refs/heads/main").unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let paths = AppPaths::with_base(tmp.path().join(".devprune-test"));
        let coordinator = ScanCoordinator::new(config, builtin_rules(), paths);
        let rx = coordinator.start();

        let mut found_paths: Vec<PathBuf> = Vec::new();

        while let Ok(event) = rx.recv() {
            match event {
                ScanEvent::Found(artifact) => found_paths.push(artifact.path),
                ScanEvent::Complete(_) => break,
                _ => {}
            }
        }

        assert!(
            !found_paths.iter().any(|p| p.to_string_lossy().contains(".git")),
            "Should never flag .git directories"
        );
    }

    #[test]
    fn compute_dir_size_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();  // 5 bytes
        std::fs::write(tmp.path().join("b.txt"), "world!").unwrap(); // 6 bytes

        let size = compute_dir_size(tmp.path());
        assert_eq!(size, 11);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune-core -- scanner`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune-core/src/scanner/
git commit -m "feat: add parallel scanner with rule matching and async size calculation"
```

---

## Phase 3: CLI

### Task 9: CLI Argument Parsing

**Files:**
- Create: `crates/devprune/src/cli.rs`
- Modify: `crates/devprune/src/main.rs`

- [ ] **Step 1: Write clap definitions**

`crates/devprune/src/cli.rs`:
```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "devprune",
    about = "Reclaim disk space by removing developer build artifacts",
    version
)]
pub struct Cli {
    /// Directories to scan (default: current directory)
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Scan user home directory (~)
    #[arg(long, conflicts_with_all = ["all", "paths"])]
    pub home: bool,

    /// Scan entire filesystem (with hardcoded skips for system dirs)
    #[arg(long, conflicts_with_all = ["home", "paths"])]
    pub all: bool,

    /// Force TUI mode
    #[arg(long, conflicts_with = "no_tui")]
    pub tui: bool,

    /// Force headless/CLI mode
    #[arg(long)]
    pub no_tui: bool,

    /// Show what would be deleted without deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON (implies --no-tui)
    #[arg(long)]
    pub json: bool,

    /// Non-interactive: delete all Safe artifacts. Without --yes, acts as dry-run.
    #[arg(long)]
    pub auto: bool,

    /// Skip confirmation prompt (only valid with --auto)
    #[arg(long, requires = "auto")]
    pub yes: bool,

    /// Only show specific categories (repeatable)
    #[arg(long = "category", value_name = "CAT")]
    pub categories: Vec<String>,

    /// Only show artifacts larger than SIZE (e.g., "100MB")
    #[arg(long, value_name = "SIZE")]
    pub min_size: Option<String>,

    /// Filter by safety level: safe, cautious, risky
    #[arg(long, value_name = "LEVEL")]
    pub safety: Option<String>,

    /// Exclude paths matching glob pattern (repeatable)
    #[arg(long = "exclude", value_name = "GLOB")]
    pub excludes: Vec<String>,

    /// Scanner thread count (default: number of CPUs)
    #[arg(long, value_name = "N")]
    pub threads: Option<usize>,

    /// Maximum directory depth
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Allow scanning across filesystem boundaries
    #[arg(long)]
    pub cross_device: bool,

    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long)]
    pub quiet: bool,

    /// Trash management subcommands
    #[command(subcommand)]
    pub trash: Option<TrashCommand>,
}

#[derive(Subcommand, Debug)]
pub enum TrashCommand {
    /// List items in trash
    #[command(name = "trash")]
    Trash {
        #[command(subcommand)]
        action: TrashAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum TrashAction {
    /// List all trashed items
    List,
    /// Restore a trashed item by ID
    Restore {
        /// The UUID of the item to restore
        id: String,
    },
    /// Permanently delete trashed items
    Purge {
        /// Only purge items older than this duration (e.g., "30d", "7d")
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,
    },
}

impl Cli {
    /// Determine the effective scan paths from CLI arguments.
    pub fn effective_paths(&self) -> Vec<PathBuf> {
        if self.home {
            vec![dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))]
        } else if self.all {
            vec![PathBuf::from("/")]
        } else if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }

    /// Whether to use TUI mode.
    pub fn use_tui(&self) -> bool {
        if self.tui {
            return true;
        }
        if self.no_tui || self.json || self.auto || self.dry_run {
            return false;
        }
        // Default: TUI if stdout is a TTY
        std::io::stdout().is_terminal()
    }
}
```

Note: No extra crate needed for TTY detection -- uses `std::io::IsTerminal` (stable since Rust 1.70). Add `use std::io::IsTerminal;` at the top of `cli.rs`.

- [ ] **Step 2: Wire up main.rs with basic headless mode**

`crates/devprune/src/main.rs`:
```rust
mod cli;

use clap::Parser;
use devprune_core::config::AppPaths;
use devprune_core::rules::catalog::builtin_rules;
use devprune_core::scanner::ScanCoordinator;
use devprune_core::types::*;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();

    let app_paths = AppPaths::resolve().expect("Could not determine application directories");

    let min_size = cli.min_size.as_deref().and_then(|s| {
        s.parse::<bytesize::ByteSize>().ok().map(|b| b.as_u64())
    });

    let categories = if cli.categories.is_empty() {
        None
    } else {
        Some(
            cli.categories
                .iter()
                .filter_map(|c| match c.to_lowercase().as_str() {
                    "dependencies" => Some(devprune_core::rules::types::Category::Dependencies),
                    "buildoutput" | "build" => Some(devprune_core::rules::types::Category::BuildOutput),
                    "cache" | "caches" => Some(devprune_core::rules::types::Category::Cache),
                    "virtualenv" | "venv" => Some(devprune_core::rules::types::Category::VirtualEnv),
                    "ide" => Some(devprune_core::rules::types::Category::IdeArtifact),
                    "coverage" => Some(devprune_core::rules::types::Category::Coverage),
                    "logs" => Some(devprune_core::rules::types::Category::Logs),
                    "compiled" | "generated" => Some(devprune_core::rules::types::Category::CompiledGenerated),
                    "misc" => Some(devprune_core::rules::types::Category::Misc),
                    _ => {
                        eprintln!("Unknown category: {}", c);
                        None
                    }
                })
                .collect(),
        )
    };

    let safety_filter = cli.safety.as_deref().and_then(|s| match s.to_lowercase().as_str() {
        "safe" => Some(devprune_core::rules::types::SafetyLevel::Safe),
        "cautious" => Some(devprune_core::rules::types::SafetyLevel::Cautious),
        "risky" => Some(devprune_core::rules::types::SafetyLevel::Risky),
        _ => {
            eprintln!("Unknown safety level: {}. Use: safe, cautious, risky", s);
            None
        }
    });

    let config = ScanConfig {
        paths: cli.effective_paths(),
        threads: cli.threads,
        max_depth: cli.max_depth,
        cross_device: cli.cross_device,
        min_size,
        categories,
        safety_filter,
        exclude_patterns: cli.excludes.clone(),
    };

    if cli.use_tui() {
        eprintln!("TUI mode not yet implemented. Use --no-tui or --dry-run for now.");
        std::process::exit(1);
    }

    // Headless mode
    let rules = builtin_rules();
    let coordinator = ScanCoordinator::new(config, rules, app_paths);
    let rx = coordinator.start();

    let mut artifacts: Vec<ArtifactInfo> = Vec::new();

    while let Ok(event) = rx.recv() {
        match event {
            ScanEvent::Found(artifact) => artifacts.push(artifact),
            ScanEvent::SizeUpdate { id, size } => {
                if let Some(a) = artifacts.iter_mut().find(|a| a.id == id) {
                    a.size = Some(size);
                }
            }
            ScanEvent::Complete(summary) => {
                if cli.json {
                    print_json(&artifacts, &summary, &cli);
                } else {
                    print_human(&artifacts, &summary);
                }
                break;
            }
            ScanEvent::Error(err) => {
                if cli.verbose > 0 {
                    eprintln!("Warning: {}", err);
                }
            }
            _ => {}
        }
    }
}

fn print_human(artifacts: &[ArtifactInfo], summary: &ScanSummary) {
    println!(
        "Scanned {} directories in {:.1}s",
        summary.dirs_visited,
        summary.duration.as_secs_f64()
    );
    println!(
        "Found {} artifacts ({} total)",
        summary.total_artifacts,
        bytesize::ByteSize(summary.total_size)
    );
    println!();

    // Group by category
    for category in devprune_core::rules::types::Category::all() {
        let cat_artifacts: Vec<_> = artifacts.iter().filter(|a| a.category == *category).collect();
        if cat_artifacts.is_empty() {
            continue;
        }

        let total: u64 = cat_artifacts.iter().filter_map(|a| a.size).sum();
        println!(
            "{} ({}, {} items)",
            category.display_name(),
            bytesize::ByteSize(total),
            cat_artifacts.len()
        );

        for artifact in &cat_artifacts {
            let size_str = artifact
                .size
                .map(|s| format!("{}", bytesize::ByteSize(s)))
                .unwrap_or_else(|| "computing...".to_string());
            let safety = artifact.safety.display_name();
            println!(
                "  {} [{}] {}",
                artifact.path.display(),
                safety,
                size_str
            );
        }
        println!();
    }
}

fn print_json(artifacts: &[ArtifactInfo], summary: &ScanSummary, cli: &Cli) {
    use std::collections::HashMap;

    // Build summary_by_category
    let mut by_category: HashMap<String, serde_json::Value> = HashMap::new();
    for category in devprune_core::rules::types::Category::all() {
        let cat_artifacts: Vec<_> = artifacts.iter().filter(|a| a.category == *category).collect();
        if !cat_artifacts.is_empty() {
            let total: u64 = cat_artifacts.iter().filter_map(|a| a.size).sum();
            by_category.insert(
                category.display_name().to_string(),
                serde_json::json!({
                    "count": cat_artifacts.len(),
                    "size_bytes": total,
                }),
            );
        }
    }

    let output = serde_json::json!({
        "scan": {
            "paths": cli.effective_paths(),
            "duration_ms": summary.duration.as_millis(),
            "total_artifacts": summary.total_artifacts,
            "total_size_bytes": summary.total_size,
            "dirs_visited": summary.dirs_visited,
        },
        "artifacts": artifacts,
        "summary_by_category": by_category,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
```

- [ ] **Step 3: Add bytesize to binary deps**

Add to `crates/devprune/Cargo.toml`:
```toml
bytesize = { workspace = true }
```

- [ ] **Step 4: Verify it compiles and runs**

Run: `cargo build`
Run: `cargo run -- --dry-run .`
Expected: Should scan current directory and print found artifacts

- [ ] **Step 5: Commit**

```bash
git add crates/devprune/src/ crates/devprune/Cargo.toml
git commit -m "feat: add CLI argument parsing and headless scan mode with human and JSON output"
```

---

## Phase 4: Trash System

### Task 10: Trash Metadata

**Files:**
- Create: `crates/devprune-core/src/trash/metadata.rs`

- [ ] **Step 1: Write metadata types and serialization with tests**

```rust
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants;
use crate::error::{DevpruneError, Result};
use crate::rules::types::Category;

/// Full metadata for a single trashed item. Source of truth lives in items/<uuid>/metadata.json.
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

/// The manifest cache: a list of all trashed items. Rebuildable from per-item metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashManifest {
    pub version: u32,
    pub entries: Vec<TrashManifestEntry>,
}

/// Lightweight entry in the manifest cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashManifestEntry {
    pub id: Uuid,
    pub original_path: PathBuf,
    pub trashed_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub rule_id: String,
    pub category: Category,
}

impl TrashManifest {
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }

    /// Write the manifest atomically: write to temp file, fsync, rename.
    pub fn write_atomic(&self, manifest_path: &Path) -> Result<()> {
        let dir = manifest_path.parent().ok_or_else(|| DevpruneError::Io {
            path: manifest_path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no parent directory"),
        })?;

        let temp_path = dir.join(".manifest.json.tmp");
        let json = serde_json::to_string_pretty(self).map_err(|e| DevpruneError::Trash {
            message: format!("Failed to serialize manifest: {}", e),
            source: Some(Box::new(e)),
        })?;

        let mut file = fs::File::create(&temp_path).map_err(|e| DevpruneError::Io {
            path: temp_path.clone(),
            source: e,
        })?;

        file.write_all(json.as_bytes()).map_err(|e| DevpruneError::Io {
            path: temp_path.clone(),
            source: e,
        })?;

        file.sync_all().map_err(|e| DevpruneError::Io {
            path: temp_path.clone(),
            source: e,
        })?;

        fs::rename(&temp_path, manifest_path).map_err(|e| DevpruneError::Io {
            path: manifest_path.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    /// Read the manifest from disk. Returns a new empty manifest if the file doesn't exist.
    pub fn read_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path).map_err(|e| DevpruneError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        serde_json::from_str(&content).map_err(|e| DevpruneError::ManifestCorrupted {
            message: format!("Failed to parse manifest: {}", e),
        })
    }

    /// Rebuild the manifest by scanning all item directories.
    pub fn rebuild_from_items(items_dir: &Path) -> Result<Self> {
        let mut entries = Vec::new();

        if !items_dir.exists() {
            return Ok(Self::new());
        }

        let read_dir = fs::read_dir(items_dir).map_err(|e| DevpruneError::Io {
            path: items_dir.to_path_buf(),
            source: e,
        })?;

        for entry in read_dir {
            let entry = entry.map_err(|e| DevpruneError::Io {
                path: items_dir.to_path_buf(),
                source: e,
            })?;

            let item_dir = entry.path();
            if !item_dir.is_dir() {
                continue;
            }

            let metadata_path = item_dir.join(constants::METADATA_FILENAME);
            let content_path = item_dir.join(constants::CONTENT_DIRNAME);

            if metadata_path.exists() && content_path.exists() {
                match fs::read_to_string(&metadata_path) {
                    Ok(content) => match serde_json::from_str::<TrashEntryMetadata>(&content) {
                        Ok(meta) => {
                            entries.push(TrashManifestEntry {
                                id: meta.id,
                                original_path: meta.original_path,
                                trashed_at: meta.trashed_at,
                                size_bytes: meta.size_bytes,
                                rule_id: meta.rule_id,
                                category: meta.category,
                            });
                        }
                        Err(e) => {
                            log::warn!("Skipping corrupt metadata at {}: {}", metadata_path.display(), e);
                        }
                    },
                    Err(e) => {
                        log::warn!("Could not read {}: {}", metadata_path.display(), e);
                    }
                }
            } else if metadata_path.exists() && !content_path.exists() {
                // Orphaned metadata (move failed) -- clean up
                log::warn!(
                    "Orphaned metadata without content at {}. Cleaning up.",
                    item_dir.display()
                );
                let _ = fs::remove_dir_all(&item_dir);
            } else if !metadata_path.exists() && content_path.exists() {
                log::warn!(
                    "Content without metadata at {}. Manual inspection needed.",
                    item_dir.display()
                );
            }
        }

        Ok(Self {
            version: 1,
            entries,
        })
    }
}

impl From<&TrashEntryMetadata> for TrashManifestEntry {
    fn from(meta: &TrashEntryMetadata) -> Self {
        Self {
            id: meta.id,
            original_path: meta.original_path.clone(),
            trashed_at: meta.trashed_at,
            size_bytes: meta.size_bytes,
            rule_id: meta.rule_id.clone(),
            category: meta.category,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_metadata() -> TrashEntryMetadata {
        TrashEntryMetadata {
            id: Uuid::new_v4(),
            original_path: PathBuf::from("/home/user/project/node_modules"),
            trashed_at: Utc::now(),
            size_bytes: 356_515_840,
            rule_id: "node_modules".to_string(),
            category: Category::Dependencies,
            original_permissions: 0o755,
            hostname: "dev-machine".to_string(),
        }
    }

    #[test]
    fn metadata_serialization_roundtrip() {
        let meta = sample_metadata();
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: TrashEntryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, meta.id);
        assert_eq!(deserialized.original_path, meta.original_path);
    }

    #[test]
    fn manifest_atomic_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let manifest_path = tmp.path().join("manifest.json");

        let mut manifest = TrashManifest::new();
        let meta = sample_metadata();
        manifest.entries.push(TrashManifestEntry::from(&meta));

        manifest.write_atomic(&manifest_path).unwrap();
        assert!(manifest_path.exists());

        let loaded = TrashManifest::read_from(&manifest_path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].id, meta.id);
    }

    #[test]
    fn manifest_read_nonexistent_returns_empty() {
        let manifest = TrashManifest::read_from(Path::new("/nonexistent/manifest.json")).unwrap();
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn manifest_rebuild_from_items() {
        let tmp = TempDir::new().unwrap();
        let items_dir = tmp.path().join("items");

        // Create two item directories with metadata and content
        let id1 = Uuid::new_v4();
        let item1_dir = items_dir.join(id1.to_string());
        fs::create_dir_all(item1_dir.join("content")).unwrap();
        let meta1 = TrashEntryMetadata {
            id: id1,
            ..sample_metadata()
        };
        fs::write(
            item1_dir.join("metadata.json"),
            serde_json::to_string(&meta1).unwrap(),
        ).unwrap();

        let id2 = Uuid::new_v4();
        let item2_dir = items_dir.join(id2.to_string());
        fs::create_dir_all(item2_dir.join("content")).unwrap();
        let meta2 = TrashEntryMetadata {
            id: id2,
            ..sample_metadata()
        };
        fs::write(
            item2_dir.join("metadata.json"),
            serde_json::to_string(&meta2).unwrap(),
        ).unwrap();

        let manifest = TrashManifest::rebuild_from_items(&items_dir).unwrap();
        assert_eq!(manifest.entries.len(), 2);
    }

    #[test]
    fn manifest_rebuild_cleans_orphaned_metadata() {
        let tmp = TempDir::new().unwrap();
        let items_dir = tmp.path().join("items");

        // Create item dir with metadata but NO content (simulates failed move)
        let id = Uuid::new_v4();
        let item_dir = items_dir.join(id.to_string());
        fs::create_dir_all(&item_dir).unwrap();
        let meta = TrashEntryMetadata {
            id,
            ..sample_metadata()
        };
        fs::write(
            item_dir.join("metadata.json"),
            serde_json::to_string(&meta).unwrap(),
        ).unwrap();

        let manifest = TrashManifest::rebuild_from_items(&items_dir).unwrap();
        assert!(manifest.entries.is_empty());
        // The orphaned directory should have been cleaned up
        assert!(!item_dir.exists());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune-core -- trash::metadata`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune-core/src/trash/metadata.rs
git commit -m "feat: add trash metadata types with atomic manifest writes and rebuild"
```

---

### Task 11: Trash Storage Operations

**Files:**
- Create: `crates/devprune-core/src/trash/storage.rs`
- Modify: `crates/devprune-core/src/trash/mod.rs`

- [ ] **Step 1: Write TrashManager struct and trash_item()**

```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::config::AppPaths;
use crate::constants;
use crate::error::{DevpruneError, Result};
use crate::rules::types::Category;
use crate::trash::metadata::*;

pub struct TrashManager {
    app_paths: AppPaths,
}

impl TrashManager {
    pub fn new(app_paths: AppPaths) -> Result<Self> {
        // Ensure trash directories exist
        fs::create_dir_all(&app_paths.items_dir).map_err(|e| DevpruneError::Io {
            path: app_paths.items_dir.clone(),
            source: e,
        })?;
        Ok(Self { app_paths })
    }

    /// Move an artifact to the trash.
    pub fn trash_item(
        &self,
        source: &Path,
        size_bytes: u64,
        rule_id: &str,
        category: Category,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let item_dir = self.app_paths.items_dir.join(id.to_string());
        let content_dir = item_dir.join(constants::CONTENT_DIRNAME);

        // Create item directory
        fs::create_dir_all(&item_dir).map_err(|e| DevpruneError::Io {
            path: item_dir.clone(),
            source: e,
        })?;

        // Get original permissions
        let permissions = fs::metadata(source)
            .map(|m| m.permissions().mode())
            .unwrap_or(0o755);

        // Get hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        // Write metadata BEFORE the move (crash recovery: orphaned metadata is safe)
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

        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| DevpruneError::Trash {
                message: format!("Failed to serialize metadata: {}", e),
                source: Some(Box::new(e)),
            })?;

        fs::write(item_dir.join(constants::METADATA_FILENAME), metadata_json)
            .map_err(|e| DevpruneError::Io {
                path: item_dir.join(constants::METADATA_FILENAME),
                source: e,
            })?;

        // Move content: try rename first, fall back to copy+verify+delete
        match fs::rename(source, &content_dir) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                // Cross-device: copy, verify, delete
                self.cross_device_move(source, &content_dir)?;
            }
            Err(e) => {
                // Clean up the item directory on failure
                let _ = fs::remove_dir_all(&item_dir);
                return Err(DevpruneError::Io {
                    path: source.to_path_buf(),
                    source: e,
                });
            }
        }

        // Rebuild manifest cache
        self.rebuild_manifest()?;

        Ok(id)
    }

    /// Restore a trashed item to its original location.
    pub fn restore_item(&self, id: Uuid) -> Result<PathBuf> {
        let item_dir = self.app_paths.items_dir.join(id.to_string());
        let metadata_path = item_dir.join(constants::METADATA_FILENAME);
        let content_dir = item_dir.join(constants::CONTENT_DIRNAME);

        // Read metadata
        let metadata_str = fs::read_to_string(&metadata_path)
            .map_err(|e| DevpruneError::Io { path: metadata_path.clone(), source: e })?;
        let metadata: TrashEntryMetadata = serde_json::from_str(&metadata_str)
            .map_err(|e| DevpruneError::ManifestCorrupted {
                message: format!("Failed to parse item metadata: {}", e),
            })?;

        let original = &metadata.original_path;

        // Check if original path already exists
        if original.exists() {
            return Err(DevpruneError::RestoreConflict {
                path: original.clone(),
            });
        }

        // Create parent directory if needed
        if let Some(parent) = original.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| DevpruneError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }

        // Move content back
        fs::rename(&content_dir, original).map_err(|e| DevpruneError::Io {
            path: original.clone(),
            source: e,
        })?;

        // Clean up item directory
        let _ = fs::remove_dir_all(&item_dir);

        // Rebuild manifest
        self.rebuild_manifest()?;

        Ok(original.clone())
    }

    /// Permanently delete a trashed item.
    pub fn purge_item(&self, id: Uuid) -> Result<()> {
        let item_dir = self.app_paths.items_dir.join(id.to_string());
        fs::remove_dir_all(&item_dir).map_err(|e| DevpruneError::Io {
            path: item_dir,
            source: e,
        })?;
        self.rebuild_manifest()?;
        Ok(())
    }

    /// Purge items older than the given number of days.
    pub fn purge_older_than(&self, days: u64) -> Result<Vec<Uuid>> {
        let manifest = self.load_manifest()?;
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let mut purged = Vec::new();

        for entry in &manifest.entries {
            if entry.trashed_at < cutoff {
                self.purge_item(entry.id)?;
                purged.push(entry.id);
            }
        }

        Ok(purged)
    }

    /// List all items in trash.
    pub fn list_items(&self) -> Result<Vec<TrashManifestEntry>> {
        let manifest = self.load_manifest()?;
        Ok(manifest.entries)
    }

    fn load_manifest(&self) -> Result<TrashManifest> {
        let manifest = TrashManifest::read_from(&self.app_paths.manifest_path)?;
        if manifest.entries.is_empty() && self.app_paths.items_dir.exists() {
            // Manifest might be stale or missing; rebuild
            return TrashManifest::rebuild_from_items(&self.app_paths.items_dir);
        }
        Ok(manifest)
    }

    fn rebuild_manifest(&self) -> Result<()> {
        let manifest = TrashManifest::rebuild_from_items(&self.app_paths.items_dir)?;
        manifest.write_atomic(&self.app_paths.manifest_path)?;
        Ok(())
    }

    /// Cross-device move: recursive copy with per-file error checking, then verify and delete.
    fn cross_device_move(&self, source: &Path, dest: &Path) -> Result<()> {
        // Recursive copy
        self.copy_recursive(source, dest)?;

        // Verify: compare file count and total size
        let source_stats = self.dir_stats(source)?;
        let dest_stats = self.dir_stats(dest)?;

        if source_stats != dest_stats {
            // Verification failed; clean up partial copy
            let _ = fs::remove_dir_all(dest);
            return Err(DevpruneError::Trash {
                message: format!(
                    "Cross-device copy verification failed: source ({} files, {} bytes) != dest ({} files, {} bytes)",
                    source_stats.0, source_stats.1, dest_stats.0, dest_stats.1
                ),
                source: None,
            });
        }

        // Delete original
        fs::remove_dir_all(source).map_err(|e| DevpruneError::Io {
            path: source.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }

    fn copy_recursive(&self, source: &Path, dest: &Path) -> Result<()> {
        if source.is_dir() {
            fs::create_dir_all(dest).map_err(|e| DevpruneError::Io {
                path: dest.to_path_buf(),
                source: e,
            })?;

            for entry in fs::read_dir(source).map_err(|e| DevpruneError::Io {
                path: source.to_path_buf(),
                source: e,
            })? {
                let entry = entry.map_err(|e| DevpruneError::Io {
                    path: source.to_path_buf(),
                    source: e,
                })?;
                let src_path = entry.path();
                let dest_path = dest.join(entry.file_name());
                self.copy_recursive(&src_path, &dest_path)?;
            }
        } else {
            fs::copy(source, dest).map_err(|e| DevpruneError::Io {
                path: source.to_path_buf(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Count files and total size in a directory tree.
    fn dir_stats(&self, path: &Path) -> Result<(u64, u64)> {
        let mut file_count: u64 = 0;
        let mut total_size: u64 = 0;

        if path.is_dir() {
            for entry in fs::read_dir(path).map_err(|e| DevpruneError::Io {
                path: path.to_path_buf(),
                source: e,
            })? {
                let entry = entry.map_err(|e| DevpruneError::Io {
                    path: path.to_path_buf(),
                    source: e,
                })?;
                let (fc, ts) = self.dir_stats(&entry.path())?;
                file_count += fc;
                total_size += ts;
            }
        } else {
            file_count = 1;
            total_size = fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
        }

        Ok((file_count, total_size))
    }
}
```

Note: Add `hostname` and `libc` to workspace dependencies:
```toml
# Cargo.toml (workspace)
hostname = "0.4"
libc = "0.2"

# crates/devprune-core/Cargo.toml
hostname = { workspace = true }
libc = { workspace = true }
```

- [ ] **Step 2: Write tests for TrashManager**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_manager() -> (TempDir, TrashManager) {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::with_base(tmp.path().to_path_buf());
        let manager = TrashManager::new(paths).unwrap();
        (tmp, manager)
    }

    #[test]
    fn trash_and_restore_cycle() {
        let (tmp, manager) = setup_manager();

        // Create a source directory to trash
        let source = tmp.path().join("project/node_modules");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("file.js"), "content").unwrap();

        // Trash it
        let id = manager
            .trash_item(&source, 7, "node_modules", Category::Dependencies)
            .unwrap();

        // Source should be gone
        assert!(!source.exists());

        // Should be in trash listing
        let items = manager.list_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, id);

        // Restore it
        let restored_path = manager.restore_item(id).unwrap();
        assert_eq!(restored_path, source);
        assert!(source.exists());
        assert!(source.join("file.js").exists());

        // Should be gone from trash
        let items = manager.list_items().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn restore_conflict_when_path_exists() {
        let (tmp, manager) = setup_manager();

        let source = tmp.path().join("project/target");
        fs::create_dir_all(&source).unwrap();

        let id = manager
            .trash_item(&source, 0, "target_rust", Category::BuildOutput)
            .unwrap();

        // Re-create the original path
        fs::create_dir_all(&source).unwrap();

        // Restore should fail with RestoreConflict
        let result = manager.restore_item(id);
        assert!(matches!(result, Err(DevpruneError::RestoreConflict { .. })));
    }

    #[test]
    fn purge_removes_permanently() {
        let (tmp, manager) = setup_manager();

        let source = tmp.path().join("to-purge");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("data"), "x").unwrap();

        let id = manager
            .trash_item(&source, 1, "test", Category::Misc)
            .unwrap();

        manager.purge_item(id).unwrap();

        let items = manager.list_items().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn restore_creates_parent_dir() {
        let (tmp, manager) = setup_manager();

        let source = tmp.path().join("deep/nested/path/node_modules");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("pkg.json"), "{}").unwrap();

        let id = manager
            .trash_item(&source, 2, "node_modules", Category::Dependencies)
            .unwrap();

        // Delete the entire parent structure
        fs::remove_dir_all(tmp.path().join("deep")).unwrap();

        // Restore should recreate parent dirs
        manager.restore_item(id).unwrap();
        assert!(source.exists());
        assert!(source.join("pkg.json").exists());
    }

    #[test]
    fn list_empty_trash() {
        let (_tmp, manager) = setup_manager();
        let items = manager.list_items().unwrap();
        assert!(items.is_empty());
    }
}
```

- [ ] **Step 2: Update trash/mod.rs**

```rust
pub mod metadata;
pub mod storage;

pub use storage::TrashManager;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p devprune-core -- trash`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/devprune-core/src/trash/
git commit -m "feat: add trash manager with move, restore, purge, and crash recovery"
```

---

## Phase 5: TUI

### Task 12: TUI Event Loop

**Files:**
- Create: `crates/devprune/src/tui/mod.rs`
- Create: `crates/devprune/src/tui/event.rs`

- [ ] **Step 1: Write event types and event loop**

The event loop multiplexes keyboard input, scan events, and tick timer into a single `AppEvent` enum. Uses a dedicated input thread and crossterm's event polling.

`crates/devprune/src/tui/mod.rs`:
```rust
pub mod app;
pub mod event;
pub mod input;
pub mod ui;
```

`crates/devprune/src/tui/event.rs`: The event loop with `AppEvent` enum, input thread, tick timer, and scan event reception.

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`

- [ ] **Step 3: Commit**

```bash
git add crates/devprune/src/tui/
git commit -m "feat: add TUI event loop multiplexing input, scan, and tick events"
```

---

### Task 13: TUI App State Machine

**Files:**
- Create: `crates/devprune/src/tui/app.rs`

- [ ] **Step 1: Write App state with tree structure and update logic**

The `App` struct holds all TUI state: tree, details, dialog, scan progress, mode. The `update()` method processes `AppEvent`s and returns actions. The `draw()` method is a pure function of state.

Key state structures:
- `TreeState`: visible rows, expanded nodes, checked items, cursor, scroll
- `TreeNodeId`: enum for Category, RuleGroup, Artifact levels
- `CheckState`: Checked, Unchecked, Indeterminate
- `AppMode`: Normal, Search, ConfirmDelete, Help, TrashBrowser

Write tests for:
- Tree expansion/collapse
- Checkbox cascade (parent checks all children, child unchecks sets parent to indeterminate)
- Search filtering
- Mode transitions

- [ ] **Step 2: Run tests**

Run: `cargo test -p devprune -- tui::app`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/devprune/src/tui/app.rs
git commit -m "feat: add TUI app state machine with tree, checkboxes, and mode transitions"
```

---

### Task 14: TUI Input Handling

**Files:**
- Create: `crates/devprune/src/tui/input.rs`

- [ ] **Step 1: Write key binding dispatch**

Maps crossterm key events to app actions based on current `AppMode`. All key bindings from the spec:
- j/k/arrows: cursor movement
- Space: toggle checkbox
- Enter: expand/collapse
- a/A: select/deselect all
- d: delete dialog
- Tab: panel focus
- /: search mode
- s: sort cycle
- f: safety filter
- t: trash browser
- r: restore
- ?: help
- q/Ctrl-C: quit

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`

- [ ] **Step 3: Commit**

```bash
git add crates/devprune/src/tui/input.rs
git commit -m "feat: add TUI input handling with vim-style key bindings"
```

---

### Task 15: TUI Rendering - Tree Widget

**Files:**
- Create: `crates/devprune/src/tui/ui/mod.rs`
- Create: `crates/devprune/src/tui/ui/tree.rs`
- Create: `crates/devprune/src/tui/ui/theme.rs`

- [ ] **Step 1: Write theme constants**

`theme.rs`: Color palette and style definitions. Categories get distinct colors. Safety levels: green (Safe), yellow (Cautious), red (Risky). Checked items green, unchecked grey, indeterminate purple.

- [ ] **Step 2: Write tree rendering**

`tree.rs`: `render_tree()` function that takes a Frame, Rect, and TreeState. Renders the 3-level tree with indentation, expand/collapse arrows, checkboxes, names, sizes, and item counts. Highlighted row gets a distinct background.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add crates/devprune/src/tui/ui/
git commit -m "feat: add TUI tree widget rendering with theme and checkbox display"
```

---

### Task 16: TUI Rendering - Details, Status Bar, Dialogs

**Files:**
- Create: `crates/devprune/src/tui/ui/details.rs`
- Create: `crates/devprune/src/tui/ui/status_bar.rs`
- Create: `crates/devprune/src/tui/ui/dialog.rs`

- [ ] **Step 1: Write details panel**

`details.rs`: Right panel showing path, size, safety level, last modified, category, and restore instructions for the highlighted artifact.

- [ ] **Step 2: Write status bar**

`status_bar.rs`: Header with spinner (or checkmark when done), dirs visited, artifacts found, elapsed time. Footer with context-sensitive key hints.

- [ ] **Step 3: Write dialog rendering**

`dialog.rs`: Centered overlay for confirmation dialogs (delete confirmation showing count and total size), help overlay (full key binding reference).

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`

- [ ] **Step 5: Commit**

```bash
git add crates/devprune/src/tui/ui/
git commit -m "feat: add TUI details panel, status bar, and dialog overlays"
```

---

### Task 17: Wire Up TUI Main Loop

**Files:**
- Modify: `crates/devprune/src/main.rs`
- Modify: `crates/devprune/src/tui/mod.rs`

- [ ] **Step 1: Create the run_tui function**

In `tui/mod.rs`, create `pub fn run_tui(config: ScanConfig, rules: Vec<Rule>, app_paths: AppPaths) -> Result<()>` that:
1. Enters raw mode and alternate screen
2. Creates the App
3. Starts the ScanCoordinator
4. Starts the event loop
5. On each event: update app state, redraw
6. On quit: restore terminal, optionally perform selected deletions

- [ ] **Step 2: Wire into main.rs**

Update `main.rs` to call `run_tui()` when `cli.use_tui()` returns true.

- [ ] **Step 3: Test manually**

Run: `cargo run -- .`
Expected: TUI launches, shows scan progress, displays found artifacts in tree view

- [ ] **Step 4: Commit**

```bash
git add crates/devprune/src/
git commit -m "feat: wire up TUI main loop with live scanning and interactive tree"
```

---

## Phase 6: Integration & Polish

### Task 18: Wire Trash Into TUI Delete Flow

**Files:**
- Modify: `crates/devprune/src/tui/app.rs`

- [ ] **Step 1: Implement delete action**

When user presses `d` with items selected:
1. Show confirmation dialog with count and total size
2. On confirm: call `TrashManager::trash_item()` for each selected item
3. Remove from tree state
4. Show success/failure summary

- [ ] **Step 2: Implement trash browser**

When user presses `t`:
1. Switch to TrashBrowser mode
2. Load trash manifest
3. Display in list view with checkboxes
4. `r` to restore, `p` to purge, `Esc` to return

- [ ] **Step 3: Test manually**

Run: `cargo run -- .`
Test: Select items, press `d`, confirm, verify items moved to trash. Press `t`, verify trash browser shows items. Select and restore.

- [ ] **Step 4: Commit**

```bash
git add crates/devprune/src/
git commit -m "feat: wire trash operations into TUI delete and trash browser views"
```

---

### Task 19: Wire Trash Into CLI

**Files:**
- Modify: `crates/devprune/src/main.rs`

- [ ] **Step 1: Implement trash subcommands**

Handle `TrashCommand::Trash { action }` in main:
- `TrashAction::List`: Print all trashed items (path, size, date, rule)
- `TrashAction::Restore { id }`: Restore item by UUID
- `TrashAction::Purge { older_than }`: Purge items, optionally filtered by age

- [ ] **Step 2: Test CLI trash commands**

Run: `cargo run -- trash list`
Run: `cargo run -- trash purge --older-than 0d` (purge everything)
Expected: Commands execute correctly

- [ ] **Step 3: Commit**

```bash
git add crates/devprune/src/main.rs
git commit -m "feat: add CLI trash list, restore, and purge subcommands"
```

---

### Task 20: Search, Sort, and Filter in TUI

**Files:**
- Modify: `crates/devprune/src/tui/app.rs`
- Modify: `crates/devprune/src/tui/input.rs`

- [ ] **Step 1: Implement search mode**

Press `/` to enter search mode. Text input filters visible tree rows by path substring match. `Esc` exits search. `Enter` locks the filter and returns to normal mode.

- [ ] **Step 2: Implement sort cycling**

Press `s` to cycle sort order: by size (descending), by name, by path. Applies within each tree level.

- [ ] **Step 3: Implement safety filter**

Press `f` to cycle safety filter: All -> Safe only -> Cautious only -> Risky only -> All.

- [ ] **Step 4: Test manually and commit**

```bash
git add crates/devprune/src/tui/
git commit -m "feat: add search, sort, and safety filter to TUI"
```

---

### Task 21: User Rule Configuration

**Files:**
- Create: `crates/devprune-core/src/rules/parser.rs`

- [ ] **Step 1: Write TOML config parser with tests**

Parse `~/.config/devprune/rules.toml` format:
```toml
[rules.disable]
ids = ["vscode", "idea"]

[[rules.custom]]
id = "my_rule"
...
```

Function signature: `pub fn load_user_rules(config_dir: &Path, builtin: Vec<Rule>) -> Result<Vec<Rule>>` that:
1. Reads the config file (return builtin rules if file doesn't exist)
2. Disables rules by ID
3. Appends custom rules
4. Validates custom rules (unique IDs, valid fields)

- [ ] **Step 2: Wire into startup**

Call `load_user_rules()` after `builtin_rules()` in main.rs.

- [ ] **Step 3: Run tests and commit**

```bash
git add crates/devprune-core/src/rules/parser.rs crates/devprune/src/main.rs
git commit -m "feat: add user rule configuration via TOML config file"
```

---

### Task 22: --auto Mode

**Files:**
- Modify: `crates/devprune/src/main.rs`

- [ ] **Step 1: Implement --auto mode**

When `--auto` is passed:
1. Run scan to completion
2. Filter to Safe artifacts only
3. Print summary of what would be deleted
4. If `--yes` is also passed: trash all Safe artifacts, print results
5. Without `--yes`: exit after printing (acts as dry-run)

- [ ] **Step 2: Test**

Run: `cargo run -- --auto .` (should print without deleting)
Run: `cargo run -- --auto --yes .` (should trash Safe artifacts)

- [ ] **Step 3: Commit**

```bash
git add crates/devprune/src/main.rs
git commit -m "feat: add --auto mode for non-interactive safe artifact cleanup"
```

---

### Task 23: Final Integration Testing

**Files:**
- Create: `crates/devprune-core/tests/integration.rs`
- Create: `crates/devprune/tests/cli_integration.rs`

- [ ] **Step 1: Write core integration tests**

End-to-end scan test: create a complex directory tree with multiple artifact types, scan it, verify all expected artifacts found and no false positives.

- [ ] **Step 2: Write CLI integration tests**

Use `std::process::Command` to invoke the binary with various flags:
- `--dry-run .` produces output
- `--json .` produces valid JSON
- `--help` prints help
- Invalid args produce error

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/devprune-core/tests/ crates/devprune/tests/
git commit -m "test: add integration tests for scanning and CLI"
```

---

## Verification

### How to test the final product end-to-end:

1. **Build**: `cargo build --release`
2. **CLI dry-run**: `./target/release/devprune --dry-run ~` -- should list all dev artifacts under home
3. **CLI JSON**: `./target/release/devprune --json . | jq .` -- verify valid JSON output
4. **TUI mode**: `./target/release/devprune ~` -- verify:
   - Tree populates with categories and artifacts
   - Expand/collapse works (Enter)
   - Checkboxes toggle (Space) with parent cascade
   - Details panel updates on cursor move
   - Search works (/)
   - Sort works (s)
   - Delete with confirmation (d) moves to trash
   - Trash browser (t) shows trashed items
   - Restore works (r)
   - Help overlay (?)
   - Quit (q)
5. **Trash operations**:
   - `devprune trash list` -- shows trashed items
   - `devprune trash restore <ID>` -- restores an item
   - `devprune trash purge` -- permanently deletes
6. **Auto mode**: `devprune --auto --yes .` -- deletes Safe artifacts
