use bytesize::ByteSize;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, AppMode};

/// Process a single keyboard event and mutate app state accordingly.
pub fn handle_input(app: &mut App, event: Event) {
    let Event::Key(key) = event else { return };

    match &app.mode.clone() {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Search { query } => handle_search(app, key, query.clone()),
        AppMode::ConfirmDelete => handle_confirm_delete(app, key),
        AppMode::Help => handle_help(app, key),
        AppMode::TrashBrowser => handle_trash_browser(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.tree.move_cursor(1),
        KeyCode::Char('k') | KeyCode::Up => app.tree.move_cursor(-1),
        KeyCode::PageDown => app.tree.move_cursor(10),
        KeyCode::PageUp => app.tree.move_cursor(-10),

        // Check / expand
        KeyCode::Char(' ') => {
            let cursor = app.tree.cursor;
            app.tree.toggle_check(cursor);
        }
        KeyCode::Enter => {
            let cursor = app.tree.cursor;
            app.tree.toggle_expand(cursor);
        }

        // Bulk selection
        KeyCode::Char('a') => app.tree.select_all(true),
        KeyCode::Char('A') => app.tree.select_all(false),

        // Delete
        KeyCode::Char('d') => {
            let (count, _) = app.tree.selection_summary();
            if count > 0 {
                app.mode = AppMode::ConfirmDelete;
            }
        }

        // Search
        KeyCode::Char('/') => {
            app.mode = AppMode::Search { query: String::new() };
            app.tree.search_filter = None;
        }

        // Sort
        KeyCode::Char('s') => {
            app.tree.sort = app.tree.sort.next();
        }

        // Trash browser
        KeyCode::Char('t') => {
            open_trash_browser(app);
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }

        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent, mut query: String) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.tree.search_filter = None;
        }
        KeyCode::Enter => {
            app.tree.search_filter = if query.is_empty() { None } else { Some(query) };
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            query.pop();
            // Live-update filter while typing.
            app.tree.search_filter = if query.is_empty() {
                None
            } else {
                Some(query.clone())
            };
            app.mode = AppMode::Search { query };
        }
        KeyCode::Char(c) => {
            query.push(c);
            app.tree.search_filter = Some(query.clone());
            app.mode = AppMode::Search { query };
        }
        _ => {}
    }
}

fn handle_confirm_delete(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            perform_trash_delete(app);
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_trash_browser(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('t') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.trash_browser.move_cursor(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.trash_browser.move_cursor(-1);
        }
        KeyCode::Char(' ') => {
            app.trash_browser.toggle_check();
        }
        KeyCode::Char('r') => {
            perform_trash_restore(app);
        }
        KeyCode::Char('p') => {
            perform_trash_purge(app);
        }
        _ => {}
    }
}

// ── Trash operations ──────────────────────────────────────────────────────────

/// Move all checked artifacts to the devprune trash, then remove them from the
/// tree and show a status message.
fn perform_trash_delete(app: &mut App) {
    let selected = app.tree.selected_artifacts().into_iter().cloned().collect::<Vec<_>>();
    if selected.is_empty() {
        app.tree.remove_checked();
        return;
    }

    let Some(ref trash_manager) = app.trash_manager else {
        // No trash manager available – fall back to just removing from tree.
        app.tree.remove_checked();
        app.status_message = Some("Removed from view (trash unavailable)".to_string());
        return;
    };

    let mut trashed = 0usize;
    let mut freed = 0u64;
    let mut errors = 0usize;

    for artifact in &selected {
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
                log::warn!("trash: failed to trash {}: {e}", artifact.path.display());
                errors += 1;
            }
        }
    }

    app.tree.remove_checked();

    let msg = if errors == 0 {
        format!("Deleted {} item{}, freed {}", trashed, if trashed == 1 { "" } else { "s" }, ByteSize(freed))
    } else {
        format!(
            "Deleted {} item{}, freed {} ({} error{})",
            trashed,
            if trashed == 1 { "" } else { "s" },
            ByteSize(freed),
            errors,
            if errors == 1 { "" } else { "s" },
        )
    };
    app.status_message = Some(msg);
}

/// Restore all checked items in the trash browser back to their original
/// location.
fn perform_trash_restore(app: &mut App) {
    let ids = app.trash_browser.selected_ids();
    if ids.is_empty() {
        return;
    }

    let Some(ref trash_manager) = app.trash_manager else {
        app.status_message = Some("Trash unavailable".to_string());
        return;
    };

    let mut restored = 0usize;
    let mut errors = 0usize;

    for id in ids {
        match trash_manager.restore_item(id) {
            Ok(_) => restored += 1,
            Err(e) => {
                log::warn!("trash: restore {id} failed: {e}");
                errors += 1;
            }
        }
    }

    // Reload the browser after the operation.
    open_trash_browser(app);

    let msg = if errors == 0 {
        format!("Restored {} item{}", restored, if restored == 1 { "" } else { "s" })
    } else {
        format!("Restored {} item{} ({} failed)", restored, if restored == 1 { "" } else { "s" }, errors)
    };
    app.status_message = Some(msg);
}

/// Permanently purge all checked items from the trash browser.
fn perform_trash_purge(app: &mut App) {
    let ids = app.trash_browser.selected_ids();
    if ids.is_empty() {
        return;
    }

    let Some(ref trash_manager) = app.trash_manager else {
        app.status_message = Some("Trash unavailable".to_string());
        return;
    };

    let mut purged = 0usize;
    let mut errors = 0usize;

    for id in ids {
        match trash_manager.purge_item(id) {
            Ok(()) => purged += 1,
            Err(e) => {
                log::warn!("trash: purge {id} failed: {e}");
                errors += 1;
            }
        }
    }

    // Reload the browser after the operation.
    open_trash_browser(app);

    let msg = if errors == 0 {
        format!("Purged {} item{}", purged, if purged == 1 { "" } else { "s" })
    } else {
        format!("Purged {} item{} ({} failed)", purged, if purged == 1 { "" } else { "s" }, errors)
    };
    app.status_message = Some(msg);
}

/// Load the trash list and switch to the TrashBrowser mode.
fn open_trash_browser(app: &mut App) {
    let items = app
        .trash_manager
        .as_ref()
        .and_then(|tm| tm.list_items().ok())
        .unwrap_or_default();
    app.trash_browser.load(items);
    app.mode = AppMode::TrashBrowser;
}
