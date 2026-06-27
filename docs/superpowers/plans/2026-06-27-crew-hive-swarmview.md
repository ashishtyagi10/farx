# crew-hive Swarm View Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Build the headless, fully-testable core of the sci-fi swarm view: a UI-independent layout engine that turns a `TaskGraph` + `Fleet` telemetry snapshot into placed nodes (constellation) or a dense status grid (heatmap), selecting the mode by agent count. This is the math the GPU pane will render; keeping it in `crew-hive` makes it unit-testable without a GPU.

**Architecture:** One module `view` in `crew-hive`. It produces **normalized** coordinates (0.0–1.0) so the renderer (crew-app, later) maps them to pixels. `constellation(graph, fleet)` lays nodes out by dependency depth (x = layer, y = spread within layer) and tags each with its state/color from the fleet. `heatmap(fleet, cols)` packs one cell per agent into a row-major grid. `ViewMode::for_count(n)` picks Constellation below a threshold, Heatmap at/above it. A small `StateColor` maps `TaskState` to an RGB triple so the renderer stays dumb. Pure functions, deterministic, no GPU/GUI.

**Tech Stack:** Rust, `serde` (layouts are serializable for the future remote/sidecar bridge), `cargo test`. No new deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total. Split into submodules before crossing it.
- **No new dependencies.**
- Layout output is **normalized** (`0.0..=1.0`); the renderer maps to pixels. Deterministic ordering (sort by id) so output is stable.
- Boundary types derive `serde::{Serialize, Deserialize}`.
- crew-hive depends on no other crew crate.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- Consumes: `crate::graph::{TaskGraph, TaskId, TaskState}`, `crate::bus::AgentId`, `crate::telemetry::Fleet`/`AgentTelemetry`.

---

### Task 1: State colors + view-mode selection

**Files:**
- Create: `crates/crew-hive/src/view/mod.rs`
- Create: `crates/crew-hive/src/view/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod view;`)

**Interfaces:**
- Produces (in `crate::view`):
  - `pub struct Rgb(pub u8, pub u8, pub u8);` — `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize`.
  - `pub fn state_color(state: TaskState) -> Rgb` — Pending=gray (120,120,130), Ready=blue (90,150,230), Running=green (0,220,140), Done=teal (60,170,160), Failed=red (230,80,80), Cancelled=dim (90,90,100).
  - `pub enum ViewMode { Constellation, Heatmap }` — `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize`.
  - `pub const HEATMAP_THRESHOLD: usize = 150;`
  - `pub fn mode_for_count(n: usize) -> ViewMode` — `Heatmap` if `n >= HEATMAP_THRESHOLD`, else `Constellation`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/view/tests.rs`:
```rust
use super::*;
use crate::graph::TaskState;

#[test]
fn state_color_distinguishes_states() {
    assert_eq!(state_color(TaskState::Running), Rgb(0, 220, 140));
    assert_eq!(state_color(TaskState::Failed), Rgb(230, 80, 80));
    assert_ne!(state_color(TaskState::Pending), state_color(TaskState::Done));
}

#[test]
fn mode_switches_at_threshold() {
    assert_eq!(mode_for_count(1), ViewMode::Constellation);
    assert_eq!(mode_for_count(HEATMAP_THRESHOLD - 1), ViewMode::Constellation);
    assert_eq!(mode_for_count(HEATMAP_THRESHOLD), ViewMode::Heatmap);
    assert_eq!(mode_for_count(5000), ViewMode::Heatmap);
}

#[test]
fn rgb_serde_roundtrip() {
    let c = Rgb(1, 2, 3);
    let j = serde_json::to_string(&c).unwrap();
    assert_eq!(serde_json::from_str::<Rgb>(&j).unwrap(), c);
}
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p crew-hive view::`
Expected: FAIL — `view` not defined.

- [ ] **Step 3: Implement**

Create `crates/crew-hive/src/view/mod.rs` with `Rgb`, `state_color`, `ViewMode`, `HEATMAP_THRESHOLD`, `mode_for_count`, and `mod`/`#[cfg(test)] mod tests;` declarations. Add `pub mod view;` to `lib.rs`. (The `constellation`/`heatmap` functions are added in Tasks 2–3; declare their submodules then.)

