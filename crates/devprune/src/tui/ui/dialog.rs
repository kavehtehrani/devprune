use bytesize::ByteSize;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::tui::app::App;
use crate::tui::ui::theme;

/// Renders a centred dialog box over the current frame.
pub fn render_confirm_delete(frame_area: Rect, buf: &mut Buffer, app: &App) {
    let (count, size) = app.tree.selection_summary();
    let dialog_width = 50u16.min(frame_area.width.saturating_sub(4));
    let dialog_height = 6u16;

    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let block = Block::default()
        .title(" Confirm Delete ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIALOG_BORDER))
        .style(Style::default().bg(theme::DIALOG_BG));

    let inner = block.inner(area);
    block.render(area, buf);

    let msg = format!(
        "Delete {} item{} ({})?",
        count,
        if count == 1 { "" } else { "s" },
        ByteSize(size)
    );

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            msg,
            Style::default()
                .fg(theme::DIALOG_TITLE)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]es", Style::default().fg(theme::FOOTER_KEY_FG)),
            Span::raw("  /  "),
            Span::styled("[n]o", Style::default().fg(theme::FOOTER_KEY_FG)),
        ]),
    ];

    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(inner, buf);
}

pub fn render_help(frame_area: Rect, buf: &mut Buffer) {
    let dialog_width = 60u16.min(frame_area.width.saturating_sub(4));
    let dialog_height = 26u16.min(frame_area.height.saturating_sub(4));
    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let block = Block::default()
        .title(" Help - Key Bindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIALOG_BORDER))
        .style(Style::default().bg(theme::DIALOG_BG));

    let inner = block.inner(area);
    block.render(area, buf);

    let bindings: &[(&str, &str)] = &[
        ("Navigation", ""),
        ("  j / Down", "Move cursor down"),
        ("  k / Up", "Move cursor up"),
        ("  PgDn / PgUp", "Move 10 rows"),
        ("", ""),
        ("Selection", ""),
        ("  Space", "Toggle item check"),
        ("  Enter", "Expand / collapse"),
        ("  a", "Select all"),
        ("  A", "Deselect all"),
        ("", ""),
        ("Actions", ""),
        ("  d", "Delete selected items"),
        ("  /", "Filter / search"),
        ("  s", "Cycle sort order"),
        ("  t", "Open trash browser"),
        ("", ""),
        ("General", ""),
        ("  ?", "Toggle this help"),
        ("  q / Ctrl-C", "Quit"),
        ("", ""),
        ("Press q, Esc, or ? to close", ""),
    ];

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() {
                if key.is_empty() {
                    Line::from("")
                } else {
                    // Section header
                    Line::from(vec![Span::styled(
                        *key,
                        Style::default()
                            .fg(theme::HEADER_FG)
                            .add_modifier(Modifier::UNDERLINED),
                    )])
                }
            } else {
                Line::from(vec![
                    Span::styled(format!("{key:<20}", key = key), Style::default().fg(theme::FOOTER_KEY_FG)),
                    Span::styled(*desc, Style::default().fg(theme::FOREGROUND)),
                ])
            }
        })
        .collect();

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(inner, buf);
}

pub fn render_trash_browser(frame_area: Rect, buf: &mut Buffer) {
    let dialog_width = 60u16.min(frame_area.width.saturating_sub(4));
    let dialog_height = 12u16.min(frame_area.height.saturating_sub(4));
    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let block = Block::default()
        .title(" Trash Browser ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIALOG_BORDER))
        .style(Style::default().bg(theme::DIALOG_BG));

    let inner = block.inner(area);
    block.render(area, buf);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Trash integration coming in a future release.",
            Style::default().fg(theme::DIMMED),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Esc / t", Style::default().fg(theme::FOOTER_KEY_FG)),
            Span::raw(" — return to main view"),
        ]),
    ];

    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .render(inner, buf);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a centred `Rect` of the given dimensions, clamped to `r`.
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(
        x.min(r.x + r.width.saturating_sub(1)),
        y.min(r.y + r.height.saturating_sub(1)),
        width.min(r.width),
        height.min(r.height),
    )
}
