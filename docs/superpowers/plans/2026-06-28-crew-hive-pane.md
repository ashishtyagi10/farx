# /crew → Hive Swarm Pane Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repoint the `/crew` command from the multi-agent broker to an in-process hive swarm pane that takes a goal, runs the `crew-hive` scheduler on a background thread, and renders the live `FleetView` glyph grid; then retire the broker.

**Architecture:** A new `PaneContent::Crew(Box<CrewPane>)` pane is spawned directly by `/crew` (no subprocess). A background `std::thread` hosts a tokio runtime running `crew-hive`'s `Scheduler`; it folds `HiveEvent`s into a `Fleet` snapshot and streams `HiveUpdate`s back over an `mpsc` channel. The winit render loop never blocks: each frame the pane renders the latest snapshot via `fleet_view()` → `render_cells()`. The broker subsystem is deleted.

**Tech Stack:** Rust 2021, tokio (background runtime), `crew-hive` (orchestration), `crew-render` (`CellView`), winit (key events). No new third-party dependencies.

## Global Constraints

- **Hard 200-line cap per `.rs` file** — total, including imports/whitespace/docs. Split into submodules before crossing it.
- **No overlay UI** — in-pane UI renders to GPU cells (`Vec<CellView>`); panes are tiles in the auto-tiling grid.
- **Pass-through keys** — only act on keys the focused pane is given by `keys.rs`; do not steal keys globally.
- **No new dependencies** — only `crew-hive` (workspace path) and `tokio` (already a workspace dep) are added to `crew-app`.
- **Warning-free** — `cargo clippy --workspace --all-targets` must pass with zero warnings; remove dead code rather than `#[allow]`.
- **Bring-your-own-provider** — provider/model resolved at runtime; default model tier `Standard` (`claude-sonnet-4-6`).
- **Commit after each task.** End commit messages with the project's `Co-Authored-By` trailer.

---

## File Structure

**Created:**
- `crates/crew-app/src/hiverun.rs` — background-thread driver: `HiveUpdate`, `HiveHandle`, `run_hive`, `spawn_hive`.
- `crates/crew-app/src/hiverun_tests.rs` — tests for `run_hive`.
- `crates/crew-app/src/crewpane/mod.rs` — `CrewPane` struct, `new`, `poll`, `cells`.
- `crates/crew-app/src/crewpane/state.rs` — `CrewStatus` enum.
- `crates/crew-app/src/crewpane/keys.rs` — `on_key` (goal editing, Enter=launch, Esc=cancel).
- `crates/crew-app/src/crewpane/render.rs` — snapshot → `Vec<CellView>` (swarm grid + summary + goal bar).
- `crates/crew-app/src/crewpane/tests.rs` — `CrewPane` unit tests.

**Modified:**
- `crates/crew-hive/src/telemetry/mod.rs` — add `Clone` to `Fleet`.
- `crates/crew-hive/src/apiagent/mod.rs` — add `ApiFactory`.
- `crates/crew-hive/src/lib.rs` — re-export `ApiFactory`; remove nothing here.
- `crates/crew-app/Cargo.toml` — add `crew-hive` + `tokio` deps.
- `crates/crew-app/src/main.rs` — add `mod hiverun; mod crewpane;`; remove the `--broker-plugin` branch.
- `crates/crew-app/src/pane.rs` — add `Crew` arm to `PaneContent`, `cells()`, `title_text()`.
- `crates/crew-app/src/keys.rs` — route keys to `PaneContent::Crew`.
- `crates/crew-app/src/poll.rs` — poll `PaneContent::Crew`.
- `crates/crew-app/src/chatspawn.rs` — rewrite `spawn_crew_pane` (no subprocess); delete `crew_broker_cmd`.

**Deleted (broker retirement):**
- `crates/crew-plugin/src/broker/` (entire dir).
- `crates/crew-plugin/src/bin/crew-broker-plugin.rs`.
- `crates/crew-plugin/src/lib.rs` broker re-export block.

---

## Task 1: crew-hive — `Fleet: Clone` + `ApiFactory`

**Files:**
- Modify: `crates/crew-hive/src/telemetry/mod.rs:36`
- Modify: `crates/crew-hive/src/apiagent/mod.rs`
- Modify: `crates/crew-hive/src/lib.rs:71`
- Test: `crates/crew-hive/src/apiagent/tests.rs`

**Interfaces:**
- Produces: `Fleet: Clone` (so snapshots can be sent over a channel). `ApiFactory::new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> ApiFactory`, implementing `AgentFactory` (every `make()` returns an `ApiAgent`). Re-exported as `crew_hive::ApiFactory`.

- [ ] **Step 1: Write the failing test**

Append to `crates/crew-hive/src/apiagent/tests.rs`:

