use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::path::PathBuf;

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum BatchRenameAction {
    None,
    Close,
    /// Apply the renames: Vec<(old_path, new_name)>
    Apply(Vec<(PathBuf, String)>),
}

#[derive(Debug, Clone, PartialEq)]
enum ActiveField {
    Find,
    Replace,
}

pub struct BatchRenameState {
    pub active: bool,
    field: ActiveField,
    pub find_pattern: String,
    pub replace_pattern: String,
    find_cursor: usize,
    replace_cursor: usize,
    /// Original file paths and names
    pub files: Vec<(PathBuf, String)>,
    /// Preview of new names (computed from find/replace)
    pub previews: Vec<String>,
    pub scroll: usize,
}

impl BatchRenameState {
    pub fn new(files: Vec<(PathBuf, String)>) -> Self {
        let previews = files.iter().map(|(_, n)| n.clone()).collect();
        Self {
            active: true,
            field: ActiveField::Find,
            find_pattern: String::new(),
            replace_pattern: String::new(),
            find_cursor: 0,
            replace_cursor: 0,
            files,
            previews,
            scroll: 0,
        }
    }

    fn update_previews(&mut self) {
        if self.find_pattern.is_empty() {
            self.previews = self.files.iter().map(|(_, n)| n.clone()).collect();
            return;
        }
        match regex::Regex::new(&self.find_pattern) {
            Ok(re) => {
                self.previews = self
                    .files
                    .iter()
                    .map(|(_, name)| {
                        re.replace_all(name, self.replace_pattern.as_str())
                            .to_string()
                    })
                    .collect();
            }
            Err(_) => {
                // Invalid regex — fallback to literal replace
                self.previews = self
                    .files
                    .iter()
                    .map(|(_, name)| name.replace(&self.find_pattern, &self.replace_pattern))
                    .collect();
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> BatchRenameAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                return BatchRenameAction::Close;
            }
            KeyCode::Tab => {
                self.field = match self.field {
                    ActiveField::Find => ActiveField::Replace,
                    ActiveField::Replace => ActiveField::Find,
                };
                return BatchRenameAction::None;
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || self.field == ActiveField::Replace
                {
                    // Apply renames
                    let renames: Vec<(PathBuf, String)> = self
                        .files
                        .iter()
                        .zip(self.previews.iter())
                        .filter(|((_, old), new)| old != *new)
                        .map(|((path, _), new)| (path.clone(), new.clone()))
                        .collect();
                    self.active = false;
                    return BatchRenameAction::Apply(renames);
                }
                // Enter in find field moves to replace
                self.field = ActiveField::Replace;
                return BatchRenameAction::None;
            }
            KeyCode::Char(ch) => {
                match self.field {
                    ActiveField::Find => {
                        self.find_pattern.insert(self.find_cursor, ch);
                        self.find_cursor += 1;
                    }
                    ActiveField::Replace => {
                        self.replace_pattern.insert(self.replace_cursor, ch);
                        self.replace_cursor += 1;
                    }
                }
                self.update_previews();
                return BatchRenameAction::None;
            }
            KeyCode::Backspace => {
                match self.field {
                    ActiveField::Find => {
                        if self.find_cursor > 0 {
                            self.find_cursor -= 1;
                            self.find_pattern.remove(self.find_cursor);
                        }
                    }
                    ActiveField::Replace => {
                        if self.replace_cursor > 0 {
                            self.replace_cursor -= 1;
                            self.replace_pattern.remove(self.replace_cursor);
                        }
                    }
                }
                self.update_previews();
                return BatchRenameAction::None;
            }
            KeyCode::Left => match self.field {
                ActiveField::Find => {
                    self.find_cursor = self.find_cursor.saturating_sub(1);
                }
                ActiveField::Replace => {
                    self.replace_cursor = self.replace_cursor.saturating_sub(1);
                }
            },
            KeyCode::Right => match self.field {
                ActiveField::Find => {
                    self.find_cursor = (self.find_cursor + 1).min(self.find_pattern.len());
                }
                ActiveField::Replace => {
                    self.replace_cursor = (self.replace_cursor + 1).min(self.replace_pattern.len());
                }
            },
            _ => {}
        }
        BatchRenameAction::None
    }
}

pub fn render_batch_rename(frame: &mut Frame, state: &BatchRenameState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 70u16.min(area.width.saturating_sub(4));
    let dialog_height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Batch Rename ({} files) ", state.files.len()))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut y_off = 0u16;

    // Find field
    let find_active = state.field == ActiveField::Find;
    let find_label_style = Style::default()
        .fg(if find_active {
            Color::Yellow
        } else {
            Color::Cyan
        })
        .bg(Color::Indexed(236));
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(" Find (regex):", find_label_style))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    y_off += 1;

    let find_input_style = Style::default().fg(Color::White).bg(if find_active {
        Color::Indexed(238)
    } else {
        Color::Indexed(237)
    });
    let find_display = format!(
        " {:<width$}",
        state.find_pattern,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(find_display, find_input_style))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    if find_active {
        frame.set_cursor_position((inner.x + 1 + state.find_cursor as u16, inner.y + y_off));
    }
    y_off += 2;

    // Replace field
    let replace_active = state.field == ActiveField::Replace;
    let replace_label_style = Style::default()
        .fg(if replace_active {
            Color::Yellow
        } else {
            Color::Cyan
        })
        .bg(Color::Indexed(236));
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Replace with:",
            replace_label_style,
        ))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    y_off += 1;

    let replace_input_style = Style::default().fg(Color::White).bg(if replace_active {
        Color::Indexed(238)
    } else {
        Color::Indexed(237)
    });
    let replace_display = format!(
        " {:<width$}",
        state.replace_pattern,
        width = (inner.width as usize).saturating_sub(2)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            replace_display,
            replace_input_style,
        ))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    if replace_active {
        frame.set_cursor_position((inner.x + 1 + state.replace_cursor as u16, inner.y + y_off));
    }
    y_off += 2;

    // Preview
    let preview_height = inner.height.saturating_sub(y_off + 1) as usize;
    let half_w = (inner.width as usize).saturating_sub(4) / 2;

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " Preview:",
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
        ))),
        Rect::new(inner.x, inner.y + y_off, inner.width, 1),
    );
    y_off += 1;

    for (i, ((_, old_name), new_name)) in state
        .files
        .iter()
        .zip(state.previews.iter())
        .skip(state.scroll)
        .take(preview_height)
        .enumerate()
    {
        let changed = old_name != new_name;
        let old_trunc: String = old_name.chars().take(half_w).collect();
        let new_trunc: String = new_name.chars().take(half_w).collect();
        let arrow = if changed { " → " } else { "   " };

        let old_style = Style::default().fg(Color::White).bg(Color::Indexed(236));
        let new_style = if changed {
            Style::default()
                .fg(Color::Green)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
        };

        let line = Line::from(vec![
            Span::styled(format!(" {:<w$}", old_trunc, w = half_w), old_style),
            Span::styled(
                arrow,
                Style::default().fg(Color::Yellow).bg(Color::Indexed(236)),
            ),
            Span::styled(format!("{:<w$}", new_trunc, w = half_w), new_style),
        ]);
        frame.render_widget(
            Paragraph::new(line),
            Rect::new(inner.x, inner.y + y_off + i as u16, inner.width, 1),
        );
    }

    // Hint
    let hint_y = inner.y + inner.height.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            " Tab=Switch field  Enter=Apply  Esc=Cancel",
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}
