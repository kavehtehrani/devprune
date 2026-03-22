use bytesize::ByteSize;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap},
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

pub fn render_confirm_quit(frame_area: Rect, buf: &mut Buffer, app: &App) {
    let dialog_width = 55u16.min(frame_area.width.saturating_sub(4));
    let dialog_height = 7u16;
    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let block = Block::default()
        .title(" quit ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIALOG_BORDER))
        .style(Style::default().bg(theme::DIALOG_BG));

    let inner = block.inner(area);
    block.render(area, buf);

    let msg = format!(
        "trash is not empty ({} item{}, {})",
        app.trash_stats.item_count,
        if app.trash_stats.item_count == 1 { "" } else { "s" },
        ByteSize(app.trash_stats.total_bytes),
    );

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(msg, Style::default().fg(theme::FOREGROUND))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[t]", Style::default().fg(theme::FOOTER_KEY_FG)),
            Span::raw(" open trash  "),
            Span::styled("[q]", Style::default().fg(theme::FOOTER_KEY_FG)),
            Span::raw(" quit anyway  "),
            Span::styled("[Esc]", Style::default().fg(theme::FOOTER_KEY_FG)),
            Span::raw(" cancel"),
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
        ("Selection", ""),
        ("  Space / Enter", "Toggle item check"),
        ("  a", "Select all"),
        ("  A", "Deselect all"),
        ("", ""),
        ("Actions", ""),
        ("  d", "Delete selected items"),
        ("  /", "Filter / search"),
        ("  s", "Cycle sort order"),
        ("  f", "Filter by safety level"),
        ("  t", "Open trash browser"),
        ("", ""),
        ("General", ""),
        ("  R", "Rescan directories"),
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

pub fn render_trash_browser(frame_area: Rect, buf: &mut Buffer, state: &mut TrashBrowserState) {
    // Use most of the available space so paths have room.
    let dialog_width = frame_area.width.saturating_sub(4).max(40);
    let dialog_height = frame_area.height.saturating_sub(4).max(10);
    let area = centered_rect(dialog_width, dialog_height, frame_area);

    Clear.render(area, buf);

    let title = format!(
        " trash ({} item{}, {}) ",
        state.items.len(),
        if state.items.len() == 1 { "" } else { "s" },
        state.sort.label(),
    );
    let hint_line = Line::from(vec![
        Span::styled(" Space", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":check "),
        Span::styled("s", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":sort "),
        Span::styled("r", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":restore "),
        Span::styled("p", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":purge "),
        Span::styled("Esc", Style::default().fg(theme::FOOTER_KEY_FG)),
        Span::raw(":back "),
    ]);

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .title_bottom(hint_line)
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

    // Reserve last two lines for the full path of the highlighted item.
    let list_height = inner.height.saturating_sub(2) as usize;
    // Adjust scroll offset so cursor stays in view (same logic as tree widget).
    if state.cursor < state.scroll_offset {
        state.scroll_offset = state.cursor;
    } else if list_height > 0 && state.cursor >= state.scroll_offset + list_height {
        state.scroll_offset = state.cursor - list_height + 1;
    }
    let offset = state.scroll_offset;

    // Fixed column widths: checkbox(5) + path(flexible) + gap(1) + size(10) + category(18)
    let w = inner.width as usize;
    let check_col = 5;   // " [x] "
    let gap = 1;          // space between path and size
    let size_col = 10;   // "  1.3 GiB "
    let cat_col = 18;    // " Build Output      "
    let path_col = w.saturating_sub(check_col + gap + size_col + cat_col);

    for (i, entry) in state.items.iter().enumerate().skip(offset).take(list_height) {
        let y = inner.y + (i - offset) as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let checked = state.checked.get(i).copied().unwrap_or(false);
        let check_sym = if checked { "[x] " } else { "[ ] " };
        let check_color = if checked { theme::CHECKBOX_CHECKED } else { theme::CHECKBOX_EMPTY };
        let is_cursor = i == state.cursor;
        let row_bg = if is_cursor { theme::HIGHLIGHT_BG } else { theme::DIALOG_BG };

        // Clear the row
        for col in inner.x..(inner.x + inner.width) {
            buf[(col, y)].set_style(Style::default().bg(row_bg)).set_char(' ');
        }

        let path_str = entry.original_path.display().to_string();
        let path_display = if path_str.len() > path_col && path_col > 3 {
            format!("...{}", &path_str[path_str.len() - path_col + 3..])
        } else {
            path_str
        };

        let size_str = format!("{:>9}", ByteSize(entry.size_bytes));
        let cat_str = entry.category.display_name();

        // Render each column at fixed positions
        let mut x = inner.x;

        // Checkbox
        let check_span = Span::styled(format!(" {check_sym}"), Style::default().fg(check_color).bg(row_bg));
        Line::from(vec![check_span]).render(Rect::new(x, y, check_col as u16, 1), buf);
        x += check_col as u16;

        // Path + gap
        let path_span = Span::styled(path_display, Style::default().fg(theme::FOREGROUND).bg(row_bg));
        Line::from(vec![path_span]).render(Rect::new(x, y, path_col as u16, 1), buf);
        x += (path_col + gap) as u16;

        // Size (right-aligned within its column)
        let size_span = Span::styled(size_str, Style::default().fg(theme::SIZE_FG).bg(row_bg));
        Line::from(vec![size_span]).render(Rect::new(x, y, size_col as u16, 1), buf);
        x += size_col as u16;

        // Category
        let cat_span = Span::styled(format!(" {cat_str}"), Style::default().fg(theme::DIMMED).bg(row_bg));
        Line::from(vec![cat_span]).render(Rect::new(x, y, cat_col as u16, 1), buf);
    }

    // Full path of highlighted item on the last two lines.
    if let Some(entry) = state.items.get(state.cursor) {
        let path_y = inner.y + inner.height.saturating_sub(2);
        let full_path = entry.original_path.display().to_string();
        let w = inner.width as usize;

        // Split path across two lines if needed.
        if full_path.len() <= w {
            Line::from(vec![
                Span::styled(format!(" {full_path}"), Style::default().fg(theme::SPINNER_FG)),
            ])
            .render(Rect::new(inner.x, path_y, inner.width, 1), buf);
        } else {
            // First line: as much as fits
            let split = w.min(full_path.len());
            let line1 = &full_path[..split];
            let line2 = &full_path[split..];
            Line::from(vec![
                Span::styled(format!(" {line1}"), Style::default().fg(theme::SPINNER_FG)),
            ])
            .render(Rect::new(inner.x, path_y, inner.width, 1), buf);
            Line::from(vec![
                Span::styled(format!(" {line2}"), Style::default().fg(theme::SPINNER_FG)),
            ])
            .render(Rect::new(inner.x, path_y + 1, inner.width, 1), buf);
        }
    }

    // Scrollbar when content overflows.
    let total_items = state.items.len();
    if total_items > list_height {
        let mut scrollbar_state = ScrollbarState::new(total_items)
            .position(offset);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_style(Style::default().fg(theme::BORDER))
            .thumb_style(Style::default().fg(theme::DIMMED))
            .render(inner, buf, &mut scrollbar_state);
    }
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
