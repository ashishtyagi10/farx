//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod color;
mod cursor;
mod listener;
mod model;
mod modes;
mod pty;
pub use model::{GridSize, HeadlessTerm, RenderCell, TermModel};
pub use modes::InputModes;
pub use pty::PtyTerm;