- [ ] **Step 4: Run to verify pass + commit**

Run: `cargo test -p crew-hive view:: && cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/view crates/crew-hive/src/lib.rs
git commit -m "feat(hive): swarm-view state colors + view-mode selection"
```

---

### Task 2: Constellation layout

**Files:**
- Create: `crates/crew-hive/src/view/constellation.rs`
- Modify: `crates/crew-hive/src/view/mod.rs` (declare `mod constellation;` + re-export)
- Modify: `crates/crew-hive/src/view/tests.rs` (append tests)

**Interfaces:**
- Consumes: `crate::graph::{TaskGraph, TaskId, TaskState}`, `crate::telemetry::Fleet`.
- Produces:
  - `pub struct Node { pub task: TaskId, pub x: f32, pub y: f32, pub color: Rgb, pub state: TaskState }` — `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - `pub struct Edge { pub from: TaskId, pub to: TaskId }` — `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize`.
  - `pub struct Constellation { pub nodes: Vec<Node>, pub edges: Vec<Edge> }` — `Clone, Debug, Default, Serialize, Deserialize`.
  - `pub fn constellation(graph: &TaskGraph, fleet: &Fleet) -> Constellation` — places each task by dependency **depth** (longest path from a root): `x = depth / max_depth` (or 0.5 when single layer), and within each depth layer, `y = (i + 1) / (layer_count + 1)` for the i-th task (sorted by id). Each node's `state` comes from the fleet (the telemetry whose `task` matches), defaulting to `TaskState::Pending` if the task has no agent yet; `color = state_color(state)`. Edges are `(dep -> task)` for every dependency. Nodes sorted by task id for determinism.
- Behavior contract (tested): a linear chain 0→1→2 lays out at increasing x (0, 0.5, 1.0); two independent roots share x=… and differ in y; edges mirror deps; node state reflects the fleet.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-hive/src/view/tests.rs`:
```rust
use crate::bus::{AgentId, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::telemetry::Fleet;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec { id: TaskId(id), title: format!("t{id}"), agent: AgentKind::Api { system: None }, model: ModelTier::Standard, deps: deps.iter().map(|d| TaskId(*d)).collect(), prompt: String::new() }
}

#[test]
fn constellation_chain_increases_x_by_depth() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let c = constellation(&g, &Fleet::new());
    let by_id = |id: u64| c.nodes.iter().find(|n| n.task == TaskId(id)).unwrap();
    assert!(by_id(0).x < by_id(1).x && by_id(1).x < by_id(2).x);
    // edges mirror deps
    assert!(c.edges.contains(&Edge { from: TaskId(0), to: TaskId(1) }));
    assert!(c.edges.contains(&Edge { from: TaskId(1), to: TaskId(2) }));
    assert_eq!(c.nodes.len(), 3);
}

#[test]
fn constellation_node_state_reflects_fleet() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned { agent: AgentId(0), task: TaskId(0) });
    let c = constellation(&g, &fleet);
    assert_eq!(c.nodes[0].state, crate::graph::TaskState::Running);
    assert_eq!(c.nodes[0].color, state_color(crate::graph::TaskState::Running));
}

#[test]
fn constellation_roots_share_layer_differ_in_y() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[])]).unwrap();
    let c = constellation(&g, &Fleet::new());
    let n0 = c.nodes.iter().find(|n| n.task == TaskId(0)).unwrap();
    let n1 = c.nodes.iter().find(|n| n.task == TaskId(1)).unwrap();
    assert_eq!(n0.x, n1.x);
    assert_ne!(n0.y, n1.y);
}
```

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive view::` (FAIL), implement `constellation.rs` (compute depth per task via longest-path over deps — memoized recursion or iterative relaxation; group by depth; assign normalized x/y; pull state from fleet by scanning `fleet.agents()` for a matching `task`), declare `mod constellation; pub use constellation::{Constellation, Node, Edge, constellation};` in `view/mod.rs`, then `cargo test -p crew-hive view::` (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Lint + commit**

Run: `cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/view
git commit -m "feat(hive): constellation layout (depth-placed nodes + dep edges)"
```

---

### Task 3: Heatmap layout + fleet view entry point

**Files:**
- Create: `crates/crew-hive/src/view/heatmap.rs`
- Modify: `crates/crew-hive/src/view/mod.rs` (declare `mod heatmap;` + re-export + add `fleet_view`)
- Modify: `crates/crew-hive/src/view/tests.rs` (append tests)

**Interfaces:**
- Consumes: `crate::telemetry::Fleet`, `crate::bus::AgentId`.
- Produces:
  - `pub struct Cell { pub agent: AgentId, pub row: usize, pub col: usize, pub color: Rgb }` — `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - `pub struct Heatmap { pub cols: usize, pub rows: usize, pub cells: Vec<Cell> }` — `Clone, Debug, Default, Serialize, Deserialize`.
  - `pub fn heatmap(fleet: &Fleet, cols: usize) -> Heatmap` — one cell per agent (ascending agent id), row-major: agent `i` → `row = i / cols, col = i % cols`; `color = state_color(agent.state)`; `rows = ceil(n / cols)`; `cols` clamped to `>= 1`.
  - `pub enum FleetView { Constellation(Constellation), Heatmap(Heatmap) }` — `Clone, Debug, Serialize, Deserialize`.
  - `pub fn fleet_view(graph: &TaskGraph, fleet: &Fleet, heatmap_cols: usize) -> FleetView` — picks by `mode_for_count(fleet.totals().live + fleet.totals().done + fleet.totals().failed)`… simpler: by the number of agents in the fleet (add `Fleet::len()` if not present, or count `fleet.agents()`); returns the matching variant.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-hive/src/view/tests.rs`:
```rust
#[test]
fn heatmap_packs_row_major() {
    let mut fleet = Fleet::new();
    for i in 0..5u64 {
        fleet.apply(&HiveEvent::AgentSpawned { agent: AgentId(i), task: TaskId(i) });
    }
    let h = heatmap(&fleet, 2);
    assert_eq!(h.cols, 2);
    assert_eq!(h.rows, 3); // ceil(5/2)
    assert_eq!(h.cells.len(), 5);
    let c4 = h.cells.iter().find(|c| c.agent == AgentId(4)).unwrap();
    assert_eq!((c4.row, c4.col), (2, 0));
}

