use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::{Instant, SystemTime};

use chrono::{DateTime, Utc};
use ignore::{WalkBuilder, WalkState};
use uuid::Uuid;

use crate::config::AppPaths;
use crate::constants::ALWAYS_SKIP_DIRS;
use crate::rules::types::Rule;
use crate::scanner::filter::find_matching_rule;
use crate::types::{ArtifactInfo, ScanConfig, ScanEvent, ScanSummary};

/// Drives a parallel, rule-based filesystem scan.
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

    /// Starts the scan in a background thread and returns a channel receiver
    /// that delivers `ScanEvent`s.
    ///
    /// The receiver will eventually receive `ScanEvent::Complete` once all
    /// scan paths have been exhausted.
    pub fn start(self) -> mpsc::Receiver<ScanEvent> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || run_scan(self.config, self.rules, self.app_paths, tx));
        rx
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn run_scan(config: ScanConfig, rules: Vec<Rule>, app_paths: AppPaths, tx: Sender<ScanEvent>) {
    let start = Instant::now();
    let dirs_visited = Arc::new(AtomicU64::new(0));
    let artifacts_found = Arc::new(AtomicU64::new(0));
    let errors: Arc<std::sync::Mutex<Vec<crate::error::ScanError>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));

    // Build the set of paths to skip. We include platform-specific system
    // paths (e.g. /proc, /tmp), the app's own trash dir, and any user-
    // configured skip paths. However, a skip path is dropped if it is an
    // ancestor of (or equal to) a scan root that the user explicitly
    // requested. This lets tests (and users) scan directories that happen to
    // live under a normally-excluded prefix such as /tmp.
    let raw_skip_paths: Vec<PathBuf> = {
        let mut v = app_paths.skip_paths();
        v.extend(config.skip_paths.iter().cloned());
        v
    };
    let skip_paths: Vec<PathBuf> = raw_skip_paths
        .into_iter()
        .filter(|sp| {
            // Keep skip_path only when NO scan root is equal to or under it.
            !config
                .paths
                .iter()
                .any(|scan_root| scan_root.starts_with(sp))
        })
        .collect();

    let rules = Arc::new(rules);

    for scan_path in &config.paths {
        let mut builder = WalkBuilder::new(scan_path);
        builder
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .follow_links(false)
            .same_file_system(false);

        if let Some(depth) = config.max_depth {
            builder.max_depth(Some(depth as usize));
        }

        let walk = builder.build_parallel();

        let tx_clone = tx.clone();
        let skip_paths_clone = skip_paths.clone();
        let rules_clone = Arc::clone(&rules);
        let dirs_visited_clone = Arc::clone(&dirs_visited);
        let artifacts_found_clone = Arc::clone(&artifacts_found);
        let errors_clone = Arc::clone(&errors);

        walk.run(|| {
            let tx = tx_clone.clone();
            let skip_paths = skip_paths_clone.clone();
            let rules = Arc::clone(&rules_clone);
            let dirs_visited = Arc::clone(&dirs_visited_clone);
            let artifacts_found = Arc::clone(&artifacts_found_clone);
            let errors = Arc::clone(&errors_clone);

            Box::new(move |entry_result| {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(err) => {
                        let scan_err = crate::error::ScanError {
                            path: PathBuf::from("<unknown>"),
                            message: err.to_string(),
                        };
                        if let Ok(mut guard) = errors.lock() {
                            guard.push(scan_err.clone());
                        }
                        let _ = tx.send(ScanEvent::Error(scan_err));
                        return WalkState::Continue;
                    }
                };

                let path = entry.path().to_path_buf();

                // Skip the root entry itself (depth 0).
                if entry.depth() == 0 {
                    return WalkState::Continue;
                }

                // Skip configured skip paths.
                if skip_paths.iter().any(|sp| path.starts_with(sp)) {
                    return WalkState::Skip;
                }

                // Always skip certain directory names (e.g. .git).
                let file_name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => return WalkState::Continue,
                };

                if ALWAYS_SKIP_DIRS.contains(&file_name) {
                    return WalkState::Skip;
                }

                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let parent = path.parent().unwrap_or_else(|| Path::new("."));

                if is_dir {
                    dirs_visited.fetch_add(1, Ordering::Relaxed);
                }

                // Try to match a rule.
                if let Some(rule) = find_matching_rule(file_name, parent, is_dir, &rules) {
                    let id = Uuid::new_v4();
                    let last_modified = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(system_time_to_datetime);

                    let artifact = ArtifactInfo {
                        id,
                        path: path.clone(),
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        rule_description: rule.description.clone(),
                        category: rule.category,
                        safety: rule.safety,
                        size: None,
                        last_modified,
                        is_directory: is_dir,
                    };

                    artifacts_found.fetch_add(1, Ordering::Relaxed);
                    let _ = tx.send(ScanEvent::Found(artifact));

                    // Compute size asynchronously via rayon.
                    if is_dir {
                        let tx_size = tx.clone();
                        let path_clone = path.clone();
                        rayon::spawn(move || {
                            let size = compute_dir_size(&path_clone);
                            let _ = tx_size.send(ScanEvent::SizeUpdate { id, size });
                        });

                        // Do not descend into matched directories.
                        return WalkState::Skip;
                    }
                }

                WalkState::Continue
            })
        });
    }

    let duration = start.elapsed();
    let summary = ScanSummary {
        total_artifacts: artifacts_found.load(Ordering::Relaxed),
        total_size: 0,
        duration,
        errors: errors.lock().map(|g| g.clone()).unwrap_or_default(),
        dirs_visited: dirs_visited.load(Ordering::Relaxed),
    };

    let _ = tx.send(ScanEvent::Complete(summary));
}

