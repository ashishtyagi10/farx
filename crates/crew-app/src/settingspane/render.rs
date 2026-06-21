//! Two-column form rendering for the settings pane, plus the font-family dropdown.
use crew_render::CellView;

use super::{Field, SettingsPane, ACCENT, BG, DIM, TEXT};

const LABEL_COL: u16 = 3;
const BOX_COL: u16 = 18;
const INNER_W: usize = 22;

/// Render the whole form into a flat `CellView` list.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let mut out = Vec::new();
    put(&mut out, LABEL_COL, 1, "Settings", ACCENT, cols, true);

    let f = p.focused_field();
    let nav = if p.draft.show_nav { "on" } else { "off" };
    field_row(
        &mut out,
        3,
        "Font family",
        &p.family_query,
        f == Field::FontFamily,
        true,
        cols,
    );
    field_row(
        &mut out,
        4,
        "Font size",
        &p.size_buf,
        f == Field::FontSize,
        true,
        cols,
    );
    field_row(
        &mut out,
        5,
        "Nav width",
        &p.nav_buf,
        f == Field::NavWidth,
        true,
        cols,
    );
    field_row(
        &mut out,
        6,
        "Show nav",
        nav,
        f == Field::ShowNav,
        false,
        cols,
    );

    button(&mut out, LABEL_COL + 2, 8, "Save", f == Field::Save, cols);
    button(
        &mut out,
        LABEL_COL + 14,
        8,
        "Cancel",
        f == Field::Cancel,
        cols,
    );

    if p.family_open {
        dropdown(&mut out, p, cols, rows);
    }
    out
}

/// One label + bracketed value-box row.
fn field_row(
    out: &mut Vec<CellView>,
    row: u16,
    label: &str,
    value: &str,
    focused: bool,
    cursor: bool,
    cols: u16,
) {
    let lab_fg = if focused { ACCENT } else { TEXT };
    put(out, LABEL_COL, row, label, lab_fg, cols, false);
    let br = if focused { ACCENT } else { DIM };
    put(out, BOX_COL, row, "[", br, cols, false);
    let mut val: String = value.chars().take(INNER_W - 1).collect();
    if cursor && focused {
        val.push('█');
    }
    let val_fg = if focused { ACCENT } else { TEXT };
    put(out, BOX_COL + 2, row, &val, val_fg, cols, false);
    put(out, BOX_COL + 3 + INNER_W as u16, row, "]", br, cols, false);
}

/// A `[ Label ]` button; accent + bold when focused.
fn button(out: &mut Vec<CellView>, col: u16, row: u16, label: &str, focused: bool, cols: u16) {
    let fg = if focused { ACCENT } else { DIM };
    put(out, col, row, &format!("[ {label} ]"), fg, cols, focused);
}

/// The type-to-search font list, drawn over the rows below the family field.
fn dropdown(out: &mut Vec<CellView>, p: &SettingsPane, cols: u16, rows: u16) {
    let list = p.filtered();
    let start = 4u16;
    let avail = rows.saturating_sub(start + 1) as usize;
    let width = (BOX_COL as usize + INNER_W + 4).min(cols as usize) - LABEL_COL as usize;
    for (i, name) in list.iter().take(avail.min(8)).enumerate() {
        let row = start + i as u16;
        let selected = i == p.family_sel;
        let fg = if selected { ACCENT } else { TEXT };
        let marker = if selected { "> " } else { "  " };
        let mut line = format!("{marker}{name}");
        while line.chars().count() < width {
            line.push(' ');
        }
        put(out, LABEL_COL, row, &line, fg, cols, selected);
    }
}

fn put(
    out: &mut Vec<CellView>,
    col: u16,
    row: u16,
    s: &str,
    fg: (u8, u8, u8),
    cols: u16,
    bold: bool,
) {
    for (i, c) in s.chars().enumerate() {
        let cc = col + i as u16;
        if cc >= cols {
            break;
        }
        out.push(CellView {
            col: cc,
            row,
            c,
            fg,
            bg: BG,
            bold,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;

    #[test]
    fn renders_settings_heading() {
        let p = SettingsPane::new(CrewConfig::default(), Vec::new());
        let cells = p.cells(60, 12);
        assert!(cells.iter().any(|c| c.row == 1 && c.c == 'S'));
    }

    #[test]
    fn tiny_pane_renders_nothing() {
        let p = SettingsPane::new(CrewConfig::default(), Vec::new());
        assert!(p.cells(10, 4).is_empty());
    }
}
