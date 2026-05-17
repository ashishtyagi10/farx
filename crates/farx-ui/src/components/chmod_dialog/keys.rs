use crossterm::event::{KeyCode, KeyEvent};

use super::state::{ChmodAction, ChmodDialogState};

impl ChmodDialogState {
    /// Handle a key event, returning the action to take.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> ChmodAction {
        match key.code {
            KeyCode::Esc => ChmodAction::Cancel,
            KeyCode::Enter => ChmodAction::Apply(self.to_mode()),
            KeyCode::Char(' ') => {
                self.bits[self.cursor] = !self.bits[self.cursor];
                ChmodAction::None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                ChmodAction::None
            }
            KeyCode::Right => {
                if self.cursor < 8 {
                    self.cursor += 1;
                }
                ChmodAction::None
            }
            KeyCode::Up => {
                // Move up one row (3 columns per row)
                if self.cursor >= 3 {
                    self.cursor -= 3;
                }
                ChmodAction::None
            }
            KeyCode::Down => {
                // Move down one row
                if self.cursor + 3 <= 8 {
                    self.cursor += 3;
                }
                ChmodAction::None
            }
            KeyCode::Tab => {
                // Cycle through all 9 positions
                self.cursor = (self.cursor + 1) % 9;
                ChmodAction::None
            }
            _ => ChmodAction::None,
        }
    }
}
