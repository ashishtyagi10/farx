use crew_render::PaneScene;

use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::layout::pane_rects_at;
use crate::pane::{build_scenes, relayout};

impl CrewApp {
    /// Build all PaneScenes for one frame: grid panes (reserved to the content
    /// rect), the top bar, and the docked sidebar (when shown).
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some(r) = &self.renderer else {
            return Vec::new();
        };
        let (cw, ch) = r.cell_size();
        let (sw_u, sh_u) = r.surface_size();
        let (sw, sh) = (sw_u as f32, sh_u as f32);
        if cw <= 0.0 || ch <= 0.0 {
            return Vec::new();
        }

        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        let bar_h = chrome::top_bar_h(ch);
        let nav_px = if self.config.show_nav {
            self.config.nav_width * scale
        } else {
            0.0
        };
        let content = chrome::content_rect(sw, sh, bar_h, self.config.show_nav, nav_px);

        let rects = pane_rects_at(
            self.panes.len(),
            content.x,
            content.y,
            content.w,
            content.h,
            GAP,
        );
        relayout(&mut self.panes, &rects, cw, ch);
        let mut scenes = build_scenes(&self.panes, self.focused);

        // Top bar
        let tb = chrome::topbar_rect(sw, bar_h);
        let top_cols = (tb.w / cw).floor() as u16;
        scenes.push(PaneScene {
            cells: chrome::topbar_cells(self.config.show_nav, top_cols),
            x: tb.x,
            y: tb.y,
            w: tb.w,
            h: tb.h,
            focused: false,
        });

        // Docked sidebar
        if self.config.show_nav {
            let sb = chrome::sidebar_rect(sw, sh, bar_h, nav_px);
            let sc = (sb.w / cw).floor() as u16;
            let sr = (sb.h / ch).floor() as u16;
            scenes.push(PaneScene {
                cells: self.sidebar.cells(sc, sr),
                x: sb.x,
                y: sb.y,
                w: sb.w,
                h: sb.h,
                focused: false,
            });
        }
        scenes
    }
}
