//! Headless swarm-view layout: normalized coordinates for the GPU pane.
use crate::graph::TaskState;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// An RGB colour triple.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Map a task state to its display colour.
pub fn state_color(state: TaskState) -> Rgb {
    match state {
        TaskState::Pending => Rgb(120, 120, 130),
        TaskState::Ready => Rgb(90, 150, 230),
        TaskState::Running => Rgb(0, 220, 140),
        TaskState::Done => Rgb(60, 170, 160),
        TaskState::Failed => Rgb(230, 80, 80),
        TaskState::Cancelled => Rgb(90, 90, 100),
    }
}

/// Which layout mode to use.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Constellation,
    Heatmap,
}

/// Switch to heatmap above this agent count.
pub const HEATMAP_THRESHOLD: usize = 150;

/// Pick the appropriate layout mode for a given agent count.
pub fn mode_for_count(n: usize) -> ViewMode {
    if n >= HEATMAP_THRESHOLD {
        ViewMode::Heatmap
    } else {
        ViewMode::Constellation
    }
}
