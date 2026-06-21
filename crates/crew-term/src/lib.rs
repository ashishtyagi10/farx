//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod model;
pub use model::{GridSize, HeadlessTerm, RenderCell, TermModel};
