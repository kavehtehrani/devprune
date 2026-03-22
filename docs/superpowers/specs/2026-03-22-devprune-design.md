# devprune - Developer Artifact Cleanup Tool

## Overview

A Rust TUI application (ratatui) that scans the filesystem for developer build artifacts, caches, and dependencies that can be safely deleted to reclaim disk space. Provides a tree-based interactive browser with safety levels, a custom trash system for recovery, and a headless CLI mode for scripting.

**Platforms**: Linux and macOS only.

---

## Project Structure

Two-crate Cargo workspace:

```
devprune/
  Cargo.toml                        # [workspace]
  crates/
    devprune-core/                   # library: scanning, rules, trash, types
      Cargo.toml
      src/
        lib.rs
        scanner/
          mod.rs                     # public scanning API (ScanCoordinator)
          walker.rs                  # parallel filesystem walk via `ignore` crate
          filter.rs                  # applies rules to walked entries
        rules/
          mod.rs                     # rule loading + matching logic
          catalog.rs                 # built-in rule catalog (compiled-in defaults)
          parser.rs                  # TOML user-config rule parser
          types.rs                   # Rule, MatchCondition, Category, SafetyLevel
        trash/
          mod.rs                     # public trash API
          metadata.rs                # TrashEntry, manifest read/write
          storage.rs                 # file move, restore, purge, crash recovery
        types.rs                     # ScanResult, ArtifactInfo, ScanEvent, etc.
        error.rs                     # DevpruneError via thiserror
        config.rs                    # app configuration, XDG paths
        constants.rs                 # all hardcoded values
    devprune/                        # single binary: TUI + CLI modes
      Cargo.toml
      src/
        main.rs                      # entry point: decides TUI vs headless
        cli.rs                       # clap argument definitions
        tui/
          mod.rs
          app.rs                     # App state machine (Elm architecture)
          event.rs                   # event loop: input + scan + tick multiplexing
          ui/
            mod.rs
            tree.rs                  # tree view widget with checkboxes
            status_bar.rs            # progress bar + scan status
            details.rs               # right panel: selected item info
            dialog.rs                # confirmation dialogs, help overlay
            theme.rs                 # colors and styles
          input.rs                   # key binding dispatch
        headless/
          mod.rs                     # non-interactive execution
          output.rs                  # human-readable + JSON formatters
```

**Rationale**: Single binary avoids users managing two executables. Core lib has zero terminal dependencies, making it independently testable.

---

## Dependencies

### devprune-core

| Crate | Purpose |
|-------|---------|
| `ignore` | Parallel directory walker (same engine as ripgrep). Handles symlinks, permission errors, cross-device boundaries. |
| `rayon` | Thread pool for parallel size calculations |
| `serde` + `serde_json` | Trash metadata serialization |
| `toml` | User rule config parsing |
| `chrono` | Timestamps for trash metadata |
| `thiserror` | Ergonomic error types |
| `uuid` | Unique trash entry IDs |
| `bytesize` | Human-readable file sizes |
| `dirs` | Cross-platform XDG / platform-native directory resolution (e.g., `~/.local/share` on Linux, `~/Library/Application Support` on macOS) |
| `log` | Logging facade |

### devprune (binary)

| Crate | Purpose |
|-------|---------|
| `devprune-core` | Core library |
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal backend (input, raw mode) |
| `clap` | CLI argument parsing |
| `env_logger` | Log output (file in TUI mode, stderr in CLI) |

---

## Scanning Engine

### Parallel Walk

Uses `ignore::WalkParallel` with gitignore support **disabled** (we want to find gitignored artifacts like `node_modules/`). Configured with:
- `git_ignore(false)`, `git_global(false)`, `git_exclude(false)`
- `follow_links(false)` -- symlinks are skipped to avoid trashing symlink targets
- Thread count: `num_cpus` by default, configurable via `--threads`

### Always-Skipped Paths

These paths are hardcoded skips regardless of scan scope:

**Linux**: `/proc`, `/sys`, `/dev`, `/run`, `/tmp`, `/snap`
**macOS**: `/System`, `/Library`, `/Volumes` (Time Machine), `/private/var/vm`
**Both**: The devprune trash directory itself (`<data_dir>/devprune/trash/`), `.git/` directories (never descended into, never matched by any rule)

### Outermost Match Wins

When a matched artifact contains subdirectories that would also match rules (e.g., `node_modules/.cache/`), only the outermost match is reported. No rule is evaluated within an already-matched subtree. This prevents double-counting sizes.

### Match-at-boundary, Skip-subtree Strategy

