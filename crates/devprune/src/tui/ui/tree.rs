use bytesize::ByteSize;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};


use crate::tui::app::{CheckState, RowRef, TreeState, VisibleRow};
use crate::tui::ui::theme;

/// Rendering state that persists frame-to-frame (scroll offset).
#[derive(Default)]
pub struct TreeWidgetState {
    pub offset: usize,
}

pub struct TreeWidget<'a> {
    pub tree: &'a TreeState,
    pub title: &'a str,
    pub bottom_title: Option<Line<'a>>,
}

impl<'a> StatefulWidget for TreeWidget<'a> {
    type State = TreeWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER));

        if let Some(bt) = self.bottom_title {
            block = block.title_bottom(bt);
        }

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 {
            return;
        }

        let rows = self.tree.visible_rows();
        let cursor = self.tree.cursor;
        let visible_height = inner.height as usize;

        // Adjust scroll offset so cursor stays in view.
        if cursor < state.offset {
            state.offset = cursor;
        } else if cursor >= state.offset + visible_height {
            state.offset = cursor - visible_height + 1;
        }

        let visible: Vec<&VisibleRow> = rows
            .iter()
            .skip(state.offset)
            .take(visible_height)
            .collect();

        for (i, row) in visible.iter().enumerate() {
            let y = inner.y + i as u16;
            let is_cursor = (state.offset + i) == cursor;
            render_row(buf, inner.x, y, inner.width, row, is_cursor);
        }
    }
}

fn render_row(buf: &mut Buffer, x: u16, y: u16, width: u16, row: &VisibleRow, is_cursor: bool) {
    let bg_style = if is_cursor {
        Style::default()
            .bg(theme::HIGHLIGHT_BG)
            .fg(theme::HIGHLIGHT_FG)
    } else {
        Style::default().fg(theme::FOREGROUND)
    };

    // Clear the row with the base style first.
    for col in x..(x + width) {
        buf[(col, y)].set_style(bg_style).set_char(' ');
    }

    let indent = "  ".repeat(row.depth as usize);

    // Expand arrow.
    let arrow = match row.expanded {
        Some(true) => "v ",
        Some(false) => "> ",
        None => "  ",
    };

    // Checkbox.
    let (check_char, check_color) = match row.check_state {
        CheckState::Checked => ("[x]", theme::CHECKBOX_CHECKED),
        CheckState::Unchecked => ("[ ]", theme::CHECKBOX_EMPTY),
        CheckState::Indeterminate => ("[~]", theme::CHECKBOX_PARTIAL),
    };

    // Name colour: category rows get special colour, artifacts inherit normal fg.
    let name_fg = match &row.row_ref {
        RowRef::Category { .. } => {
            // Pull category out from the name to get its colour — we don't
            // have direct access to the category variant here, so use white.
            theme::HEADER_FG
        }
        RowRef::RuleGroup { .. } => theme::FOREGROUND,
        RowRef::Artifact { .. } => theme::DIMMED,
    };

    let size_str = if row.size > 0 {
        ByteSize(row.size).to_string()
    } else {
        "?".to_string()
    };

    let count_str = match row.item_count {
        Some(n) => format!(" ({n})"),
        None => String::new(),
    };

    // Build up spans.
    let spans: Vec<Span> = vec![
        Span::raw(indent),
        Span::styled(arrow, Style::default().fg(theme::DIMMED)),
        Span::styled(check_char, Style::default().fg(check_color)),
        Span::raw(" "),
        Span::styled(
            row.name.clone(),
            if is_cursor {
                Style::default()
                    .fg(theme::HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(name_fg)
            },
        ),
        Span::styled(count_str, Style::default().fg(theme::COUNT_FG)),
    ];

    // Right-aligned size.
    let size_span = Span::styled(size_str.clone(), Style::default().fg(theme::SIZE_FG));
    let left_line = Line::from(spans);
    let left_text = left_line.to_string();
    let left_width = left_text.len() as u16;
    let size_len = size_str.len() as u16;

    // Render left part.
    let left_area = Rect::new(x, y, width.saturating_sub(size_len + 1), 1);
    left_line.render(left_area, buf);

    // Render size right-aligned, only if it fits.
    if width > left_width + size_len {
        let size_x = x + width - size_len;
        let _ = size_span; // re-create
        let size_span2 = Span::styled(size_str, Style::default().fg(theme::SIZE_FG));
        let size_area = Rect::new(size_x, y, size_len, 1);
        Line::from(vec![size_span2]).render(size_area, buf);
    }

    // Apply highlight bg across the full row width (cannot set style before render
    // because ratatui widgets overwrite cell styles).
    if is_cursor {
        for col in x..(x + width) {
            buf[(col, y)].set_bg(theme::HIGHLIGHT_BG);
        }
    }
}
