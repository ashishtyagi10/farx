use crew_term::{GridSize, RenderCell};
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

/// Map a winit key press event to the bytes that should be sent to the PTY.
pub fn key_to_bytes(event: &KeyEvent) -> Option<Vec<u8>> {
    if !event.state.is_pressed() {
        return None;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Backspace) => Some(vec![0x7f]),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Escape) => Some(vec![0x1b]),
        Key::Named(NamedKey::Space) => Some(b" ".to_vec()),
        Key::Character(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}

/// Flatten visible cells into rows of text for the single-string renderer.
pub fn cells_to_string(cells: &[RenderCell], size: GridSize) -> String {
    let mut grid = vec![vec![' '; size.cols as usize]; size.rows as usize];
    for c in cells {
        if (c.row as usize) < grid.len() && (c.col as usize) < grid[0].len() {
            grid[c.row as usize][c.col as usize] = c.c;
        }
    }
    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
