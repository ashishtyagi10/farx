//! Slash commands that operate on files or change panel/selection state.

use farx_core::Action;

use super::super::App;

impl App {
    /// Dispatch file/selection/panel slash commands. Returns `true` if matched.
    pub(super) fn slash_files(&mut self, cmd: &str, args: &str) -> bool {
        match cmd {
            "/yank" | "/copy-path" => self.dispatch(Action::CopyPathToClipboard),
            "/yank-names" | "/copy-names" => self.dispatch(Action::CopyNameToClipboard),
            "/checksum" | "/sha256" => self.dispatch(Action::ShowChecksums),
            "/chmod" | "/permissions" | "/perms" => self.dispatch(Action::ChmodDialog),
            "/actions" => self.dispatch(Action::ShowQuickActions),
            "/diff" | "/compare-files" => self.dispatch(Action::DiffFiles),
            "/ssh" => {
                if args.is_empty() {
                    self.feedback
                        .error("Usage: /ssh user@host:/path".to_string());
                } else {
                    self.dispatch(Action::SshBrowse(args.to_string()));
                }
            }
            "/duplicates" | "/dupes" => self.dispatch(Action::FindDuplicates),
            "/treemap" | "/usage" => self.dispatch(Action::ShowTreemap),
            "/rename-batch" | "/bulk-rename" => self.dispatch(Action::BatchRename),
            "/undo" => self.dispatch(Action::Undo),
            "/extract" => self.dispatch(Action::ExtractArchive),
            "/compress" | "/zip" => self.dispatch(Action::CompressSelection),
            "/size" => self.calculate_dir_size(),
            "/stats" | "/wc" => self.dispatch(Action::ShowFileStats),
            "/symlink" | "/ln" => self.dispatch(Action::CreateSymlinkDialog),
            "/select" => {
                if args.is_empty() {
                    self.dispatch(Action::SelectByMaskDialog);
                } else {
                    self.apply_mask_selection(args, true);
                }
            }
            "/deselect" => {
                if args.is_empty() {
                    self.dispatch(Action::DeselectByMaskDialog);
                } else {
                    self.apply_mask_selection(args, false);
                }
            }
            "/invert" => self.dispatch(Action::InvertSelection),
            "/compare" | "/cmp" => self.dispatch(Action::CompareDirectories),
            "/swap" => self.dispatch(Action::SwapPanels),
            "/open" => self.dispatch(Action::OpenSystemApp),
            "/terminal" | "/term" => self.dispatch(Action::OpenTerminalHere),
            "/touch" => self.dispatch(Action::TouchFile),
            _ => return false,
        }
        true
    }
}