```rust
#[test]
fn api_factory_makes_an_agent_and_fleet_clones() {
    use crate::agent::AgentFactory;
    use crate::graph::{AgentKind, ModelTier};
    use crate::provider::MockProvider;
    use crate::telemetry::Fleet;
    use std::sync::Arc;

    let provider = Arc::new(MockProvider::new("ok"));
    let factory = crate::apiagent::ApiFactory::new(provider, ModelTier::Standard, 256);
    // make() must return a usable agent for an Api task kind.
    let _agent = factory.make(&AgentKind::Api { system: None });

    // Fleet must be Clone so the app can snapshot it across a channel.
    let fleet = Fleet::new();
    let _copy = fleet.clone();
}
```

If `MockProvider::new` has a different constructor, check `crates/crew-hive/src/provider/mock.rs` and match it; the test only needs any working `MockProvider`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-hive api_factory_makes_an_agent_and_fleet_clones`
Expected: FAIL — `no function or associated item named 'new' found for struct 'ApiFactory'` (or `ApiFactory` not found) and/or `Fleet: Clone` not satisfied.

- [ ] **Step 3: Add `Clone` to `Fleet`**

In `crates/crew-hive/src/telemetry/mod.rs`, change the `Fleet` derive line:

```rust
#[derive(Debug, Default, Clone)]
pub struct Fleet {
    agents: BTreeMap<u64, AgentTelemetry>,
}
```

- [ ] **Step 4: Add `ApiFactory` to the apiagent module**

In `crates/crew-hive/src/apiagent/mod.rs`, add after the `impl Agent for ApiAgent` block (it already imports `Agent`, `AgentKind`, `ModelTier`, `Provider`, `Arc`):

```rust
// ---------------------------------------------------------------------------
// ApiFactory
// ---------------------------------------------------------------------------

use crate::agent::AgentFactory;

/// Agent factory that makes native [`ApiAgent`]s sharing one provider.
/// `make` ignores per-task tier (the scheduler only passes `AgentKind`); all
/// agents use the configured `tier`. Per-task tiers are a follow-up.
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    tier: ModelTier,
    max_tokens: u32,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> Self {
        Self {
            provider,
            tier,
            max_tokens,
        }
    }
}

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(ApiAgent::new(
            Arc::clone(&self.provider),
            self.tier,
            self.max_tokens,
        ))
    }
}
```

- [ ] **Step 5: Re-export `ApiFactory`**

In `crates/crew-hive/src/lib.rs`, change the ApiAgent re-export line (currently line ~71 `pub use apiagent::ApiAgent;`) to:

```rust
// ApiAgent
pub use apiagent::{ApiAgent, ApiFactory};
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p crew-hive api_factory_makes_an_agent_and_fleet_clones`
Expected: PASS

- [ ] **Step 7: Check file size & clippy**

Run: `wc -l crates/crew-hive/src/apiagent/mod.rs`
Expected: ≤ 200. If over, move `ApiFactory` into a new `crates/crew-hive/src/apiagent/factory.rs` submodule (`mod factory; pub use factory::ApiFactory;`).

Run: `cargo clippy -p crew-hive --all-targets`
Expected: zero warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-hive/
git commit -m "feat(hive): Fleet: Clone + ApiFactory for native-API swarms

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: crew-app — `hiverun` background driver

**Files:**
- Modify: `crates/crew-app/Cargo.toml`
- Create: `crates/crew-app/src/hiverun.rs`
- Create: `crates/crew-app/src/hiverun_tests.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod hiverun;`)

**Interfaces:**
- Consumes: `crew_hive::{StubPlanner, Planner, AgentFactory, StubFactory, ApiFactory, AnthropicProvider, LlmPlanner, Provider, ModelTier, TaskGraph, Fleet, Blackboard, EventBus, Scheduler, RunOutcome, HiveEvent}`.
- Produces:
  - `enum HiveUpdate { Planned(TaskGraph), Telemetry(Fleet), Finished(RunOutcome), Error(String) }`
  - `struct HiveHandle { pub rx: std::sync::mpsc::Receiver<HiveUpdate>, pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool> }`
  - `async fn run_hive(goal: String, planner: Arc<dyn Planner>, factory: Arc<dyn AgentFactory>, tx: std::sync::mpsc::Sender<HiveUpdate>, cancel: Arc<AtomicBool>)`
  - `fn spawn_hive(goal: String) -> HiveHandle`

- [ ] **Step 1: Add dependencies**

In `crates/crew-app/Cargo.toml`, under `[dependencies]`, add:

```toml
crew-hive = { path = "../crew-hive" }
tokio = { workspace = true }
```

- [ ] **Step 2: Write the failing test**

Create `crates/crew-app/src/hiverun_tests.rs`:

```rust
use super::*;
use crew_hive::{StubFactory, StubPlanner};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[tokio::test]
async fn run_hive_emits_planned_then_finished() {
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let planner: Arc<dyn crew_hive::Planner> = Arc::new(StubPlanner { fanout: 2 });
    let factory: Arc<dyn crew_hive::AgentFactory> = Arc::new(StubFactory);

    run_hive("build a thing".into(), planner, factory, tx, cancel).await;

    let updates: Vec<HiveUpdate> = rx.try_iter().collect();
    assert!(
        matches!(updates.first(), Some(HiveUpdate::Planned(_))),
        "first update should be Planned, got {updates:?}"
    );
    assert!(
        matches!(updates.last(), Some(HiveUpdate::Finished(_))),
        "last update should be Finished, got {updates:?}"
    );
    assert!(
        updates.iter().any(|u| matches!(u, HiveUpdate::Telemetry(_))),
        "expected at least one Telemetry update"
    );
}
```

For this to compile, `HiveUpdate` must derive `Debug` (the `{updates:?}` format). Ensure the derive in Step 4 includes `Debug`.

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p crew-app run_hive_emits_planned_then_finished`
Expected: FAIL — `hiverun` module / `run_hive` not found.

