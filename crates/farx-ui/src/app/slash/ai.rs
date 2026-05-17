//! Slash commands that launch the AI bar, the AI tools panel, or an
//! embedded terminal running a specific CLI assistant.

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;

use super::super::App;

impl App {
    /// Dispatch AI/shell slash commands. Returns `true` if `cmd` matched.
    pub(super) fn slash_ai(&mut self, cmd: &str, _args: &str) -> bool {
        match cmd {
            "/ai" => self.ai_bar = Some(AiBarState::new()),
            "/ai-tools" | "/ait" => self.ai_panel = Some(AiPanelState::new()),
            "/claude" => self.spawn_embedded_terminal("claude", &[]),
            "/codex" => self.spawn_embedded_terminal("codex", &[]),
            "/copilot" => self.spawn_embedded_terminal("gh", &["copilot"]),
            "/gemini" => self.spawn_embedded_terminal("gemini", &[]),
            "/opencode" => self.spawn_embedded_terminal("opencode", &[]),
            "/shell" | "/sh" | "/bash" | "/zsh" => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                self.spawn_embedded_terminal(&shell, &[]);
            }
            _ => return false,
        }
        true
    }
}
