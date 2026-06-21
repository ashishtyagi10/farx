//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Fixed-width sidebar column on the left, inset by `gap` top/bottom/left so it
/// aligns vertically with the gap-inset grid panes.
pub fn sidebar_rect(sh: f32, nav_px: f32, gap: f32) -> Rect {
    Rect {
        x: gap,
        y: gap,
        w: nav_px,
        h: sh - 2.0 * gap,
    }
}

/// The content area for grid panes: everything to the right of the sidebar. When
/// the sidebar is shown, leave one `gap` of space between it and the first pane
/// (the grid's own internal gap supplies the remaining inset).
pub fn content_rect(sw: f32, sh: f32, show_nav: bool, nav_px: f32, gap: f32) -> Rect {
    let x = if show_nav { nav_px + gap } else { 0.0 };
    Rect {
        x,
        y: 0.0,
        w: sw - x,
        h: sh,
    }
}

pub fn point_in(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rect_no_nav() {
        assert_eq!(
            content_rect(1000.0, 800.0, false, 200.0, 8.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 800.0
            }
        );
    }

    #[test]
    fn content_rect_with_nav() {
        // x = nav_px + gap; sidebar right edge sits at gap+nav_px, leaving one gap.
        assert_eq!(
            content_rect(1000.0, 800.0, true, 200.0, 8.0),
            Rect {
                x: 208.0,
                y: 0.0,
                w: 792.0,
                h: 800.0
            }
        );
    }

    #[test]
    fn sidebar_rect_inset_by_gap() {
        assert_eq!(
            sidebar_rect(800.0, 200.0, 8.0),
            Rect {
                x: 8.0,
                y: 8.0,
                w: 200.0,
                h: 784.0
            }
        );
    }

    #[test]
    fn point_in_bounds() {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            w: 30.0,
            h: 30.0,
        };
        assert!(point_in(r, 5.0, 5.0));
        assert!(!point_in(r, 100.0, 5.0));
    }
}
