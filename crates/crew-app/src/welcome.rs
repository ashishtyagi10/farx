//! The empty-screen welcome: the word CREW centred on the canvas with a pulsing
//! glow, a keyboard hint below it, and a version stamp in the corner.
use crew_render::CellView;

const WORD: &str = "CREW";
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
/// Columns between successive letters.
const STEP: u16 = 2;
/// Frames for one brighten→dim→brighten pulse of a letter.
const PULSE: u64 = 56;
/// Poll ticks per rendered frame: the poll loop runs at ~60 Hz, but the
/// idle welcome animation only needs ~20 fps, so we redraw every third tick and
/// advance the animation frame by one — cutting idle redraws to a third.
pub const ANIM_DIV: u64 = 3;

/// Whether this poll `tick` should redraw the welcome screen (every [`ANIM_DIV`]).
pub fn anim_should_redraw(tick: u64) -> bool {
    tick.is_multiple_of(ANIM_DIV)
}

/// Colour for letter `i` at `tick`: each letter pulses between the accent
/// colour and a muted dim, out of phase with the others.
fn letter_style(tick: u64, i: usize) -> ((u8, u8, u8), bool) {
    let phase = (tick / 2 + i as u64 * 11) % PULSE;
    let half = PULSE / 2;
    let dist = if phase < half { phase } else { PULSE - phase }; // 0 = brightest
    if dist == 0 {
        return (crate::palette::accent(), true);
    }
    let t = crew_theme::theme();
    let frac = dist as f32 / half as f32; // 0..1 → bright..dim
    let lerp = |a: u8, b: u8| (a as f32 + frac * (b as f32 - a as f32)) as u8;
    let acc = crate::palette::accent();
    let dim = t.text_muted;
    (
        (lerp(acc.0, dim.0), lerp(acc.1, dim.1), lerp(acc.2, dim.2)),
        false,
    )
}

/// Render one animation frame: CREW wordmark centred, hint below, version stamp.
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();

    let letters: Vec<char> = WORD.chars().collect();
    let span = (letters.len() as u16 - 1) * STEP + 1;
    if span < cols {
        let start_col = (cols - span) / 2;
        let row = rows / 2;
        let t = crew_theme::theme();
        for (i, &ch) in letters.iter().enumerate() {
            let (fg, bold) = letter_style(tick, i);
            cells.push(CellView {
                col: start_col + i as u16 * STEP,
                row,
                c: ch,
                fg,
                bg: t.page_bg,
                bold,
                italic: false,
            });
        }

        // A dim hint two rows below the wordmark, when it fits.
        let hint_w = HINT.chars().count() as u16;
        let hint_row = row + 2;
        if hint_w < cols && hint_row < rows {
            let hstart = (cols - hint_w) / 2;
            for (i, ch) in HINT.chars().enumerate() {
                cells.push(CellView {
                    col: hstart + i as u16,
                    row: hint_row,
                    c: ch,
                    fg: t.hint_fg,
                    bg: t.page_bg,
                    bold: false,
                    italic: false,
                });
            }
        }
    }

    // Version stamp in the bottom-right corner.
    let t = crew_theme::theme();
    let ver = concat!("v", env!("CARGO_PKG_VERSION"));
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols && rows > 0 {
        let vstart = cols - vw - 1;
        for (i, ch) in ver.chars().enumerate() {
            cells.push(CellView {
                col: vstart + i as u16,
                row: rows - 1,
                c: ch,
                fg: t.dim,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_crew_in_bounds() {
        let cells = welcome_cells_animated(80, 24, 7);
        assert!(cells.iter().all(|c| c.col < 80 && c.row < 24));
        // every CREW letter is present on the center row
        for ch in WORD.chars() {
            assert!(cells.iter().any(|c| c.c == ch && c.row == 12));
        }
        // the dim hint is shown two rows below the wordmark
        assert!(cells
            .iter()
            .any(|c| c.row == 14 && c.fg == crew_theme::theme().hint_fg));
        // a faint version stamp sits on the bottom row
        assert!(cells
            .iter()
            .any(|c| c.c == 'v' && c.row == 23 && c.fg == crew_theme::theme().dim));
    }

    #[test]
    fn letters_pulse_over_time() {
        // a letter's colour changes between frames (shimmer, not constant)
        let a = letter_style(0, 0);
        let b = letter_style(20, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }

    #[test]
    fn anim_redraws_one_in_every_anim_div_ticks() {
        let redraws = (0..ANIM_DIV * 4).filter(|&t| anim_should_redraw(t)).count();
        assert_eq!(redraws as u64, 4, "one redraw per ANIM_DIV ticks");
        assert!(anim_should_redraw(0) && anim_should_redraw(ANIM_DIV));
        assert!(!anim_should_redraw(1));
    }

    #[test]
    fn empty_screen_produces_cells() {
        let cells = welcome_cells_animated(80, 24, 0);
        assert!(!cells.is_empty());
    }
}
