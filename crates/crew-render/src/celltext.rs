//! Rich-text buffer builder for CellGrid.
use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

use crate::cellgrid::CellView;

/// Font metrics shared across all pane buffers.
pub(crate) struct FontParams {
    pub font_size: f32,
    pub line_height: f32,
    /// Chosen family name; `None`/empty falls back to the system monospace.
    pub family: Option<String>,
}

/// The cosmic-text `Family` for an optional family name (empty/`None` → system monospace).
pub(crate) fn family_from(opt: &Option<String>) -> Family<'_> {
    match opt {
        Some(name) if !name.is_empty() => Family::Name(name),
        _ => Family::Monospace,
    }
}

/// Sorted, de-duplicated names of all installed monospace font families.
pub(crate) fn monospace_families(font_system: &FontSystem) -> Vec<String> {
    let mut names: Vec<String> = font_system
        .db()
        .faces()
        .filter(|f| f.monospaced)
        .flat_map(|f| f.families.iter().map(|(name, _)| name.clone()))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Shape "M" and return its advance as the cell width for `family`.
pub(crate) fn probe_cell_width(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    font_size: f32,
    family: Family,
) -> f32 {
    let attrs = Attrs::new().family(family);
    buffer.set_text(font_system, "M", &attrs, Shaping::Advanced, None);
    for run in buffer.layout_runs() {
        if let Some(g) = run.glyphs.first() {
            return g.w;
        }
    }
    font_size * 0.6
}

/// Build a new `Buffer` for one pane's cells at the given cols/rows.
/// The buffer is sized to `(w, h)` pixels and laid out as a cols×rows grid.
pub(crate) fn build_pane_buffer(
    font_system: &mut FontSystem,
    cells: &[CellView],
    cols: usize,
    rows: usize,
    w: f32,
    h: f32,
    params: &FontParams,
) -> Buffer {
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(params.font_size, params.line_height),
    );
    buffer.set_wrap(font_system, Wrap::None);
    buffer.set_size(font_system, Some(w), Some(h));

    fill_rich_text(&mut buffer, font_system, cells, cols, rows, &params.family);
    buffer
}

/// Fill an existing `Buffer` with rich-text spans for `cells` laid out in cols×rows.
pub(crate) fn fill_rich_text<'a>(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    cells: &[CellView],
    cols: usize,
    rows: usize,
    family: &'a Option<String>,
) {
    let fam = family_from(family);
    // Bucket cells into a 2-D grid (row × col).
    let mut grid: Vec<Vec<Option<&CellView>>> = vec![vec![None; cols]; rows];
    for cell in cells {
        let r = cell.row as usize;
        let c = cell.col as usize;
        if r < rows && c < cols {
            grid[r][c] = Some(cell);
        }
    }

    let default_attrs = Attrs::new().family(fam);

    // Collect span strings + attrs; keep strings alive so we can borrow them.
    let mut span_strings: Vec<String> = Vec::new();
    let mut span_attrs: Vec<Attrs<'a>> = Vec::new();

    for (row_i, row) in grid.iter().enumerate() {
        for cell_opt in row.iter() {
            let (ch, attrs) = match cell_opt {
                Some(cell) => {
                    let mut a = Attrs::new()
                        .family(fam)
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

/// Compute monospace cell dimensions for the given font size without a GPU.
/// Returns `(cell_w, cell_h)` where `cell_h = font_size * 1.25`.
pub(crate) fn cell_metrics(
    fs: &mut FontSystem,
    font_size: f32,
    family: &Option<String>,
) -> (f32, f32) {
    let cell_h = font_size * 1.25;
    let mut probe_buf = Buffer::new(fs, Metrics::new(font_size, cell_h));
    probe_buf.set_wrap(fs, Wrap::None);
    probe_buf.set_size(fs, Some(4096.0), Some(4096.0));
    let cell_w = probe_cell_width(&mut probe_buf, fs, font_size, family_from(family));
    (cell_w, cell_h)
}

#[cfg(test)]
#[path = "celltext_tests.rs"]
mod tests;
