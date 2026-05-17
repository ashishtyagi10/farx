//! Application-control dispatch: quit / panel focus / panel swap / hidden /
//! refresh / view toggles / show-overlays / sort / drive menu.

use farx_core::{Action, PanelSide, SortField};

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;
use crate::components::help::HelpState;
use crate::components::menu::MenuState;
use crate::components::search::SearchState;

use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_control(&mut self, action: &Action) -> bool {
        match action {
            Action::Quit => self.running = false,
            Action::SwitchPanel => self.cycle_focus(),
            Action::FocusLeftPanel => {
                self.focused_terminal = None;
                self.active_panel = PanelSide::Left;
            }
            Action::FocusRightPanel => {
                self.focused_terminal = None;
                self.active_panel = PanelSide::Right;
            }
            Action::SwapPanels => {
                std::mem::swap(&mut self.left_panel, &mut self.right_panel);
                self.left_panel.side = PanelSide::Left;
                self.right_panel.side = PanelSide::Right;
                std::mem::swap(&mut self.left_tree, &mut self.right_tree);
                self.update_fs_watcher();
            }
            Action::GotoRoot => {
                let root = if cfg!(windows) {
                    std::path::PathBuf::from("C:\\")
                } else {
                    std::path::PathBuf::from("/")
                };
                self.navigate_to(root);
            }
            Action::ToggleHidden => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                let sh = self.config.general.show_hidden_files;
                self.left_tree.show_hidden = sh;
                self.left_tree.rebuild();
                self.right_tree.show_hidden = sh;
                self.right_tree.rebuild();
            }
            Action::RefreshPanel => self.active_tree().rebuild(),
            Action::TogglePanels => self.panels_visible = !self.panels_visible,
            Action::ShowHelp => self.help = Some(HelpState::new()),
            Action::ShowMenu => self.menu = Some(MenuState::new()),
            Action::ShowSearchDialog => {
                let dir = self.active_panel_ref().current_dir.clone();
                self.search = Some(SearchState::new(dir));
            }
            Action::ShowInfoPanel => self.show_info_panel = !self.show_info_panel,
            Action::ShowAiBar => self.ai_bar = Some(AiBarState::new()),
            Action::ShowAiPanel => self.ai_panel = Some(AiPanelState::new()),
            Action::LaunchAiTool(tool) => {
                let (cmd, args) = tool.command();
                self.spawn_embedded_terminal(cmd, args);
            }
            Action::ToggleFilter => {
                self.filter_active = !self.filter_active;
                if !self.filter_active {
                    self.filter_pattern.clear();
                    self.active_tree().rebuild();
                }
            }
            Action::SortByName => self.toggle_sort(SortField::Name),
            Action::SortByExtension => self.toggle_sort(SortField::Extension),
            Action::SortBySize => self.toggle_sort(SortField::Size),
            Action::SortByDate => self.toggle_sort(SortField::Modified),
            Action::ShowDriveMenu(_) => {
                let output = std::process::Command::new("df")
                    .args(["-h", "--output=target,size,avail,pcent"])
                    .output()
                    .or_else(|_| std::process::Command::new("df").args(["-h"]).output());
                match output {
                    Ok(out) => {
                        let text = String::from_utf8_lossy(&out.stdout).to_string();
                        self.feedback.show_output("Volumes", text);
                    }
                    Err(e) => self.feedback.error(format!("df: {}", e)),
                }
            }
            _ => return false,
        }
        true
    }
}
