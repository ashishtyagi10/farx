//! History and bookmarks: directory back/forward, bookmark list/add,
//! recent directories view.

use farx_core::{Action, PanelSide};

use crate::components::bookmarks::{save_bookmarks, Bookmark, BookmarkState};

use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_nav(&mut self, action: &Action) -> bool {
        match action {
            Action::HistoryBack => {
                let went_back = self.active_tree().go_back();
                if went_back {
                    let new_root = self.active_tree_ref().root.clone();
                    match self.active_panel {
                        PanelSide::Left => self.left_panel.current_dir = new_root,
                        PanelSide::Right => self.right_panel.current_dir = new_root,
                    }
                } else {
                    self.feedback.info("No history to go back to".to_string());
                }
            }
            Action::HistoryForward => {
                let went_fwd = self.active_tree().go_forward();
                if went_fwd {
                    let new_root = self.active_tree_ref().root.clone();
                    match self.active_panel {
                        PanelSide::Left => self.left_panel.current_dir = new_root,
                        PanelSide::Right => self.right_panel.current_dir = new_root,
                    }
                } else {
                    self.feedback
                        .info("No history to go forward to".to_string());
                }
            }
            Action::ShowBookmarks => {
                self.bookmarks_panel = Some(BookmarkState::new(self.bookmarks.clone()));
            }
            Action::AddBookmark => {
                let dir = self.active_tree_ref().root.clone();
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                if self.bookmarks.iter().any(|b| b.path == dir) {
                    self.feedback.info("Already bookmarked".to_string());
                } else {
                    self.bookmarks.push(Bookmark {
                        name,
                        path: dir.clone(),
                    });
                    save_bookmarks(&self.bookmarks);
                    self.feedback.info(format!("Bookmarked: {}", dir.display()));
                }
            }
            Action::ShowRecentDirectories => {
                let tree = self.active_tree_ref();
                let mut dirs: Vec<String> = Vec::new();
                dirs.push(format!("  * {}", tree.root.display()));
                for dir in tree.history_back.iter().rev() {
                    dirs.push(format!("    {}", dir.display()));
                }
                if dirs.len() <= 1 {
                    self.feedback.info("No directory history yet".to_string());
                } else {
                    self.feedback
                        .show_output("Recent Directories", dirs.join("\n"));
                }
            }
            _ => return false,
        }
        true
    }
}
