use std::sync::Arc;

use winit::event::Modifiers;
use winit::window::Window;

use crate::chat::ChatPane;
use crate::layout::Rect;
use crate::pane::{spawn_pane, Pane, PaneContent};
use crate::session::grid_for;
use crew_plugin::{Plugin, PluginCommand};
use crew_render::Renderer;
use crew_term::GridSize;

/// Fallback grid size when the GPU cell size is not yet known (zero).
pub(crate) const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
pub(crate) const POLL_MS: u64 = 16;
pub(crate) const GAP: f32 = 4.0;

#[derive(Default)]
pub struct CrewApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) focused: usize,
    pub(crate) mods: Modifiers,
    pub(crate) cursor: (f32, f32),
}

impl CrewApp {
    pub(crate) fn current_grid(renderer: &Renderer) -> GridSize {
        let (cell_w, cell_h) = renderer.cell_size();
        if cell_w > 0.0 && cell_h > 0.0 {
            let (sw, sh) = renderer.surface_size();
            grid_for(sw, sh, cell_w, cell_h)
        } else {
            FALLBACK_SIZE
        }
    }

    /// Spawn a new terminal pane and focus it.
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match spawn_pane("bash", "sh", grid) {
            Ok(pane) => {
                self.panes.push(pane);
                self.focused = self.panes.len() - 1;
            }
            Err(e) => eprintln!("spawn_new_pane failed: {e:#}"),
        }
    }

    /// Spawn a new chat pane (backed by a plugin) and focus it.
    pub fn spawn_chat_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);

        let cmd = std::env::var("CREW_CHAT_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-echo-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-echo-plugin".to_string())
        });

        match Plugin::spawn(&cmd, &[]) {
            Ok(mut plugin) => {
                if let Err(e) = plugin.send(&PluginCommand::Hello { v: 1 }) {
                    eprintln!("spawn_chat_pane: plugin hello error: {e}");
                }
                let chat = ChatPane::new(plugin, String::new());
                self.panes.push(Pane {
                    content: PaneContent::Chat(chat),
                    grid,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.0,
                        h: 0.0,
                    },
                });
                self.focused = self.panes.len() - 1;
            }
            Err(e) => eprintln!("spawn_chat_pane failed: {e:#}"),
        }
    }

    /// Close pane at `idx`.  Returns `true` if the app should exit.
    pub fn close_pane(&mut self, idx: usize) -> bool {
        if idx < self.panes.len() {
            self.panes.remove(idx);
        }
        if self.panes.is_empty() {
            return true;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        false
    }

    /// Handle a Super-chord key.  Returns `true` if the app should exit.
    pub(crate) fn handle_super_chord(&mut self, s: &str) -> bool {
        let n = self.panes.len().max(1);
        match s {
            "t" => self.spawn_new_pane(),
            "j" => self.spawn_chat_pane(),
            "w" => return self.close_pane(self.focused),
            "[" => self.focused = (self.focused + n - 1) % n,
            "]" => self.focused = (self.focused + 1) % n,
            s if s.len() == 1 => {
                if let Some(d) = s.chars().next().and_then(|c| c.to_digit(10)) {
                    if d >= 1 {
                        let i = (d - 1) as usize;
                        if i < self.panes.len() {
                            self.focused = i;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
