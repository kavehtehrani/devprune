use std::fs;
use std::path::Path;

use tempfile::TempDir;

use devprune_core::config::AppPaths;
use devprune_core::rules::catalog::builtin_rules;
use devprune_core::scanner::ScanCoordinator;
use devprune_core::types::{ArtifactInfo, ScanConfig, ScanEvent};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds an `AppPaths` whose trash/config dirs live inside a scratch TempDir
/// so the scanner never interacts with real user data.
fn scratch_app_paths(scratch: &TempDir) -> AppPaths {
    AppPaths::with_base(scratch.path().join("app"))
}

/// Drains `rx` until `ScanEvent::Complete` and returns every `Found` artifact.
fn collect_found(rx: std::sync::mpsc::Receiver<ScanEvent>) -> Vec<ArtifactInfo> {
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

/// Creates a file at `path`, creating all parent directories as needed.
fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

// ---------------------------------------------------------------------------
// Complex tree fixture
//
// Layout inside the scan root:
//
//   js-project/
//     package.json
//     node_modules/          <- should be found
//       lodash/
//         index.js
//   rust-project/
//     Cargo.toml
//     src/
//       main.rs
//     target/                <- should be found (Cargo.toml context)
//       debug/
//         devprune
//   py-project/
//     requirements.txt
//     app.py
//     __pycache__/           <- should be found
//       app.cpython-311.pyc
//     .pytest_cache/         <- should be found
//       README.md
//     .venv/                 <- should be found (requirements.txt context)
//       lib/
//         python3.11/
//   plain-project/           <- should NOT be found
//     src/
//       lib.rs
//     docs/
//       readme.md
//   .git/                    <- should be SKIPPED entirely
//     objects/
//       node_modules/        <- must not surface (inside .git)
// ---------------------------------------------------------------------------

struct ComplexFixture {
    /// Keep the TempDir alive so files are not deleted while the test runs.
    _root: TempDir,
    /// Scratch TempDir for app state (trash, config).
    _app: TempDir,
    scan_root: std::path::PathBuf,
}

impl ComplexFixture {
    fn create() -> Self {
        let root = TempDir::new().unwrap();
        let app = TempDir::new().unwrap();
        let scan_root = root.path().to_path_buf();

        // --- js-project ---
        touch(&scan_root.join("js-project/package.json"));
        touch(&scan_root.join("js-project/node_modules/lodash/index.js"));

        // --- rust-project ---
        touch(&scan_root.join("rust-project/Cargo.toml"));
        touch(&scan_root.join("rust-project/src/main.rs"));
        touch(&scan_root.join("rust-project/target/debug/devprune"));

        // --- py-project ---
        touch(&scan_root.join("py-project/requirements.txt"));
        touch(&scan_root.join("py-project/app.py"));
        touch(&scan_root.join("py-project/__pycache__/app.cpython-311.pyc"));
        touch(&scan_root.join("py-project/.pytest_cache/README.md"));
        touch(&scan_root.join("py-project/.venv/lib/python3.11/site.py"));

        // --- plain-project (no artifacts) ---
        touch(&scan_root.join("plain-project/src/lib.rs"));
        touch(&scan_root.join("plain-project/docs/readme.md"));

        // --- .git (must be fully skipped) ---
        // Even with node_modules inside, nothing from .git should surface.
        touch(&scan_root.join(".git/objects/pack/foo.pack"));
        touch(&scan_root.join(".git/node_modules/some-dep/index.js"));

        ComplexFixture {
            _root: root,
            _app: app,
            scan_root,
        }
    }

    fn run_scan(&self) -> Vec<ArtifactInfo> {
        // We need the app scratch dir to build AppPaths from, but we already
        // captured it in _app. Rebuild from its path instead.
        let app_paths = AppPaths::with_base(self._app.path().join("app"));
        let config = ScanConfig {
            paths: vec![self.scan_root.clone()],
            ..Default::default()
        };
        let coordinator = ScanCoordinator::new(config, builtin_rules(), app_paths);
        collect_found(coordinator.start())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn node_modules_is_found() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    assert!(
        found.iter().any(|a| a.path.ends_with("node_modules")),
        "node_modules should be flagged; got: {:?}",
        found.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn rust_target_is_found_with_cargo_context() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    assert!(
        found
            .iter()
            .any(|a| a.path.ends_with("target") && a.rule_id == "cargo-target"),
        "Rust target/ should be found with cargo-target rule; got: {:?}",
        found
            .iter()
            .map(|a| (&a.path, &a.rule_id))
            .collect::<Vec<_>>()
    );
}

#[test]
fn pycache_is_found() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    assert!(
        found.iter().any(|a| a.path.ends_with("__pycache__")),
        "__pycache__ should be flagged; got: {:?}",
        found.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn pytest_cache_is_found() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    assert!(
        found.iter().any(|a| a.path.ends_with(".pytest_cache")),
        ".pytest_cache should be flagged; got: {:?}",
        found.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn venv_is_found_with_requirements_context() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    assert!(
        found.iter().any(|a| a.path.ends_with(".venv")),
        ".venv should be flagged (requirements.txt present); got: {:?}",
        found.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn plain_project_not_found() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    let plain_hits: Vec<_> = found
        .iter()
        .filter(|a| a.path.to_string_lossy().contains("plain-project"))
        .collect();

    assert!(
        plain_hits.is_empty(),
        "plain-project should produce no hits; got: {:?}",
        plain_hits.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn git_directory_is_skipped() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    let git_hits: Vec<_> = found
        .iter()
        .filter(|a| a.path.components().any(|c| c.as_os_str() == ".git"))
        .collect();

    assert!(
        git_hits.is_empty(),
        ".git directory and its contents must be skipped; got: {:?}",
        git_hits.iter().map(|a| &a.path).collect::<Vec<_>>()
    );
}

#[test]
fn no_false_positives_in_complex_tree() {
    let fixture = ComplexFixture::create();
    let found = fixture.run_scan();

    // Only these paths should be found. Collect their trailing names.
    let expected_names = [
        "node_modules",
        "target",
        "__pycache__",
        ".pytest_cache",
        ".venv",
    ];

    for artifact in &found {
        let name = artifact
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        assert!(
            expected_names.contains(&name),
            "unexpected artifact found: {} (rule: {})",
            artifact.path.display(),
            artifact.rule_id
        );
    }
}

#[test]
fn complete_event_is_always_received() {
    let tmp = TempDir::new().unwrap();
    let app_paths = scratch_app_paths(&tmp);

    // Even a completely empty directory must produce a Complete event.
    let config = ScanConfig {
        paths: vec![tmp.path().to_path_buf()],
        ..Default::default()
    };
    let coordinator = ScanCoordinator::new(config, builtin_rules(), app_paths);
    let rx = coordinator.start();

    let mut complete_received = false;
    for event in rx {
        if matches!(event, ScanEvent::Complete(_)) {
            complete_received = true;
            break;
        }
    }

    assert!(complete_received, "ScanEvent::Complete was never sent");
}
