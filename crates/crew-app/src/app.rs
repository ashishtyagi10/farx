use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crew_render::Renderer;
use crew_term::GridSize;

use crate::layout::pane_rects;
use crate::pane::{build_scenes, relayout, spawn_pane, Pane};
use crate::session::{grid_for, key_to_bytes};

/// Fallback grid size when the GPU cell size is not yet known (zero).
const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
const POLL_MS: u64 = 16;
const GAP: f32 = 4.0;

#[derive(Default)]
pub struct CrewApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    panes: Vec<Pane>,
    focused: usize,
}

impl CrewApp {
    /// Derive a fresh grid from the current surface + cell size, or use FALLBACK.
    fn current_grid(renderer: &Renderer) -> GridSize {
        let (cell_w, cell_h) = renderer.cell_size();
        if cell_w > 0.0 && cell_h > 0.0 {
            let (sw, sh) = renderer.surface_size();
            grid_for(sw, sh, cell_w, cell_h)
        } else {
            FALLBACK_SIZE
        }
    }

    /// Spawn a new pane and make it the focused pane.  Request a redraw.
    /// Pre-wired for Task 4 key bindings.
    #[allow(dead_code)]
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        if let Ok(pane) = spawn_pane("bash", "sh", grid) {
            self.panes.push(pane);
            self.focused = self.panes.len() - 1;
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Close pane at `idx`.  If no panes remain, signal exit via a pending flag.
    /// Returns `true` if the app should exit.  Pre-wired for Task 4 key bindings.
    #[allow(dead_code)]
    pub fn close_pane(&mut self, idx: usize) -> bool {
        if idx < self.panes.len() {
            self.panes.remove(idx);
        }
        if self.panes.is_empty() {
            return true;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        false
    }
}

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        match Renderer::new(window.clone()) {
            Ok(renderer) => {
                let initial_grid = Self::current_grid(&renderer);
                self.renderer = Some(renderer);
                self.window = Some(window.clone());

                if let Ok(pane) = spawn_pane("bash", "sh", initial_grid) {
                    self.panes.push(pane);
                    self.focused = 0;
                }
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("GPU init failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else { return };

        let total: usize = self.panes.iter_mut().map(|p| p.pty.try_read()).sum();
        if total > 0 {
            window.request_redraw();
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(POLL_MS),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(bytes) = key_to_bytes(&event) {
                    if let Some(pane) = self.panes.get_mut(self.focused) {
                        if let Err(e) = pane
                            .input
                            .write_all(&bytes)
                            .and_then(|_| pane.input.flush())
                        {
                            eprintln!("pty write error: {e}");
                        }
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                if self.panes.is_empty() {
                    return;
                }
                let (cell_w, cell_h) = renderer.cell_size();
                let (sw, sh) = renderer.surface_size();
                let rects = pane_rects(self.panes.len(), sw as f32, sh as f32, GAP);
                if cell_w > 0.0 && cell_h > 0.0 {
                    relayout(&mut self.panes, &rects, cell_w, cell_h);
                }
                let scenes = build_scenes(&self.panes, self.focused);
                renderer.frame(&scenes);
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CrewApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
