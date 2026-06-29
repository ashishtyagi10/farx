//! The app's themeable colour palette. Render code is a web of free functions
//! that never see the config, so the one user-tunable colour — the **accent**
//! (Crew green by default) — lives here behind a lock-free global set once at
//! startup (and re-set by `/reload` / settings). `accent()` returns the built-in
//! default until `set_accent` is called, so tests and headless paths are
//! unaffected.
use std::sync::atomic::{AtomicU32, Ordering};

/// The built-in accent: Crew green.
pub const DEFAULT_ACCENT: (u8, u8, u8) = (0, 255, 160);

fn pack((r, g, b): (u8, u8, u8)) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn unpack(v: u32) -> (u8, u8, u8) {
    ((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Packed accent RGB, initialised to [`DEFAULT_ACCENT`].
static ACCENT: AtomicU32 = AtomicU32::new((DEFAULT_ACCENT.1 as u32) << 8 | DEFAULT_ACCENT.2 as u32);

/// Set the active accent colour (called from config at startup / on reload).
pub fn set_accent(rgb: (u8, u8, u8)) {
    ACCENT.store(pack(rgb), Ordering::Relaxed);
}

/// The active accent colour — [`DEFAULT_ACCENT`] until [`set_accent`] is called.
pub fn accent() -> (u8, u8, u8) {
    unpack(ACCENT.load(Ordering::Relaxed))
}

/// The active accent as a ratatui [`Color`](ratatui::style::Color), for the
/// overlay widgets (help / command menu / settings / far) drawn with ratatui.
pub fn accent_color() -> ratatui::style::Color {
    let (r, g, b) = accent();
    ratatui::style::Color::Rgb(r, g, b)
}

/// Parse a `#rrggbb` / `rrggbb` hex string into an RGB triple. Returns `None`
/// for anything that isn't exactly six hex digits (optionally `#`-prefixed).
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Serialises tests that read or mutate the accent global. Any test that calls
/// [`set_accent`] — or asserts against [`accent`]/[`accent_color`] — should hold
/// this guard so the process-wide value isn't changed mid-assertion by a
/// concurrently-running test.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_round_trips() {
        for rgb in [(0, 255, 160), (255, 255, 255), (0, 0, 0), (18, 200, 7)] {
            assert_eq!(unpack(pack(rgb)), rgb);
        }
    }

    #[test]
    fn static_initialiser_matches_default() {
        // Serialise with the tests that mutate the accent global, so we read it
        // at rest rather than mid-`set_accent`.
        let _g = crate::palette::test_guard();
        // The const-expr initialiser must equal pack(DEFAULT_ACCENT).
        assert_eq!(ACCENT.load(Ordering::Relaxed), pack(DEFAULT_ACCENT));
    }

    #[test]
    fn parse_hex_accepts_with_and_without_hash() {
        assert_eq!(parse_hex("#00ffa0"), Some((0, 255, 160)));
        assert_eq!(parse_hex("00FFA0"), Some((0, 255, 160)));
        assert_eq!(parse_hex("  #123456 "), Some((0x12, 0x34, 0x56)));
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#fff"), None); // shorthand not supported
        assert_eq!(parse_hex("#gggggg"), None);
        assert_eq!(parse_hex("0xffaa00"), None);
    }

    #[test]
    fn set_then_accent_round_trips() {
        // Serialise with any other test that reads the accent global.
        let _g = crate::palette::test_guard();
        set_accent((10, 20, 30));
        assert_eq!(accent(), (10, 20, 30));
        assert_eq!(accent_color(), ratatui::style::Color::Rgb(10, 20, 30));
        set_accent(DEFAULT_ACCENT); // restore so other tests see the default
        assert_eq!(accent(), DEFAULT_ACCENT);
    }
}
