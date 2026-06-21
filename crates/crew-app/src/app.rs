use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crew_render::{GridMetrics, Renderer};
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::session::{key_to_bytes, to_cellviews};

const PTY_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
const POLL_MS: u64 = 16;

#[derive(Default)]
pub struct CrewApp {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    pty: Option<PtyTerm>,
    input: Option<Box<dyn std::io::Write + Send>>,
}

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        match Renderer::new(window.clone()) {
            Ok(renderer) => {
                let pty = PtyTerm::spawn(PTY_SIZE, "bash")
                    .or_else(|_| PtyTerm::spawn(PTY_SIZE, "sh"))
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
                    cols: PTY_SIZE.cols,
                    rows: PTY_SIZE.rows,
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
