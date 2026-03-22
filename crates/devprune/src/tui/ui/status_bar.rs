use bytesize::ByteSize;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
    Frame,
};

use crate::tui::app::{App, AppMode};
use crate::tui::ui::theme;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Render the header content (scan status) into the given area.
pub fn render_header_content(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let spans: Vec<Span> = if app.scan_complete {
        let (count, size) = app.tree.selection_summary();
        let total: u64 = app.tree.categories.iter().map(|c| c.total_size).sum();
        let n_arts: usize = app.tree.categories
            .iter()
            .flat_map(|c| c.children.iter())
            .map(|g| g.children.len())
            .sum();
        vec![
            Span::styled(" ✔ scan complete", Style::default().fg(theme::COMPLETE_FG)),
            Span::styled(
                format!(
                    "  {} artifacts  {}  |  {} selected  {}",
                    n_arts,
                    ByteSize(total),
                    count,
                    ByteSize(size),
                ),
                Style::default().fg(theme::HEADER_FG),
            ),
        ]
    } else {
        let spinner = SPINNER_FRAMES
            [(app.tick_count as usize / 2) % SPINNER_FRAMES.len()];
        let elapsed = app.scan_progress.elapsed_ms;
        vec![
            Span::styled(format!(" {spinner}"), Style::default().fg(theme::SPINNER_FG)),
            Span::styled(
                format!(
                    "  scanning... {} dirs  {} found  {}ms",
                    app.scan_progress.dirs_visited,
                    app.scan_progress.artifacts_found,
                    elapsed,
                ),
                Style::default().fg(theme::HEADER_FG),
            ),
        ]
    };

    Line::from(spans).render(area, frame.buffer_mut());
}

/// Render a proportional bar showing relative size of each category.
/// Each category gets a colored segment proportional to its total size.
pub fn render_proportional_bar(frame: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let total: u64 = app.tree.categories.iter().map(|c| c.total_size).sum();
    if total == 0 {
        let empty = Line::from(vec![
            Span::styled(" no artifacts found", Style::default().fg(theme::DIMMED)),
        ]);
        empty.render(area, frame.buffer_mut());
        return;
    }

    let w = area.width as usize;
    let buf = frame.buffer_mut();

    // Collect categories with their sizes, sorted largest first.
    let mut cats: Vec<(devprune_core::rules::types::Category, u64)> = app
        .tree
        .categories
        .iter()
        .map(|c| (c.category, c.total_size))
        .filter(|(_, s)| *s > 0)
        .collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1));

    // Assign character widths proportionally, minimum 1 char per category.
    let mut widths: Vec<usize> = cats
        .iter()
        .map(|(_, size)| (((*size as f64 / total as f64) * w as f64).round() as usize).max(1))
        .collect();

    // Adjust to fit exactly within available width.
    let sum: usize = widths.iter().sum();
    if sum > w {
        // Shrink the largest segments.
        let mut excess = sum - w;
        for width in widths.iter_mut() {
            if excess == 0 {
                break;
            }
            if *width > 1 {
                let take = (*width - 1).min(excess);
                *width -= take;
                excess -= take;
            }
        }
    }

    // Render each segment.
    let mut x = area.x;
    for (i, (cat, size)) in cats.iter().enumerate() {
        let seg_w = widths[i];
        if seg_w == 0 {
            continue;
        }
        let color = theme::category_color(*cat);
        let label = format!(" {} {} ", cat.display_name(), ByteSize(*size));

        // Fill segment with block chars and overlay the label if it fits.
        for col in x..(x + seg_w as u16).min(area.x + area.width) {
            buf[(col, area.y)]
                .set_char('█')
                .set_fg(color);
        }

        // Overlay the label text if it fits within the segment.
        if label.len() <= seg_w {
            let label_start = x;
            for (j, ch) in label.chars().enumerate() {
                let col = label_start + j as u16;
                if col < area.x + area.width {
                    buf[(col, area.y)]
                        .set_char(ch)
                        .set_fg(ratatui::style::Color::Black)
                        .set_bg(color);
                }
            }
        }

        x += seg_w as u16;
    }
}

/// Returns key hints for the current mode.
pub fn mode_hints(mode: &AppMode) -> Vec<(&'static str, &'static str)> {
    match mode {
        AppMode::Normal => vec![
            ("Space", "check"),
            ("a/A", "all/none"),
            ("d", "delete"),
            ("/", "search"),
            ("s", "sort"),
            ("f", "filter by safety"),
            ("t", "trash"),
            ("R", "rescan"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppMode::Search { .. } => vec![
            ("Enter", "apply"),
            ("Esc", "cancel"),
            ("Backspace", "delete char"),
        ],
        AppMode::ConfirmDelete => vec![
            ("y/Enter", "confirm"),
            ("n/Esc", "cancel"),
        ],
        AppMode::ConfirmQuit => vec![
            ("t", "open trash"),
            ("q", "quit anyway"),
            ("Esc", "cancel"),
        ],
        AppMode::Help => vec![
            ("q/Esc", "close"),
        ],
        AppMode::TrashBrowser => vec![
            ("j/k", "move"),
            ("Space", "check"),
            ("r", "restore"),
            ("p", "purge"),
            ("Esc/t", "back"),
        ],
    }
}
