use std::io::Write;

use anyhow::Context;
use crew_render::PaneScene;
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::layout::Rect;
use crate::session::to_cellviews;

/// A single terminal pane: owns its PTY, writer, grid size, and pixel rect.
pub struct Pane {
    pub pty: PtyTerm,
    pub input: Box<dyn Write + Send>,
    pub grid: GridSize,
    pub rect: Rect,
}

/// Spawn a pane, trying `shell_primary` first and falling back to `shell_fallback`.
pub fn spawn_pane(
    shell_primary: &str,
    shell_fallback: &str,
    grid: GridSize,
) -> anyhow::Result<Pane> {
    let pty = PtyTerm::spawn(grid, shell_primary)
        .or_else(|_| PtyTerm::spawn(grid, shell_fallback))
        .with_context(|| {
            format!("failed to spawn shell (tried {shell_primary}, {shell_fallback})")
        })?;
    let input = pty.writer();
    Ok(Pane {
        pty,
        input,
        grid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
    })
}

/// Assign pixel rects to panes and resize each PTY when its grid changes.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        pane.rect = rect;
        let cols = ((rect.w / cell_w).floor() as u16).max(1);
        let rows = ((rect.h / cell_h).floor() as u16).max(1);
        if cols != pane.grid.cols || rows != pane.grid.rows {
            let new_grid = GridSize { cols, rows };
            pane.pty.resize(new_grid);
            pane.grid = new_grid;
        }
    }
}

/// Build a `Vec<PaneScene>` from the current pane state (for `renderer.frame`).
pub fn build_scenes(panes: &[Pane], focused: usize) -> Vec<PaneScene> {
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| PaneScene {
            cells: to_cellviews(&p.pty.cells()),
            x: p.rect.x,
            y: p.rect.y,
            w: p.rect.w,
            h: p.rect.h,
            focused: i == focused,
        })
        .collect()
}
