//! Recursive panel layout tree.
//!
//! Public API is re-exported here so `pub use panel_layout::*;` in `lib.rs`
//! continues to expose `LayoutNode` and `PanelLeaf`.

mod split;
mod traversal;
mod types;

#[cfg(test)]
mod tests;

pub use types::{LayoutNode, PanelLeaf};
