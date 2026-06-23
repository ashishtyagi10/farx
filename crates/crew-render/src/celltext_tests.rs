use super::*;

fn params(family: Option<String>) -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        family,
    }
}

#[test]
fn cell_metrics_larger_font_gives_larger_dimensions() {
    let mut fs = FontSystem::new();
    let small = cell_metrics(&mut fs, 12.0, &None);
    let large = cell_metrics(&mut fs, 24.0, &None);
    assert!(large.0 > small.0, "cell_w should grow with font size");
    assert!(large.1 > small.1, "cell_h should grow with font size");
    assert_eq!(large.1, 24.0 * 1.25, "cell_h is 1.25× font size");
}

#[test]
fn cell_metrics_height_is_125_percent() {
    let mut fs = FontSystem::new();
    assert_eq!(cell_metrics(&mut fs, 16.0, &None).1, 20.0);
}

#[test]
fn family_from_maps_named_and_default() {
    match family_from(&Some("Menlo".to_string())) {
        Family::Name(n) => assert_eq!(n, "Menlo"),
        _ => panic!("named family should map to Family::Name"),
    }
    assert!(matches!(family_from(&None), Family::Monospace));
    assert!(matches!(
        family_from(&Some(String::new())),
        Family::Monospace
    ));
}

#[test]
fn monospace_families_sorted_and_deduped() {
    let fs = FontSystem::new();
    let names = monospace_families(&fs);
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "names must be sorted");
    let mut deduped = names.clone();
    deduped.dedup();
    assert_eq!(deduped.len(), names.len(), "names must be de-duplicated");
}

#[test]
fn build_pane_buffer_lays_out_grid_with_styles() {
    let mut fs = FontSystem::new();
    let cells = vec![
        CellView {
            col: 0,
            row: 0,
            c: 'h',
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            bold: true,
            italic: false,
        },
        CellView {
            col: 1,
            row: 0,
            c: 'i',
            fg: (10, 20, 30),
            bg: (0, 0, 0),
            bold: false,
            italic: true,
        },
        // row 1 left empty at col 0 → exercises the None-gap branch
        CellView {
            col: 1,
            row: 1,
            c: 'x',
            fg: (1, 2, 3),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
        },
    ];
    let buf = build_pane_buffer(&mut fs, &cells, 3, 2, 24.0, 36.0, &params(None));
    assert!(
        buf.layout_runs().count() >= 1,
        "buffer should lay out lines"
    );
}

#[test]
fn build_pane_buffer_handles_empty_cells() {
    let mut fs = FontSystem::new();
    // Empty family string also exercises the system-monospace fallback.
    let buf = build_pane_buffer(&mut fs, &[], 2, 2, 16.0, 32.0, &params(Some(String::new())));
    assert!(buf.layout_runs().count() <= 2);
}

#[test]
fn build_pane_buffer_ignores_out_of_range_cells() {
    let mut fs = FontSystem::new();
    // A cell beyond cols/rows must be dropped without panicking.
    let cells = vec![CellView {
        col: 9,
        row: 9,
        c: 'z',
        fg: (1, 1, 1),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
    }];
    let _ = build_pane_buffer(&mut fs, &cells, 2, 2, 16.0, 32.0, &params(None));
}