When the walker encounters a directory matching a rule:
1. The rule is evaluated (including context markers)
2. If matched, the subtree is **not descended into** -- this is critical for performance
3. A `ScanEvent::Found(ArtifactInfo { size: None })` is sent immediately
4. A separate rayon task computes the directory size asynchronously
5. `ScanEvent::SizeUpdate { id, size }` updates the TUI when ready

### Data Flow

```
ignore::WalkParallel (N threads)
  --> filter checks rules per DirEntry
    --> mpsc::Sender<ScanEvent>
      --> TUI event loop / headless collector

ScanEvent variants:
  Found(ArtifactInfo)           # artifact discovered (size pending, includes last_modified)
  SizeUpdate { id, size }       # async size calculation complete
  Progress(ProgressInfo)        # dirs visited, artifacts found, elapsed
  Error(ScanError)              # permission denied, etc. (non-fatal)
  Complete(ScanSummary)         # scan finished
```

`ArtifactInfo` captures `last_modified` from the directory's metadata at discovery time. This is shown in the details panel and JSON output.

### Context Markers

Generic directory names only match when sibling files confirm the context:

| Directory | Required sibling(s) |
|-----------|---------------------|
| `target/` | `Cargo.toml` OR `pom.xml` OR `build.sbt` |
| `build/` | `build.gradle` OR `CMakeLists.txt` OR `setup.py` OR `package.json` |
| `dist/` | `package.json` OR `setup.py` OR `pyproject.toml` |
| `vendor/` | `composer.json` OR `go.mod` |
| `out/` | `build.gradle` OR `.classpath` |
| `coverage/` | `package.json` OR `pytest.ini` OR `setup.cfg` OR `.coveragerc` |

Unambiguous names (`node_modules/`, `__pycache__/`, `.pytest_cache/`) match unconditionally.

---

## Rule System

### Rule Structure

```rust
pub struct Rule {
    pub id: String,                    // e.g. "node_modules"
    pub name: String,                  // e.g. "Node.js Dependencies"
    pub category: Category,
    pub safety: SafetyLevel,
    pub match_condition: MatchCondition,
    pub context_markers: Vec<String>,  // sibling files that must exist
    pub description: String,
    pub enabled: bool,
}
```

**Categories**: Dependencies, BuildOutput, Cache, VirtualEnv, IdeArtifact, Coverage, Logs, CompiledGenerated, Misc

**Safety levels**:
- `Safe` -- fully regenerable from source (node_modules, target/, __pycache__)
- `Cautious` -- likely regenerable but may have local state (.venv with locally installed packages)
- `Risky` -- may contain user config or state (.vscode/, .idea/)

**Match conditions**: DirName, DirGlob, FileName, FileGlob, FileExtension

### Built-in Catalog

Compiled into the binary. Full list of detection rules:

**Dependencies:**
- `node_modules/` - Node.js (Safe)
- `bower_components/` - Bower (Safe)
- `vendor/` - PHP Composer / Go (Safe, context: composer.json / go.mod)
- `.bundle/` - Ruby Bundler (Safe)
- `Pods/` - CocoaPods (Safe, context: Podfile)
- `.pub-cache/` - Dart/Flutter (Safe)
- `elm-stuff/` - Elm (Safe)
- `jspm_packages/` - jspm (Safe)

**Build outputs:**
- `target/` - Rust/Maven/sbt (Safe, context: Cargo.toml / pom.xml / build.sbt)
- `build/` - Gradle/CMake/general (Safe, context required)
- `dist/` - JS/TS bundlers (Safe, context required)
- `out/` - various (Cautious, context required)
- `_build/` - Elixir Mix (Safe, context: mix.exs)
- `.build/` - Swift (Safe, context: Package.swift)
- `cmake-build-*/` - CLion CMake (Safe, glob match)

**Caches:**
- `__pycache__/` - Python bytecode (Safe)
- `.pytest_cache/` - pytest (Safe)
- `.mypy_cache/` - mypy (Safe)
- `.ruff_cache/` - ruff (Safe)
- `.parcel-cache/` - Parcel (Safe)
- `.turbo/` - Turborepo (Safe)
- `.next/` - Next.js (Safe, context: package.json)
- `.nuxt/` - Nuxt.js (Safe, context: package.json)
- `.angular/` - Angular CLI (Safe)
- `.svelte-kit/` - SvelteKit (Safe)
- `.gradle/caches/` - Gradle (Safe)
- `.sass-cache/` - Sass (Safe)
- `.eslintcache` - ESLint (Safe, file)
- `.stylelintcache` - Stylelint (Safe, file)

