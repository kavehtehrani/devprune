mod cli;
mod tui;

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use bytesize::ByteSize;
use clap::Parser;
use serde::Serialize;
use uuid::Uuid;

use devprune_core::config::AppPaths;
use devprune_core::rules::catalog::builtin_rules;
use devprune_core::rules::parser::load_user_rules;
use devprune_core::rules::types::{Category, Rule, SafetyLevel};
use devprune_core::scanner::ScanCoordinator;
use devprune_core::trash::storage::TrashManager;
use devprune_core::types::{ArtifactInfo, ScanConfig, ScanEvent};

use cli::{Cli, TrashAction, TrashCommand};

fn main() {
    let cli = Cli::parse();
    init_logger(cli.verbose, cli.quiet);

    // Handle trash subcommands before anything else.
    if let Some(TrashCommand::Trash { ref action }) = cli.trash {
        match run_trash_command(action, &cli) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.use_tui() {
        let app_paths = match devprune_core::config::AppPaths::resolve() {
            Some(p) => p,
            None => {
                eprintln!("error: could not resolve application data directories");
                std::process::exit(1);
            }
        };
        let scan_config = match build_scan_config(&cli, &app_paths) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        };
        let base_rules = match load_user_rules(&app_paths.config_dir, builtin_rules()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: could not load user rules: {e}");
                builtin_rules()
            }
        };
        let rules = filter_rules(&cli, base_rules);
        match tui::run_tui(scan_config, rules, app_paths) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let result = if cli.auto {
        run_auto(&cli)
    } else {
        run_headless(&cli)
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Shared scan helper
// ---------------------------------------------------------------------------

/// Run the scan described by `config` and `rules` and return the collected
/// artifacts together with the scan duration in milliseconds.
///
/// Handles: creating the coordinator, driving the event loop, applying size
/// updates, and applying the optional `--min-size` filter.
fn collect_scan_results(
    config: ScanConfig,
    rules: Vec<Rule>,
    app_paths: AppPaths,
    cli: &Cli,
) -> anyhow::Result<(Vec<ArtifactInfo>, u128)> {
    let coordinator = ScanCoordinator::new(config, rules, app_paths);
    let rx = coordinator.start();

    let mut artifacts: Vec<ArtifactInfo> = Vec::new();
    let mut size_updates: HashMap<Uuid, u64> = HashMap::new();
    let mut duration_ms = 0u128;

    for event in rx {
        match event {
            ScanEvent::Found(a) => artifacts.push(a),
            ScanEvent::SizeUpdate { id, size } => {
                size_updates.insert(id, size);
            }
            ScanEvent::Complete(summary) => {
                duration_ms = summary.duration.as_millis();
            }
            ScanEvent::Error(e) => {
                if !cli.quiet {
                    eprintln!("warning: {e}");
                }
            }
            ScanEvent::Progress(_) => {}
        }
    }

    // Apply size updates back to artifacts.
    for artifact in &mut artifacts {
        if let Some(&size) = size_updates.get(&artifact.id) {
            artifact.size = Some(size);
        }
    }

    // Apply --min-size filter post-collection (size is computed asynchronously).
    if let Some(min_bytes) = parse_min_size(cli)? {
        artifacts.retain(|a| a.size.map(|s| s >= min_bytes).unwrap_or(false));
    }

    Ok((artifacts, duration_ms))
}

// ---------------------------------------------------------------------------
// Headless scan
// ---------------------------------------------------------------------------

fn run_headless(cli: &Cli) -> anyhow::Result<()> {
    let app_paths = AppPaths::resolve().ok_or_else(|| {
        anyhow::anyhow!("could not resolve application data directories")
    })?;

    let scan_config = build_scan_config(cli, &app_paths)?;
    let base_rules = load_user_rules(&app_paths.config_dir, builtin_rules()).unwrap_or_else(|e| {
        if !cli.quiet {
            eprintln!("warning: could not load user rules: {e}");
        }
        builtin_rules()
    });
    let rules = filter_rules(cli, base_rules);

    let (artifacts, duration_ms) = collect_scan_results(scan_config, rules, app_paths, cli)?;

    if cli.json {
        print_json(cli, &artifacts, duration_ms)?;
    } else {
        print_human(cli, &artifacts, duration_ms);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Auto mode
// ---------------------------------------------------------------------------

fn run_auto(cli: &Cli) -> anyhow::Result<()> {
    let app_paths = AppPaths::resolve().ok_or_else(|| {
        anyhow::anyhow!("could not resolve application data directories")
    })?;

    let scan_config = build_scan_config(cli, &app_paths)?;
    let base_rules = load_user_rules(&app_paths.config_dir, builtin_rules()).unwrap_or_else(|e| {
        if !cli.quiet {
            eprintln!("warning: could not load user rules: {e}");
        }
        builtin_rules()
    });
    // --auto always restricts to Safe artifacts regardless of other flags.
    let mut rules = filter_rules(cli, base_rules);
    rules.retain(|r| r.safety == SafetyLevel::Safe);

    let (artifacts, duration_ms) =
        collect_scan_results(scan_config, rules, app_paths.clone(), cli)?;

    if artifacts.is_empty() {
        if !cli.quiet {
            println!("No Safe artifacts found.");
        }
        return Ok(());
    }

    let total_size: u64 = artifacts.iter().filter_map(|a| a.size).sum();

    if !cli.quiet {
        println!(
            "Found {} Safe artifact{} totalling {} in {}ms",
            artifacts.len(),
            plural(artifacts.len()),
            ByteSize(total_size),
            duration_ms,
        );
        for a in &artifacts {
            let size_str = a.size.map(|s| ByteSize(s).to_string()).unwrap_or_else(|| "?".to_string());
            println!("  {}  {}", size_str, a.path.display());
        }
    }

    if cli.yes {
        // Actually trash everything.
        let trash_manager = TrashManager::new(app_paths)
            .map_err(|e| anyhow::anyhow!("could not initialise trash: {e}"))?;

        let mut trashed = 0usize;
        let mut freed = 0u64;
        let mut errors = 0usize;

        for artifact in &artifacts {
            match trash_manager.trash_item(
                &artifact.path,
                artifact.size.unwrap_or(0),
                &artifact.rule_id,
                artifact.category,
            ) {
                Ok(_) => {
                    trashed += 1;
                    freed += artifact.size.unwrap_or(0);
                }
                Err(e) => {
                    if !cli.quiet {
                        eprintln!("warning: failed to trash {}: {e}", artifact.path.display());
                    }
                    errors += 1;
                }
            }
        }

        if !cli.quiet {
            if errors == 0 {
                println!(
                    "Deleted {} item{}, freed {}.",
                    trashed,
                    plural(trashed),
                    ByteSize(freed),
                );
            } else {
                println!(
                    "Deleted {} item{}, freed {} ({} error{}).",
                    trashed,
                    plural(trashed),
                    ByteSize(freed),
                    errors,
                    plural(errors),
                );
            }
        }
    } else if !cli.quiet {
        println!("(dry-run: pass --yes to actually delete)");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Config builders
// ---------------------------------------------------------------------------

fn build_scan_config(cli: &Cli, _app_paths: &AppPaths) -> anyhow::Result<ScanConfig> {
    let paths = cli
        .effective_paths()
        .into_iter()
        .map(|p| {
            if p == PathBuf::from(".") {
                std::env::current_dir().unwrap_or(p)
            } else {
                p
            }
        })
        .collect();

    let skip_paths = cli
        .excludes
        .iter()
        .map(|s| PathBuf::from(s))
        .collect::<Vec<_>>();

    let max_depth = cli.max_depth.map(|d| d as u32);

    Ok(ScanConfig {
        paths,
        max_depth,
        follow_symlinks: cli.cross_device,
        include_categories: vec![],
        exclude_categories: vec![],
        min_size_bytes: None, // applied post-scan after sizes are resolved
        skip_paths,
    })
}

/// Filters the built-in rules by the requested categories and safety level.
fn filter_rules(
    cli: &Cli,
    mut rules: Vec<devprune_core::rules::types::Rule>,
) -> Vec<devprune_core::rules::types::Rule> {
    if !cli.categories.is_empty() {
        let requested: Vec<Category> = cli
            .categories
            .iter()
            .filter_map(|s| parse_category(s))
            .collect();
        if !requested.is_empty() {
            rules.retain(|r| requested.contains(&r.category));
        }
    }

    if let Some(ref level_str) = cli.safety {
        if let Some(max_safety) = parse_safety_level(level_str) {
            rules.retain(|r| r.safety <= max_safety);
        }
    }

    rules
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_min_size(cli: &Cli) -> anyhow::Result<Option<u64>> {
    match &cli.min_size {
        None => Ok(None),
        Some(s) => {
            let size: ByteSize = s
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid --min-size value: {s}"))?;
            Ok(Some(size.as_u64()))
        }
    }
}

fn parse_category(s: &str) -> Option<Category> {
    match s.to_lowercase().as_str() {
        "dependencies" => Some(Category::Dependencies),
        "buildoutput" | "build-output" | "build_output" => Some(Category::BuildOutput),
        "cache" => Some(Category::Cache),
        "virtualenv" | "virtual-env" | "virtual_env" => Some(Category::VirtualEnv),
        "ideartifact" | "ide-artifact" | "ide_artifact" => Some(Category::IdeArtifact),
        "coverage" => Some(Category::Coverage),
        "logs" => Some(Category::Logs),
        "compiledgenerated" | "compiled-generated" | "compiled_generated" => {
            Some(Category::CompiledGenerated)
        }
        "misc" => Some(Category::Misc),
        _ => None,
    }
}

fn parse_safety_level(s: &str) -> Option<SafetyLevel> {
    match s.to_lowercase().as_str() {
        "safe" => Some(SafetyLevel::Safe),
        "cautious" => Some(SafetyLevel::Cautious),
        "risky" => Some(SafetyLevel::Risky),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Pluralization
// ---------------------------------------------------------------------------

/// Returns `""` when `count` is 1, otherwise `"s"`.
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_human(cli: &Cli, artifacts: &[ArtifactInfo], duration_ms: u128) {
    if cli.quiet {
        return;
    }

    if artifacts.is_empty() {
        println!("No artifacts found.");
        return;
    }

    // Group by category for readable output.
    let mut by_category: HashMap<Category, Vec<&ArtifactInfo>> = HashMap::new();
    for a in artifacts {
        by_category.entry(a.category).or_default().push(a);
    }

    let total_size: u64 = artifacts.iter().filter_map(|a| a.size).sum();

    // Print in a deterministic order.
    let mut categories: Vec<Category> = by_category.keys().copied().collect();
    categories.sort_by_key(|c| c.display_name());

    for cat in &categories {
        let group = &by_category[cat];
        let group_size: u64 = group.iter().filter_map(|a| a.size).sum();
        println!(
            "\n{} ({} item{}, {})",
            cat.display_name(),
            group.len(),
            plural(group.len()),
            ByteSize(group_size)
        );
        if !cli.stats_only {
            println!("{}", "-".repeat(60));
            for a in group.iter() {
                let size_str = a
                    .size
                    .map(|s| ByteSize(s).to_string())
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  [{:<8}]  {}  {}",
                    a.safety.display_name(),
                    size_str,
                    a.path.display()
                );
            }
        }
    }

    println!();
    println!(
        "Found {} artifact{} totalling {} in {}ms",
        artifacts.len(),
        plural(artifacts.len()),
        ByteSize(total_size),
        duration_ms
    );

    if cli.dry_run {
        println!("(dry-run: nothing was deleted)");
    }
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonOutput {
    scan: JsonScan,
    artifacts: Vec<JsonArtifact>,
    summary_by_category: HashMap<String, JsonCategorySummary>,
}

#[derive(Serialize)]
struct JsonScan {
    paths: Vec<String>,
    duration_ms: u128,
    total_artifacts: usize,
    total_size_bytes: u64,
}

#[derive(Serialize)]
struct JsonArtifact {
    id: String,
    path: String,
    rule_id: String,
    rule_name: String,
    category: String,
    safety: String,
    size_bytes: Option<u64>,
    is_directory: bool,
}

#[derive(Serialize)]
struct JsonCategorySummary {
    count: usize,
    size_bytes: u64,
}

fn print_json(cli: &Cli, artifacts: &[ArtifactInfo], duration_ms: u128) -> anyhow::Result<()> {
    let total_size: u64 = artifacts.iter().filter_map(|a| a.size).sum();

    let scan_paths: Vec<String> = cli
        .effective_paths()
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let mut summary_by_category: HashMap<String, JsonCategorySummary> = HashMap::new();
    for a in artifacts {
        let key = a.category.display_name().to_string();
        let entry = summary_by_category.entry(key).or_insert(JsonCategorySummary {
            count: 0,
            size_bytes: 0,
        });
        entry.count += 1;
        entry.size_bytes += a.size.unwrap_or(0);
    }

    let json_artifacts: Vec<JsonArtifact> = artifacts
        .iter()
        .map(|a| JsonArtifact {
            id: a.id.to_string(),
            path: a.path.display().to_string(),
            rule_id: a.rule_id.clone(),
            rule_name: a.rule_name.clone(),
            category: a.category.display_name().to_string(),
            safety: a.safety.display_name().to_string(),
            size_bytes: a.size,
            is_directory: a.is_directory,
        })
        .collect();

    let output = JsonOutput {
        scan: JsonScan {
            paths: scan_paths,
            duration_ms,
            total_artifacts: artifacts.len(),
            total_size_bytes: total_size,
        },
        artifacts: json_artifacts,
        summary_by_category,
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &output)?;
    writeln!(handle)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Trash CLI commands
// ---------------------------------------------------------------------------

fn run_trash_command(action: &TrashAction, cli: &Cli) -> anyhow::Result<()> {
    let app_paths = AppPaths::resolve()
        .ok_or_else(|| anyhow::anyhow!("could not resolve application data directories"))?;
    let trash_manager = TrashManager::new(app_paths)
        .map_err(|e| anyhow::anyhow!("could not initialise trash: {e}"))?;

    match action {
        TrashAction::List => {
            let items = trash_manager
                .list_items()
                .map_err(|e| anyhow::anyhow!("failed to list trash: {e}"))?;

            if items.is_empty() {
                println!("Trash is empty.");
                return Ok(());
            }

            println!(
                "{:<36}  {:<9}  {:<20}  {:<18}  {}",
                "ID", "Size", "Trashed At", "Category", "Original Path"
            );
            println!("{}", "-".repeat(110));
            for item in &items {
                println!(
                    "{:<36}  {:>9}  {:<20}  {:<18}  {}",
                    item.id,
                    ByteSize(item.size_bytes).to_string(),
                    item.trashed_at.format("%Y-%m-%d %H:%M UTC"),
                    item.category.display_name(),
                    item.original_path.display(),
                );
            }
            println!();
            let total: u64 = items.iter().map(|i| i.size_bytes).sum();
            println!(
                "{} item{}, {} total",
                items.len(),
                plural(items.len()),
                ByteSize(total)
            );
        }

        TrashAction::Restore { id } => {
            let uuid: Uuid = id
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid UUID: {id}"))?;
            let restored = trash_manager
                .restore_item(uuid)
                .map_err(|e| anyhow::anyhow!("restore failed: {e}"))?;
            if !cli.quiet {
                println!("Restored to {}", restored.display());
            }
        }

        TrashAction::Purge { older_than } => {
            if let Some(duration_str) = older_than {
                let days = parse_duration_days(duration_str)
                    .ok_or_else(|| anyhow::anyhow!("invalid duration: {duration_str} (expected e.g. 30d)"))?;
                let purged = trash_manager
                    .purge_older_than(days)
                    .map_err(|e| anyhow::anyhow!("purge failed: {e}"))?;
                if !cli.quiet {
                    println!(
                        "Purged {} item{} older than {days} day{}.",
                        purged.len(),
                        plural(purged.len()),
                        plural(days as usize),
                    );
                }
            } else {
                // Purge everything.
                let items = trash_manager
                    .list_items()
                    .map_err(|e| anyhow::anyhow!("failed to list trash: {e}"))?;
                let mut count = 0usize;
                for item in &items {
                    trash_manager
                        .purge_item(item.id)
                        .map_err(|e| anyhow::anyhow!("purge failed for {}: {e}", item.id))?;
                    count += 1;
                }
                if !cli.quiet {
                    println!(
                        "Purged {} item{}.",
                        count,
                        plural(count)
                    );
                }
            }
        }
    }

    Ok(())
}

/// Parse a duration string of the form "Nd" (e.g. "30d", "7d") and return the
/// number of days. Returns `None` when the string cannot be parsed.
fn parse_duration_days(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.ends_with('d') || s.ends_with('D') {
        s[..s.len() - 1].parse::<u64>().ok()
    } else {
        // Allow bare numbers too (treat as days).
        s.parse::<u64>().ok()
    }
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

fn init_logger(verbose: u8, quiet: bool) {
    if quiet {
        return;
    }
    let level = match verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .init();
}
