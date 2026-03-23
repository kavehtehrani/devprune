# devprune

A terminal tool that reclaims disk space by finding and removing developer build artifacts, caches, dependencies, and other cruft.

Scans your filesystem for `node_modules`, `target/`, `.venv`, `__pycache__`, `build/`, and 50+ other artifact types across many languages and frameworks. Operates as an interactive TUI or headless CLI.

> [!NOTE]  
> This project is heavily coded with AI and is currently in beta.

## Install

### Pre-built binaries

Download from [GitHub Releases](https://github.com/kavehtehrani/devprune/releases) for:

| Platform | Target |
|----------|--------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple Silicon | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` (untested) |

### From source

Requires Rust 1.85+:

```
cargo install --path crates/devprune
```

## Usage

```
devprune [OPTIONS] [PATHS...]
```

By default, scans the current directory and opens the interactive TUI.

### Examples

```bash
# Scan current directory (TUI)
devprune

# Scan specific directories
devprune ~/projects ~/work

# Scan home directory
devprune --home

# Headless mode, JSON output
devprune --no-tui --json ~/projects

# Only show summary stats
devprune --no-tui --stats-only ~/projects

# Dry run (report only, don't delete)
devprune --dry-run ~/projects

# Auto-delete all Safe artifacts
devprune --auto --yes ~/projects

# Only dependencies larger than 100MB
devprune --category dependencies --min-size 100MB

# Only Safe artifacts, skip risky ones
devprune --safety safe
```

### Options

#### Path selection

| Flag | Description |
|------|-------------|
| `<PATHS>` | Directories to scan (default: `.`) |
| `--home` | Scan the home directory |
| `--all` | Scan from filesystem root |

#### Mode

| Flag | Description |
|------|-------------|
| `--tui` | Force TUI mode |
| `--no-tui` | Headless mode (no interactive UI) |
| `--auto` | Auto-select all Safe artifacts for deletion |
| `--yes` | Confirm automatic deletion (required with `--auto`) |
| `--dry-run` | Report what would be deleted without deleting |

#### Output

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--stats-only` | Print summary counts and totals only |

#### Filtering

| Flag | Description |
|------|-------------|
| `--category CAT` | Filter by category (repeatable) |
| `--min-size SIZE` | Minimum artifact size (e.g. `10MB`, `1GiB`) |
| `--safety LEVEL` | Maximum safety level: `safe`, `cautious`, or `risky` |
| `--exclude GLOB` | Glob patterns to exclude (repeatable) |

#### Performance

| Flag | Description |
|------|-------------|
| `--threads N` | Number of worker threads |
| `--max-depth N` | Maximum directory depth |
| `--cross-device` | Scan across filesystem boundaries |

### Trash management

Deleted items go to a devprune-managed trash, allowing restoration:

```bash
devprune trash list                    # List trashed items
devprune trash restore <ID>            # Restore by UUID
devprune trash purge                   # Permanently delete all
devprune trash purge --older-than 30d  # Purge items older than 30 days
```

## TUI

The interactive TUI shows a tree of detected artifacts grouped by category and rule, with a details panel on the right.

### Key bindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate |
| `Space`/`Enter` | Toggle selection |
| `a`/`A` | Select all / deselect all |
| `d` | Delete selected |
| `/` | Search / filter |
| `s` | Cycle sort order |
| `f` | Filter by safety level |
| `c` | Browse for scan directory |
| `t` | Open trash browser |
| `R` | Rescan |
| `?` | Help |
| `q` | Quit |

## What it detects

devprune ships with 50+ rules covering artifacts from many languages and tools. Each rule has a safety level:

- **Safe** - definitely safe to delete, recreated by build/install commands
- **Cautious** - usually safe but may need verification
- **Risky** - may contain custom configs not easily recovered

### Categories

| Category | Examples | Rules |
|----------|----------|-------|
| Dependencies | `node_modules`, `vendor`, `.bundle`, `Pods` | 9 |
| Build output | `target/`, `build/`, `dist/`, `_build/` | 14 |
| Caches | `__pycache__`, `.next`, `.gradle`, `.turbo` | 14 |
| Virtual environments | `.venv`, `venv`, `.tox`, `.nox` | 6 |
| IDE artifacts | `.idea`, `.vscode`, `.vs`, `*.swp` | 5 |
| Coverage | `coverage/`, `htmlcov/`, `.nyc_output` | 4 |
| Logs | `npm-debug.log*`, `yarn-error.log*` | 3 |
| Compiled/generated | `.eggs`, `*.egg-info`, `.terraform` | 5 |
| Misc | `.expo`, `.stack-work`, `zig-cache` | 9 |

Many rules use context markers (e.g. requiring `Cargo.toml` next to a `target/` dir) to avoid false positives on generic directory names like `build` or `dist`.