/// Recursively sums the sizes of all files under `path`.
pub fn compute_dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(walker) = ignore::WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .build()
        .collect::<Result<Vec<_>, _>>()
    {
        for entry in walker {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn system_time_to_datetime(st: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(st)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::rules::catalog::builtin_rules;

    fn default_app_paths(tmp: &tempfile::TempDir) -> AppPaths {
        AppPaths::with_base(tmp.path().join("app"))
    }

    /// Drains events from `rx` until `ScanEvent::Complete` is received,
    /// collecting all `Found` events.
    fn collect_events(rx: mpsc::Receiver<ScanEvent>) -> Vec<ArtifactInfo> {
        let mut found = Vec::new();
        for event in rx {
            match event {
                ScanEvent::Found(a) => found.push(a),
                ScanEvent::Complete(_) => break,
                _ => {}
            }
        }
        found
    }

    #[test]
    fn scanner_finds_node_modules() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("my-project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::create_dir_all(project.join("node_modules").join("lodash")).unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), default_app_paths(&tmp));
        let rx = coordinator.start();
        let found = collect_events(rx);

        assert!(
            found.iter().any(|a| a.path.ends_with("node_modules")),
            "expected node_modules to be found, got: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scanner_finds_rust_target_with_context() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("rust-project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
        fs::create_dir_all(project.join("target").join("debug")).unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), default_app_paths(&tmp));
        let rx = coordinator.start();
        let found = collect_events(rx);

        assert!(
            found.iter().any(|a| a.path.ends_with("target")),
            "expected target dir to be found, got: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scanner_finds_pycache() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("py-project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(project.join("__pycache__")).unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), default_app_paths(&tmp));
        let rx = coordinator.start();
        let found = collect_events(rx);

        assert!(
            found.iter().any(|a| a.path.ends_with("__pycache__")),
            "expected __pycache__ to be found"
        );
    }

    #[test]
    fn scanner_does_not_flag_regular_dirs() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("plain-project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::create_dir_all(project.join("tests")).unwrap();
        fs::write(project.join("main.rs"), "fn main() {}").unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), default_app_paths(&tmp));
        let rx = coordinator.start();
        let found = collect_events(rx);

        assert!(
            found.is_empty(),
            "expected no artifacts in a plain project, got: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scanner_skips_git_directories() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("git-project");
        fs::create_dir_all(&project).unwrap();
        // Create a .git directory with a node_modules inside. The scanner
        // must skip .git entirely and never report the inner node_modules.
        fs::create_dir_all(project.join(".git").join("node_modules")).unwrap();

        let config = ScanConfig {
            paths: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), default_app_paths(&tmp));
        let rx = coordinator.start();
        let found = collect_events(rx);

        assert!(
            found.is_empty(),
            "scanner should not report artifacts inside .git, got: {:?}",
            found.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    #[test]
    fn compute_dir_size_works() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("sample");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.txt"), "hello").unwrap(); // 5 bytes
        fs::write(dir.join("b.txt"), "world!").unwrap(); // 6 bytes
        let sub = dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("c.txt"), "!!").unwrap(); // 2 bytes

        let size = compute_dir_size(&dir);
        assert_eq!(size, 13, "expected 5 + 6 + 2 = 13 bytes, got {}", size);
    }
}