- [ ] **Step 4: Implement `hiverun.rs`**

Create `crates/crew-app/src/hiverun.rs`:

```rust
//! Background driver for the `/crew` hive swarm. A `std::thread` hosts a tokio
//! runtime that plans a goal into a task graph, runs the scheduler, and streams
//! `HiveUpdate`s back to the `CrewPane` over an `mpsc` channel. The render loop
//! never blocks: it only drains the channel.
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crew_hive::{
    AgentFactory, AnthropicProvider, ApiFactory, Blackboard, EventBus, Fleet, HiveEvent,
    LlmPlanner, ModelTier, Planner, Provider, RunOutcome, Scheduler, TaskGraph,
};
use tokio::sync::broadcast::error::RecvError;

const BUS_CAPACITY: usize = 1024;
const CONCURRENCY: usize = 8;
const MAX_TOKENS: u32 = 2048;

/// A snapshot update sent from the hive thread to the pane.
#[derive(Debug)]
pub enum HiveUpdate {
    Planned(TaskGraph),
    Telemetry(Fleet),
    Finished(RunOutcome),
    Error(String),
}

/// Handle the pane holds onto a running hive: a receiver for updates and a
/// cooperative cancel flag (set on Esc).
pub struct HiveHandle {
    pub rx: Receiver<HiveUpdate>,
    pub cancel: Arc<AtomicBool>,
}

/// Plan + run a hive to completion, streaming updates over `tx`. Generic over
/// planner/factory so tests drive it with stubs (no network).
pub async fn run_hive(
    goal: String,
    planner: Arc<dyn Planner>,
    factory: Arc<dyn AgentFactory>,
    tx: Sender<HiveUpdate>,
    cancel: Arc<AtomicBool>,
) {
    let graph = match planner.plan(&goal).await {
        Ok(g) => g,
        Err(e) => {
            let _ = tx.send(HiveUpdate::Error(e.to_string()));
            return;
        }
    };
    let _ = tx.send(HiveUpdate::Planned(graph.clone()));

    let bus = EventBus::new(BUS_CAPACITY);
    let board = Blackboard::new();

    // Fold bus events into a Fleet snapshot, emitting Telemetry on each event.
    let mut sub = bus.subscribe();
    let tx_fold = tx.clone();
    let folder = tokio::spawn(async move {
        let mut fleet = Fleet::new();
        loop {
            match sub.recv().await {
                Ok(ev) => {
                    fleet.apply(&ev);
                    if tx_fold.send(HiveUpdate::Telemetry(fleet.clone())).is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    let sched =
        Scheduler::new(graph, board, bus.clone(), factory, CONCURRENCY).with_cancel(cancel);
    let outcome = sched.run().await;

    // Drop our bus handle so the folder's subscription closes and it returns.
    drop(bus);
    let _ = folder.await;
    let _ = tx.send(HiveUpdate::Finished(outcome));
    let _ = HiveEvent::AgentSpawned; // (no-op marker; remove if it lints)
}

/// Spawn a real hive run on a background thread with a fresh tokio runtime,
/// wired to the Anthropic provider. Returns immediately with a `HiveHandle`.
pub fn spawn_hive(goal: String) -> HiveHandle {
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_thread = Arc::clone(&cancel);
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(HiveUpdate::Error(format!("runtime: {e}")));
                return;
            }
        };
        rt.block_on(async move {
            let planner_provider = match AnthropicProvider::from_env() {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(HiveUpdate::Error(e.to_string()));
                    return;
                }
            };
            let agent_provider: Arc<dyn Provider> = match AnthropicProvider::from_env() {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    let _ = tx.send(HiveUpdate::Error(e.to_string()));
                    return;
                }
            };
            let planner: Arc<dyn Planner> = Arc::new(LlmPlanner {
                provider: planner_provider,
                tier: ModelTier::Standard,
            });
            let factory: Arc<dyn AgentFactory> =
                Arc::new(ApiFactory::new(agent_provider, ModelTier::Standard, MAX_TOKENS));
            run_hive(goal, planner, factory, tx, cancel_thread).await;
        });
    });
    HiveHandle { rx, cancel }
}

#[cfg(test)]
#[path = "hiverun_tests.rs"]
mod tests;
```

