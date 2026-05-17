//! Modal / overlay render stack. Each `if let Some(...)` paints over what
//! came before. Drawing order is significant: the update modal is painted
//! last so it always sits on top.

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::components::ai_bar::render_ai_bar;
use crate::components::ai_panel::render_ai_panel;
use crate::components::batch_rename::render_batch_rename;
use crate::components::bookmarks::render_bookmarks;
use crate::components::chmod_dialog::render_chmod_dialog;
use crate::components::dialog::render_dialog;
use crate::components::feedback::render_feedback;
use crate::components::fuzzy_finder::render_fuzzy_finder;
use crate::components::menu::render_menu;
use crate::components::progress::render_progress;
use crate::components::quick_actions::render_quick_actions;
use crate::components::search::render_search;
use crate::components::update_modal::render_update_modal;

use super::super::App;

impl App {
    /// Paint the stack of optional overlays (menu, search, AI bar, dialogs,
    /// progress, output, update modal). `panel_area` is the file-panel
    /// rect used as the canvas for the scrollable output panel.
    pub(super) fn render_overlays(&self, frame: &mut Frame, panel_area: Rect) {
        if let Some(ref menu) = self.menu {
            render_menu(frame, menu, &self.theme);
        }

        if let Some(ref search) = self.search {
            render_search(frame, search, &self.theme);
        }

        if let Some(ref ai_bar) = self.ai_bar {
            render_ai_bar(frame, ai_bar, &self.theme);
        }

        if let Some(ref bm_panel) = self.bookmarks_panel {
            render_bookmarks(frame, bm_panel, &self.theme);
        }

        if let Some(ref ff) = self.fuzzy_finder {
            render_fuzzy_finder(frame, ff, &self.theme);
        }

        if let Some(ref qa) = self.quick_actions {
            render_quick_actions(frame, qa, &self.theme);
        }

        if let Some(ref ai_panel) = self.ai_panel {
            render_ai_panel(frame, ai_panel, &self.theme);
        }

        if let Some(ref br) = self.batch_rename {
            render_batch_rename(frame, br, &self.theme);
        }

        if let Some(ref chmod) = self.chmod_dialog {
            render_chmod_dialog(frame, chmod, &self.theme);
        }

        if let Some(ref dialog) = self.dialog {
            render_dialog(frame, dialog, &self.theme);
        }

        if let Some(ref progress) = self.progress {
            render_progress(frame, progress, &self.theme);
        }

        if self.feedback.output_visible {
            render_feedback(frame, panel_area, &self.feedback);
        }

        if let Some(ref state) = self.update_state {
            render_update_modal(frame, state, &self.theme);
        }
    }
}
