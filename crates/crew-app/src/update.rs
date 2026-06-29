//! Background self-update with progress in the left-nav UPDATE card, and an
//! automatic restart once the new binary is in place. `/update` starts a worker
//! thread that checks GitHub, downloads the latest release over the running
//! binary, and streams stage updates back to the UI — no separate shell pane,
//! no manual relaunch.
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::app::CrewApp;

/// Spinner frames cycled on the UPDATE card while a stage is in flight.
pub(crate) const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
/// Poll ticks per spinner frame (~62 Hz loop → ~10 fps).
const SPINNER_DIV: u64 = 6;
/// How long "updated — restarting…" shows before the re-exec.
const RESTART_DELAY: Duration = Duration::from_millis(900);
/// How long a terminal note (up-to-date / failed) lingers before auto-dismiss.
const NOTE_TTL: Duration = Duration::from_secs(5);

/// A stage message streamed from the worker thread to the UI.
pub(crate) enum UpdateMsg {
    Checking,
    Downloading(String),
    Installed(String),
    UpToDate(String),
    Failed(String),
}

/// Where the update is right now, mirrored from the latest [`UpdateMsg`].
pub(crate) enum Stage {
    Checking,
    Downloading(String),
    Done(String),
    Note(String),
}

/// Live update state held on `CrewApp` while `/update` runs.
pub(crate) struct UpdateState {
    rx: Receiver<UpdateMsg>,
    pub(crate) stage: Stage,
    pub(crate) spinner: usize,
    frame: u64,
    /// Restart-at (after Done) or clear-at (after a terminal note).
    deadline: Option<Instant>,
}

impl UpdateState {
    fn new(rx: Receiver<UpdateMsg>) -> Self {
        Self {
            rx,
            stage: Stage::Checking,
            spinner: 0,
            frame: 0,
            deadline: None,
        }
    }

    /// Drain pending worker messages into `stage`. Returns true if it changed.
    /// `try_recv` ending in either `Empty` or `Disconnected` stops the drain.
    fn drain(&mut self, now: Instant) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.rx.try_recv() {
            self.apply(msg, now);
            changed = true;
        }
        changed
    }

    fn apply(&mut self, msg: UpdateMsg, now: Instant) {
        self.stage = match msg {
            UpdateMsg::Checking => Stage::Checking,
            UpdateMsg::Downloading(v) => Stage::Downloading(v),
            UpdateMsg::Installed(v) => {
                self.deadline = Some(now + RESTART_DELAY);
                Stage::Done(v)
            }
            UpdateMsg::UpToDate(v) => {
                self.deadline = Some(now + NOTE_TTL);
                Stage::Note(format!("already up to date (v{v})"))
            }
            UpdateMsg::Failed(e) => {
                self.deadline = Some(now + NOTE_TTL);
                Stage::Note(format!("update failed: {e}"))
            }
        };
    }

    /// True while a network/disk stage is in flight (so the spinner animates).
    pub(crate) fn animating(&self) -> bool {
        matches!(self.stage, Stage::Checking | Stage::Downloading(_))
    }

    /// Advance the spinner on a throttle; returns true when its frame changed.
    fn tick_anim(&mut self) -> bool {
        self.frame = self.frame.wrapping_add(1);
        if self.frame.is_multiple_of(SPINNER_DIV) {
            self.spinner = self.spinner.wrapping_add(1);
            return true;
        }
        false
    }

    fn restart_due(&self, now: Instant) -> bool {
        matches!(self.stage, Stage::Done(_)) && self.deadline.is_some_and(|d| now >= d)
    }

    fn clear_due(&self, now: Instant) -> bool {
        matches!(self.stage, Stage::Note(_)) && self.deadline.is_some_and(|d| now >= d)
    }

    /// Build a state parked at `stage` (no worker thread) — for card-render tests.
    #[cfg(test)]
    pub(crate) fn for_test(stage: Stage) -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        Self {
            rx,
            stage,
            spinner: 0,
            frame: 0,
            deadline: None,
        }
    }
}

impl CrewApp {
    /// Start the background self-update (the `/update` command). A no-op while one
    /// is already running, so a double `/update` doesn't spawn two workers.
    pub(crate) fn start_update(&mut self) {
        if self.update.as_ref().is_some_and(UpdateState::animating) {
            self.set_status("update already in progress");
            return;
        }
        self.update = Some(UpdateState::new(crate::updatefetch::spawn_worker()));
        self.set_status("checking for updates…");
        self.redraw();
    }

    /// Drive the active update each poll tick. Returns `true` when Crew should
    /// re-exec into the freshly installed binary (the caller then exits the loop).
    pub(crate) fn poll_update(&mut self, now: Instant) -> UpdateTick {
        let mut tick = UpdateTick::default();
        let mut clear = false;
        if let Some(u) = self.update.as_mut() {
            tick.redraw = u.drain(now);
            if u.restart_due(now) {
                tick.restart = true;
                return tick;
            }
            if u.animating() && u.tick_anim() {
                tick.redraw = true;
            }
            clear = u.clear_due(now);
        }
        if clear {
            self.update = None;
            tick.redraw = true;
        }
        tick
    }
}

/// What one [`CrewApp::poll_update`] tick wants the caller to do.
#[derive(Default)]
pub(crate) struct UpdateTick {
    pub(crate) redraw: bool,
    pub(crate) restart: bool,
}

/// Re-exec the (now-updated) binary as a fresh, detached process. The caller
/// exits the event loop immediately after, handing the window to the new build.
pub(crate) fn restart_into_new_binary() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
}
