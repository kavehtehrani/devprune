use bytesize::ByteSize;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    buffer::Buffer,
};

use devprune_core::types::ArtifactInfo;

use crate::tui::ui::theme;

pub struct DetailsPanel<'a> {
    pub artifact: Option<&'a ArtifactInfo>,
}

impl<'a> Widget for DetailsPanel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER));

        let inner = block.inner(area);
        block.render(area, buf);

        let Some(art) = self.artifact else {
            let hint = Paragraph::new("Select an artifact to see details.")
                .style(Style::default().fg(theme::DIMMED));
            hint.render(inner, buf);
            return;
        };

        let path_str = art.path.display().to_string();
        let name = art
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_str.clone());

        let size_str = art
            .size
            .map(|s| ByteSize(s).to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let modified_str = art
            .last_modified
            .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("Name:     ", Style::default().fg(theme::DIMMED)),
            Span::styled(name, Style::default().fg(theme::FOREGROUND)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Path:     ", Style::default().fg(theme::DIMMED)),
            Span::styled(path_str, Style::default().fg(theme::DIMMED)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Size:     ", Style::default().fg(theme::DIMMED)),
            Span::styled(size_str, Style::default().fg(theme::SIZE_FG)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Category: ", Style::default().fg(theme::DIMMED)),
            Span::styled(
                art.category.display_name(),
                Style::default().fg(theme::category_color(art.category)),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Rule:     ", Style::default().fg(theme::DIMMED)),
            Span::styled(art.rule_name.clone(), Style::default().fg(theme::FOREGROUND)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Safety:   ", Style::default().fg(theme::DIMMED)),
            Span::styled(
                art.safety.display_name(),
                Style::default().fg(theme::safety_color(art.safety)),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Modified: ", Style::default().fg(theme::DIMMED)),
            Span::styled(modified_str, Style::default().fg(theme::DIMMED)),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![Span::styled(
            art.safety.description(),
            Style::default().fg(theme::DIMMED),
        )]));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Space: toggle  d: delete",
            Style::default().fg(theme::FOOTER_KEY_FG),
        )]));

        let para = Paragraph::new(lines).wrap(Wrap { trim: true });
        para.render(inner, buf);
    }
}
