//! Core slash commands: application control, navigation, view toggles,
//! filtering, searching, bookmarks.

use std::path::PathBuf;

use farx_core::{Action, SortField};

use crate::components::bookmarks::BookmarkState;
use crate::components::help::HelpState;
use crate::components::menu::MenuState;
use crate::components::search::SearchState;

use super::super::App;

impl App {
    /// Dispatch core slash commands. Returns `true` if `cmd` matched.
    pub(super) fn slash_core(&mut self, cmd: &str, args: &str) -> bool {
        match cmd {
            "/exit" | "/quit" | "/q" => self.running = false,
            "/help" | "/h" => self.help = Some(HelpState::new()),
            "/update" => self.start_update_check(),
            "/refresh" | "/r" => {
                self.left_tree.rebuild();
                self.right_tree.rebuild();
                self.feedback.info("Refreshed".to_string());
            }
            "/hidden" => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                let sh = self.config.general.show_hidden_files;
                self.left_tree.show_hidden = sh;
                self.left_tree.rebuild();
                self.right_tree.show_hidden = sh;
                self.right_tree.rebuild();
                self.feedback.info(format!(
                    "Hidden files: {}",
                    if sh { "shown" } else { "hidden" }
                ));
            }
            "/sort" => match args {
                "name" => self.toggle_sort(SortField::Name),
                "ext" => self.toggle_sort(SortField::Extension),
                "size" => self.toggle_sort(SortField::Size),
                "date" => self.toggle_sort(SortField::Modified),
                _ => self
                    .feedback
                    .error("Usage: /sort name|ext|size|date".to_string()),
            },
            "/menu" => self.menu = Some(MenuState::new()),
            "/info" => self.show_info_panel = !self.show_info_panel,
            "/search" | "/find" => {
                let dir = self.active_panel_ref().current_dir.clone();
                self.search = Some(SearchState::new(dir));
            }
            "/grep" | "/content-search" => {
                let dir = self.active_tree_ref().root.clone();
                self.search = Some(SearchState::new_content_focused(dir));
            }
            "/find-file" | "/ff" => self.dispatch(Action::ShowFuzzyFinder),
            "/filter" => {
                if args.is_empty() {
                    self.filter_active = true;
                    self.filter_pattern.clear();
                } else {
                    self.filter_pattern = args.to_string();
                    self.active_tree().filter = args.to_string();
                    self.active_tree().rebuild();
                    self.feedback.info(format!("Filter: {}", args));
                }
            }
            "/back" => self.dispatch(Action::HistoryBack),
            "/forward" | "/fwd" => self.dispatch(Action::HistoryForward),
            "/recent" | "/history" => self.dispatch(Action::ShowRecentDirectories),
            "/bookmark" | "/bm" => {
                if args.is_empty() {
                    self.bookmarks_panel = Some(BookmarkState::new(self.bookmarks.clone()));
                } else {
                    self.dispatch(Action::AddBookmark);
                }
            }
            "/cd" => self.slash_cd_or_goto(args, true),
            "/goto" | "/go" | "/g" => self.slash_cd_or_goto(args, false),
            _ => return false,
        }
        true
    }

    /// Shared implementation for `/cd` and `/goto`: navigate to a path or
    /// fall back to the home directory / goto dialog if no args.
    fn slash_cd_or_goto(&mut self, args: &str, cd_style: bool) {
        if args.is_empty() {
            if cd_style {
                if let Some(home) = dirs::home_dir() {
                    self.navigate_to(home);
                }
            } else {
                self.dispatch(Action::GotoDirectoryDialog);
            }
            return;
        }
        let path = if args.starts_with('~') {
            dirs::home_dir()
                .unwrap_or_default()
                .join(args.trim_start_matches("~/"))
        } else if args.starts_with('/') {
            PathBuf::from(args)
        } else {
            self.active_tree_ref().root.join(args)
        };
        if path.is_dir() {
            self.navigate_to(path);
        } else {
            self.feedback.error(format!("Not a directory: {}", args));
        }
    }
}
