//! Agent-grid navigation and view commands: cycle focus (`/next`, `/prev`),
//! toggle back to the last tile (`/last`), and clear a tile's view (`/clear`,
//! `/clearall`). Kept separate from `agents.rs` to stay within the file-size cap.

use super::super::App;

impl App {
    /// Dispatch agent navigation/view commands. Returns `true` if `cmd` matched.
    pub(super) fn slash_agents_nav(&mut self, cmd: &str, _args: &str) -> bool {
        match cmd {
            "/next" => self.cycle_focus(),
            "/prev" => self.cycle_focus_back(),
            "/last" => self.focus_last_agent(),
            "/clear" => self.clear_focused_agent(),
            "/clearall" => self.clear_all_agents(),
            _ => return false,
        }
        true
    }

    /// `/last` — focus the tile that was focused before the current one.
    fn focus_last_agent(&mut self) {
        match self.last_focused_terminal {
            Some(id) if self.terminal_by_id(id).is_some() => {
                self.focus_tile(id);
                let title = self
                    .terminal_by_id(id)
                    .map(|t| t.title.clone())
                    .unwrap_or_default();
                self.feedback.info(format!("Focused {}", title));
            }
            _ => self.feedback.info("No previous tile".to_string()),
        }
    }

    /// `/clear` — reset the focused tile's rendered view.
    fn clear_focused_agent(&mut self) {
        let Some(id) = self.focused_terminal else {
            self.feedback.error("No focused agent to clear".to_string());
            return;
        };
        if let Some(t) = self.terminal_by_id_mut(id) {
            t.clear_screen();
        }
        self.feedback.info("Cleared tile".to_string());
    }

    /// `/clearall` — reset every tile's rendered view.
    fn clear_all_agents(&mut self) {
        if self.terminals.is_empty() {
            self.feedback.info("No agents to clear".to_string());
            return;
        }
        for t in self.terminals.iter_mut() {
            t.clear_screen();
        }
        self.feedback.info("Cleared all tiles".to_string());
    }
}
