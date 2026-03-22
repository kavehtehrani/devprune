pub mod details;
pub mod dialog;
pub mod status_bar;
pub mod theme;
pub mod tree;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders},
};

use crate::tui::app::{App, AppMode};
use crate::tui::ui::{
    details::DetailsPanel,
    dialog::{render_confirm_delete, render_help, render_trash_browser},
    status_bar::render_header_content,
    tree::{TreeWidget, TreeWidgetState},
};

/// Draw the full TUI frame.
pub fn draw(frame: &mut Frame, app: &App, tree_state: &mut TreeWidgetState) {
    let area = frame.area();

    // ── Main layout: header block | body | footer block ─────────────────
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header block (border + 1 line + border)
            Constraint::Min(0),    // body
            Constraint::Length(1), // footer (single line with top border)
        ])
        .split(area);

    // ── Header block ────────────────────────────────────────────────────
    let header_block = Block::default()
        .title(" devprune ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let header_inner = header_block.inner(main_chunks[0]);
    frame.render_widget(header_block, main_chunks[0]);
    render_header_content(frame, header_inner, app);

    // ── Body: tree (65%) | details (35%) ────────────────────────────────
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[1]);

    // Build the artifacts block title and optional bottom title for filter status.
    let tree_title = " artifacts ".to_string();
    let mut bottom_parts: Vec<Span> = Vec::new();
    if app.tree.safety_filter.is_active() {
        let safety_color = match app.tree.safety_filter {
            crate::tui::app::SafetyFilter::Safe => theme::safety_color(devprune_core::rules::types::SafetyLevel::Safe),
            crate::tui::app::SafetyFilter::Cautious => theme::safety_color(devprune_core::rules::types::SafetyLevel::Cautious),
            crate::tui::app::SafetyFilter::Risky => theme::safety_color(devprune_core::rules::types::SafetyLevel::Risky),
            crate::tui::app::SafetyFilter::All => theme::FOOTER_FG,
        };
        bottom_parts.push(Span::styled(" showing: ", Style::default().fg(theme::DIMMED)));
        bottom_parts.push(Span::styled(app.tree.safety_filter.label(), Style::default().fg(safety_color)));
        bottom_parts.push(Span::raw(" "));
    }
    if let Some(ref q) = app.tree.search_filter {
        bottom_parts.push(Span::styled(" search: ", Style::default().fg(theme::DIMMED)));
        bottom_parts.push(Span::styled(format!("\"{}\"", q), Style::default().fg(theme::SPINNER_FG)));
        bottom_parts.push(Span::raw(" "));
    }
    if let Some(ref msg) = app.status_message {
        bottom_parts.push(Span::styled(format!(" {} ", msg), Style::default().fg(theme::COMPLETE_FG)));
    }

    frame.render_stateful_widget(
        TreeWidget {
            tree: &app.tree,
            title: &tree_title,
            bottom_title: if bottom_parts.is_empty() { None } else { Some(Line::from(bottom_parts)) },
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

    // ── Footer block ────────────────────────────────────────────────────
    let hints = status_bar::mode_hints(&app.mode);
    let hint_spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::raw(" "),
                Span::styled(*key, Style::default().fg(theme::FOOTER_KEY_FG)),
                Span::raw(":"),
                Span::styled(*desc, Style::default().fg(theme::FOOTER_FG)),
            ]
        })
        .collect();

    let footer_line = Line::from(hint_spans);
    use ratatui::widgets::Widget;
    footer_line.render(main_chunks[2], frame.buffer_mut());

    // ── Overlay dialogs ─────────────────────────────────────────────────
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

    // ── Search overlay ──────────────────────────────────────────────────
    if let AppMode::Search { query } = &app.mode {
        let prompt = format!(" search: / {query}_ ");
        let search_line = Line::from(vec![
            Span::styled(prompt, Style::default().fg(theme::FOOTER_KEY_FG)),
        ]);
        search_line.render(main_chunks[2], frame.buffer_mut());
    }
}
