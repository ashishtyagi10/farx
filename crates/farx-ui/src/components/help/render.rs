use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;

use super::content::build_help_lines;
use super::state::HelpState;

pub fn render_help(frame: &mut Frame, state: &HelpState, _theme: &Theme) {
    let area = frame.area();

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Farx Help - FAR Manager Keybindings ")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(22, 22, 26)),
        )
        .style(
            Style::default()
                .bg(Color::Rgb(22, 22, 26))
                .fg(Color::Rgb(200, 200, 210)),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = build_help_lines();

    // Apply scroll
    let visible_lines: Vec<Line> = help_text.into_iter().skip(state.scroll_offset).collect();

    let paragraph = Paragraph::new(visible_lines);
    frame.render_widget(paragraph, inner);
}
