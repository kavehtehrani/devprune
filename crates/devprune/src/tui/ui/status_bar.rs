use bytesize::ByteSize;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use crate::tui::app::{App, AppMode};
use crate::tui::ui::theme;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Header<'a> {
    pub app: &'a App,
}

impl<'a> Widget for Header<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the header row.
        for col in area.x..(area.x + area.width) {
            buf[(col, area.y)]
                .set_style(Style::default().bg(theme::HEADER_BG).fg(theme::HEADER_FG))
                .set_char(' ');
        }

        let version = env!("CARGO_PKG_VERSION");
        let title = format!(" devprune v{version} ");

        let status: Vec<Span> = if self.app.scan_complete {
            let (count, size) = self.app.tree.selection_summary();
            let total: u64 = self.app.tree.categories.iter().map(|c| c.total_size).sum();
            let n_arts: usize = self.app.tree.categories
                .iter()
                .flat_map(|c| c.children.iter())
                .map(|g| g.children.len())
                .sum();
            vec![
                Span::styled("  ", Style::default()),
                Span::styled("✔ Scan complete", Style::default().fg(theme::COMPLETE_FG)),
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
            let frame = SPINNER_FRAMES
                [(self.app.tick_count as usize / 2) % SPINNER_FRAMES.len()];
            let elapsed = self.app.scan_progress.elapsed_ms;
            vec![
                Span::styled("  ", Style::default()),
                Span::styled(frame, Style::default().fg(theme::SPINNER_FG)),
                Span::styled(
                    format!(
                        "  Scanning… {} dirs  {} found  {}ms",
                        self.app.scan_progress.dirs_visited,
                        self.app.scan_progress.artifacts_found,
                        elapsed,
                    ),
                    Style::default().fg(theme::HEADER_FG),
                ),
            ]
        };

        let left = Line::from(vec![Span::styled(
            title,
            Style::default().fg(theme::HEADER_FG),
        )]);

        let right = Line::from(status);

        // Render left title.
        left.render(
            Rect::new(area.x, area.y, area.width / 2, 1),
            buf,
        );
        // Render status on the right half.
        right.render(
            Rect::new(area.x + area.width / 2, area.y, area.width / 2, 1),
            buf,
        );

        // Ensure full row has correct background.
        for col in area.x..(area.x + area.width) {
            buf[(col, area.y)].set_bg(theme::HEADER_BG);
        }
    }
}

pub struct Footer<'a> {
    pub app: &'a App,
}

impl<'a> Widget for Footer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear both footer lines.
        for row in area.y..(area.y + area.height) {
            for col in area.x..(area.x + area.width) {
                buf[(col, row)]
                    .set_style(Style::default().bg(theme::FOOTER_BG).fg(theme::FOOTER_FG))
                    .set_char(' ');
            }
        }

        // Line 1: sort / filter status, or a transient status message.
        let line1_content = if let Some(ref msg) = self.app.status_message {
            Line::from(vec![Span::styled(
                format!(" {msg} "),
                Style::default().fg(theme::COMPLETE_FG),
            )])
        } else {
            let sort_label = self.app.tree.sort.label();
            let filter_label = self.app.tree.search_filter.as_deref().unwrap_or("");
            let status_line = if filter_label.is_empty() {
                format!(" sort:{sort_label} ")
            } else {
                format!(" sort:{sort_label}  filter:\"{filter_label}\" ")
            };
            Line::from(vec![Span::styled(
                status_line,
                Style::default().fg(theme::FOOTER_FG),
            )])
        };
        line1_content.render(Rect::new(area.x, area.y, area.width, 1), buf);

        if area.height < 2 {
            return;
        }

        // Line 2: context-sensitive hints.
        let hints = mode_hints(&self.app.mode);
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

        Line::from(hint_spans)
            .render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
    }
}

fn mode_hints(mode: &AppMode) -> Vec<(&'static str, &'static str)> {
    match mode {
        AppMode::Normal => vec![
            ("j/k", "move"),
            ("Space", "check"),
            ("Enter", "expand"),
            ("a/A", "all/none"),
            ("d", "delete"),
            ("/", "search"),
            ("s", "sort"),
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
            ("r", "restore"),
            ("p", "purge"),
            ("Esc/t", "back"),
        ],
    }
}
