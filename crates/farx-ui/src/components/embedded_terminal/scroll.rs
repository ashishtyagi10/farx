//! Scrollback navigation for an embedded terminal session. The vt100 parser
//! keeps a history buffer; these helpers move the visible window through it.

use super::session::TerminalSession;

/// Rows moved per mouse-wheel notch.
pub const SCROLL_STEP: usize = 3;

impl TerminalSession {
    /// Scroll up into the scrollback history by `n` rows. Clamped to the
    /// buffer size by the vt100 parser.
    pub fn scroll_up(&mut self, n: usize) {
        let pos = self.parser.screen().scrollback();
        self.parser.set_scrollback(pos + n);
    }

    /// Scroll down toward the live bottom by `n` rows.
    pub fn scroll_down(&mut self, n: usize) {
        let pos = self.parser.screen().scrollback();
        self.parser.set_scrollback(pos.saturating_sub(n));
    }

    /// Current scrollback offset in rows (0 = pinned to the live bottom).
    pub fn scrollback(&self) -> usize {
        self.parser.screen().scrollback()
    }
}
