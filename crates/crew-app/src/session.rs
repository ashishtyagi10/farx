use crew_term::{GridSize, RenderCell};
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::layout::Rect;

/// Return the index of the first rect that contains physical pixel `(x, y)`.
pub fn pane_at(rects: &[Rect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .position(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
}

/// Compute the terminal grid size that fits in `width x height` pixels given
/// the font cell dimensions.  Each dimension is clamped to a minimum of 1.
pub fn grid_for(width: u32, height: u32, cell_w: f32, cell_h: f32) -> GridSize {
    let cols = ((width as f32 / cell_w).floor() as u16).max(1);
    let rows = ((height as f32 / cell_h).floor() as u16).max(1);
    GridSize { cols, rows }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pane_rects_at;

    #[test]
    fn pane_at_two_panes() {
        // 2 panes side-by-side in 800x600 with no gap → left pane [0,400) right [400,800)
        let rects = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 0.0);
        assert_eq!(pane_at(&rects, 10.0, 10.0), Some(0));
        assert_eq!(pane_at(&rects, 410.0, 10.0), Some(1));
        assert_eq!(pane_at(&rects, 800.0, 10.0), None);
    }

    #[test]
    fn grid_for_basic() {
        let g = grid_for(800, 600, 10.0, 20.0);
        assert_eq!(g.cols, 80);
        assert_eq!(g.rows, 30);
    }

    #[test]
    fn grid_for_clamps_to_one() {
        let g = grid_for(0, 0, 10.0, 20.0);
        assert_eq!(g.cols, 1);
        assert_eq!(g.rows, 1);
    }

    #[test]
    fn grid_for_floors_partial_cells() {
        // 805 / 10 = 80.5 → floor → 80
        let g = grid_for(805, 601, 10.0, 20.0);
        assert_eq!(g.cols, 80);
        assert_eq!(g.rows, 30);
    }
}
