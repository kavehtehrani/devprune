use std::fs;
use std::process::Command;

use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helper: build a Command pointing at the compiled devprune binary.
// ---------------------------------------------------------------------------

fn devprune_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_devprune"))
}

/// Creates a minimal TempDir containing a `node_modules` directory so the
/// scanner has something to report.
fn temp_dir_with_node_modules() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("node_modules/lodash")).unwrap();
    fs::write(tmp.path().join("package.json"), "{}").unwrap();
    tmp
}

// ---------------------------------------------------------------------------
// --help
// ---------------------------------------------------------------------------

#[test]
fn help_flag_exits_zero() {
    let output = devprune_binary()
        .arg("--help")
        .output()
        .expect("failed to run devprune --help");

    assert!(
        output.status.success(),
        "--help should exit 0; got {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("devprune") || stdout.contains("Usage"),
        "--help output should mention the program name or usage; got:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// --version
// ---------------------------------------------------------------------------

#[test]
fn version_flag_exits_zero() {
    let output = devprune_binary()
        .arg("--version")
        .output()
        .expect("failed to run devprune --version");

    assert!(
        output.status.success(),
        "--version should exit 0; got {:?}",
        output.status
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("devprune"),
        "--version output should contain the program name; got:\n{combined}"
    );
}

// ---------------------------------------------------------------------------
// --dry-run <path>
// ---------------------------------------------------------------------------

#[test]
fn dry_run_finds_node_modules() {
    let tmp = temp_dir_with_node_modules();

    let output = devprune_binary()
        .arg("--dry-run")
        .arg(tmp.path())
        .output()
        .expect("failed to run devprune --dry-run");

    assert!(
        output.status.success(),
        "--dry-run should exit 0; got {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("node_modules"),
        "--dry-run output should mention node_modules; got:\n{stdout}"
    );
}

#[test]
fn dry_run_on_empty_dir_exits_zero() {
    let tmp = TempDir::new().unwrap();

    let output = devprune_binary()
        .arg("--dry-run")
        .arg(tmp.path())
        .output()
        .expect("failed to run devprune --dry-run on empty dir");

    assert!(
        output.status.success(),
        "--dry-run on empty dir should exit 0; got {:?}",
        output.status
    );
}

// ---------------------------------------------------------------------------
// --json <path>
// ---------------------------------------------------------------------------

#[test]
fn json_flag_produces_valid_json() {
    let tmp = temp_dir_with_node_modules();

    let output = devprune_binary()
        .arg("--json")
        .arg(tmp.path())
        .output()
        .expect("failed to run devprune --json");

    assert!(
        output.status.success(),
        "--json should exit 0; got {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("--json output is not valid JSON: {e}\noutput:\n{stdout}"));

    // Verify top-level keys the format guarantees.
    assert!(
        parsed.get("scan").is_some(),
        "JSON output should have a 'scan' key; got:\n{stdout}"
    );
    assert!(
        parsed.get("artifacts").is_some(),
        "JSON output should have an 'artifacts' key; got:\n{stdout}"
    );
    assert!(
        parsed.get("summary_by_category").is_some(),
        "JSON output should have a 'summary_by_category' key; got:\n{stdout}"
    );
}

#[test]
fn json_output_contains_node_modules_artifact() {
    let tmp = temp_dir_with_node_modules();

    let output = devprune_binary()
        .arg("--json")
        .arg(tmp.path())
        .output()
        .expect("failed to run devprune --json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("node_modules"),
        "JSON artifacts should include the node_modules path; got:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// trash list
// ---------------------------------------------------------------------------

#[test]
fn trash_list_exits_zero() {
    let output = devprune_binary()
        .args(["trash", "list"])
        .output()
        .expect("failed to run devprune trash list");

    assert!(
        output.status.success(),
        "`trash list` should exit 0 (empty trash is fine); got {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Invalid arguments
// ---------------------------------------------------------------------------

#[test]
fn unknown_flag_exits_nonzero() {
    let output = devprune_binary()
        .arg("--this-flag-does-not-exist")
        .output()
        .expect("failed to run devprune with unknown flag");

    assert!(
        !output.status.success(),
        "unknown flag should exit non-zero; got {:?}",
        output.status
    );
}

#[test]
fn auto_without_yes_still_exits_zero() {
    // --auto without --yes is allowed (it just prints a dry-run notice) and
    // should not crash.
    let tmp = temp_dir_with_node_modules();

    let output = devprune_binary()
        .arg("--auto")
        .arg(tmp.path())
        .output()
        .expect("failed to run devprune --auto");

    // The binary exits 0 in this case even without --yes.
    assert!(
        output.status.success(),
        "--auto without --yes should exit 0; got {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