**Virtual environments:**
- `.venv/`, `venv/` - Python venv (Cautious, context: requirements.txt / pyproject.toml / setup.py)
- `env/` - Python venv (Risky, context: requirements.txt / pyproject.toml / setup.py -- `env/` is a common generic name, so it gets Risky safety level)
- `.conda/` - Conda (Cautious)
- `.tox/` - tox (Safe, context: tox.ini)
- `.nox/` - nox (Safe, context: noxfile.py)

**IDE artifacts:**
- `.idea/` - JetBrains (Risky)
- `.vscode/` - VS Code (Risky)
- `*.swp`, `*.swo` - Vim swap (Safe, file)
- `.vs/` - Visual Studio (Cautious)

**Coverage and test:**
- `coverage/` - general (Safe, context required)
- `htmlcov/` - Python coverage (Safe)
- `.nyc_output/` - NYC/Istanbul (Safe)
- `.coverage` - Python coverage file (Safe)

**Logs:**
- `npm-debug.log*` - npm (Safe, file glob)
- `yarn-debug.log*`, `yarn-error.log*` - Yarn (Safe, file glob)

**Compiled/generated:**
- `.eggs/` - Python eggs (Safe)
- `*.egg-info/` - Python egg metadata (Safe, dir glob)
- `.tsbuildinfo` - TypeScript (Safe, file)
- `.terraform/` - Terraform providers (Cautious)
- `.serverless/` - Serverless Framework (Cautious)

**Misc:**
- `.docusaurus/` - Docusaurus (Safe)
- `.expo/` - Expo/React Native (Safe)
- `.meteor/local/` - Meteor (Safe)
- `.stack-work/` - Haskell Stack (Safe)
- `.cabal-sandbox/` - Haskell Cabal (Safe)
- `_deps/` - CMake FetchContent (Safe)
- `zig-cache/`, `zig-out/`, `.zig-cache/` - Zig (Safe)

### User Configuration

Optional override file at `~/.config/devprune/rules.toml`:

```toml
# Disable built-in rules
[rules.disable]
ids = ["vscode_dir", "idea_dir"]

# Add custom rules
[[rules.custom]]
id = "my_logs"
name = "App log directories"
category = "Logs"
safety = "Safe"
match_condition = { type = "DirName", value = "logs" }
context_markers = ["package.json"]
description = "Node.js application log directories"
enabled = true
```

---

## Custom Trash System

### Directory Layout

```
<data_dir>/devprune/trash/        # resolved via `dirs` crate
  manifest.json                    # cache index (rebuildable from per-item metadata)
  items/
    <uuid>/
      metadata.json                # source of truth per item
      content/                     # the actual trashed files/directories
```

`<data_dir>` is `~/.local/share` on Linux, `~/Library/Application Support` on macOS, resolved by the `dirs` crate.

### Metadata

**items/\<uuid\>/metadata.json** (source of truth per item):
```json
{
  "id": "uuid-here",
  "original_path": "/home/user/project/node_modules",
  "trashed_at": "2026-03-22T10:30:00Z",
  "size_bytes": 356515840,
  "rule_id": "node_modules",
  "category": "Dependencies",
  "original_permissions": 493,
  "hostname": "dev-machine"
}
```

**manifest.json** is a **rebuildable cache**. It is derived entirely from the per-item `metadata.json` files. If the manifest is missing or corrupt, it is rebuilt by scanning the `items/` directory. This eliminates the entire class of manifest corruption bugs.

**Manifest writes use atomic replacement**: write to a temp file in the same directory, `fsync`, then `rename` into place. This guarantees the manifest is never half-written.

### Operations

**Trash**: Generate UUID > create `items/<uuid>/` dir > write `metadata.json` > move content via `rename()` (or copy+verify+delete for cross-device) > rebuild manifest cache.

**Restore**: Read `metadata.json` > verify original path doesn't already exist > if parent dir is missing, create it > move content back > remove item dir > rebuild manifest cache. If original path already exists, error with a message suggesting `--force` (overwrites) or `--to <path>` (restore to alternate location).

**Purge**: `rm -rf` item dir > rebuild manifest cache. Supports `--older-than <duration>`.

**Auto-purge**: Items older than 30 days (configurable). Checked at startup or via `devprune trash purge --older-than 30d`.

### Trash Size Limits

- Configurable max trash size (default: none). When exceeded, the oldest items are auto-purged to make room.
- Before a cross-device trash (copy path), check available disk space on the trash partition. Abort with a clear error if insufficient.

### Crash Recovery

Per-item `metadata.json` is the source of truth. On startup (or when manifest is missing/corrupt):
1. Scan `items/` for all UUID directories
2. Read each `metadata.json` to rebuild the manifest
3. Orphaned directories without `metadata.json` but with `content/` are logged as warnings for manual inspection
4. Orphaned `metadata.json` without `content/` means the move failed -- clean up the item dir

