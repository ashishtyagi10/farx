//! Slash commands for managing the agent grid: list tiles, focus a tile by
//! number, rename the focused tile, close the others, and restart the focused
//! tile. All are command-driven so they never steal keys from the focused agent.

use super::super::App;

impl App {
    /// Dispatch agent-grid slash commands. Returns `true` if `cmd` matched.
    pub(super) fn slash_agents(&mut self, cmd: &str, args: &str) -> bool {
        match cmd {
            "/agents" | "/ls" => self.list_agents(),
            "/focus" | "/f" => self.focus_agent_by_arg(args),
            "/title" => self.rename_focused_agent(args),
            "/only" => self.close_other_agents(),
            "/restart" => self.restart_focused_agent(),
            _ => return false,
        }
        true
    }

    /// Agent tile ids in display order: full tiles first (most-recently-active),
    /// then minimized ones. This is the numbering `/agents` and `/focus` use.
    fn agent_order(&self) -> Vec<usize> {
        self.grid
            .full()
            .iter()
            .chain(self.grid.minimized().iter())
            .copied()
            .collect()
    }

    /// `/agents` — list every running tile with its number, title, state and cwd.
    fn list_agents(&mut self) {
        let order = self.agent_order();
        if order.is_empty() {
            self.feedback.info("No agents running".to_string());
            return;
        }
        let minimized = self.grid.minimized().to_vec();
        let lines: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let n = i + 1;
                let (title, cwd, alive, attention) = match self.terminal_by_id(*id) {
                    Some(t) => (
                        t.title.clone(),
                        t.cwd.display().to_string(),
                        t.alive,
                        t.has_attention,
                    ),
                    None => (String::from("?"), String::new(), false, false),
                };
                let mut flags = Vec::new();
                if self.focused_terminal == Some(*id) {
                    flags.push("focused");
                }
                if minimized.contains(id) {
                    flags.push("minimized");
                }
                if !alive {
                    flags.push("exited");
                }
                if attention {
                    flags.push("●");
                }
                let suffix = if flags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", flags.join(", "))
                };
                format!("  {}. {}{} — {}", n, title, suffix, cwd)
            })
            .collect();
        self.feedback.show_output("Agents", lines.join("\n"));
    }

    /// `/focus N` — focus the tile numbered `N` (1-based, as shown by `/agents`).
    fn focus_agent_by_arg(&mut self, args: &str) {
        let order = self.agent_order();
        if order.is_empty() {
            self.feedback.info("No agents to focus".to_string());
            return;
        }
        let n: usize = match args.trim().parse() {
            Ok(n) if n >= 1 && n <= order.len() => n,
            _ => {
                self.feedback
                    .error(format!("Usage: /focus 1..{}", order.len()));
                return;
            }
        };
        let id = order[n - 1];
        self.focus_tile(id);
        let title = self
            .terminal_by_id(id)
            .map(|t| t.title.clone())
            .unwrap_or_default();
        self.feedback.info(format!("Focused {}. {}", n, title));
    }

    /// `/title <text>` — rename the focused agent tile.
    fn rename_focused_agent(&mut self, args: &str) {
        let title = args.trim();
        if title.is_empty() {
            self.feedback
                .error("Usage: /title <new tile name>".to_string());
            return;
        }
        let Some(id) = self.focused_terminal else {
            self.feedback
                .error("No focused agent to rename".to_string());
            return;
        };
        if let Some(t) = self.terminal_by_id_mut(id) {
            t.title = title.to_string();
        }
        self.feedback.info(format!("Renamed tile to {}", title));
    }

    /// `/only` — close every tile except the focused one (tmux `kill-pane -a`).
    fn close_other_agents(&mut self) {
        let Some(keep) = self.focused_terminal else {
            self.feedback.error("No focused agent to keep".to_string());
            return;
        };
        let others: Vec<usize> = self
            .agent_order()
            .into_iter()
            .filter(|id| *id != keep)
            .collect();
        let n = others.len();
        for id in others {
            self.close_terminal(id);
        }
        self.feedback.info(format!("Closed {} other tile(s)", n));
    }

    /// `/restart` — respawn the focused tile's original program in its cwd.
    fn restart_focused_agent(&mut self) {
        let Some(id) = self.focused_terminal else {
            self.feedback
                .error("No focused agent to restart".to_string());
            return;
        };
        let Some(t) = self.terminal_by_id(id) else {
            return;
        };
        let cmd = t.spawn_cmd.clone();
        let args = t.spawn_args.clone();
        let cwd = t.cwd.clone();
        self.close_terminal(id);
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.spawn_embedded_terminal_in(&cmd, &arg_refs, cwd);
    }
}