Note: delete the `let _ = HiveEvent::AgentSpawned;` marker line and the `HiveEvent` import if clippy flags them as unused — they are only there as a reminder that `HiveEvent` flows through the bus internally. (The bus/fold uses `HiveEvent` via `fleet.apply`, which takes `&HiveEvent`; if the import is unused after removing the marker, drop it.)

- [ ] **Step 5: Register the module**

In `crates/crew-app/src/main.rs`, add alongside the other `mod` declarations:

```rust
mod hiverun;
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p crew-app run_hive_emits_planned_then_finished`
Expected: PASS

- [ ] **Step 7: Size + clippy**

Run: `wc -l crates/crew-app/src/hiverun.rs`
Expected: ≤ 200. If over, split `spawn_hive` into `crates/crew-app/src/hiverun_spawn.rs` (`#[path]` submodule) and re-export.

Run: `cargo clippy -p crew-app --all-targets`
Expected: zero warnings. Remove the marker line / unused `HiveEvent` import if flagged.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-app/Cargo.toml crates/crew-app/src/hiverun.rs crates/crew-app/src/hiverun_tests.rs crates/crew-app/src/main.rs Cargo.lock
git commit -m "feat(app): hiverun — background tokio driver for the hive swarm

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: crew-app — `CrewPane` (state, poll, keys, render)

**Files:**
- Create: `crates/crew-app/src/crewpane/mod.rs`
- Create: `crates/crew-app/src/crewpane/state.rs`
- Create: `crates/crew-app/src/crewpane/keys.rs`
- Create: `crates/crew-app/src/crewpane/render.rs`
- Create: `crates/crew-app/src/crewpane/tests.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod crewpane;`)

**Interfaces:**
- Consumes: `crate::hiverun::{HiveHandle, HiveUpdate, spawn_hive}`, `crew_hive::{TaskGraph, Fleet, view::{fleet_view, render_cells}}`, `crew_render::CellView`, `winit::event::KeyEvent`.
- Produces:
  - `enum CrewStatus { Idle, Running, Done, Failed(String) }`
  - `struct CrewPane { pub goal: String, pub status: CrewStatus, pub graph: Option<TaskGraph>, pub fleet: Fleet, pub handle: Option<HiveHandle> }`
  - `CrewPane::new() -> CrewPane`
  - `CrewPane::poll(&mut self) -> bool` (true if any update applied)
  - `CrewPane::cells(&self, cols: u16, rows: u16) -> Vec<CellView>`
  - `CrewPane::on_key(&mut self, key: &KeyEvent)`

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/crewpane/tests.rs`:

```rust
use super::*;
use crate::hiverun::{HiveHandle, HiveUpdate};
use crew_hive::Fleet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

fn press(key: Key) -> KeyEvent {
    // Construct a minimal pressed KeyEvent. winit's KeyEvent is non_exhaustive
    // in some versions; if direct construction fails, gate these key tests
    // behind a small helper in the app's existing test utilities instead.
    KeyEvent {
        physical_key: winit::keyboard::PhysicalKey::Unidentified(
            winit::keyboard::NativeKeyCode::Unidentified,
        ),
        logical_key: key,
        text: None,
        location: winit::keyboard::KeyLocation::Standard,
        state: ElementState::Pressed,
        repeat: false,
        platform_specific: Default::default(),
    }
}

#[test]
fn idle_pane_renders_a_hint() {
    let pane = CrewPane::new();
    let cells = pane.cells(40, 10);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.to_lowercase().contains("goal"), "idle hint missing: {text:?}");
}

#[test]
fn typing_edits_the_goal() {
    let mut pane = CrewPane::new();
    pane.on_key(&press(Key::Character("h".into())));
    pane.on_key(&press(Key::Character("i".into())));
    assert_eq!(pane.goal, "hi");
    pane.on_key(&press(Key::Named(NamedKey::Backspace)));
    assert_eq!(pane.goal, "h");
}

