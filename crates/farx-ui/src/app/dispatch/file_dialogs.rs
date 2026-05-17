//! Dialog-triggering actions: copy/move/delete confirms, mkdir/rename/touch
//! input dialogs, goto / symlink / select-by-mask, chmod, diff.

use farx_core::Action;

use crate::components::chmod_dialog::ChmodDialogState;
use crate::components::dialog::DialogState;
use crate::components::diff_view::DiffViewState;
use crate::components::feedback::ConfirmAction;

use super::super::pending::PendingOperation;
use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_file_dialogs(&mut self, action: &Action) -> bool {
        match action {
            Action::CopyDialog => {
                let sources = self.collect_selected_paths();
                if sources.is_empty() {
                    return true;
                }
                let names = self.collect_selected_names();
                let dest = self.inactive_tree_root();
                let detail = format!("{} → {}", names.join(", "), dest.display());
                self.feedback
                    .ask_confirm("Copy?", detail, ConfirmAction::Copy { sources, dest });
            }
            Action::CopySameDir => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    if node.entry.name == ".." || node.entry.is_dir {
                        return true;
                    }
                    let source = node.entry.path.clone();
                    let default_name = format!("{}_copy", node.entry.name);
                    self.pending_op = Some(PendingOperation::CopySameDir { source });
                    self.dialog = Some(DialogState::new_input(
                        "Copy to same directory",
                        "Enter new name:",
                        default_name,
                    ));
                }
            }
            Action::MoveDialog => {
                let sources = self.collect_selected_paths();
                if sources.is_empty() {
                    return true;
                }
                let names = self.collect_selected_names();
                let dest = self.inactive_tree_root();
                let detail = format!("{} → {}", names.join(", "), dest.display());
                self.feedback
                    .ask_confirm("Move?", detail, ConfirmAction::Move { sources, dest });
            }
            Action::DeleteDialog => {
                let targets = self.collect_selected_paths();
                if targets.is_empty() {
                    return true;
                }
                let names = self.collect_selected_names();
                let detail = names.join(", ");
                self.feedback
                    .ask_confirm("Delete?", detail, ConfirmAction::Delete { targets });
            }
            Action::MkDirDialog => {
                let parent = self.active_panel_ref().current_dir.clone();
                self.pending_op = Some(PendingOperation::MkDir { parent });
                self.dialog = Some(DialogState::new_input(
                    "Create directory",
                    "Enter directory name:",
                    "",
                ));
            }
            Action::RenameDialog => {
                if let Some(entry) = self.active_panel_ref().current_entry() {
                    if entry.name == ".." {
                        return true;
                    }
                    let original = entry.path.clone();
                    let current_name = entry.name.clone();
                    self.pending_op = Some(PendingOperation::Rename { original });
                    self.dialog = Some(DialogState::new_input(
                        "Rename",
                        "Enter new name:",
                        current_name,
                    ));
                }
            }
            Action::CreateFileDialog => {
                let parent = self.active_panel_ref().current_dir.clone();
                self.pending_op = Some(PendingOperation::CreateFile { parent });
                self.dialog = Some(DialogState::new_input(
                    "Create file",
                    "Enter file name:",
                    "",
                ));
            }
            Action::GotoDirectoryDialog => {
                let current = self.active_tree_ref().root.to_string_lossy().to_string();
                self.pending_op = Some(PendingOperation::GotoDirectory);
                self.dialog = Some(DialogState::new_input(
                    "Go to directory",
                    "Enter path:",
                    current,
                ));
            }
            Action::CreateSymlinkDialog => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    if node.entry.name == ".." {
                        return true;
                    }
                    let target = node.entry.path.clone();
                    let default_name = format!("{}_link", node.entry.name);
                    self.pending_op = Some(PendingOperation::CreateSymlink { target });
                    self.dialog = Some(DialogState::new_input(
                        "Create symlink",
                        "Enter link name:",
                        default_name,
                    ));
                }
            }
            Action::SelectByMaskDialog => {
                self.pending_op = Some(PendingOperation::SelectByMask);
                self.dialog = Some(DialogState::new_input(
                    "Select by mask",
                    "Enter pattern (e.g. *.rs, test*):",
                    "*",
                ));
            }
            Action::DeselectByMaskDialog => {
                self.pending_op = Some(PendingOperation::DeselectByMask);
                self.dialog = Some(DialogState::new_input(
                    "Deselect by mask",
                    "Enter pattern (e.g. *.rs, test*):",
                    "*",
                ));
            }
            Action::ChmodDialog => {
                #[cfg(unix)]
                {
                    if let Some(node) = self.active_tree_ref().current_node() {
                        if let Some(mode) = node.entry.mode {
                            self.chmod_dialog =
                                Some(ChmodDialogState::new(node.entry.path.clone(), mode));
                        } else {
                            self.feedback
                                .error("Cannot read file permissions".to_string());
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    self.feedback
                        .error("Chmod is only available on Unix systems".to_string());
                }
            }
            Action::DiffFiles => {
                let left_file = self
                    .left_tree
                    .current_node()
                    .and_then(|n| (!n.entry.is_dir).then(|| n.entry.path.clone()));
                let right_file = self
                    .right_tree
                    .current_node()
                    .and_then(|n| (!n.entry.is_dir).then(|| n.entry.path.clone()));
                match (left_file, right_file) {
                    (Some(left), Some(right)) => match DiffViewState::new(left, right) {
                        Ok(dv) => self.diff_view = Some(dv),
                        Err(e) => self.feedback.error(format!("Diff failed: {}", e)),
                    },
                    _ => self
                        .feedback
                        .error("Place cursor on a file in each panel to diff".to_string()),
                }
            }
            _ => return false,
        }
        true
    }
}
