//! Docked-sidebar layout geometry. The sidebar is a fixed-width, full-height
//! column on the left; the content area (grid panes) fills the rest. A future
//! AppBar (unified title bar) will own the visible toggle + options.
use crate::layout::Rect;

/// Padding inset for the input bar card.
pub const INPUT_PAD: f32 = 6.0;

/// Height in physical px reserved for the bottom input bar (≈3 cell rows + padding).
pub fn input_h(cell_h: f32) -> f32 {
    cell_h * 3.0 + INPUT_PAD
}

/// Bottom strip for the docked input bar, spanning the **action area** only
/// (`content`'s x/width, gap-inset to match pane width) — never under the sidebar.
pub fn inputbar_rect(content: Rect, sh: f32, ih: f32, gap: f32) -> Rect {
    Rect {
        x: content.x + gap,
        y: sh - ih + gap,
        w: content.w - 2.0 * gap,
        h: ih - 2.0 * gap,
    }
}

/// Fixed-width sidebar column on the left spanning the **entire** height (inset by
/// `gap` on all sides) — it runs alongside both the panes and the input bar.
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
/// (the grid's own internal gap supplies the remaining inset). `ih` is the
/// input-bar height subtracted from the bottom.
pub fn content_rect(sw: f32, sh: f32, show_nav: bool, nav_px: f32, gap: f32, ih: f32) -> Rect {
    let x = if show_nav { nav_px + gap } else { 0.0 };
    Rect {
        x,
        y: 0.0,
        w: sw - x,
        h: sh - ih,
    }
}

pub fn point_in(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rect_no_nav_with_ih() {
        // h = sh - ih = 800 - 60 = 740
        assert_eq!(
            content_rect(1000.0, 800.0, false, 200.0, 8.0, 60.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 740.0
            }
        );
    }

    #[test]
    fn content_rect_with_nav_with_ih() {
        // x = nav_px + gap = 208; w = 1000 - 208 = 792; h = 800 - 60 = 740
        assert_eq!(
            content_rect(1000.0, 800.0, true, 200.0, 8.0, 60.0),
            Rect {
                x: 208.0,
                y: 0.0,
                w: 792.0,
                h: 740.0
            }
        );
    }

    #[test]
    fn sidebar_rect_full_height() {
        // full height: h = sh - 2*gap = 800 - 16 = 784 (input bar does NOT shrink it)
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
    fn inputbar_rect_spans_action_area() {
        // content (with nav) = {x:208, w:792}; input: x=208+8=216,
        // y=800-60+8=748, w=792-16=776, h=60-16=44
        let content = content_rect(1000.0, 800.0, true, 200.0, 8.0, 60.0);
        assert_eq!(
            inputbar_rect(content, 800.0, 60.0, 8.0),
            Rect {
                x: 216.0,
                y: 748.0,
                w: 776.0,
                h: 44.0
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
