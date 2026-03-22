pub mod details;
pub mod dialog;
pub mod status_bar;
pub mod theme;
pub mod tree;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders},
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

    // ── Outer block wrapping the entire UI ──────────────────────────────
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // ── Main layout: header | body | footer ──────────────────────────────
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // body
            Constraint::Length(2), // footer
        ])
        .split(inner);

    frame.render_widget(Header { app }, main_chunks[0]);

    // ── Body: tree (65%) | details (35%) ─────────────────────────────────
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[1]);

    // Build the tree panel title showing active filters.
    let mut title_parts: Vec<String> = Vec::new();
    if app.tree.safety_filter.is_active() {
        title_parts.push(format!("showing: {}", app.tree.safety_filter.label()));
    }
    if let Some(ref q) = app.tree.search_filter {
        title_parts.push(format!("search: \"{}\"", q));
    }
    let tree_title = if title_parts.is_empty() {
        " artifacts ".to_string()
    } else {
        format!(" artifacts [{}] ", title_parts.join(" | "))
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
            render_trash_browser(area, frame.buffer_mut(), &app.trash_browser);
        }
        AppMode::Normal | AppMode::Search { .. } => {}
    }

    // ── Search query overlay in status bar ────────────────────────────────
    if let AppMode::Search { query } = &app.mode {
        let search_area = main_chunks[2];
        let prompt = format!("/ {query}_");
        use ratatui::text::Line;
        use ratatui::text::Span;
        use ratatui::widgets::Widget;
        use theme::{FOOTER_BG, FOOTER_KEY_FG};
        Line::from(vec![
            Span::styled(
                format!(" search: {prompt} "),
                Style::default().fg(FOOTER_KEY_FG).bg(FOOTER_BG),
            ),
        ])
        .render(
            ratatui::layout::Rect::new(search_area.x, search_area.y, search_area.width, 1),
            frame.buffer_mut(),
        );
    }
}
