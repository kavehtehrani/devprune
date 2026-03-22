use bytesize::ByteSize;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::tui::app::{App, TrashBrowserState};
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
        ("  Space / Enter", "Toggle item check"),
        ("  l / Right", "Expand node"),
        ("  h / Left", "Collapse node (or jump to parent)"),
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

pub fn render_trash_browser(frame_area: Rect, buf: &mut Buffer, state: &TrashBrowserState) {
    let dialog_width = 78u16.min(frame_area.width.saturating_sub(4));
    let dialog_height = 20u16.min(frame_area.height.saturating_sub(4));
    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let title = format!(" Trash Browser ({} item{}) ", state.items.len(), if state.items.len() == 1 { "" } else { "s" });
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIALOG_BORDER))
        .style(Style::default().bg(theme::DIALOG_BG));

    let inner = block.inner(area);
    block.render(area, buf);

    if state.items.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "The trash is empty.",
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
        return;
    }

    // Leave the last line for key hints.
    let list_height = inner.height.saturating_sub(1) as usize;
    // Scroll so cursor is always visible.
    let offset = if state.cursor >= list_height {
        state.cursor - list_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, entry) in state.items.iter().enumerate().skip(offset).take(list_height) {
        let checked = state.checked.get(i).copied().unwrap_or(false);
        let check_sym = if checked { "[x]" } else { "[ ]" };
        let check_color = if checked { theme::CHECKBOX_CHECKED } else { theme::CHECKBOX_EMPTY };
        let is_cursor = i == state.cursor;
        let path_str = entry.original_path.display().to_string();
        // Truncate long paths.
        let max_path = inner.width.saturating_sub(30) as usize;
        let path_display = if path_str.len() > max_path && max_path > 3 {
            format!("...{}", &path_str[path_str.len() - max_path + 3..])
        } else {
            path_str
        };
        let size_str = ByteSize(entry.size_bytes).to_string();
        let row_bg = if is_cursor { theme::HIGHLIGHT_BG } else { theme::DIALOG_BG };

        let line = Line::from(vec![
            Span::styled(format!(" {check_sym} "), Style::default().fg(check_color).bg(row_bg)),
            Span::styled(format!("{:<42} ", path_display), Style::default().fg(theme::FOREGROUND).bg(row_bg)),
            Span::styled(format!("{:>9} ", size_str), Style::default().fg(theme::SIZE_FG).bg(row_bg)),
            Span::styled(
                entry.category.display_name(),
                Style::default().fg(theme::DIMMED).bg(row_bg),
            ),
        ]);
        lines.push(line);
    }

    // Key hints on the last line.
    lines.push(Line::from(vec![
        Span::styled("Space", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":check  "),
        Span::styled("r", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":restore  "),
        Span::styled("p", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":purge  "),
        Span::styled("Esc", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":back"),
    ]));

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
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
