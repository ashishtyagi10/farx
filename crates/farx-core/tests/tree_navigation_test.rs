//! Unit tests for `TreeState` navigation (expand/collapse/cursor/select).
//! Kept as an integration test so `tree/navigation.rs` stays within the
//! file-size cap.

use farx_core::TreeState;

fn build_tree() -> (tempfile::TempDir, TreeState) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("subdir")).unwrap();
    std::fs::write(tmp.path().join("subdir/inner.txt"), b"x").unwrap();
    std::fs::write(tmp.path().join("top.txt"), b"y").unwrap();
    let mut t = TreeState::new(tmp.path().to_path_buf());
    t.rebuild();
    (tmp, t)
}

fn index_of(t: &TreeState, name: &str) -> usize {
    t.visible_nodes
        .iter()
        .position(|n| n.entry.name == name)
        .expect("node not found")
}

#[test]
fn toggle_expand_shows_and_hides_children() {
    let (_tmp, mut t) = build_tree();
    let sub = index_of(&t, "subdir");
    t.cursor = sub;
    assert_eq!(t.current_node().unwrap().entry.name, "subdir");

    t.toggle_expand();
    assert!(t.visible_nodes.iter().any(|n| n.entry.name == "inner.txt"));

    t.cursor = index_of(&t, "subdir");
    t.toggle_expand();
    assert!(!t.visible_nodes.iter().any(|n| n.entry.name == "inner.txt"));
}

#[test]
fn expand_moves_into_first_child_and_collapse_returns_to_parent() {
    let (_tmp, mut t) = build_tree();
    let sub = index_of(&t, "subdir");
    t.cursor = sub;

    // expand() on a collapsed dir expands it and steps onto the first child.
    t.expand();
    assert_eq!(t.cursor, sub + 1);
    assert_eq!(t.current_node().unwrap().entry.name, "inner.txt");

    // collapse() from a child jumps back to the parent directory node.
    t.collapse();
    assert_eq!(t.cursor, sub);

    // collapse() on the expanded dir collapses it.
    t.collapse();
    assert!(!t.visible_nodes.iter().any(|n| n.entry.name == "inner.txt"));
}

#[test]
fn cursor_movement_clamps() {
    let (_tmp, mut t) = build_tree();
    let last = t.visible_nodes.len() - 1;
    t.move_cursor(-100);
    assert_eq!(t.cursor, 0);
    t.move_cursor(100);
    assert_eq!(t.cursor, last);
    t.move_cursor_to(999);
    assert_eq!(t.cursor, last);
}

#[test]
fn scroll_and_select() {
    let (_tmp, mut t) = build_tree();
    let last = t.visible_nodes.len() - 1;
    t.cursor = last;
    t.scroll_to_cursor(1);
    assert_eq!(t.scroll_offset, last);
    t.cursor = 0;
    t.scroll_to_cursor(5);
    assert_eq!(t.scroll_offset, 0);

    let top = index_of(&t, "top.txt");
    t.cursor = top;
    t.toggle_select();
    assert!(t.selected.contains(&top));
    t.cursor = top;
    t.toggle_select();
    assert!(!t.selected.contains(&top));
}

#[test]
fn toggle_select_skips_dotdot() {
    let (_tmp, mut t) = build_tree();
    // The root tree includes a ".." entry at index 0; selecting it is skipped.
    if t.visible_nodes.first().map(|n| n.entry.name.as_str()) == Some("..") {
        t.cursor = 0;
        t.toggle_select();
        assert!(t.selected.is_empty());
    }
}
