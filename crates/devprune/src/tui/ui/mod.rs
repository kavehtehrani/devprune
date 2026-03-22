pub mod details;
pub mod dialog;
pub mod status_bar;
pub mod theme;
pub mod tree;

/// Returns `""` when `count` is 1, otherwise `"s"`.
pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

use bytesize::ByteSize;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::tui::app::{App, AppMode};
use crate::tui::ui::{
    details::DetailsPanel,
    dialog::{
        render_confirm_delete, render_confirm_quit, render_fs_browser, render_help,
        render_trash_browser,
    },
    status_bar::{render_header_content, render_proportional_bar},
    tree::{TreeWidget, TreeWidgetState},
};

/// Draw the full TUI frame.
pub fn draw(frame: &mut Frame, app: &mut App, tree_state: &mut TreeWidgetState) {
    let area = frame.area();

    // ── Main layout: path | header | body | footer ──────────────────────
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // path block (border + path line + border)
            Constraint::Length(4), // header block (border + status + proportional bar + border)
            Constraint::Min(0),    // body (tree + details)
            Constraint::Length(4), // footer block (border + 2 lines for wrapping + border)
        ])
        .split(area);

    // ── Path block ────────────────────────────────────────────────────
    let path_display = app
        .scan_paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let path_block = Block::default()
        .title(" path ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let path_inner = path_block.inner(main_chunks[0]);
    frame.render_widget(path_block, main_chunks[0]);

    let path_line = Line::from(vec![Span::styled(
        format!(" {path_display}"),
        Style::default().fg(theme::HEADER_FG),
    )]);
    path_line.render(path_inner, frame.buffer_mut());

    // ── Header block ────────────────────────────────────────────────────
    let header_block = Block::default()
        .title(" devprune ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let header_inner = header_block.inner(main_chunks[1]);
    frame.render_widget(header_block, main_chunks[1]);

    // Split header inner: line 1 = scan status, line 2 = proportional bar
    if header_inner.height >= 2 {
        let status_area = Rect::new(header_inner.x, header_inner.y, header_inner.width, 1);
        let bar_area = Rect::new(header_inner.x, header_inner.y + 1, header_inner.width, 1);
        render_header_content(frame, status_area, app);
        render_proportional_bar(frame, bar_area, app);
    } else {
        render_header_content(frame, header_inner, app);
    }

    // ── Body: tree (65%) | details (35%) ────────────────────────────────
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(main_chunks[2]);

    // Bottom-left: filter + search + status message
    let safety_color = match app.tree.safety_filter {
        crate::tui::app::SafetyFilter::Safe => {
            theme::safety_color(devprune_core::rules::types::SafetyLevel::Safe)
        }
        crate::tui::app::SafetyFilter::Cautious => {
            theme::safety_color(devprune_core::rules::types::SafetyLevel::Cautious)
        }
        crate::tui::app::SafetyFilter::Risky => {
            theme::safety_color(devprune_core::rules::types::SafetyLevel::Risky)
        }
        crate::tui::app::SafetyFilter::All => theme::FOOTER_FG,
    };
    let mut bottom_left: Vec<Span> = vec![
        Span::styled(" filter: ", Style::default().fg(theme::DIMMED)),
        Span::styled(
            app.tree.safety_filter.label(),
            Style::default().fg(safety_color),
        ),
        Span::raw(" "),
    ];
    if let Some(ref q) = app.tree.search_filter {
        bottom_left.push(Span::styled(
            " search: ",
            Style::default().fg(theme::DIMMED),
        ));
        bottom_left.push(Span::styled(
            format!("\"{}\"", q),
            Style::default().fg(theme::SPINNER_FG),
        ));
        bottom_left.push(Span::raw(" "));
    }
    if let Some(ref msg) = app.status_message {
        bottom_left.push(Span::styled(
            format!(" {} ", msg),
            Style::default().fg(theme::COMPLETE_FG),
        ));
    }

    // Bottom-right: trash stats
    let trash_text = if app.trash_stats.item_count > 0 {
        format!(
            "trash: {} item{}, {} ",
            app.trash_stats.item_count,
            plural(app.trash_stats.item_count),
            ByteSize(app.trash_stats.total_bytes),
        )
    } else {
        "trash: empty ".to_string()
    };
    let trash_color = if app.trash_stats.item_count > 0 {
        theme::FOOTER_KEY_FG
    } else {
        theme::DIMMED
    };
    let bottom_right = Line::from(vec![Span::styled(
        trash_text,
        Style::default().fg(trash_color),
    )]);

    frame.render_stateful_widget(
        TreeWidget {
            tree: &app.tree,
            title: " artifacts ",
            bottom_title: Some(Line::from(bottom_left)),
            bottom_right: Some(bottom_right),
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

    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    let footer_inner = footer_block.inner(main_chunks[3]);
    frame.render_widget(footer_block, main_chunks[3]);
    use ratatui::widgets::Paragraph;
    Paragraph::new(Line::from(hint_spans))
        .wrap(ratatui::widgets::Wrap { trim: true })
        .render(footer_inner, frame.buffer_mut());

    // ── Overlay dialogs ─────────────────────────────────────────────────
    match &app.mode {
        AppMode::ConfirmDelete => {
            render_confirm_delete(area, frame.buffer_mut(), app);
        }
        AppMode::Help => {
            render_help(area, frame.buffer_mut());
        }
        AppMode::TrashBrowser => {
            render_trash_browser(area, frame.buffer_mut(), &mut app.trash_browser);
        }
        AppMode::ConfirmQuit => {
            render_confirm_quit(area, frame.buffer_mut(), app);
        }
        AppMode::ChangePath => {
            render_fs_browser(area, frame.buffer_mut(), &mut app.fs_browser);
        }
        AppMode::Normal | AppMode::Search { .. } => {}
    }

    // ── Search overlay (replaces footer content) ──────────────────────
    if let AppMode::Search { query } = &app.mode {
        let prompt = format!("/ {query}_");
        let search_block = Block::default()
            .title(" search ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::FOOTER_KEY_FG));
        let search_inner = search_block.inner(main_chunks[3]);
        frame.render_widget(search_block, main_chunks[3]);
        ratatui::widgets::Paragraph::new(Line::from(vec![Span::styled(
            prompt,
            Style::default().fg(theme::FOOTER_KEY_FG),
        )]))
        .render(search_inner, frame.buffer_mut());
    }
}
