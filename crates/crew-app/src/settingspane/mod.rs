//! Settings form pane: a two-column layout with Tab navigation, a type-to-search
//! font-family dropdown, editable numeric fields, and Save/Cancel buttons.
mod keys;
mod render;

use crew_render::CellView;
use winit::event::KeyEvent;

use crate::config::CrewConfig;

/// Label shown for "no explicit family — use the system monospace".
pub(crate) const DEFAULT_FAMILY_LABEL: &str = "System monospace";

/// Focusable elements of the form, in Tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    FontFamily,
    FontSize,
    NavWidth,
    ShowNav,
    Save,
    Cancel,
}

pub(crate) const FIELDS: [Field; 6] = [
    Field::FontFamily,
    Field::FontSize,
    Field::NavWidth,
    Field::ShowNav,
    Field::Save,
    Field::Cancel,
];

/// Outcome of a key press in the settings form.
pub enum SettingsAction {
    /// Save: apply this config and close the pane.
    Apply(CrewConfig),
    /// Cancel: discard edits and close the pane.
    Cancel,
}

pub struct SettingsPane {
    pub(crate) draft: CrewConfig,
    pub(crate) families: Vec<String>,
    pub(crate) focus: usize,
    pub(crate) family_query: String,
    pub(crate) family_open: bool,
    pub(crate) family_sel: usize,
    pub(crate) size_buf: String,
    pub(crate) nav_buf: String,
}

impl SettingsPane {
    pub fn new(cfg: CrewConfig, families: Vec<String>) -> Self {
        let family_query = cfg
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FAMILY_LABEL.to_string());
        let size_buf = format!("{}", cfg.font_size as i32);
        let nav_buf = format!("{}", cfg.nav_width as i32);
        Self {
            draft: cfg,
            families,
            focus: 0,
            family_query,
            family_open: false,
            family_sel: 0,
            size_buf,
            nav_buf,
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent, shift: bool) -> Option<SettingsAction> {
        keys::reduce(self, key, shift)
    }

    /// Mouse-wheel / page scroll: move the open font dropdown's selection, or
    /// otherwise step field focus (committing each field on the way). Positive
    /// `lines` moves toward the top.
    pub fn scroll(&mut self, lines: i32) {
        if self.family_open {
            let n = self.filtered().len() as i64;
            if n > 0 {
                self.family_sel = (self.family_sel as i64 - lines as i64).clamp(0, n - 1) as usize;
            }
            return;
        }
        let up = lines > 0;
        for _ in 0..lines.unsigned_abs().min(FIELDS.len() as u32) {
            if (up && self.focus == 0) || (!up && self.focus == FIELDS.len() - 1) {
                break;
            }
            keys::commit_field(self);
            keys::move_focus(self, up);
        }
    }

    pub(crate) fn focused_field(&self) -> Field {
        FIELDS[self.focus.min(FIELDS.len() - 1)]
    }

    /// Family names matching the current query (case-insensitive substring),
    /// always led by the system-default label.
    pub(crate) fn filtered(&self) -> Vec<String> {
        let q = self.family_query.to_lowercase();
        let mut out = Vec::new();
        if q.is_empty() || DEFAULT_FAMILY_LABEL.to_lowercase().contains(&q) {
            out.push(DEFAULT_FAMILY_LABEL.to_string());
        }
        for f in &self.families {
            if q.is_empty() || f.to_lowercase().contains(&q) {
                out.push(f.clone());
            }
        }
        out
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
