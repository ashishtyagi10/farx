//! Section dividers for the sidebar: a single horizontal rule with an inline
//! legend embedded near the left edge — the HTML `<fieldset>`/`<legend>` look
//! flattened to one line: `─ TITLE ──────────`. Sidebar sections stack as these
//! rules (no enclosing box) with their content beneath.
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

/// Draw a horizontal rule across `[1..=cols-2]` on row 0 with `title` embedded
/// near the left (`─ TITLE ──────`). The rule uses `border`; the title uses
/// `title_fg`. Callers shift the returned cells to the section's top row.
pub fn section_header(
    title: &str,
    cols: u16,
    border: (u8, u8, u8),
    title_fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> Vec<CellView> {
    let mut v = Vec::new();
    if cols < 4 {
        return v;
    }
    let right = cols - 2; // inclusive last column of the rule
    let mut col = 1u16;
    v.push(cell(col, 0, '─', border, bg));
    col += 1;
    if col <= right {
        v.push(cell(col, 0, ' ', border, bg));
        col += 1;
    }
    for tc in title.chars() {
        if col > right {
            break;
        }
        v.push(cell(col, 0, tc, title_fg, bg));
        col += 1;
    }
    if col <= right {
        v.push(cell(col, 0, ' ', border, bg));
        col += 1;
    }
    while col <= right {
        v.push(cell(col, 0, '─', border, bg));
        col += 1;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_header_has_rule_and_legend() {
        let cells = section_header("SYS", 16, (70, 130, 140), (0, 255, 160), (8, 8, 16));
        // a horizontal rule is drawn on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        // the legend sits on the same row in the title colour
        assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
        // no box glyphs — this is a flat divider, not a card
        assert!(!cells
            .iter()
            .any(|c| matches!(c.c, '╭' | '╮' | '╰' | '╯' | '│')));
    }

    #[test]
    fn section_header_too_narrow_is_empty() {
        assert!(section_header("x", 3, (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
    }
}