#[test]
fn enter_on_empty_goal_stays_idle() {
    let mut pane = CrewPane::new();
    pane.on_key(&press(Key::Named(NamedKey::Enter)));
    assert!(matches!(pane.status, CrewStatus::Idle));
    assert!(pane.handle.is_none());
}

#[test]
fn poll_applies_updates_from_the_channel() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut pane = CrewPane::new();
    pane.handle = Some(HiveHandle {
        rx,
        cancel: Arc::new(AtomicBool::new(false)),
    });
    pane.status = CrewStatus::Running;

    let mut fleet = Fleet::new();
    fleet.apply(&crew_hive::HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(0),
        task: crew_hive::TaskId(0),
    });
    tx.send(HiveUpdate::Telemetry(fleet)).unwrap();

    let changed = pane.poll();
    assert!(changed);
    assert_eq!(pane.fleet.len(), 1);
}
```

If `KeyEvent` cannot be constructed directly (winit marks it `#[non_exhaustive]`), keep `idle_pane_renders_a_hint` and `poll_applies_updates_from_the_channel`, and replace the two key tests with a direct call to a key-decoding helper: refactor `on_key` to delegate to a pure `fn apply_key(&mut self, logical: &Key, pressed: bool)` and test that instead. Decide this when you see the compiler result in Step 2.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app crewpane`
Expected: FAIL — `crewpane` module not found. (If `KeyEvent` construction errors, apply the Step 1 fallback now.)

- [ ] **Step 3: Implement `state.rs`**

Create `crates/crew-app/src/crewpane/state.rs`:

```rust
//! Lifecycle state of a `/crew` swarm pane.

/// Where a `/crew` pane is in its lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub enum CrewStatus {
    /// No run yet; the goal bar is accepting input.
    Idle,
    /// A hive run is in flight.
    Running,
    /// The run finished (some tasks may have failed; see the fleet totals).
    Done,
    /// Setup/plan failed (e.g. missing API key); carries the message to show.
    Failed(String),
}
```

- [ ] **Step 4: Implement `render.rs`**

Create `crates/crew-app/src/crewpane/render.rs`:

```rust
//! Render a `CrewPane` snapshot to GPU cells: the swarm glyph grid fills the
//! top, a one-line status sits above a `> goal` input bar on the bottom row.
use crew_hive::view::{fleet_view, render_cells};
use crew_render::CellView;

use super::state::CrewStatus;
use super::CrewPane;

const BG: (u8, u8, u8) = (0, 0, 0);
const FG: (u8, u8, u8) = (210, 210, 220);

/// Lay `text` into cells at `row`, starting at column 0, clipped to `cols`.
fn put_str(out: &mut Vec<CellView>, row: u16, text: &str, cols: u16, fg: (u8, u8, u8)) {
    for (i, ch) in text.chars().enumerate() {
        if i as u16 >= cols {
            break;
        }
        out.push(CellView {
            col: i as u16,
            row,
            c: ch,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

/// Build the full cell list for a pane of `cols`×`rows`.
pub fn cells(pane: &CrewPane, cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols == 0 || rows == 0 {
        return out;
    }
    let goal_row = rows - 1;
    let status_row = rows.saturating_sub(2);
    let swarm_rows = rows.saturating_sub(2);

    // Swarm grid (or idle hint).
    match (&pane.graph, pane.fleet.is_empty()) {
        (Some(graph), false) => {
            let view = fleet_view(graph, &pane.fleet, cols as usize);
            for g in render_cells(&view, cols, swarm_rows) {
                out.push(CellView {
                    col: g.col,
                    row: g.row,
                    c: g.ch,
                    fg: (g.color.0, g.color.1, g.color.2),
                    bg: BG,
                    bold: false,
                    italic: false,
                });
            }
        }
        _ => {
            put_str(&mut out, 0, "Enter a goal for the swarm…", cols, FG);
        }
    }

    // Status line.
    let totals = pane.fleet.totals();
    let status = match &pane.status {
        CrewStatus::Idle => String::new(),
        CrewStatus::Running => format!("running · live {} done {}", totals.live, totals.done),
        CrewStatus::Done => format!("done {} · failed {}", totals.done, totals.failed),
        CrewStatus::Failed(e) => format!("error: {e}"),
    };
    if !status.is_empty() && status_row != goal_row {
        put_str(&mut out, status_row, &status, cols, (150, 150, 160));
    }

    // Goal bar.
    let bar = format!("> {}", pane.goal);
    put_str(&mut out, goal_row, &bar, cols, FG);
    out
}
```

- [ ] **Step 5: Implement `keys.rs`**

Create `crates/crew-app/src/crewpane/keys.rs`:

