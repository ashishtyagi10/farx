//! The empty-screen welcome: ASCII-art CREW banner centred on the canvas
//! with a per-column shimmer, a tagline + keyboard hint, and a version stamp.
use crew_render::CellView;

/// ASCII-art "CREW" — Standard figlet style; every row padded to equal width.
const BANNER: &[&str] = &[
    r"  ____   ____   _____  __        __",
    r" / ___| |  _ \ | ____| \ \      / /",
    r"| |     | |_) ||  _|    \ \ /\ / / ",
    r"| |___  |  _ < | |___    \ V  V /  ",
    r" \____| |_| \_||_____|    \_/\_/   ",
];
/// Character width every banner line must equal (asserted in tests).
pub const BANNER_W: u16 = 35;
const BANNER_H: u16 = BANNER.len() as u16;
const TAGLINE: &str = "fast terminals. clean flow.";
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
/// Ticks for one full wave cycle (brighten → dim → brighten).
const PULSE: u64 = 56;
/// Poll ticks per rendered frame; idle animation runs at ~20 fps.
pub const ANIM_DIV: u64 = 3;

/// Whether this poll `tick` should redraw the welcome screen.
pub fn anim_should_redraw(tick: u64) -> bool {
    tick.is_multiple_of(ANIM_DIV)
}

/// Per-column shimmer: returns `(fg, bold)` for banner column `col` at `tick`.
/// Phase offset proportional to `col` creates a left-to-right wave across the art.
fn col_style(tick: u64, col: u16) -> ((u8, u8, u8), bool) {
    let phase = (tick / 2 + u64::from(col) * 3) % PULSE;
    let half = PULSE / 2;
    let dist = if phase < half { phase } else { PULSE - phase };
    if dist == 0 {
        return (crate::palette::accent(), true);
    }
    let t = crew_theme::theme();
    let frac = dist as f32 / half as f32;
    let lerp = |a: u8, b: u8| (a as f32 + frac * (b as f32 - a as f32)) as u8;
    let (ar, ag, ab) = crate::palette::accent();
    let (dr, dg, db) = t.text_muted;
    ((lerp(ar, dr), lerp(ag, dg), lerp(ab, db)), false)
}

/// Push every character of `s` as cells starting at `(col, row)`.
// rustfmt::skip keeps the CellView struct literal on one line (7 fields → 9-line expand otherwise).
#[rustfmt::skip]
fn push_str(cells: &mut Vec<CellView>, row: u16, col: u16, s: &str, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    for (i, ch) in s.chars().enumerate() {
        cells.push(CellView { col: col + i as u16, row, c: ch, fg, bg, bold: false, italic: false });
    }
}

/// Render one animation frame: CREW ASCII-art banner centred, tagline + hint
/// below, version stamp bottom-right. Falls back to a spaced single-line "CREW"
/// when the banner doesn't fit (too few cols or rows). All cells stay within
/// `cols × rows`.
// rustfmt::skip preserves compact inline struct literals to stay within the 200-line budget.
#[rustfmt::skip]
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 { return Vec::new(); }
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let bg = t.page_bg;

    if BANNER_W < cols && BANNER_H + 4 < rows {
        let top  = (rows.saturating_sub(BANNER_H + 4)) / 2;
        let left = (cols - BANNER_W) / 2;
        for (li, line) in BANNER.iter().enumerate() {
            let row = top + li as u16;
            if row >= rows { break; }
            for (ci, ch) in line.chars().enumerate() {
                if ch == ' ' { continue; }
                let abs_col = left + ci as u16;
                if abs_col >= cols { break; }
                let (fg, bold) = col_style(tick, ci as u16);
                cells.push(CellView { col: abs_col, row, c: ch, fg, bg, bold, italic: false });
            }
        }
        // Tagline one row below the banner.
        let tl_row = top + BANNER_H + 1;
        let tl_w = TAGLINE.chars().count() as u16;
        if tl_row < rows && tl_w < cols {
            push_str(&mut cells, tl_row, (cols - tl_w) / 2, TAGLINE, t.hint_fg, bg);
        }
        // Keyboard hint below the tagline.
        let hint_row = tl_row + 1;
        let hint_w = HINT.chars().count() as u16;
        if hint_row < rows && hint_w < cols {
            push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
        }
    } else {
        // Fallback: spaced single-line "CREW" with per-column shimmer.
        let letters: Vec<char> = "CREW".chars().collect();
        let span = (letters.len() as u16 - 1) * 2 + 1;
        if span < cols {
            let row   = rows / 2;
            let start = (cols - span) / 2;
            for (i, &ch) in letters.iter().enumerate() {
                let (fg, bold) = col_style(tick, i as u16 * 7);
                cells.push(CellView { col: start + i as u16 * 2, row, c: ch, fg, bg, bold, italic: false });
            }
            let hint_w   = HINT.chars().count() as u16;
            let hint_row = row + 2;
            if hint_w < cols && hint_row < rows {
                push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
            }
        }
    }

    // Version stamp bottom-right.
    let ver = concat!("v", env!("CARGO_PKG_VERSION"));
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols {
        push_str(&mut cells, rows - 1, cols - vw - 1, ver, t.dim, bg);
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_cells_in_bounds() {
        let cells = welcome_cells_animated(80, 24, 7);
        assert!(!cells.is_empty());
        assert!(
            cells.iter().all(|c| c.col < 80 && c.row < 24),
            "cell out of 80×24 bounds"
        );
    }

    #[test]
    fn banner_lines_equal_width() {
        for line in BANNER {
            let w = line.chars().count() as u16;
            assert_eq!(
                w, BANNER_W,
                "banner line width {w} ≠ BANNER_W {BANNER_W}: {line:?}"
            );
        }
    }

    #[test]
    fn hint_present() {
        let cells = welcome_cells_animated(80, 24, 0);
        let hint_fg = crew_theme::theme().hint_fg;
        assert!(
            cells.iter().any(|c| c.fg == hint_fg),
            "no hint_fg cells in welcome output"
        );
    }

    #[test]
    fn version_stamp_present() {
        let cells = welcome_cells_animated(80, 24, 0);
        let dim = crew_theme::theme().dim;
        assert!(
            cells
                .iter()
                .any(|c| c.c == 'v' && c.row == 23 && c.fg == dim),
            "no version stamp on bottom row"
        );
    }

    #[test]
    fn shimmer_changes_over_time() {
        let a = col_style(0, 0);
        let b = col_style(20, 0);
        assert_ne!(a, b, "shimmer colour did not change between frames");
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }

    #[test]
    fn empty_screen_produces_cells() {
        assert!(!welcome_cells_animated(80, 24, 0).is_empty());
    }

    #[test]
    fn anim_redraws_one_in_every_anim_div_ticks() {
        let redraws = (0..ANIM_DIV * 4).filter(|&t| anim_should_redraw(t)).count();
        assert_eq!(redraws as u64, 4, "one redraw per ANIM_DIV ticks");
        assert!(anim_should_redraw(0) && anim_should_redraw(ANIM_DIV));
        assert!(!anim_should_redraw(1));
    }
}