#[test]
fn fleet_view_picks_constellation_when_small() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned { agent: AgentId(0), task: TaskId(0) });
    assert!(matches!(fleet_view(&g, &fleet, 16), FleetView::Constellation(_)));
}
```

- [ ] **Step 2: Run fail → implement → pass**

Implement `heatmap.rs` + `fleet_view` + any helper (`Fleet::len`/`is_empty` if needed — add to telemetry, `#[cfg(test)]`-safe, but here it's used by production `view` code so it's a normal pub method). Declare/re-export in `view/mod.rs`. `cargo test -p crew-hive view::` PASS. Keep ≤ 200 lines.

- [ ] **Step 3: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets`.
```bash
git add crates/crew-hive/src/view crates/crew-hive/src/telemetry
git commit -m "feat(hive): heatmap layout + fleet_view entry point (mode by count)"
```

---

## Self-Review

- **Spec coverage:** "constellation (default): agents as nodes, edges = deps, color = state" → `constellation()`. "heatmap auto-engages past ~150 agents" → `mode_for_count` + `heatmap()`. "fleet HUD data" → fleet `totals()` already exists. Headless + deterministic + serde (so the future remote bridge can ship layouts). ✅
- **Placeholder scan:** Task 1 complete; Tasks 2–3 give interfaces + tests + the exact placement formulas (depth-based x, spread y, row-major heatmap). ✅
- **No new deps / no GUI / no LLM.** ✅
- **File sizes:** each ≤ 200 (constellation depth calc is the largest; split if needed). ✅

## Where this sits

The testable core of the sci-fi swarm view. The follow-on (in crew-app, GUI) maps these normalized layouts to GPU cells/quads and adds drill-down — that step needs a GPU to runtime-verify, so it's deferred to hands-on testing. Remaining engine plans: **batch mode + cost/model governance**, and **remote spill + sidecar bridge**.
