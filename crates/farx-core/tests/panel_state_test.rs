//! Unit tests for `PanelState` cursor/selection/sort logic. Kept as an
//! integration test so `panel_state.rs` stays within the file-size cap.

use std::path::PathBuf;

use farx_core::{FileEntry, PanelSide, PanelState, SortField, SortOrder};

fn entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        path: PathBuf::from(name),
        is_dir,
        is_symlink: false,
        is_hidden: name.starts_with('.'),
        size,
        modified: None,
        extension: name.rsplit_once('.').map(|(_, e)| e.to_string()),
        readonly: false,
        mode: None,
    }
}

fn panel_with(names: &[(&str, bool, u64)]) -> PanelState {
    let mut p = PanelState::new(PanelSide::Left, PathBuf::from("/tmp"));
    p.entries = names.iter().map(|(n, d, s)| entry(n, *d, *s)).collect();
    p
}

#[test]
fn current_entry_tracks_cursor() {
    let mut p = panel_with(&[("a", false, 1), ("b", false, 2)]);
    assert_eq!(p.current_entry().unwrap().name, "a");
    p.cursor = 1;
    assert_eq!(p.current_entry().unwrap().name, "b");
    p.cursor = 9;
    assert!(p.current_entry().is_none());
}

#[test]
fn toggle_select_inserts_removes_and_advances() {
    let mut p = panel_with(&[("a", false, 1), ("b", false, 2)]);
    p.toggle_select();
    assert!(p.selected.contains(&0));
    assert_eq!(p.cursor, 1); // advanced
    p.cursor = 0;
    p.toggle_select();
    assert!(!p.selected.contains(&0)); // removed
}

#[test]
fn select_move_skips_dotdot() {
    let mut p = panel_with(&[("..", true, 0), ("a", false, 1)]);
    p.select_move(1);
    assert!(p.selected.is_empty()); // ".." not selected
    assert_eq!(p.cursor, 1);
}

#[test]
fn select_range_to_selects_inclusive_and_moves_cursor() {
    let mut p = panel_with(&[("a", false, 1), ("b", false, 2), ("c", false, 3)]);
    p.select_range_to(2);
    assert_eq!(p.selected.len(), 3);
    assert_eq!(p.cursor, 2);
}

#[test]
fn selected_entries_returns_sorted_refs() {
    let mut p = panel_with(&[("a", false, 1), ("b", false, 2), ("c", false, 3)]);
    p.selected.insert(2);
    p.selected.insert(0);
    let names: Vec<&str> = p
        .selected_entries()
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, vec!["a", "c"]);
}

#[test]
fn sort_entries_dirs_first_then_by_field() {
    let mut p = panel_with(&[("b.txt", false, 10), ("dir", true, 0), ("a.txt", false, 20)]);
    p.sort_field = SortField::Name;
    p.sort_order = SortOrder::Ascending;
    p.sort_entries();
    let order: Vec<&str> = p.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(order, vec!["dir", "a.txt", "b.txt"]);

    p.sort_field = SortField::Size;
    p.sort_order = SortOrder::Descending;
    p.sort_entries();
    assert_eq!(p.entries[0].name, "dir");
    assert_eq!(p.entries[1].name, "a.txt"); // size 20
}

#[test]
fn move_cursor_clamps_and_handles_empty() {
    let mut p = panel_with(&[("a", false, 1), ("b", false, 2)]);
    p.move_cursor(-5);
    assert_eq!(p.cursor, 0);
    p.move_cursor(99);
    assert_eq!(p.cursor, 1);
    p.move_cursor_to(99);
    assert_eq!(p.cursor, 1);
    let mut empty = panel_with(&[]);
    empty.move_cursor(3);
    assert_eq!(empty.cursor, 0);
    empty.move_cursor_to(3);
    assert_eq!(empty.cursor, 0);
}

#[test]
fn scroll_to_cursor_keeps_cursor_visible() {
    let mut p = panel_with(&[]);
    p.entries = (0..20).map(|i| entry(&format!("f{i}"), false, i)).collect();
    p.cursor = 15;
    p.scroll_to_cursor(10);
    assert!(p.cursor >= p.scroll_offset && p.cursor < p.scroll_offset + 10);
    p.cursor = 2;
    p.scroll_to_cursor(10);
    assert_eq!(p.scroll_offset, 2);
    p.scroll_to_cursor(0); // no-op guard
}

#[test]
fn quick_search_jumps_and_clears() {
    let mut p = panel_with(&[("alpha", false, 1), ("beta", false, 2), ("gamma", false, 3)]);
    p.enter_quick_search('b');
    assert_eq!(p.cursor, 1);
    assert_eq!(p.quick_search.as_deref(), Some("b"));
    p.clear_quick_search();
    assert!(p.quick_search.is_none());
}
