pub mod details;
pub mod dialog;
pub mod status_bar;
pub mod theme;
pub mod tree;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::tui::app::{App, AppMode};
use crate::tui::ui::{
    details::DetailsPanel,
    dialog::{render_confirm_delete, render_help, render_trash_browser},
    status_bar::{Footer, Header},
    tree::{TreeWidget, TreeWidgetState},
};

/// Draw the full TUI frame.
pub fn draw(frame: &mut Frame, app: &App, tree_state: &mut TreeWidgetState) {
    let area = frame.area();

    // ── Main layout: header | body | footer ──────────────────────────────
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // body
            Constraint::Length(2), // footer
        ])
        .split(area);

    frame.render_widget(Header { app }, main_chunks[0]);

    // ── Body: tree (65%) | details (35%) ─────────────────────────────────
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[1]);

    // Determine the title suffix for the tree panel (shows active sort / filter).
    let sort_label = app.tree.sort.label();
    let tree_title = if let Some(ref q) = app.tree.search_filter {
        format!(" Artifacts [sort:{sort_label}  filter:\"{q}\"] ")
    } else {
        format!(" Artifacts [sort:{sort_label}] ")
    };

    frame.render_stateful_widget(
        TreeWidget {
            tree: &app.tree,
            title: &tree_title,
        },
        body_chunks[0],
        tree_state,
    );

    frame.render_widget(
        DetailsPanel {
            artifact: app.tree.cursor_artifact(),
        },
        body_chunks[1],
    );

    frame.render_widget(Footer { app }, main_chunks[2]);

    // ── Overlay dialogs ───────────────────────────────────────────────────
    match &app.mode {
        AppMode::ConfirmDelete => {
            render_confirm_delete(area, frame.buffer_mut(), app);
        }
        AppMode::Help => {
            render_help(area, frame.buffer_mut());
        }
        AppMode::TrashBrowser => {
            render_trash_browser(area, frame.buffer_mut());
        }
        AppMode::Normal | AppMode::Search { .. } => {}
    }

    // ── Search query overlay in status bar ────────────────────────────────
    if let AppMode::Search { query } = &app.mode {
        let search_area = main_chunks[2];
        // Render the live search prompt over the footer first line.
        let prompt = format!("/ {query}_");
        use ratatui::style::Style;
        use ratatui::text::Line;
        use ratatui::widgets::Widget;
        use theme::{FOOTER_BG, FOOTER_KEY_FG};
        Line::from(vec![
            ratatui::text::Span::styled(
                format!(" Search: {prompt} "),
                Style::default().fg(FOOTER_KEY_FG).bg(FOOTER_BG),
            ),
        ])
        .render(
            ratatui::layout::Rect::new(search_area.x, search_area.y, search_area.width, 1),
            frame.buffer_mut(),
        );
    }
}
