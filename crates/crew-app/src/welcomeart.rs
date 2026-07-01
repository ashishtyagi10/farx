//! Animated "dev at the terminal" scene for the welcome screen: a little figure
//! typing at a monitor whose shell-prompt cursor blinks and whose hand taps.
//! Split from `welcome.rs` so each file stays within the line budget.
use crew_render::CellView;

/// Scene width/height in cells (figure + monitor, side by side).
pub const SCENE_W: u16 = 26;
pub const SCENE_H: u16 = 3;
/// Poll-frames each blink/tap pose holds (~20 fps → ~2.5 poses/sec).
const BEAT: u64 = 8;
const PROMPT: &str = "crew:~$";

/// Push `s`'s non-space chars at `row`, starting at column `x`, in `color`.
/// Spaces advance the cursor without drawing, so the page shows through.
/// Returns the column just past the string.
#[rustfmt::skip]
fn seg(cells: &mut Vec<CellView>, row: u16, x: u16, s: &str, color: (u8, u8, u8), bg: (u8, u8, u8)) -> u16 {
    for (i, ch) in s.chars().enumerate() {
        if ch != ' ' {
            cells.push(CellView { col: x + i as u16, row, c: ch, fg: color, bg, bold: false, italic: false });
        }
    }
    x + s.chars().count() as u16
}

/// Draw the worker scene with its top-left at `(top, left)`. `fg` colours the
/// figure and monitor frame; `accent` colours the live shell prompt + cursor.
/// Two-pose animation keyed off `tick`: the hand flicks and the cursor blinks.
#[rustfmt::skip]
pub fn scene(cells: &mut Vec<CellView>, top: u16, left: u16, tick: u64,
             fg: (u8, u8, u8), accent: (u8, u8, u8), bg: (u8, u8, u8)) {
    let f = (tick / BEAT) % 2;
    let hand = if f == 0 { '╯' } else { '╮' }; // wrist tap
    let cur = if f == 0 { '▋' } else { ' ' };  // blinking prompt cursor
    let bar = "─".repeat(14);

    // Figure head/legs rows and the monitor's top/bottom bezel, in the figure hue.
    seg(cells, top,     left, &format!(" (•_•)    ┌{bar}┐"), fg, bg);
    seg(cells, top + 2, left, &format!(r" /    \   └{bar}┘"), fg, bg);

    // Middle row: torso + tapping hand + monitor bezel, then the shell prompt in
    // the accent hue with the blinking cursor block just after it.
    let y = top + 1;
    let mut x = left;
    x = seg(cells, y, x, &format!("<)   ){hand}   │  "), fg, bg);
    x = seg(cells, y, x, PROMPT, accent, bg);
    x = seg(cells, y, x, " ", fg, bg);
    x = seg(cells, y, x, &cur.to_string(), accent, bg);
    seg(cells, y, x, "   │", fg, bg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_stays_within_its_box() {
        let mut cells = Vec::new();
        scene(&mut cells, 10, 4, 0, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        assert!(!cells.is_empty());
        assert!(cells
            .iter()
            .all(|c| c.col >= 4 && c.col < 4 + SCENE_W && c.row >= 10 && c.row < 10 + SCENE_H));
    }

    #[test]
    fn prompt_is_drawn_in_accent() {
        let mut cells = Vec::new();
        scene(&mut cells, 0, 0, 0, (1, 1, 1), (9, 9, 9), (0, 0, 0));
        // The "crew:~$" glyphs carry the accent colour.
        assert!(cells.iter().any(|c| c.c == 'c' && c.fg == (9, 9, 9)));
    }

    #[test]
    fn animation_differs_between_poses() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        scene(&mut a, 0, 0, 0, (1, 1, 1), (9, 9, 9), (0, 0, 0));
        scene(&mut b, 0, 0, BEAT, (1, 1, 1), (9, 9, 9), (0, 0, 0));
        // Cursor blinks off on the second pose → fewer drawn cells, and the hand
        // glyph changes. The two frames must not be identical.
        let chars = |v: &[CellView]| v.iter().map(|c| (c.col, c.row, c.c)).collect::<Vec<_>>();
        assert_ne!(chars(&a), chars(&b));
    }
}
