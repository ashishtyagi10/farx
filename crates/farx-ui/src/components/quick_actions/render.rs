use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::types::QuickActionsState;
use crate::theme::Theme;

pub fn render_quick_actions(frame: &mut Frame, state: &QuickActionsState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 50u16.min(area.width.saturating_sub(4));
    let dialog_height = (state.actions.len() as u16 + 4).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Actions: {} ", state.file_name))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let visible = (inner.height.saturating_sub(1)) as usize;
    for (i, action) in state.actions.iter().take(visible).enumerate() {
        let is_selected = i == state.cursor;
        let style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(24))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let display = format!(" {} ", action.label);
        let truncated: String = display.chars().take(inner.width as usize).collect();
        frame.render_widget(
            Paragraph::new(Span::styled(truncated, style)),
            Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
        );
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Enter=Run  Esc=Close",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
