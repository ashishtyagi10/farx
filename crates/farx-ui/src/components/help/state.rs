use crossterm::event::{KeyCode, KeyEvent};

pub struct HelpState {
    pub active: bool,
    pub scroll_offset: usize,
}

impl Default for HelpState {
    fn default() -> Self {
        Self {
            active: true,
            scroll_offset: 0,
        }
    }
}

impl HelpState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => {
                self.active = false;
                true // consumed
            }
            KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                self.scroll_offset += 1;
                true
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                true
            }
            KeyCode::PageDown => {
                self.scroll_offset += 20;
                true
            }
            _ => true,
        }
    }
}
