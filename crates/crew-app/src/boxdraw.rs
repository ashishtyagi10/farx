//! Box-drawing helpers for grouping sidebar sections into bordered cards.
use crew_render::CellView;

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
    }
}

/// Draw a rounded-corner box outline `╭─╮ │ ╰─╯` spanning the inclusive cell
/// rectangle `[left..=right] x [top..=bottom]`.
pub fn rounded_box(
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = Vec::new();
    if right <= left || bottom <= top {
        return v;
    }
    v.push(cell(left, top, '╭', fg, bg));
    v.push(cell(right, top, '╮', fg, bg));
    v.push(cell(left, bottom, '╰', fg, bg));
    v.push(cell(right, bottom, '╯', fg, bg));
    for col in (left + 1)..right {
        v.push(cell(col, top, '─', fg, bg));
        v.push(cell(col, bottom, '─', fg, bg));
    }
    for row in (top + 1)..bottom {
        v.push(cell(left, row, '│', fg, bg));
        v.push(cell(right, row, '│', fg, bg));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_box_has_four_corners() {
        let cells = rounded_box(0, 0, 5, 4, (0, 255, 160), (8, 8, 16));
        let has = |ch: char| cells.iter().any(|c| c.c == ch);
        assert!(has('╭') && has('╮') && has('╰') && has('╯'));
    }

    #[test]
    fn rounded_box_degenerate_is_empty() {
        assert!(rounded_box(5, 5, 5, 5, (0, 0, 0), (0, 0, 0)).is_empty());
    }
}
