use crossterm::event::{KeyCode, KeyEvent};

use super::catalog::build_actions;
use super::types::{QuickActionResult, QuickActionsState};

impl QuickActionsState {
    pub fn new(file_name: String, extension: Option<&str>, is_dir: bool) -> Self {
        let actions = build_actions(&file_name, extension, is_dir);
        Self {
            active: true,
            actions,
            cursor: 0,
            file_name,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> QuickActionResult {
        match key.code {
            KeyCode::Esc => {
                self.active = false;
                QuickActionResult::Close
            }
            KeyCode::Enter => {
                if let Some(action) = self.actions.get(self.cursor) {
                    self.active = false;
                    QuickActionResult::Execute(action.command.clone())
                } else {
                    QuickActionResult::None
                }
            }
            KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                QuickActionResult::None
            }
            KeyCode::Down => {
                if self.cursor + 1 < self.actions.len() {
                    self.cursor += 1;
                }
                QuickActionResult::None
            }
            _ => QuickActionResult::None,
        }
    }
}