```rust
//! Key handling for a `/crew` pane: edit the goal, Enter launches the run,
//! Esc cancels a running swarm.
use std::sync::atomic::Ordering;

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::state::CrewStatus;
use super::CrewPane;

impl CrewPane {
    /// Handle one winit key event. Only acts on key presses.
    pub fn on_key(&mut self, key: &KeyEvent) {
        if !key.state.is_pressed() {
            return;
        }
        match &key.logical_key {
            Key::Named(NamedKey::Enter) => self.launch(),
            Key::Named(NamedKey::Escape) => self.cancel(),
            Key::Named(NamedKey::Backspace) => {
                self.goal.pop();
            }
            Key::Named(NamedKey::Space) => self.goal.push(' '),
            Key::Character(s) => {
                if let Some(c) = s.chars().next() {
                    self.goal.push(c);
                }
            }
            _ => {}
        }
    }

    /// Launch a hive run for the current goal (no-op if empty or already running).
    fn launch(&mut self) {
        if self.goal.trim().is_empty() || matches!(self.status, CrewStatus::Running) {
            return;
        }
        self.fleet = crew_hive::Fleet::new();
        self.graph = None;
        self.handle = Some(crate::hiverun::spawn_hive(self.goal.clone()));
        self.status = CrewStatus::Running;
    }

    /// Signal cooperative cancellation to a running swarm.
    fn cancel(&mut self) {
        if let Some(h) = &self.handle {
            h.cancel.store(true, Ordering::Relaxed);
        }
    }
}
```

- [ ] **Step 6: Implement `mod.rs`**

Create `crates/crew-app/src/crewpane/mod.rs`:

```rust
//! The `/crew` pane: a swarm-view-first interface. A goal typed on the bottom
//! bar launches a `crew-hive` run on a background thread (`crate::hiverun`);
//! the live `Fleet` snapshot renders as a glyph grid each frame.
mod keys;
mod render;
mod state;

pub use state::CrewStatus;

use crew_hive::{Fleet, TaskGraph};
use crew_render::CellView;

use crate::hiverun::{HiveHandle, HiveUpdate};

pub struct CrewPane {
    pub goal: String,
    pub status: CrewStatus,
    pub graph: Option<TaskGraph>,
    pub fleet: Fleet,
    pub handle: Option<HiveHandle>,
}

impl CrewPane {
    pub fn new() -> Self {
        Self {
            goal: String::new(),
            status: CrewStatus::Idle,
            graph: None,
            fleet: Fleet::new(),
            handle: None,
        }
    }

    /// Drain pending updates from the background hive. Returns true if anything
    /// changed (so the caller can request a redraw).
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let Some(handle) = &self.handle else {
            return false;
        };
        // Collect first to avoid borrowing `self.handle` while mutating fields.
        let updates: Vec<HiveUpdate> = handle.rx.try_iter().collect();
        for update in updates {
            changed = true;
            match update {
                HiveUpdate::Planned(g) => self.graph = Some(g),
                HiveUpdate::Telemetry(f) => self.fleet = f,
                HiveUpdate::Finished(_) => self.status = CrewStatus::Done,
                HiveUpdate::Error(e) => self.status = CrewStatus::Failed(e),
            }
        }
        changed
    }

    /// Render to GPU cells.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::cells(self, cols, rows)
    }
}

impl Default for CrewPane {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
```

- [ ] **Step 7: Register the module**

In `crates/crew-app/src/main.rs`, add:

```rust
mod crewpane;
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p crew-app crewpane`
Expected: PASS (all four tests, or the adjusted set per the Step 1 fallback).

- [ ] **Step 9: Size + clippy**

Run: `wc -l crates/crew-app/src/crewpane/*.rs`
Expected: every file ≤ 200.

Run: `cargo clippy -p crew-app --all-targets`
Expected: zero warnings.

- [ ] **Step 10: Commit**

```bash
git add crates/crew-app/src/crewpane/ crates/crew-app/src/main.rs
git commit -m "feat(app): CrewPane — swarm-view-first /crew pane

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Wire `CrewPane` into the app (pane / keys / poll / spawn)

**Files:**
- Modify: `crates/crew-app/src/pane.rs:21,67,50`
- Modify: `crates/crew-app/src/keys.rs:123`
- Modify: `crates/crew-app/src/poll.rs:38`
- Modify: `crates/crew-app/src/chatspawn.rs:18`
- Test: `crates/crew-app/src/pane.rs` (tests module)

**Interfaces:**
- Consumes: `crate::crewpane::CrewPane`.
- Produces: `/crew` slash command opens a focused `CrewPane`; `title_text()` returns `"crew"`.

- [ ] **Step 1: Write the failing test**

In `crates/crew-app/src/pane.rs`, add to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn crew_pane_title_is_crew() {
    let p = Pane {
        content: PaneContent::Crew(Box::new(crate::crewpane::CrewPane::new())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        activity: false,
        bell: false,
    };
    assert_eq!(p.title_text(), "crew");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app crew_pane_title_is_crew`
