//! A live swarm pane: runs a `crew_hive` task graph on a background worker via
//! [`SwarmHandle`], drains its events into a [`Fleet`] each frame, and renders
//! the fleet as a constellation + HUD with [`swarm_cells`]. Spawned by `/swarm`,
//! which launches a self-contained demo graph driven by always-succeeding stub
//! agents (no API keys, no network) — a live, end-to-end demonstration of the
//! scheduler → bridge → view pipeline.
use std::sync::Arc;

use crew_hive::agent::StubFactory;
use crew_hive::{AgentKind, Fleet, ModelTier, TaskGraph, TaskId, TaskSpec};
use crew_render::CellView;

use crate::swarm::bridge::SwarmHandle;
use crate::swarm::view::swarm_cells;

/// A pane that visualises a running swarm. Owns the off-thread engine handle and
/// the accumulated fleet telemetry; both are cheap to drain each frame.
pub struct SwarmPane {
    handle: SwarmHandle,
    fleet: Fleet,
}

impl SwarmPane {
    /// Launch a self-contained demo swarm: a fan-out/merge graph executed by
    /// always-succeeding stub agents. Requires no API keys or network.
    pub fn demo() -> Self {
        Self::with_graph(demo_graph())
    }

    /// Run `graph` on a background worker with stub agents at concurrency 4.
    fn with_graph(graph: TaskGraph) -> Self {
        let factory = Arc::new(StubFactory);
        let handle = SwarmHandle::spawn(graph, factory, 4);
        Self {
            handle,
            fleet: Fleet::new(),
        }
    }

    /// Drain any pending engine events into the fleet. Returns `true` when at
    /// least one event was applied (the view changed and the pane should redraw).
    pub fn poll(&mut self) -> bool {
        self.handle.drain(&mut self.fleet) > 0
    }

    /// Render the current fleet to cell views for a `cols × rows` grid.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        swarm_cells(self.handle.graph(), &self.fleet, cols, rows)
    }
}

impl Drop for SwarmPane {
    /// Stop the background scheduler when the pane closes, so a dismissed swarm
    /// doesn't keep spawning tasks on its worker thread.
    fn drop(&mut self) {
        self.handle.cancel();
    }
}

/// A small fan-out/merge demo graph: one root, three parallel workers, one merge
/// — enough structure to show the constellation layout and HUD counters.
fn demo_graph() -> TaskGraph {
    let task = |id: u64, title: &str, deps: Vec<TaskId>| TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps,
        prompt: format!("demo task {id}"),
    };
    TaskGraph::new(vec![
        task(0, "plan", vec![]),
        task(1, "research", vec![TaskId(0)]),
        task(2, "build", vec![TaskId(0)]),
        task(3, "test", vec![TaskId(0)]),
        task(4, "merge", vec![TaskId(1), TaskId(2), TaskId(3)]),
    ])
    .expect("demo graph is valid")
}

#[cfg(test)]
#[path = "swarmpane_tests.rs"]
mod tests;
