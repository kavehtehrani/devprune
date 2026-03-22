pub mod app;
pub mod event;
pub mod input;
pub mod ui;

use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use devprune_core::config::AppPaths;
use devprune_core::rules::types::Rule;
use devprune_core::scanner::ScanCoordinator;
use devprune_core::trash::storage::TrashManager;
use devprune_core::types::{ScanConfig, ScanEvent};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::tui::{
    app::App,
    event::{AppEvent, next_event},
    input::handle_input,
    ui::{draw, tree::TreeWidgetState},
};

pub fn run_tui(config: ScanConfig, rules: Vec<Rule>, app_paths: AppPaths) -> anyhow::Result<()> {
    // ── Set up terminal ───────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── Start scan ────────────────────────────────────────────────────────
    let coordinator = ScanCoordinator::new(config, rules, app_paths.clone());
    let scan_rx = coordinator.start();

    // ── Trash manager ─────────────────────────────────────────────────────
    let trash_manager = TrashManager::new(app_paths.clone()).ok();

    // ── App state ─────────────────────────────────────────────────────────
    let mut app = App::new(app_paths, trash_manager);
    let mut tree_state = TreeWidgetState::default();

    // ── Event loop ────────────────────────────────────────────────────────
    let result = run_loop(&mut terminal, &mut app, &mut tree_state, scan_rx);

    // ── Restore terminal unconditionally ──────────────────────────────────
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tree_state: &mut TreeWidgetState,
    scan_rx: std::sync::mpsc::Receiver<ScanEvent>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app, tree_state))?;

        match next_event(&scan_rx) {
            Some(AppEvent::Input(event)) => handle_input(app, event),
            Some(AppEvent::Scan(ev)) => handle_scan_event(app, ev),
            Some(AppEvent::Tick) => app.on_tick(),
            None => {}
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_scan_event(app: &mut App, event: ScanEvent) {
    match event {
        ScanEvent::Found(artifact) => {
            app.scan_progress.artifacts_found += 1;
            app.tree.add_artifact(artifact);
        }
        ScanEvent::SizeUpdate { id, size } => {
            app.tree.update_size(id, size);
        }
        ScanEvent::Progress(info) => {
            app.scan_progress.dirs_visited = info.dirs_visited;
            app.scan_progress.artifacts_found = info.artifacts_found;
            app.scan_progress.elapsed_ms = info.elapsed.as_millis() as u64;
        }
        ScanEvent::Error(e) => {
            app.scan_errors.push(e.to_string());
        }
        ScanEvent::Complete(_summary) => {
            app.scan_complete = true;
        }
    }
}
