use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderCell {
    pub col: u16,
    pub row: u16,
    pub c: char,
}

pub trait TermModel {
    fn feed(&mut self, bytes: &[u8]);
    fn cells(&self) -> Vec<RenderCell>;
    fn resize(&mut self, size: GridSize);
}

// alacritty_terminal needs a Dimensions impl describing the viewport.
#[derive(Clone, Copy)]
struct Dims {
    cols: usize,
    rows: usize,
}

impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

// A no-op event listener — we don't react to terminal events yet.
#[derive(Clone)]
struct NoopListener;

impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

// Shared core: a Term + an ANSI processor. Used by HeadlessTerm (and PtyTerm in Task 3).
struct TermCore {
    term: Term<NoopListener>,
    parser: Processor,
}

impl TermCore {
    fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let term = Term::new(Config::default(), &dims, NoopListener);
        Self {
            term,
            parser: Processor::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    fn cells(&self) -> Vec<RenderCell> {
        let content = self.term.renderable_content();
        // display_iter yields Indexed<&Cell>; Indexed derefs to Cell, so .c is available.
        // point.line is i32 (0 = top of viewport); point.column is usize.
        content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0' && ind.point.line.0 >= 0)
            .map(|ind| RenderCell {
                col: ind.point.column.0 as u16,
                row: ind.point.line.0 as u16,
                c: ind.c,
            })
            .collect()
    }

    fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
    }
}

pub struct HeadlessTerm {
    core: TermCore,
}

impl HeadlessTerm {
    pub fn new(size: GridSize) -> Self {
        Self {
            core: TermCore::new(size),
        }
    }
}

impl TermModel for HeadlessTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self) -> Vec<RenderCell> {
        self.core.cells()
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}

// ── PtyTerm: TermModel backed by a real shell child process ──────────────────

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::sync::mpsc::{channel, Receiver};

pub struct PtyTerm {
    core: TermCore,
    master: Box<dyn portable_pty::MasterPty + Send>,
    rx: Receiver<Vec<u8>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let child = pair.slave.spawn_command(CommandBuilder::new(shell))?;
        // Drop the slave end so EOF propagates when the child exits.
        drop(pair.slave);

        // Spawn a reader thread: portable-pty reads are blocking.
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            core: TermCore::new(size),
            master: pair.master,
            rx,
            _child: child,
        })
    }

    /// Returns a fresh writer to the master PTY end (sends input to the shell).
    pub fn writer(&self) -> Box<dyn std::io::Write + Send> {
        self.master.take_writer().expect("pty writer already taken")
    }

    /// Drains all pending bytes from the reader thread into the terminal model.
    /// Returns the total number of bytes consumed.
    pub fn try_read(&mut self) -> usize {
        let mut total = 0;
        while let Ok(chunk) = self.rx.try_recv() {
            total += chunk.len();
            self.core.feed(&chunk);
        }
        total
    }
}

impl TermModel for PtyTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self) -> Vec<RenderCell> {
        self.core.cells()
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
        let _ = self.master.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A headless Term (no PTY) we can feed bytes into deterministically.
    #[test]
    fn feeding_text_appears_in_cells() {
        let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 5 });
        term.feed(b"hi");
        let cells = term.cells();
        let text: String = {
            let mut row0: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            row0.sort_by_key(|c| c.col);
            row0.iter().map(|c| c.c).collect()
        };
        assert_eq!(text, "hi");
    }
}

#[cfg(test)]
mod pty_tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration, Instant};

    #[test]
    fn echo_roundtrips_through_pty() {
        let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
        let mut w = term.writer();
        // Echo a unique token, then read until it shows up on the grid.
        w.write_all(b"printf CREWOK\n").unwrap();
        w.flush().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while Instant::now() < deadline {
            term.try_read();
            let line: String = {
                let mut cs: Vec<_> = term.cells();
                cs.sort_by_key(|c| (c.row, c.col));
                cs.iter().map(|c| c.c).collect()
            };
            if line.contains("CREWOK") {
                found = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(found, "expected CREWOK to appear on the terminal grid");
    }
}
