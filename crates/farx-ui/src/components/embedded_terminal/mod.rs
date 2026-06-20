//! Embedded terminal session backed by a PTY and a vt100 parser.
//!
//! Split into:
//! - [`session`]: PTY spawn lifecycle, output polling, input writing, resize,
//!   vt100 parsing glue.
//! - [`input`]: key event to PTY byte encoding.
//! - [`render`]: ratatui rendering of the vt100 screen.
//! - [`scroll`]: scrollback navigation (mouse-wheel history).

mod input;
mod render;
mod scroll;
mod session;
mod thumbnail;

pub use input::key_to_bytes;
pub use render::render_terminal;
pub use scroll::SCROLL_STEP;
pub use session::{OutputWaker, TerminalSession};
pub use thumbnail::render_thumbnail;
