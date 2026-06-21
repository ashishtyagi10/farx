use crew_term::RenderCell;
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

/// Map `crew_term::RenderCell` slices to `crew_render::CellView` — field-for-field.
pub fn to_cellviews(cells: &[RenderCell]) -> Vec<crew_render::CellView> {
    cells
        .iter()
        .map(|c| crew_render::CellView {
            col: c.col,
            row: c.row,
            c: c.c,
            fg: c.fg,
            bg: c.bg,
            bold: c.bold,
            italic: c.italic,
        })
        .collect()
}
