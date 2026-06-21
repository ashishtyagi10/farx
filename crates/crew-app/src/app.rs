use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crew_render::{GridMetrics, Renderer};
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::session::{grid_for, key_to_bytes, to_cellviews};

/// Fallback grid size when the GPU cell size is not yet known (zero).
const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
const POLL_MS: u64 = 16;

pub struct CrewApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    pty: Option<PtyTerm>,
    input: Option<Box<dyn std::io::Write + Send>>,
    /// Current terminal grid dimensions — kept in sync with both the renderer
    /// viewport and the PTY so the shell reflows on every window resize.
    grid: GridSize,
}

impl Default for CrewApp {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            pty: None,
            input: None,
            grid: FALLBACK_SIZE,
        }
    }
}

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        match Renderer::new(window.clone()) {
            Ok(renderer) => {
                // Derive initial grid from actual surface + cell dimensions.
                let (cell_w, cell_h) = renderer.cell_size();
                let initial_grid = if cell_w > 0.0 && cell_h > 0.0 {
                    let (sw, sh) = renderer.surface_size();
                    grid_for(sw, sh, cell_w, cell_h)
                } else {
                    FALLBACK_SIZE
                };
                self.grid = initial_grid;

                let pty = PtyTerm::spawn(initial_grid, "bash")
                    .or_else(|_| PtyTerm::spawn(initial_grid, "sh"))
                    .ok();
                let input = pty.as_ref().map(|p| p.writer());
                self.renderer = Some(renderer);
                self.pty = pty;
                self.input = input;
                self.window = Some(window.clone());
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("GPU init failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(pty), Some(window)) = (&mut self.pty, &self.window) else {
            return;
        };

        let new_bytes = pty.try_read();
        if new_bytes > 0 {
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
                    if let Some(writer) = &mut self.input {
                        if let Err(e) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
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
                    let (cell_w, cell_h) = renderer.cell_size();
                    if cell_w > 0.0 && cell_h > 0.0 {
                        let new_grid = grid_for(size.width, size.height, cell_w, cell_h);
                        if new_grid.cols != self.grid.cols || new_grid.rows != self.grid.rows {
                            self.grid = new_grid;
                            if let Some(pty) = &mut self.pty {
                                pty.resize(new_grid);
                            }
                        }
                    }
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let (Some(renderer), Some(pty)) = (&mut self.renderer, &self.pty) else {
                    return;
                };

                let (cell_w, cell_h) = renderer.cell_size();
                let metrics = GridMetrics {
                    cell_w,
                    cell_h,
                    cols: self.grid.cols,
                    rows: self.grid.rows,
                };
                let views = to_cellviews(&pty.cells());
                renderer.frame(&views, metrics);
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
