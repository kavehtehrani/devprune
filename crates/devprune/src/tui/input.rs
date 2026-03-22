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
            app.mode = AppMode::TrashBrowser;
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
            // Stub: remove items from tree until trash integration lands.
            app.tree.remove_checked();
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
        // Navigation, restore, purge are stubs for now.
        _ => {}
    }
}
