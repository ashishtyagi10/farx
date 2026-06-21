#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Pack `n` tiles near-square into `width`x`height`, each inset by `gap`.
pub fn pane_rects(n: usize, width: f32, height: f32, gap: f32) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let tile_w = width / cols as f32;
    let tile_h = height / rows as f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let c = i % cols;
        let r = i / cols;
        out.push(Rect {
            x: c as f32 * tile_w + gap,
            y: r as f32 * tile_h + gap,
            w: tile_w - 2.0 * gap,
            h: tile_h - 2.0 * gap,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 0.5, "{a} != {b}");
    }

    #[test]
    fn one_pane_fills_minus_gap() {
        let r = pane_rects(1, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 1);
        approx(r[0].x, 0.0);
        approx(r[0].y, 0.0);
        approx(r[0].w, 800.0);
        approx(r[0].h, 600.0);
    }

    #[test]
    fn two_panes_side_by_side() {
        let r = pane_rects(2, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 2);
        approx(r[0].w, 400.0);
        approx(r[1].x, 400.0);
        approx(r[0].h, 600.0);
    }

    #[test]
    fn four_panes_two_by_two() {
        let r = pane_rects(4, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 4);
        approx(r[0].w, 400.0);
        approx(r[0].h, 300.0);
        approx(r[3].x, 400.0);
        approx(r[3].y, 300.0);
    }

    #[test]
    fn zero_panes_empty() {
        assert!(pane_rects(0, 800.0, 600.0, 4.0).is_empty());
    }
}
