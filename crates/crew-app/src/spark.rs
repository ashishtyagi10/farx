//! Rolling history + line-chart rendering for the sidebar. A [`History`] keeps the
//! last N samples; [`line_cells`] traces them as a single-row braille **line**
//! chart converted to Crew cells. The chart "moves" as samples are pushed on the
//! sidebar's existing ~1 Hz refresh — it costs nothing beyond the repaint that
//! already happens each second, so animation never compromises performance.
use std::collections::VecDeque;

use crew_render::CellView;

const BG: (u8, u8, u8) = (0, 0, 0);

/// Fixed-capacity ring of recent samples (oldest at the front, newest at back).
pub struct History {
    cap: usize,
    data: VecDeque<u64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            cap,
            data: VecDeque::with_capacity(cap),
        }
    }

    /// Append a sample, dropping the oldest once capacity is reached.
    pub fn push(&mut self, v: u64) {
        if self.data.len() == self.cap {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The most recent `width` samples (or fewer), oldest first — what fills the
    /// chart left→right so the newest reading sits at the right edge.
    fn tail(&self, width: usize) -> Vec<u64> {
        let start = self.data.len().saturating_sub(width);
        self.data.iter().skip(start).copied().collect()
    }
}

/// Braille dot bit for sub-cell (`dot_col` 0..2 left→right, `dot_row` 0..4
/// top→bottom) per the Unicode Braille Patterns layout (U+2800 base).
fn braille_bit(dot_col: usize, dot_row: usize) -> u8 {
    // Left column = dots 1,2,3,7; right column = dots 4,5,6,8.
    const LEFT: [u8; 4] = [0x01, 0x02, 0x04, 0x40];
    const RIGHT: [u8; 4] = [0x08, 0x10, 0x20, 0x80];
    if dot_col == 0 {
        LEFT[dot_row]
    } else {
        RIGHT[dot_row]
    }
}

/// Render `hist` as a single-row braille **line** chart `width` cells wide, its
/// left edge at `col0` on `row`. Each cell packs a 2×4 dot grid, so the line has
/// two horizontal points and four vertical levels per cell. `max` scales the
/// height (e.g. 100 for a percentage); `0` auto-scales to the window's peak. The
/// newest sample sits at the right edge. Empty with no history or no width.
pub fn line_cells(
    hist: &History,
    width: u16,
    col0: u16,
    row: u16,
    max: u64,
    fg: (u8, u8, u8),
) -> Vec<CellView> {
    if width == 0 || hist.is_empty() {
        return Vec::new();
    }
    // Two braille sub-columns per cell → horizontal resolution of 2·width points.
    let subcols = width as usize * 2;
    let data = hist.tail(subcols);
    // Fixed scale when given, else auto-scale to the window's peak (min 1).
    let scale = if max > 0 {
        max
    } else {
        data.iter().copied().max().unwrap_or(1).max(1)
    };
    // Right-align so the newest sample lands on the rightmost sub-column.
    let offset = subcols - data.len();

    let mut bits = vec![0u8; width as usize];
    for (k, &v) in data.iter().enumerate() {
        let x = offset + k;
        let frac = (v as f64 / scale as f64).clamp(0.0, 1.0);
        // Map value to a vertical dot level: high value → top row (0).
        let height = (frac * 3.0).round() as usize; // 0..=3
        let dot_row = 3 - height;
        bits[x / 2] |= braille_bit(x % 2, dot_row);
    }

    let mut cells = Vec::new();
    for (i, &b) in bits.iter().enumerate() {
        if b == 0 {
            continue; // no point in this cell — leave it transparent
        }
        let c = char::from_u32(0x2800 + b as u32).unwrap_or(' ');
        cells.push(CellView {
            col: col0 + i as u16,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
    cells
}

/// Sidebar convenience: render `hist` as a percentage (0–100) line chart indented
/// under the section legend (col 3), spanning the rest of `cols` on `row`.
pub fn cpu_row(hist: &History, cols: u16, row: u16) -> Vec<CellView> {
    if cols <= 5 {
        return Vec::new();
    }
    let fg = crate::palette::accent();
    line_cells(hist, cols.saturating_sub(4), 3, row, 100, fg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_caps_and_keeps_newest() {
        let mut h = History::new(3);
        for v in [1, 2, 3, 4, 5] {
            h.push(v);
        }
        // capacity 3 keeps the three newest, oldest first
        assert_eq!(h.tail(10), vec![3, 4, 5]);
    }

    #[test]
    fn tail_returns_at_most_width() {
        let mut h = History::new(10);
        for v in [10, 20, 30, 40] {
            h.push(v);
        }
        assert_eq!(h.tail(2), vec![30, 40]);
    }

    #[test]
    fn empty_history_renders_nothing() {
        let h = History::new(8);
        assert!(line_cells(&h, 8, 0, 0, 100, (0, 255, 160)).is_empty());
    }

    #[test]
    fn renders_braille_line_in_bounds() {
        let mut h = History::new(32);
        for v in [10, 40, 70, 100, 0, 55, 80, 20] {
            h.push(v);
        }
        let cells = line_cells(&h, 8, 3, 4, 100, (0, 255, 160));
        assert!(!cells.is_empty());
        // shifted to the requested origin and within the requested width
        assert!(cells.iter().all(|c| c.row == 4));
        assert!(cells.iter().all(|c| (3..3 + 8).contains(&c.col)));
        // every glyph is a Unicode braille pattern, not a bar block
        assert!(cells
            .iter()
            .all(|c| ('\u{2800}'..='\u{28FF}').contains(&c.c)));
        assert!(!cells.iter().any(|c| c.c == '█'));
    }

    #[test]
    fn high_value_sits_above_low_value() {
        // A peak sample's dot must render higher (more top dots set) than a
        // trough sample's — proving the line tracks value height.
        let mut peak = History::new(2);
        peak.push(100);
        let top = &line_cells(&peak, 1, 0, 0, 100, (0, 0, 0))[0];

        let mut trough = History::new(2);
        trough.push(0);
        let bottom = &line_cells(&trough, 1, 0, 0, 100, (0, 0, 0))[0];

        // Top dot for a full value (dot 4, 0x08 at the rightmost sub-column),
        // bottom dot for a zero value (dot 8, 0x80).
        let top_bits = top.c as u32 - 0x2800;
        let bottom_bits = bottom.c as u32 - 0x2800;
        assert_eq!(top_bits, 0x08, "full value should set the top-right dot");
        assert_eq!(
            bottom_bits, 0x80,
            "zero value should set the bottom-right dot"
        );
    }
}