### Cross-Filesystem

`rename()` first. On `EXDEV`, fall back to recursive copy with per-file error checking > verify total file count and size match > delete original. Size-only verification is insufficient (a partial copy could match on some filesystems due to sparse files or preallocation).

---

## TUI Architecture

### Elm Architecture (TEA)

```rust
struct App {
    state: AppState,
    tree: TreeState,
    details: DetailsState,
    dialog: Option<Dialog>,
    scan_progress: ScanProgress,
    mode: AppMode,               // Normal, Search, ConfirmDelete, Help, TrashBrowser
}
```

The event loop multiplexes three sources:
1. **Keyboard/mouse** via crossterm (dedicated input thread)
2. **Scan events** via mpsc channel (from scanner threads)
3. **Tick timer** (100ms interval for UI refresh)

All converge into `AppEvent` enum, processed by `App::update()` which returns state mutations. `App::draw()` is a pure function of state.

### Tree Structure (3 levels)

```
Category (Dependencies, Build Outputs, ...)
  Rule Group (node_modules, vendor, target, ...)
    Individual Artifact (/home/user/project-a/node_modules, ...)
```

Checkbox states: `[x]` checked, `[ ]` unchecked, `[~]` indeterminate (some children checked). Checking a parent cascades to all children.

### Layout

```
+----------------------------------------------------------------------+
| devprune v0.1.0  Scanning: ~/code  [spinner] 1,204 dirs | 37 found | 2.1s |
+----------------------------------------------------------------------+
| [Tree Panel - 65%]                        | [Details Panel - 35%]    |
|                                           |                          |
| v [x] Dependencies (14.2 GB, 23 items)   | node_modules/            |
|   v [x] node_modules/ (12.1 GB)          | Path: ~/proj-b/node_mod  |
|       [x] ~/proj-a/node_modules  1.2 GB  | Size: 340 MB             |
|      >[x] ~/proj-b/node_modules  340 MB  | Safety: Safe             |
|       [ ] ~/proj-c/node_modules   89 MB  | Modified: 2 days ago     |
|   > [ ] vendor/ (2.1 GB, 5 items)        |                          |
| > [ ] Build Outputs (8.3 GB)             | Restore: npm install     |
| > [~] Caches (1.1 GB)                    |                          |
+----------------------------------------------------------------------+
| Space: toggle  Enter: expand  d: delete  /: search  ?: help  q: quit|
+----------------------------------------------------------------------+
```

### Key Bindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Move cursor |
| `Space` | Toggle checkbox |
| `Enter` | Expand/collapse node |
| `a` | Select all |
| `A` | Deselect all |
| `d` | Delete selected (confirmation dialog) |
| `Tab` | Switch panel focus |
| `/` | Search mode |
| `s` | Cycle sort (size, name, path) |
| `f` | Filter by safety level |
| `t` | Trash browser (see below) |
| `r` | Restore selected from trash |
| `?` | Help overlay |
| `q` / `Ctrl-C` | Quit |

### Trash Browser (`t` key)

Pressing `t` switches `AppMode` to `TrashBrowser`. The layout reuses the same two-panel structure:

- **Left panel**: List of trashed items, sorted by date (newest first). Each row shows: original path (truncated), size, trashed date, rule name. Checkboxes for multi-select.
- **Right panel**: Details for highlighted item -- full original path, trash date, size, rule, hostname.
- **Actions**: `r` to restore selected, `p` to permanently purge selected, `Esc` or `t` to return to main view.
- **Search**: `/` filters by path substring within the trash list.

### Progress Display

Since a parallel filesystem walk has no known total (we cannot know how many directories exist upfront), the header shows a **spinner** with live counters rather than a percentage bar: dirs visited, artifacts found, elapsed time. The spinner changes to a checkmark when the scan completes.

---

## CLI Interface

