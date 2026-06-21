//! Rich-text buffer builder for CellGrid.
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Shaping, Style, Weight};

use crate::cellgrid::{CellView, GridMetrics};

/// Shape "M" and return its advance as the monospace cell width.
pub(crate) fn probe_cell_width(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    font_size: f32,
) -> f32 {
    let attrs = Attrs::new().family(Family::Monospace);
    buffer.set_text(font_system, "M", &attrs, Shaping::Advanced, None);
    for run in buffer.layout_runs() {
        if let Some(g) = run.glyphs.first() {
            return g.w;
        }
    }
    font_size * 0.6
}

/// Build a rich-text Buffer from cell data: one logical row per terminal row,
/// with per-character Attrs for colour, weight, and style.
pub(crate) fn build_rich_text(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cells: &[CellView],
    metrics: &GridMetrics,
) {
    let rows = metrics.rows as usize;
    let cols = metrics.cols as usize;

    // Bucket cells into a 2-D grid (row × col).
    let mut grid: Vec<Vec<Option<&CellView>>> = vec![vec![None; cols]; rows];
    for cell in cells {
        let r = cell.row as usize;
        let c = cell.col as usize;
        if r < rows && c < cols {
            grid[r][c] = Some(cell);
        }
    }

    let default_attrs = Attrs::new().family(Family::Monospace);

    // Collect span strings + attrs; keep strings alive so we can borrow them.
    let mut span_strings: Vec<String> = Vec::new();
    let mut span_attrs: Vec<Attrs<'static>> = Vec::new();

    for (row_i, row) in grid.iter().enumerate() {
        for cell_opt in row.iter() {
            let (ch, attrs) = match cell_opt {
                Some(cell) => {
                    let mut a = Attrs::new()
                        .family(Family::Monospace)
                        .color(Color::rgb(cell.fg.0, cell.fg.1, cell.fg.2));
                    if cell.bold {
                        a = a.weight(Weight::BOLD);
                    }
                    if cell.italic {
                        a = a.style(Style::Italic);
                    }
                    (cell.c.to_string(), a)
                }
                None => (" ".to_string(), default_attrs.clone()),
            };
            span_strings.push(ch);
            span_attrs.push(attrs);
        }
        if row_i + 1 < rows {
            span_strings.push("\n".to_string());
            span_attrs.push(default_attrs.clone());
        }
    }

    let spans: Vec<(&str, Attrs<'_>)> = span_strings
        .iter()
        .zip(span_attrs.iter())
        .map(|(s, a)| (s.as_str(), a.clone()))
        .collect();

    buffer.set_rich_text(font_system, spans, &default_attrs, Shaping::Advanced, None);
}
