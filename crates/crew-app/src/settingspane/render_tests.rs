use super::*;
use crate::config::CrewConfig;

fn pane() -> SettingsPane {
    SettingsPane::new(CrewConfig::default(), Vec::new())
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    // Gap-preserving: pad to each cell's column (blank cells are not emitted).
    let mut s = String::new();
    for (col, c) in v {
        while s.chars().count() < col as usize {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

#[test]
fn every_field_renders_on_a_tall_pane() {
    let cells = pane().cells(80, 24);
    let all: String = (0..24).map(|r| row_text(&cells, r) + "\n").collect();
    for f in FIELDS.iter().take(FIELDS.len() - 2) {
        assert!(
            all.contains(label_of(*f)),
            "missing field '{}' in:\n{all}",
            label_of(*f)
        );
    }
    assert!(all.contains("[ Save ]") && all.contains("[ Cancel ]"));
}

#[test]
fn focused_row_carries_marker_and_cursor() {
    let cells = pane().cells(80, 24);
    let first = row_text(&cells, 1);
    assert!(
        first.trim_start().starts_with("\u{203a} Font family"),
        "got: {first}"
    );
    assert!(first.contains('\u{2588}'), "cursor missing: {first}");
}

#[test]
fn short_pane_scrolls_to_keep_focus_visible() {
    let mut p = pane();
    p.focus = 14; // NotifyPatterns, the last field row
    let cells = p.cells(80, 10);
    let all: String = (0..10).map(|r| row_text(&cells, r) + "\n").collect();
    assert!(
        all.contains("Notify patterns"),
        "focused row visible: {all}"
    );
    assert!(all.contains('\u{2191}'), "up hint expected: {all}");
    assert!(
        !all.contains("Font family"),
        "first row scrolled out: {all}"
    );
}

#[test]
fn scroll_offset_windows_the_focus() {
    assert_eq!(scroll_offset(0, 15, 7), 0);
    assert_eq!(scroll_offset(6, 15, 7), 0);
    assert_eq!(scroll_offset(7, 15, 7), 1);
    assert_eq!(scroll_offset(14, 15, 7), 8);
    assert_eq!(scroll_offset(16, 15, 7), 8); // Save/Cancel keep the tail
    assert_eq!(scroll_offset(3, 15, 20), 0); // everything fits
}

#[test]
fn tiny_pane_renders_nothing() {
    assert!(pane().cells(10, 4).is_empty());
}

#[test]
fn theme_value_names_the_current_theme() {
    let (v, cursor) = value_of(&pane(), Field::Theme);
    assert!(v.contains("paper-dark"), "got: {v}");
    assert!(!cursor, "theme is a picker, not a text field");
}