```
devprune [OPTIONS] [PATHS...]

Arguments:
  [PATHS...]              Directories to scan (default: current directory)

Scan scope (mutually exclusive with PATHS):
  --home                  Scan user home directory (~)
  --all                   Scan entire filesystem (/ with hardcoded skips for /proc, /sys, /dev, etc.)

Output modes:
  --tui                   Force TUI mode (default when TTY)
  --no-tui                Force headless/CLI mode
  --dry-run               Show what would be deleted, don't delete
  --json                  JSON output (implies --no-tui)
  --auto                  Non-interactive: scan and print what would be deleted (like --dry-run).
                          Combine with --yes to actually delete all Safe artifacts.
  --yes                   Skip confirmation prompt (only valid with --auto)

Filtering:
  --category <CAT>        Only show specific categories (repeatable)
  --min-size <SIZE>       Only show artifacts larger than SIZE (e.g. "100MB")
  --safety <LEVEL>        Filter by safety level: safe, cautious, risky
  --exclude <GLOB>        Exclude paths matching glob pattern (repeatable)

Scan options:
  --threads <N>           Scanner thread count (default: num_cpus)
  --max-depth <N>         Maximum directory depth
  --cross-device          Allow scanning across filesystem boundaries

Trash management:
  trash list              List items in trash
  trash restore <ID>      Restore a trashed item
  trash purge             Permanently delete all trashed items
  trash purge --older-than <DURATION>

General:
  -v, --verbose           Increase verbosity
  -q, --quiet             Suppress non-essential output
  -h, --help
  -V, --version
```

### JSON Output Format

```json
{
  "scan": {
    "paths": ["/home/user"],
    "duration_ms": 3200,
    "total_artifacts": 45,
    "total_size_bytes": 28991029248
  },
  "artifacts": [
    {
      "path": "/home/user/project-a/node_modules",
      "rule": "node_modules",
      "category": "Dependencies",
      "safety": "Safe",
      "size_bytes": 356515840,
      "last_modified": "2026-03-20T14:30:00Z"
    }
  ],
  "summary_by_category": {
    "Dependencies": { "count": 23, "size_bytes": 15300000000 },
    "BuildOutput": { "count": 12, "size_bytes": 8900000000 }
  }
}
```

---

## Error Handling

- **Scanner errors are non-fatal.** Permission denied on a directory is collected and shown in TUI as a dismissible count. Included in JSON output.
- **Trash errors are surfaced immediately.** Failed moves show which item and why.
- **Config errors are fatal at startup** with a clear message.
- **No panics in the TUI.** All fallible paths use `?` propagation.
- **Logging**: `log` + `env_logger`. TUI mode logs to `~/.local/share/devprune/devprune.log`. CLI mode logs to stderr.

---

## Testing Strategy

### Unit Tests (devprune-core)

- **Rules**: Each MatchCondition variant, context marker validation, TOML loading (valid/invalid/override), catalog integrity.
- **Scanner**: `tempfile::TempDir` with crafted directory structures. Verify correct matches and non-matches. Edge cases: symlinks, permission denied, empty dirs.
- **Trash**: Full lifecycle (trash > verify > restore > verify). Manifest rebuild after simulated crash. Cross-device path. Concurrent access.

### Integration Tests

- End-to-end scan of constructed test trees
- CLI output format (human + JSON)
- Dry-run produces output without modifying filesystem

### TUI Tests

- `App::update()` is pure state transitions, testable without a terminal
- Checkbox cascade logic
- Search filtering
- Snapshot tests via ratatui `TestBackend`

---

## Implementation Phases

### Phase 1: Core Scanning + Basic CLI
Set up workspace. Implement types, errors, constants. Build rule types and initial catalog (top 10 rules). Implement parallel walker with `ignore`. Wire up clap CLI with `--dry-run` and `--json`.

### Phase 2: Trash System
Trash directory management and manifest. Move-to-trash with metadata. Restore and purge. Crash recovery. Wire into CLI subcommands.

### Phase 3: TUI
Event loop (input + scan + tick). Tree data structure and state. Render tree with checkboxes and cascade. Details panel. Status bar with progress. Delete confirmation dialog. Search, sort, filter. Trash browser view.

### Phase 4: Polish
Fill remaining rules in catalog. User config file support. `--auto` mode. Performance tuning. macOS testing. Full test suite.

---

## Known Challenges

| Challenge | Mitigation |
|-----------|------------|
| Large dir size calculation is slow | Async size computation with "computing..." placeholder in UI |
| False positives on generic names (build/, dist/) | Context markers require sibling files to confirm |
| Full home dir scan is slow | Skip known unproductive dirs. `ignore` crate's filter_entry gives per-entry skip control |
| Cross-filesystem trash moves | Detect EXDEV, fall back to copy-verify-delete |
| TUI responsiveness during scan | Channel-based architecture; batch pending events per tick |
| Symlinks to artifacts | `follow_links(false)` -- only trash real directories |
| Hardlinks in artifacts | Moving to trash breaks hardlinks -- trashed copy becomes independent. Acceptable trade-off; mentioned in `--verbose` output when detected. |
| Rule ID consistency | Rule IDs are bare names without slashes (e.g., `node_modules` not `node_modules/`). Catalog listing uses slashes for human readability only. |