Expected: FAIL — no variant `Crew` on `PaneContent`.

- [ ] **Step 3: Add the `Crew` variant + dispatch arms in `pane.rs`**

In `crates/crew-app/src/pane.rs`:

Add the import near the top:

```rust
use crate::crewpane::CrewPane;
```

Add the variant to `PaneContent`:

```rust
pub enum PaneContent {
    Terminal(Box<TermPane>),
    Chat(ChatPane),
    Settings(SettingsPane),
    Far(FarPane),
    Crew(Box<CrewPane>),
}
```

Add the `title_text()` arm (in the `match &self.content` for titles):

```rust
            PaneContent::Crew(_) => "crew".into(),
```

Add the `cells()` arm:

```rust
            PaneContent::Crew(c) => c.cells(self.grid.cols, self.grid.rows),
```

- [ ] **Step 4: Route keys in `keys.rs`**

In `crates/crew-app/src/keys.rs`, in the focused-pane `match &mut pane.content` block (next to `PaneContent::Chat(c) => c.on_key(event),`), add:

```rust
                PaneContent::Crew(c) => c.on_key(event),
```

- [ ] **Step 5: Poll in `poll.rs`**

In `crates/crew-app/src/poll.rs`, change the per-pane `match &mut p.content`. Replace the combined Settings/Far arm so Crew is polled:

```rust
                PaneContent::Crew(c) => c.poll(),
                PaneContent::Settings(_) | PaneContent::Far(_) => false,
```

- [ ] **Step 6: Rewrite `spawn_crew_pane` (no subprocess)**

In `crates/crew-app/src/chatspawn.rs`, replace the `spawn_crew_pane` function with:

```rust
    /// Spawn the `/crew` pane: an in-process hive swarm view. Typing a goal on
    /// the bottom bar launches a `crew-hive` run (see `crate::hiverun`); the
    /// live fleet renders as a glyph grid. Named "crew" for its title bar.
    pub(crate) fn spawn_crew_pane(&mut self) {
        use crate::crewpane::CrewPane;
        use crate::pane::{Pane, PaneContent};
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::Crew(Box::new(CrewPane::new())),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: Some("crew".to_string()),
            activity: false,
            bell: false,
        });
        self.focus_new_pane();
    }
```

Then delete the now-unused `crew_broker_cmd` function from `chatspawn.rs`. Leave `spawn_chat_pane`, `spawn_plugin_pane`, `echo_plugin_cmd`, and `orchestrator_plugin_cmd` intact (still used by `chords.rs`). If removing `crew_broker_cmd` leaves `Plugin`/`PluginCommand` imports partly unused, keep only what `spawn_plugin_pane` still needs.

- [ ] **Step 7: Run test + full build**

Run: `cargo test -p crew-app crew_pane_title_is_crew`
Expected: PASS

