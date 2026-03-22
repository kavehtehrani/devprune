use bytesize::ByteSize;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use uuid::Uuid;

use super::app::{App, AppMode};

/// Process a single keyboard event and mutate app state accordingly.
pub fn handle_input(app: &mut App, event: Event) {
    let Event::Key(key) = event else { return };

    match &app.mode {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Search { query } => {
            let query = query.clone(); // only clone the string we need
            handle_search(app, key, query);
        }
        AppMode::ConfirmDelete => handle_confirm_delete(app, key),
        AppMode::ConfirmQuit => handle_confirm_quit(app, key),
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

        // Check
        KeyCode::Char(' ') | KeyCode::Enter => {
            let cursor = app.tree.cursor;
            app.tree.toggle_check(cursor);
        }

        // Expand / collapse tree nodes
        KeyCode::Right => {
            let cursor = app.tree.cursor;
            app.tree.expand(cursor);
        }
        KeyCode::Left => {
            let cursor = app.tree.cursor;
            app.tree.collapse(cursor);
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

        // Safety filter
        KeyCode::Char('f') => {
            app.tree.safety_filter = app.tree.safety_filter.next();
        }

        // Trash browser
        KeyCode::Char('t') => {
            open_trash_browser(app);
        }

        // Rescan
        KeyCode::Char('R') => {
            app.rescan_requested = true;
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }

        // Quit
        KeyCode::Char('q') => try_quit(app),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => try_quit(app),

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

fn try_quit(app: &mut App) {
    if app.trash_stats.item_count > 0 {
        app.mode = AppMode::ConfirmQuit;
    } else {
        app.should_quit = true;
    }
}

fn handle_confirm_quit(app: &mut App, key: KeyEvent) {
    match key.code {
        // Quit anyway
        KeyCode::Char('q') | KeyCode::Char('y') => {
            app.should_quit = true;
        }
        // Open trash browser
        KeyCode::Char('t') => {
            open_trash_browser(app);
        }
        // Cancel, go back
        KeyCode::Esc | KeyCode::Char('n') => {
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
        KeyCode::PageDown => {
            app.trash_browser.move_cursor(10);
        }
        KeyCode::PageUp => {
            app.trash_browser.move_cursor(-10);
        }
        KeyCode::Char(' ') => {
            app.trash_browser.toggle_check();
        }
        KeyCode::Char('a') => {
            app.trash_browser.select_all(true);
        }
        KeyCode::Char('A') => {
            app.trash_browser.select_all(false);
        }
        KeyCode::Char('s') => {
            app.trash_browser.cycle_sort();
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

/// Returns `""` when `count` is 1, otherwise `"s"`.
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Shared helper for trash manager operations that iterate over a list of IDs,
/// call a per-item function, track success/error counts, then refresh stats.
///
/// `items` is the list of IDs to process. `op` is called for each ID and
/// returns a `Result`. `post_refresh` is called after all items are processed
/// (e.g. to reload the trash browser). Returns `(success_count, error_count)`.
fn run_trash_op<F, P>(
    app: &mut App,
    items: Vec<Uuid>,
    mut op: F,
    post_refresh: P,
) -> (usize, usize)
where
    F: FnMut(&devprune_core::trash::storage::TrashManager, Uuid) -> devprune_core::error::Result<()>,
    P: FnOnce(&mut App),
{
    let Some(ref trash_manager) = app.trash_manager else {
        app.set_status_message("Trash unavailable".to_string());
        return (0, 0);
    };

    let mut ok = 0usize;
    let mut err = 0usize;

    for id in items {
        match op(trash_manager, id) {
            Ok(()) => ok += 1,
            Err(e) => {
                log::warn!("trash op failed for {id}: {e}");
                err += 1;
            }
        }
    }

    app.refresh_trash_stats();
    post_refresh(app);
    (ok, err)
}

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
        app.set_status_message("Removed from view (trash unavailable)".to_string());
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
    app.refresh_trash_stats();

    let msg = if errors == 0 {
        format!("Deleted {} item{}, freed {}", trashed, plural(trashed), ByteSize(freed))
    } else {
        format!(
            "Deleted {} item{}, freed {} ({} error{})",
            trashed,
            plural(trashed),
            ByteSize(freed),
            errors,
            plural(errors),
        )
    };
    app.set_status_message(msg);
}

/// Restore all checked items in the trash browser back to their original
/// location.
fn perform_trash_restore(app: &mut App) {
    let ids = app.trash_browser.selected_ids();
    if ids.is_empty() {
        return;
    }

    let (restored, errors) = run_trash_op(
        app,
        ids,
        |tm, id| tm.restore_item(id).map(|_| ()),
        open_trash_browser,
    );

    let msg = if errors == 0 {
        format!("Restored {} item{}", restored, plural(restored))
    } else {
        format!("Restored {} item{} ({} failed)", restored, plural(restored), errors)
    };
    app.set_status_message(msg);
}

/// Permanently purge all checked items from the trash browser.
fn perform_trash_purge(app: &mut App) {
    let ids = app.trash_browser.selected_ids();
    if ids.is_empty() {
        return;
    }

    let (purged, errors) = run_trash_op(
        app,
        ids,
        |tm, id| tm.purge_item(id),
        open_trash_browser,
    );

    let msg = if errors == 0 {
        format!("Purged {} item{}", purged, plural(purged))
    } else {
        format!("Purged {} item{} ({} failed)", purged, plural(purged), errors)
    };
    app.set_status_message(msg);
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
