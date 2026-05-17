//! Terminal-spawn + palette/finder + plugin list + tab operations.

use farx_core::{Action, PanelSide};

use crate::components::fuzzy_finder::FuzzyFinderState;
use crate::components::quick_actions::QuickActionsState;

use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_term_palette(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenTerminalHere => {
                let dir = self.active_tree_ref().root.to_string_lossy().to_string();
                let result = if cfg!(target_os = "macos") {
                    std::process::Command::new("open")
                        .args(["-a", "Terminal", &dir])
                        .spawn()
                } else if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd")
                        .args(["/C", "start", "cmd", "/K", &format!("cd /d {}", dir)])
                        .spawn()
                } else {
                    std::process::Command::new("sh")
                        .args(["-c", &format!("cd '{}' && ${{TERMINAL:-xterm}} &", dir)])
                        .spawn()
                };
                match result {
                    Ok(_) => self.feedback.info("Terminal opened".to_string()),
                    Err(e) => self.feedback.error(format!("Terminal: {}", e)),
                }
            }
            Action::ShowQuickActions => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let name = node.entry.name.clone();
                    let ext = node.entry.extension.as_deref();
                    let is_dir = node.entry.is_dir;
                    self.quick_actions = Some(QuickActionsState::new(name, ext, is_dir));
                }
            }
            Action::RunShellAction(cmd) => {
                self.command_line.input = cmd.clone();
                self.smart_execute_command();
            }
            Action::ShowFuzzyFinder => {
                let root = self.active_tree_ref().root.clone();
                self.fuzzy_finder = Some(FuzzyFinderState::new(root));
            }
            Action::ShowPluginCommands => {
                if let Some(ref engine) = self.plugin_engine {
                    let cmds = engine.list_commands();
                    if cmds.is_empty() {
                        self.feedback.info(
                            "No plugins loaded. Place .lua files in ~/.config/farx/plugins/"
                                .to_string(),
                        );
                    } else {
                        let lines: Vec<String> = cmds
                            .iter()
                            .map(|c| {
                                format!("  /{} — {} ({})", c.name, c.description, c.plugin_file)
                            })
                            .collect();
                        self.feedback.show_output("Plugins", lines.join("\n"));
                    }
                } else {
                    self.feedback
                        .error("Plugin engine not available".to_string());
                }
            }
            Action::NewTab => {
                let root = self.active_tree_ref().root.clone();
                let show_hidden = self.config.general.show_hidden_files;
                match self.active_panel {
                    PanelSide::Left => self.left_tree.new_tab(root, show_hidden),
                    PanelSide::Right => self.right_tree.new_tab(root, show_hidden),
                }
            }
            Action::CloseTab => {
                let closed = match self.active_panel {
                    PanelSide::Left => self.left_tree.close_tab(),
                    PanelSide::Right => self.right_tree.close_tab(),
                };
                if !closed {
                    self.feedback.info("Cannot close the last tab".to_string());
                }
            }
            Action::NextTab => match self.active_panel {
                PanelSide::Left => self.left_tree.next_tab(),
                PanelSide::Right => self.right_tree.next_tab(),
            },
            Action::PrevTab => match self.active_panel {
                PanelSide::Left => self.left_tree.prev_tab(),
                PanelSide::Right => self.right_tree.prev_tab(),
            },
            Action::SwitchTab(idx) => match self.active_panel {
                PanelSide::Left => self.left_tree.switch_to(*idx),
                PanelSide::Right => self.right_tree.switch_to(*idx),
            },
            _ => return false,
        }
        true
    }
}
