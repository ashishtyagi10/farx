use crossterm::event::{KeyCode, KeyEvent};
use farx_core::AiTool;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum AiPanelAction {
    None,
    Close,
    Launch(AiTool),
}

pub struct AiPanelState {
    pub tools: &'static [AiTool],
    pub cursor: usize,
    pub scroll_offset: usize,
    pub visible_items: usize,
}

impl Default for AiPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl AiPanelState {
    pub fn new() -> Self {
        Self {
            tools: AiTool::all(),
            cursor: 0,
            scroll_offset: 0,
            visible_items: 0,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AiPanelAction {
        let len = self.tools.len();
        let page = self.visible_items.max(1);
        match key.code {
            KeyCode::Esc => AiPanelAction::Close,
            KeyCode::Enter => {
                if let Some(&tool) = self.tools.get(self.cursor) {
                    AiPanelAction::Launch(tool)
                } else {
                    AiPanelAction::None
                }
            }
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                AiPanelAction::None
            }
            KeyCode::Down => {
                if self.cursor + 1 < len {
                    self.cursor += 1;
                }
                AiPanelAction::None
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(page);
                AiPanelAction::None
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + page).min(len.saturating_sub(1));
                AiPanelAction::None
            }
            KeyCode::Home => {
                self.cursor = 0;
                AiPanelAction::None
            }
            KeyCode::End => {
                self.cursor = len.saturating_sub(1);
                AiPanelAction::None
            }
            KeyCode::Char(ch @ '1'..='9') => {
                let idx = (ch as usize) - ('1' as usize);
                if idx < len {
                    AiPanelAction::Launch(self.tools[idx])
                } else {
                    AiPanelAction::None
                }
            }
            _ => AiPanelAction::None,
        }
    }
}

pub fn render_ai_panel(frame: &mut Frame, state: &mut AiPanelState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 52u16.min(area.width.saturating_sub(4));
    let item_count = state.tools.len() as u16;
    // 2 border + 1 blank + items*2 (label+desc) + 1 blank + 1 hint
    let dialog_height = (2 + 1 + item_count * 2 + 1 + 1).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI Coding Tools ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Magenta).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Reserve last row for the hint; each item takes 2 rows (label + desc).
    let item_rows = inner.height.saturating_sub(1);
    let visible_items = (item_rows / 2).max(1) as usize;
    state.visible_items = visible_items;

    // Keep cursor within the visible window.
    if state.cursor < state.scroll_offset {
        state.scroll_offset = state.cursor;
    } else if state.cursor >= state.scroll_offset + visible_items {
        state.scroll_offset = state.cursor + 1 - visible_items;
    }
    let max_offset = state.tools.len().saturating_sub(visible_items);
    if state.scroll_offset > max_offset {
        state.scroll_offset = max_offset;
    }

    let end = (state.scroll_offset + visible_items).min(state.tools.len());
    let mut row = 0u16;
    for (i, tool) in state.tools[state.scroll_offset..end].iter().enumerate() {
        let absolute = state.scroll_offset + i;
        if row + 1 >= inner.height {
            break;
        }
        let is_selected = absolute == state.cursor;

        let label_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(54)) // purple highlight
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
        };

        let pointer = if is_selected { ">" } else { " " };
        let label = format!(" {} {}. {} ", pointer, absolute + 1, tool.label());
        let truncated: String = label.chars().take(inner.width as usize).collect();
        frame.render_widget(
            Paragraph::new(Span::styled(truncated, label_style)),
            Rect::new(inner.x, inner.y + row, inner.width, 1),
        );
        row += 1;

        if row < inner.height {
            let desc_style = if is_selected {
                Style::default().fg(Color::Gray).bg(Color::Indexed(54))
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
            };
            let desc = format!("     {}", tool.description());
            let truncated: String = desc.chars().take(inner.width as usize).collect();
            frame.render_widget(
                Paragraph::new(Span::styled(truncated, desc_style)),
                Rect::new(inner.x, inner.y + row, inner.width, 1),
            );
            row += 1;
        }
    }

    if inner.width > 0 {
        let arrow_x = inner.x + inner.width - 1;
        let arrow = Style::default().fg(Color::Yellow).bg(Color::Indexed(236));
        if state.scroll_offset > 0 {
            frame.render_widget(
                Paragraph::new(Span::styled("↑", arrow)),
                Rect::new(arrow_x, inner.y, 1, 1),
            );
        }
        if end < state.tools.len() && item_rows >= 1 {
            frame.render_widget(
                Paragraph::new(Span::styled("↓", arrow)),
                Rect::new(arrow_x, inner.y + item_rows - 1, 1, 1),
            );
        }
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Enter=Launch  1-9=Quick  PgUp/PgDn  Esc=Close",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
