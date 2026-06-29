//! Translate a scroll wheel into input bytes for a full-screen terminal program.
//! An alternate-screen app (vim, less, claude) has no scrollback of its own, so
//! the wheel must reach the program: as mouse-wheel events when it enabled mouse
//! reporting, or as arrow keys under xterm "alternate scroll" otherwise. A plain
//! shell (no alt-screen, no mouse) returns `None` and keeps Crew's own scrollback.
use crew_term::InputModes;

/// Most wheel ticks forwarded from a single event, so a fast flick can't flood
/// the program with hundreds of keypresses.
const MAX_TICKS: u32 = 10;

/// Bytes to forward a wheel scroll of `lines` (>0 = up/older) at the hovered,
/// 0-based `cell`, or `None` to fall back to local scrollback.
pub fn wheel_bytes(m: &InputModes, lines: i32, cell: (u16, u16)) -> Option<Vec<u8>> {
    if lines == 0 {
        return None;
    }
    let up = lines > 0;
    let n = lines.unsigned_abs().min(MAX_TICKS) as usize;
    if m.mouse {
        Some(repeat(&mouse_tick(m.sgr_mouse, up, cell), n))
    } else if m.alt_screen && m.alternate_scroll {
        Some(repeat(&arrow(m.app_cursor, up), n))
    } else {
        None
    }
}

/// Bytes for a page scroll (Shift+PageUp/Down) — a real PageUp/PageDown key, but
/// only inside the alternate screen; otherwise `None` so Crew pages scrollback.
pub fn page_bytes(m: &InputModes, up: bool) -> Option<Vec<u8>> {
    m.alt_screen.then(|| {
        if up {
            b"\x1b[5~".to_vec()
        } else {
            b"\x1b[6~".to_vec()
        }
    })
}

fn repeat(seq: &[u8], n: usize) -> Vec<u8> {
    seq.repeat(n)
}

/// A single cursor-up/down key: `ESC O A/B` under app-cursor mode, else `ESC [ A/B`.
fn arrow(app_cursor: bool, up: bool) -> Vec<u8> {
    let intro = if app_cursor { b'O' } else { b'[' };
    let dir = if up { b'A' } else { b'B' };
    vec![0x1b, intro, dir]
}

/// One wheel-button report. Wheel up/down are CSI mouse buttons 64/65.
fn mouse_tick(sgr: bool, up: bool, cell: (u16, u16)) -> Vec<u8> {
    let btn: u32 = if up { 64 } else { 65 };
    let x = cell.0 as u32 + 1; // mouse coordinates are 1-based
    let y = cell.1 as u32 + 1;
    if sgr {
        format!("\x1b[<{btn};{x};{y}M").into_bytes()
    } else {
        // Legacy X10: each field offset by 32, clamped into a single byte.
        let enc = |v: u32| (v + 32).min(255) as u8;
        vec![0x1b, b'[', b'M', enc(btn), enc(x), enc(y)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes() -> InputModes {
        InputModes::default()
    }

    #[test]
    fn plain_shell_keeps_local_scrollback() {
        // No alt-screen, no mouse → None, so Crew scrolls its own history.
        assert_eq!(wheel_bytes(&modes(), 3, (0, 0)), None);
        assert_eq!(wheel_bytes(&modes(), -3, (0, 0)), None);
    }

    #[test]
    fn zero_lines_is_noop() {
        let m = InputModes {
            alt_screen: true,
            alternate_scroll: true,
            ..modes()
        };
        assert_eq!(wheel_bytes(&m, 0, (0, 0)), None);
    }

    #[test]
    fn alt_screen_sends_arrow_keys() {
        let m = InputModes {
            alt_screen: true,
            alternate_scroll: true,
            ..modes()
        };
        // Two ticks up → two cursor-up keys (normal cursor mode).
        assert_eq!(wheel_bytes(&m, 2, (5, 5)), Some(b"\x1b[A\x1b[A".to_vec()));
        // Down → cursor-down.
        assert_eq!(wheel_bytes(&m, -1, (5, 5)), Some(b"\x1b[B".to_vec()));
    }

    #[test]
    fn app_cursor_uses_ss3_arrows() {
        let m = InputModes {
            alt_screen: true,
            alternate_scroll: true,
            app_cursor: true,
            ..modes()
        };
        assert_eq!(wheel_bytes(&m, 1, (0, 0)), Some(b"\x1bOA".to_vec()));
    }

    #[test]
    fn alt_screen_without_alternate_scroll_falls_back() {
        let m = InputModes {
            alt_screen: true,
            alternate_scroll: false,
            ..modes()
        };
        assert_eq!(wheel_bytes(&m, 1, (0, 0)), None);
    }

    #[test]
    fn mouse_mode_emits_sgr_wheel_at_one_based_cell() {
        let m = InputModes {
            mouse: true,
            sgr_mouse: true,
            ..modes()
        };
        // Hovered cell (3,5) → 1-based (4,6); wheel up = button 64.
        assert_eq!(wheel_bytes(&m, 1, (3, 5)), Some(b"\x1b[<64;4;6M".to_vec()));
        // Wheel down = button 65.
        assert_eq!(wheel_bytes(&m, -1, (3, 5)), Some(b"\x1b[<65;4;6M".to_vec()));
    }

    #[test]
    fn mouse_mode_legacy_encoding_offsets_by_32() {
        let m = InputModes {
            mouse: true,
            sgr_mouse: false,
            ..modes()
        };
        // button 64 -> 96, x=1 -> 33, y=1 -> 33.
        assert_eq!(
            wheel_bytes(&m, 1, (0, 0)),
            Some(vec![0x1b, b'[', b'M', 96, 33, 33])
        );
    }

    #[test]
    fn wheel_ticks_are_capped() {
        let m = InputModes {
            alt_screen: true,
            alternate_scroll: true,
            ..modes()
        };
        let bytes = wheel_bytes(&m, 1000, (0, 0)).unwrap();
        assert_eq!(bytes.len(), MAX_TICKS as usize * 3); // 3 bytes per arrow
    }

    #[test]
    fn page_bytes_only_in_alt_screen() {
        assert_eq!(page_bytes(&modes(), true), None);
        let alt = InputModes {
            alt_screen: true,
            ..modes()
        };
        assert_eq!(page_bytes(&alt, true), Some(b"\x1b[5~".to_vec()));
        assert_eq!(page_bytes(&alt, false), Some(b"\x1b[6~".to_vec()));
    }
}
