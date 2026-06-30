use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver};

use crate::model::{GridSize, RenderCell, TermCore, TermModel};

/// Upper bound on chunks buffered from the reader thread. At 8 KiB per chunk
/// this caps buffered PTY output at ~8 MiB and applies backpressure to a runaway
/// program: once the OS pipe buffer and this queue fill, the child's `write`
/// blocks, throttling it to our drain rate instead of piling up unbounded
/// memory and unbounded parse work.
const CHANNEL_CAP: usize = 1024;

/// Maximum bytes drained from the PTY into the parser per `try_read` — i.e. per
/// poll tick. Without this cap a program that floods output (`yes`, `cat` of a
/// huge file, a noisy build) makes a single `try_read` parse the entire backlog
/// synchronously on the main thread, freezing rendering and input in EVERY pane
/// until it finishes. Capping per tick keeps the UI responsive; any remainder is
/// consumed on following ticks (see `has_pending`).
const READ_BUDGET: usize = 256 * 1024;

pub struct PtyTerm {
    core: TermCore,
    master: Box<dyn portable_pty::MasterPty + Send>,
    rx: Receiver<Vec<u8>>,
    exited: bool,
    /// Set by `try_read` when it stopped at `READ_BUDGET` with bytes still
    /// queued, so the caller can keep the poll loop hot until the backlog drains.
    pending: bool,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
    /// Spawn a shell (no extra args).  Delegates to `spawn_args`.
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        Self::spawn_args(size, shell, &[])
    }

    /// Spawn `command` with `args` in a new PTY of the given size.
    pub fn spawn_args(size: GridSize, command: &str, args: &[String]) -> anyhow::Result<Self> {
        Self::spawn_in(size, command, args, None)
    }

    /// Spawn `command` with `args` in a new PTY, starting in `cwd` when given
    /// (otherwise the child inherits the host process's working directory).
    pub fn spawn_in(
        size: GridSize,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        // Advertise a capable terminal so TUI programs behave (env is otherwise
        // inherited from the host process, so $HOME/$PATH etc. are present).
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd)?;
        // Drop the slave end so EOF propagates when the child exits.
        drop(pair.slave);

        // Spawn a reader thread: portable-pty reads are blocking. The channel is
        // bounded so a flooding child can't pile up unbounded output in memory —
        // a full queue blocks the reader (and in turn the child) until the main
        // thread drains it.
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = sync_channel::<Vec<u8>>(CHANNEL_CAP);
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
            exited: false,
            pending: false,
            _child: child,
        })
    }

    /// True once the child process has exited and all its output has been drained
    /// (the reader thread ended and the channel disconnected). Set by `try_read`.
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Returns a fresh writer to the master PTY end (sends input to the shell).
    pub fn writer(&self) -> Box<dyn std::io::Write + Send> {
        self.master.take_writer().expect("pty writer already taken")
    }

    /// Drains pending bytes from the reader thread into the terminal model,
    /// returning the number of bytes consumed this tick. At most `READ_BUDGET`
    /// bytes are drained per call so one flooding pane can't stall the event
    /// loop; when bytes remain queued past the budget, `has_pending` returns true
    /// and the rest is consumed on the next tick.
    pub fn try_read(&mut self) -> usize {
        use std::sync::mpsc::TryRecvError;
        let mut total = 0;
        self.pending = false;
        loop {
            // Stop once this tick's budget is spent. The reader thread can refill
            // the channel as fast as we drain it (a flooding child), so without
            // this cap the loop never sees `Empty` and parses forever, hanging
            // the event loop. Leftover bytes are flagged via `pending`.
            if total >= READ_BUDGET {
                self.pending = true;
                break;
            }
            match self.rx.try_recv() {
                Ok(chunk) => {
                    total += chunk.len();
                    self.core.feed(&chunk);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Reader thread ended → child exited and output is drained.
                    self.exited = true;
                    break;
                }
            }
        }
        total
    }

    /// True when the last `try_read` left bytes queued (it hit `READ_BUDGET`).
    /// The poll loop uses this to keep draining promptly rather than waiting a
    /// full tick, so flooded output catches up without ever blocking the UI.
    pub fn has_pending(&self) -> bool {
        self.pending
    }
}

impl PtyTerm {
    /// Scroll the viewport by `delta` lines into scrollback (positive = older).
    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    /// Jump back to the live bottom of the terminal.
    pub fn scroll_to_bottom(&mut self) {
        self.core.scroll_to_bottom();
    }

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    /// Whether the program enabled bracketed-paste mode.
    pub fn bracketed_paste(&self) -> bool {
        self.core.bracketed_paste()
    }

    /// The DEC private modes that decide how a scroll wheel is routed (alternate
    /// screen, mouse reporting, app-cursor keys).
    pub fn input_modes(&self) -> crate::modes::InputModes {
        self.core.input_modes()
    }

    /// Begin a mouse selection at viewport cell (col, row); `block` = rectangular.
    pub fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        self.core.sel_start(col, row, block);
    }

    /// Extend the active selection to viewport cell (col, row).
    pub fn sel_update(&mut self, col: u16, row: u16) {
        self.core.sel_update(col, row);
    }

    /// Clear any active selection.
    pub fn sel_clear(&mut self) {
        self.core.sel_clear();
    }

    /// The selected text, or `None` when nothing (non-empty) is selected.
    pub fn sel_text(&self) -> Option<String> {
        self.core.sel_text()
    }

    /// The program-set window title (OSC 0/2), empty if none.
    pub fn title(&self) -> String {
        self.core.title()
    }

    /// The directory the program reported via OSC 7 if it changed since the last
    /// call, else `None` — used to retitle the pane when the user `cd`s inside it.
    pub fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.core.take_cwd()
    }

    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub fn take_clipboard(&self) -> Option<String> {
        self.core.take_clipboard()
    }

    /// Take a pending bell (rung since the last check), clearing it.
    pub fn take_bell(&self) -> bool {
        self.core.take_bell()
    }
}

impl TermModel for PtyTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self, focused: bool) -> Vec<RenderCell> {
        self.core.cells(focused)
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
                let mut cs: Vec<_> = term.cells(true);
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

    #[test]
    fn try_read_caps_bytes_per_tick_under_flood() {
        // A program that floods stdout: a single tick must not drain the whole
        // backlog, or it would block the event loop (and every other pane).
        let mut term = PtyTerm::spawn(GridSize { cols: 80, rows: 24 }, "sh").unwrap();
        let mut w = term.writer();
        w.write_all(b"yes crew-flood-line\n").unwrap();
        w.flush().unwrap();
        // Let the reader thread buffer well past one tick's budget.
        std::thread::sleep(Duration::from_millis(250));

        // The budget is checked between chunks, so the final 8 KiB reader chunk
        // can overshoot slightly — the point is the drain is *bounded* to roughly
        // the budget instead of consuming the whole flood (which would hang).
        let n = term.try_read();
        assert!(
            n <= READ_BUDGET + 8192,
            "one tick drained {n} bytes, far over the {READ_BUDGET}-byte budget"
        );
        assert!(
            term.has_pending(),
            "expected a backlog to remain after a budget-capped read"
        );

        // Stop `yes` so the child doesn't keep spinning after the test.
        let _ = w.write_all(&[0x03]); // Ctrl-C to the foreground process group
        let _ = w.flush();
    }
}
