use super::*;

fn cell(col: u16, row: u16, c: char) -> CellView {
    CellView {
        col,
        row,
        c,
        fg: (0, 0, 0),
        bg: (1, 1, 1),
        bold: false,
        italic: false,
    }
}

/// "hello" on row 0, "world" on row 1.
fn grid() -> Vec<CellView> {
    let mut v = Vec::new();
    for (i, c) in "hello".chars().enumerate() {
        v.push(cell(i as u16, 0, c));
    }
    for (i, c) in "world".chars().enumerate() {
        v.push(cell(i as u16, 1, c));
    }
    v
}

#[test]
fn single_row_partial_selection() {
    let sel = CellSel {
        pane: 0,
        anchor: (1, 0),
        cursor: (3, 0),
    };
    assert_eq!(selection_text(&grid(), &sel), "ell");
}

#[test]
fn selection_is_direction_agnostic() {
    let fwd = CellSel {
        pane: 0,
        anchor: (1, 0),
        cursor: (3, 0),
    };
    let rev = CellSel {
        pane: 0,
        anchor: (3, 0),
        cursor: (1, 0),
    };
    assert_eq!(selection_text(&grid(), &fwd), selection_text(&grid(), &rev));
}

#[test]
fn multi_row_selection_joins_with_newline() {
    // From row 0 col 2 through row 1 col 2: "llo" + "wor".
    let sel = CellSel {
        pane: 0,
        anchor: (2, 0),
        cursor: (2, 1),
    };
    assert_eq!(selection_text(&grid(), &sel), "llo\nwor");
}

#[test]
fn gaps_become_spaces_and_trailing_trimmed() {
    // "a" at col 0, "b" at col 3 on row 0; select the whole row.
    let cells = vec![cell(0, 0, 'a'), cell(3, 0, 'b')];
    let sel = CellSel {
        pane: 0,
        anchor: (0, 0),
        cursor: (9, 0),
    };
    assert_eq!(selection_text(&cells, &sel), "a  b");
}

#[test]
fn highlight_only_touches_selected_cells() {
    let mut cells = grid();
    let sel = CellSel {
        pane: 0,
        anchor: (0, 0),
        cursor: (1, 0),
    };
    highlight(&mut cells, &sel, (9, 9, 9));
    // Row 0 cols 0,1 washed; everything else keeps its original bg.
    for c in &cells {
        let washed = c.row == 0 && c.col <= 1;
        assert_eq!(c.bg == (9, 9, 9), washed, "cell {},{}", c.col, c.row);
    }
}

#[test]
fn empty_when_selection_misses_all_glyphs() {
    let sel = CellSel {
        pane: 0,
        anchor: (20, 5),
        cursor: (25, 5),
    };
    assert_eq!(selection_text(&grid(), &sel), "");
}