Run: `cargo check --workspace`
Expected: clean (broker still present and compiling at this point — that's fine; removed in Task 5).

- [ ] **Step 8: Clippy**

Run: `cargo clippy -p crew-app --all-targets`
Expected: zero warnings. If `large_enum_variant` fires on `PaneContent`, the `Crew` variant is already boxed; box any other newly-large variant only if the lint names it.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src/pane.rs crates/crew-app/src/keys.rs crates/crew-app/src/poll.rs crates/crew-app/src/chatspawn.rs
git commit -m "feat(app): wire /crew to the in-process hive swarm pane

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Retire the broker

**Files:**
- Delete: `crates/crew-plugin/src/broker/` (entire directory)
- Delete: `crates/crew-plugin/src/bin/crew-broker-plugin.rs`
- Modify: `crates/crew-plugin/src/lib.rs:1,6-9`
- Modify: `crates/crew-app/src/main.rs:71-77`

**Interfaces:**
- Produces: a workspace with no broker code, no `--broker-plugin` flag, no `crew-broker-plugin` binary. Echo/orchestrator chat plugins and `ChatPane` remain fully functional.

- [ ] **Step 1: Delete broker source + binary**

```bash
git rm -r crates/crew-plugin/src/broker
git rm crates/crew-plugin/src/bin/crew-broker-plugin.rs
```

- [ ] **Step 2: Remove broker from `crew-plugin/src/lib.rs`**

Delete the `mod broker;` line and the entire `pub use broker::{ … };` block (the re-export of `known_adapters, parse_routing, run_broker_stdio, Adapter, Broker, CliAdapter, Envelope, Hop, HopKind, Normalize, Registry, Routing, RunStats`). The file should keep `mod echo; mod host; mod orchestrator; mod protocol;` and their re-exports.

Resulting `crates/crew-plugin/src/lib.rs`:

```rust
mod echo;
mod host;
mod orchestrator;
mod protocol;
pub use echo::respond;
pub use host::Plugin;
pub use orchestrator::plan;
pub use protocol::{PluginCommand, PluginEvent};
```

- [ ] **Step 3: Remove the `--broker-plugin` branch from `main.rs`**

In `crates/crew-app/src/main.rs`, delete the block:

```rust
    // When the `/crew` pane spawns this binary as its multi-agent broker (a
    // re-exec of `crew` with this flag), run the JSON-line broker loop and exit
    // before any GUI initialization. This means `/crew` works wherever `crew`
    // is installed without shipping a separate plugin binary.
    if std::env::args().skip(1).any(|a| a == "--broker-plugin") {
        return crew_plugin::run_broker_stdio();
    }
```

- [ ] **Step 4: Verify no dangling references**

Run: `grep -rn "broker\|run_broker_stdio\|known_adapters\|parse_routing\|crew-broker-plugin\|--broker-plugin" crates/ --include='*.rs' --include='*.toml'`
Expected: no matches (comments referencing the old design in `chatspawn.rs` should already be gone after Task 4; remove any stragglers this surfaces).

- [ ] **Step 5: Full workspace build + tests + clippy**

Run: `cargo fmt --all`
Run: `cargo check --workspace`
Expected: clean.

Run: `cargo clippy --workspace --all-targets`
Expected: zero warnings across the whole workspace.

Run: `cargo test --workspace`
Expected: all green (broker tests are gone; echo/orchestrator/chat tests still pass).

- [ ] **Step 6: Confirm 200-line cap holds**

Run: `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 200 {print}'`
Expected: no output (no file over 200 lines).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: retire the multi-agent broker (superseded by the hive)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/CREW.md`

**Interfaces:** none (docs only).

- [ ] **Step 1: Update `/crew` description**

In `README.md` and `docs/CREW.md`, find the existing `/crew` / broker description and update it (add, don't rewrite surrounding content, matching existing style):

- `/crew` opens an **in-process hive swarm** pane: type a goal on the bottom bar; the hive decomposes it into a task DAG and runs a bounded pool of native-API agents, rendering the live swarm as a constellation/heatmap glyph grid.
- Keys in the `/crew` pane: type to edit the goal, **Enter** launches the swarm, **Esc** cancels a running swarm.
- Provider: set `ANTHROPIC_API_KEY` (bring-your-own-provider; default model tier Sonnet). With no key, the pane shows an error status instead of running.
- Remove any documentation of the broker / `--broker-plugin` / claude⇆codex⇆opencode peer chat.

- [ ] **Step 2: Verify build of doc-embedded checks (if any) + fmt**

Run: `cargo fmt --all && cargo check --workspace`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/CREW.md
git commit -m "docs: /crew now opens the in-process hive swarm

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Final Verification (gate)

- [ ] `cargo fmt --all -- --check` — clean
- [ ] `cargo check --workspace` — clean
- [ ] `cargo clippy --workspace --all-targets` — zero warnings
- [ ] `cargo test --workspace` — all green
- [ ] `find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 200 {print}'` — no output
- [ ] `grep -rn "broker" crates/ --include='*.rs'` — no matches
- [ ] Manual smoke (optional, needs `ANTHROPIC_API_KEY`): run `crew`, type `/crew`, enter a goal, watch the swarm grid animate; press Esc to cancel.

---

## Self-Review Notes

- **Spec coverage:** swarm-view-first pane (Task 3 render), goal bar bottom row (Task 3 render), in-process tokio thread (Task 2), native `ApiAgent` backend (Task 1 `ApiFactory` + Task 2 wiring), Esc cancel (Task 3 keys → scheduler `with_cancel` in Task 2), broker removal incl. `--broker-plugin`/binary/re-exports (Task 5), provider-from-env with no-key message (Task 2 `spawn_hive` → `HiveUpdate::Error` → `CrewStatus::Failed`), tests in both crates (Tasks 1–4), docs (Task 6). All spec sections map to a task.
- **Type consistency:** `HiveUpdate`/`HiveHandle`/`run_hive`/`spawn_hive` signatures are identical across Tasks 2–3; `CrewPane` field names (`goal`/`status`/`graph`/`fleet`/`handle`) match between `mod.rs`, `keys.rs`, `render.rs`, and tests; `CrewStatus` variants (`Idle`/`Running`/`Done`/`Failed(String)`) are consistent.
- **Known winit risk:** direct `KeyEvent` construction in tests may fail if the type is `#[non_exhaustive]`; Task 3 Step 1 carries the fallback (extract a pure `apply_key` and test that).
