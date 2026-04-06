use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::path::PathBuf;

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum BookmarkAction {
    None,
    Close,
    GoTo(PathBuf),
    Delete(usize),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub path: PathBuf,
}

pub struct BookmarkState {
    pub active: bool,
    pub bookmarks: Vec<Bookmark>,
    pub cursor: usize,
    pub scroll: usize,
}

impl BookmarkState {
    pub fn new(bookmarks: Vec<Bookmark>) -> Self {
        Self {
            active: true,
            bookmarks,
            cursor: 0,
            scroll: 0,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> BookmarkAction {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                BookmarkAction::Close
            }
            KeyCode::Enter => {
                if let Some(bm) = self.bookmarks.get(self.cursor) {
                    let path = bm.path.clone();
                    self.active = false;
                    BookmarkAction::GoTo(path)
                } else {
                    BookmarkAction::None
                }
            }
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    if self.cursor < self.scroll {
                        self.scroll = self.cursor;
                    }
                }
                BookmarkAction::None
            }
            KeyCode::Down => {
                if self.cursor + 1 < self.bookmarks.len() {
                    self.cursor += 1;
                }
                BookmarkAction::None
            }
            KeyCode::Delete | KeyCode::F(8) => {
                if !self.bookmarks.is_empty() {
                    let idx = self.cursor;
                    self.bookmarks.remove(idx);
                    if self.cursor >= self.bookmarks.len() && self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    BookmarkAction::Delete(idx)
                } else {
                    BookmarkAction::None
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
                self.scroll = 0;
                BookmarkAction::None
            }
            KeyCode::End => {
                if !self.bookmarks.is_empty() {
                    self.cursor = self.bookmarks.len() - 1;
                }
                BookmarkAction::None
            }
            _ => BookmarkAction::None,
        }
    }
}

pub fn render_bookmarks(frame: &mut Frame, state: &BookmarkState, _theme: &Theme) {
    let area = frame.area();

    let dialog_width = 60u16.min(area.width.saturating_sub(4));
    let dialog_height = 16u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Bookmarks (Ctrl+B) ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if state.bookmarks.is_empty() {
        let msg = Line::from(Span::styled(
            " No bookmarks. Press Alt+B to bookmark current directory.",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(
            Paragraph::new(msg),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );
    } else {
        let visible = (inner.height.saturating_sub(2)) as usize;
        let scroll = if state.cursor >= state.scroll + visible {
            state.cursor - visible + 1
        } else if state.cursor < state.scroll {
            state.cursor
        } else {
            state.scroll
        };

        for (i, bm) in state
            .bookmarks
            .iter()
            .skip(scroll)
            .take(visible)
            .enumerate()
        {
            let is_selected = scroll + i == state.cursor;
            let style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Indexed(24))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).bg(Color::Indexed(236))
            };

            let display = format!(" {:<name_w$} {}", bm.name, bm.path.display(), name_w = 12);
            let truncated: String = display.chars().take(inner.width as usize).collect();
            frame.render_widget(
                Paragraph::new(Span::styled(truncated, style)),
                Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
            );
        }
    }

    // Hint bar
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint = " Enter=Go  Del/F8=Remove  Esc=Close";
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        )),
        Rect::new(inner.x, hint_y, inner.width, 1),
    );
}

/// Load bookmarks from the config directory.
pub fn load_bookmarks() -> Vec<Bookmark> {
    let path = bookmarks_file_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Save bookmarks to the config directory.
pub fn save_bookmarks(bookmarks: &[Bookmark]) {
    let path = bookmarks_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(bookmarks) {
        let _ = std::fs::write(&path, json);
    }
}

fn bookmarks_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("farx")
        .join("bookmarks.json")
}
