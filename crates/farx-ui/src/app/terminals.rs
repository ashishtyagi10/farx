//! Embedded-terminal lifecycle: spawn into the grid, close + remove, and
//! Tab/F4 cycle through terminals.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::components::embedded_terminal::{OutputWaker, TerminalSession};

use super::App;

impl App {
    /// Look up a terminal by stable id (linear scan).
    pub(crate) fn terminal_by_id(&self, id: usize) -> Option<&TerminalSession> {
        self.terminals.iter().find(|t| t.id == id)
    }

    /// Look up a terminal mutably by stable id (linear scan).
    pub(crate) fn terminal_by_id_mut(&mut self, id: usize) -> Option<&mut TerminalSession> {
        self.terminals.iter_mut().find(|t| t.id == id)
    }

    /// Register the event sender that PTY reader threads use to wake the
    /// render loop when an embedded terminal produces output.
    pub fn set_event_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::event::Event>,
    ) {
        self.terminal_event_tx = Some(tx);
    }

    /// Build a waker that coalesces output notifications into a single queued
    /// [`crate::event::Event::TerminalOutput`] until the loop drains output.
    fn output_waker(&self) -> Option<OutputWaker> {
        let tx = self.terminal_event_tx.clone()?;
        let pending = self.output_pending.clone();
        Some(Arc::new(move || {
            if !pending.swap(true, Ordering::SeqCst) {
                let _ = tx.send(crate::event::Event::TerminalOutput);
            }
        }))
    }

    /// Spawn an embedded terminal in the current directory.
    pub(super) fn spawn_embedded_terminal(&mut self, cmd: &str, args: &[&str]) {
        let dir = self.active_tree_ref().root.clone();
        self.spawn_embedded_terminal_in(cmd, args, dir);
    }

    /// Spawn an embedded terminal in `dir` and add it to the agent grid.
    pub(super) fn spawn_embedded_terminal_in(
        &mut self,
        cmd: &str,
        args: &[&str],
        dir: std::path::PathBuf,
    ) {
        let rows = 24;
        let cols = 80;
        let waker = self.output_waker();

        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        match TerminalSession::spawn(terminal_id, cmd, args, &dir, rows, cols, waker) {
            Ok(session) => {
                let title = session.title.clone();
                self.terminals.push(session);
                self.focused_terminal = Some(terminal_id);
                self.grid.add(terminal_id);
                self.feedback.info(format!("{} · {}", title, dir.display()));
            }
            Err(e) => {
                self.feedback
                    .error(format!("Failed to spawn terminal: {}", e));
            }
        }
    }

    /// Close a terminal session and remove it from the grid.
    pub(super) fn close_terminal(&mut self, terminal_id: usize) {
        if self.terminal_by_id(terminal_id).is_none() {
            return;
        }

        self.terminals.retain(|t| t.id != terminal_id);
        self.grid.remove(terminal_id);

        if self.focused_terminal == Some(terminal_id) {
            self.focused_terminal = None;
        }
    }

    /// Cycle focus to the next agent tile — Tab / F4 key (and `/next`).
    pub(super) fn cycle_focus(&mut self) {
        self.step_focus(1);
    }

    /// Cycle focus to the previous agent tile — `/prev`.
    pub(super) fn cycle_focus_back(&mut self) {
        self.step_focus(-1);
    }

    /// Move focus by `delta` steps through `grid.full()` then
    /// `grid.minimized()` (wrapping). Focusing a minimized id calls
    /// `grid.touch(id)` so it promotes into the full set on the next frame.
    fn step_focus(&mut self, delta: isize) {
        let order: Vec<usize> = self
            .grid
            .full()
            .iter()
            .chain(self.grid.minimized().iter())
            .copied()
            .collect();
        if order.is_empty() {
            self.focused_terminal = None;
            return;
        }
        let len = order.len() as isize;
        let next = match self.focused_terminal {
            Some(cur) => {
                let i = order.iter().position(|x| *x == cur).unwrap_or(0) as isize;
                order[(((i + delta) % len + len) % len) as usize]
            }
            None => order[0],
        };
        self.focus_tile(next);
    }

    /// Focus tile `id`: record the previously-focused tile for `/last`, promote
    /// it if minimized, and clear its attention flag.
    pub(crate) fn focus_tile(&mut self, id: usize) {
        if self.focused_terminal != Some(id) {
            self.last_focused_terminal = self.focused_terminal;
        }
        self.focused_terminal = Some(id);
        // Only reorder when focusing a currently-minimized tile (pull it up
        // into the grid). Cycling among full tiles keeps their positions
        // stable so they don't shuffle under the user.
        if self.grid.minimized().contains(&id) {
            self.grid.touch(id);
        }
        if let Some(t) = self.terminal_by_id_mut(id) {
            t.has_attention = false;
        }
    }

    /// Poll all terminal sessions for new output. Called on each tick and
    /// whenever a [`crate::event::Event::TerminalOutput`] wake-up arrives.
    pub fn poll_terminals(&mut self) {
        // Clear the coalescing flag first so output arriving during this drain
        // queues a fresh wake-up rather than being missed.
        self.output_pending.store(false, Ordering::SeqCst);
        let focused_tid = self.focused_terminal;
        for term in self.terminals.iter_mut() {
            let got_output = term.poll_output();
            if got_output && Some(term.id) != focused_tid {
                term.has_attention = true;
            }
        }
    }
}
