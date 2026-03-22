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
