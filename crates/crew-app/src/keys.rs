//! Keyboard event dispatch for CrewApp.
use std::io::Write;

use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::session::key_to_bytes;
use crate::settingspane::SettingsAction;

impl CrewApp {
    /// Dispatch a single `KeyEvent` from `window_event`.
    pub(crate) fn on_key_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &winit::event::KeyEvent,
    ) {
        let mstate = self.mods.state();

        // Cmd+Q / Ctrl+Q quits the app.
        if event.state.is_pressed()
            && (mstate.super_key() || mstate.control_key())
            && matches!(&event.logical_key, Key::Character(s) if s.as_str() == "q")
        {
            event_loop.exit();
            return;
        }

        // Super-chords (e.g. Cmd+I, Cmd+T, …) are handled first.
        if mstate.super_key() && event.state.is_pressed() {
            if let Key::Character(s) = &event.logical_key {
                let s = s.to_string();
                if self.handle_super_chord(&s) {
                    event_loop.exit();
                }
            }
            self.redraw();
            return;
        }

        // When the input bar is focused, all non-super keys go to it.
        if self.input.focused {
            if event.state.is_pressed()
                && matches!(&event.logical_key, Key::Named(NamedKey::Escape))
            {
                self.input.focused = false;
                self.redraw();
                return;
            }
            let submitted = self.input.on_key(event);
            if let Some(line) = submitted {
                if self.submit_input(line) {
                    event_loop.exit();
                    return;
                }
            }
            self.redraw();
            return;
        }

        // Route non-super keys to the focused pane.
        let focused = self.focused;
        let shift = mstate.shift_key();
        let mut settings_action: Option<SettingsAction> = None;
        if let Some(pane) = self.panes.get_mut(focused) {
            match &mut pane.content {
                PaneContent::Terminal(t) => {
                    if let Some(bytes) = key_to_bytes(event) {
                        // Typing snaps the view back to the live bottom.
                        t.pty.scroll_to_bottom();
                        if let Err(e) = t.input.write_all(&bytes).and_then(|_| t.input.flush()) {
                            eprintln!("pty write error: {e}");
                        }
                    }
                }
                PaneContent::Chat(c) => c.on_key(event),
                PaneContent::Settings(s) => {
                    settings_action = s.on_key(event, shift);
                }
            }
        }
        if let Some(action) = settings_action {
            if let SettingsAction::Apply(cfg) = action {
                self.apply_settings(cfg);
            }
            // Save and Cancel both close the settings pane.
            self.close_pane(focused);
        }
        self.redraw();
    }
}
