mod accessors;
mod ai_glue;
mod chrome;
mod commands;
mod confirm;
mod dialogs;
mod dispatch;
mod fs_watcher;
mod globs;
mod helpers;
mod keys;
mod lifecycle;
mod mouse;
mod pending;
mod render;
mod selection_ops;
mod shell_commands;
mod slash;
mod state;
mod terminals;
mod text_detect;
mod tick;
mod tools;
mod update_flow;

use std::path::PathBuf;

use farx_core::{Action, PanelSide, SortField};

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;
use crate::components::batch_rename::BatchRenameState;
use crate::components::bookmarks::{save_bookmarks, Bookmark, BookmarkState};
use crate::components::chmod_dialog::ChmodDialogState;
use crate::components::dialog::DialogState;
use crate::components::diff_view::DiffViewState;
use crate::components::editor::EditorState;
use crate::components::feedback::ConfirmAction;
use crate::components::fuzzy_finder::FuzzyFinderState;
use crate::components::help::HelpState;
use crate::components::menu::MenuState;
use crate::components::quick_actions::QuickActionsState;
use crate::components::search::SearchState;

pub use self::state::App;

use self::helpers::format_size_human;
use self::pending::{PendingOperation, UndoEntry};

impl App {
    /// Execute an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        if self.dispatch_tree_nav(&action) || self.dispatch_selection(&action) {
            return;
        }
    }
}
