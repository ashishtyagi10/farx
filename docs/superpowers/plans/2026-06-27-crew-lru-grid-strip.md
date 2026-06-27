# LRU Grid + Minimized Strip Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cap Crew's auto-tiling grid at a small number of full tiles and demote the least-recently-active panes into a minimized thumbnail strip at the bottom — the prerequisite for decoupling "running" from "visible" (Phase 0 of the swarm goal, `docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`).

**Architecture:** A new pure `grid` module in `crew-app` holds an LRU of pane **indices** (`GridLayout`) and a placement function (`compose_grid`) that grids the most-recently-active tiles in the main content area and lays the rest across a reserved bottom strip. `CrewApp` keeps one `GridLayout` and maintains it at exactly **two** sites — `build_frame` (reconcile newly-spawned panes + `touch` the focused pane each frame) and `close_pane` (the sole pane-removal site, with index-shift fixup). Rendering switches from "grid every pane equally" to "grid the full set, draw the rest as thumbnails." Everything is additive to existing modules; no file crosses 200 lines.

**Tech Stack:** Rust, the existing `crew-app` types (`layout::Rect` with `f32` fields, `pane_rects_at`, `chrome::content_rect`, `panecard::push_card`, `paneview::build_scenes`), `cargo test`.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total (imports, whitespace, doc comments included). Split into submodules before crossing it.
- **Auto-tiling grid only** — `cols = ceil(sqrt(n))`, near-square, row-major. No layout-switching system.
- **Panels are fieldset cards** — a rounded border with a legend on the top edge (`panecard`/`titled_card`), never a filled title bar.
- **Pass-through keys** — do not add chords or steal keys in this plan.
- **No new dependencies.**
- The LRU is keyed by the **pane's `Vec<Pane>` index** (same id space as `self.focused`, `self.panes[i]`). `close_pane` is the only site that removes a pane and therefore shifts indices.
- Full-tile cap: `MAX_FULL_TILES = 6`. Tiles beyond the 6 most-recently-active are minimized.

---

### Task 1: LRU state — `GridLayout`

A pure structure tracking pane indices in most-recently-active-first order, with an index-shift fixup for pane removal.

**Files:**
- Create: `crates/crew-app/src/grid/mod.rs`
- Create: `crates/crew-app/src/grid/state.rs`
- Create: `crates/crew-app/src/grid/tests.rs`
- Modify: `crates/crew-app/src/lib.rs` (add `mod grid;` and re-export)

**Interfaces:**
- Produces:
  - `pub const MAX_FULL_TILES: usize = 6;`
  - `pub struct GridLayout` with `pub fn new() -> Self`, `Default`.
  - `pub fn add(&mut self, idx: usize)` — insert as most-recently-active (front); if present, move to front (no duplicate).
  - `pub fn touch(&mut self, idx: usize)` — move an existing `idx` to the front; no-op if absent.
  - `pub fn on_close(&mut self, idx: usize)` — remove `idx`, then decrement every stored index greater than `idx` (keeps the LRU consistent with `Vec::remove` shifting later panes down).
  - `pub fn full(&self) -> &[usize]` — the up-to-`MAX_FULL_TILES` most-recently-active indices.
  - `pub fn minimized(&self) -> &[usize]` — the remaining (least-recently-active) indices.
  - `pub fn len(&self) -> usize`, `pub fn is_empty(&self) -> bool`.

- [ ] **Step 1: Find where `lib.rs` declares its modules**

Run: `grep -n "^mod \|^pub mod \|^pub use " crates/crew-app/src/lib.rs | head -40`
Expected: a list of `mod <name>;` lines (e.g. `mod chrome;`, `mod layout;`, `mod pane;`, …). Note the alphabetical neighbors of `grid` (between `farpane`/`hit` etc.) so the new line matches the file's ordering.

- [ ] **Step 2: Create the module skeleton and wire it into the crate**

Create `crates/crew-app/src/grid/mod.rs`:

```rust
//! Agent grid LRU: tracks pane indices in most-recently-active order, caps the
//! number of full tiles, and demotes the rest to a minimized strip. Pure and
//! UI-independent; `build_frame` consumes it to place panes. See
//! `compute`/`compose_grid` for turning this state into pixel rects.

mod state;

#[cfg(test)]
mod tests;

pub use state::{GridLayout, MAX_FULL_TILES};
```

Modify `crates/crew-app/src/lib.rs` — add `mod grid;` in alphabetical position with the other `mod` lines. (It is consumed only inside the crate, so a `mod grid;` declaration is enough; no `pub use` needed unless other modules import `crate::grid::…`, which they will — keep it `mod grid;` and import via `crate::grid::{…}`.)

- [ ] **Step 3: Write the failing tests**

Create `crates/crew-app/src/grid/tests.rs`:

```rust
use super::*;

#[test]
fn add_orders_most_recent_first() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    assert_eq!(g.full(), &[2, 1, 0]);
    assert!(g.minimized().is_empty());
    assert_eq!(g.len(), 3);
}

#[test]
fn add_is_idempotent_and_promotes() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(0); // re-add existing -> front, no duplicate
    assert_eq!(g.full(), &[0, 1]);
    assert_eq!(g.len(), 2);
}

#[test]
fn seventh_tile_minimizes_least_recent() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    assert_eq!(g.full(), &[6, 5, 4, 3, 2, 1]);
    assert_eq!(g.minimized(), &[0]);
}

#[test]
fn touch_promotes_into_full_set() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    g.touch(0); // 0 was minimized; promote it, demote current LRU (1)
    assert_eq!(g.full()[0], 0);
    assert_eq!(g.minimized(), &[1]);
}

#[test]
fn touch_absent_is_noop() {
    let mut g = GridLayout::new();
    g.add(0);
    g.touch(99);
    assert_eq!(g.full(), &[0]);
}

#[test]
fn on_close_removes_and_shifts_indices() {
    // Panes [0,1,2,3] added; close index 1. After Vec::remove(1), old panes
    // 2,3 become indices 1,2. The LRU must reflect that.
    let mut g = GridLayout::new();
    for idx in 0..4 {
        g.add(idx); // order front->back: 3,2,1,0
    }
    g.on_close(1);
    // 1 is gone; 2->1 and 3->2. Order was [3,2,1,0] -> drop 1 -> [3,2,0]
    // -> shift (>1 decremented): 3->2, 2->1, 0 stays -> [2,1,0].
    assert_eq!(g.full(), &[2, 1, 0]);
    assert_eq!(g.len(), 3);
}

#[test]
fn empty_state() {
    let g = GridLayout::new();
    assert!(g.is_empty());
    assert_eq!(g.len(), 0);
    assert!(g.full().is_empty());
    assert!(g.minimized().is_empty());
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p crew-app grid::`
Expected: FAIL — `GridLayout` / `MAX_FULL_TILES` not found (compile error).

- [ ] **Step 5: Implement `GridLayout`**

Create `crates/crew-app/src/grid/state.rs`:

```rust
/// Maximum number of panes shown at full size; the rest are minimized.
pub const MAX_FULL_TILES: usize = 6;

/// Tracks pane indices in most-recently-active-first order. The first
/// `MAX_FULL_TILES` are full tiles; the remainder are minimized (LRU).
#[derive(Debug, Clone, Default)]
pub struct GridLayout {
    /// Pane indices, most-recently-active first.
    order: Vec<usize>,
}

impl GridLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `idx` as the most-recently-active pane. If present, moves it to
    /// the front rather than duplicating.
    pub fn add(&mut self, idx: usize) {
        self.order.retain(|x| *x != idx);
        self.order.insert(0, idx);
    }

    /// Move an existing `idx` to the front. No-op if `idx` is absent.
    pub fn touch(&mut self, idx: usize) {
        if let Some(pos) = self.order.iter().position(|x| *x == idx) {
            let v = self.order.remove(pos);
            self.order.insert(0, v);
        }
    }

    /// Remove `idx`, then shift every stored index above it down by one to
    /// match `Vec::remove` reindexing the panes after a close.
    pub fn on_close(&mut self, idx: usize) {
        self.order.retain(|x| *x != idx);
        for x in &mut self.order {
            if *x > idx {
                *x -= 1;
            }
        }
    }

    fn split(&self) -> usize {
        self.order.len().min(MAX_FULL_TILES)
    }

    /// Indices shown full (most-recently-active, up to the cap).
    pub fn full(&self) -> &[usize] {
        &self.order[..self.split()]
    }

    /// Indices minimized (least-recently-active beyond the cap).
    pub fn minimized(&self) -> &[usize] {
        &self.order[self.split()..]
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p crew-app grid::`
Expected: PASS — all 7 tests green.

- [ ] **Step 7: Format, lint, commit**

Run: `cargo fmt && cargo clippy -p crew-app --all-targets`
Expected: warning-free.

```bash
git add crates/crew-app/src/grid/mod.rs crates/crew-app/src/grid/state.rs crates/crew-app/src/grid/tests.rs crates/crew-app/src/lib.rs
git commit -m "feat(crew): GridLayout LRU over pane indices (6 full, rest minimized)"
```

---

### Task 2: Placement — `compose_grid`

Turn a `GridLayout` + content rect into concrete pixel rects: full tiles gridded in the main region, minimized tiles laid left-to-right across a reserved bottom strip. Reuses the existing `pane_rects_at` packing.

**Files:**
- Create: `crates/crew-app/src/grid/compose.rs`
- Modify: `crates/crew-app/src/grid/mod.rs` (declare `mod compose;` and re-export)
- Modify: `crates/crew-app/src/grid/tests.rs` (append compose tests)

**Interfaces:**
- Consumes: `GridLayout`, `MAX_FULL_TILES` (Task 1); `crate::layout::{Rect, pane_rects_at}`.
- Produces:
  - `pub const MINIMIZED_STRIP_ROWS: f32 = 4.0;`
  - `pub struct GridRects { pub full: Vec<(usize, Rect)>, pub minimized: Vec<(usize, Rect)> }`
  - `pub fn compose_grid(content: Rect, layout: &GridLayout, cell_h: f32, gap: f32) -> GridRects` — when `layout.minimized()` is non-empty, reserves a bottom strip `MINIMIZED_STRIP_ROWS * cell_h + 2*gap` tall (clamped to `content.h`); grids `layout.full()` into the region above it (or all of `content` when nothing is minimized) via `pane_rects_at`; lays the minimized indices evenly across one row of the strip. **Both the `full` and `minimized` vecs are sorted by pane index** so tiles keep stable positions (focusing a pane never moves a tile — the LRU only decides full-vs-minimized *membership*, not display order). Empty layout → both vecs empty.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/grid/tests.rs`:

```rust
use crate::layout::Rect;

fn content() -> Rect {
    Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }
}

#[test]
fn compose_empty_is_empty() {
    let g = GridLayout::new();
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert!(out.full.is_empty());
    assert!(out.minimized.is_empty());
}

#[test]
fn compose_no_minimized_uses_full_height() {
    let mut g = GridLayout::new();
    g.add(0);
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert_eq!(out.full.len(), 1);
    assert!(out.minimized.is_empty());
    assert_eq!(out.full[0].0, 0); // index preserved
    // One tile, no strip: height is content minus the grid gap insets only.
    assert!(out.full[0].1.h > 560.0, "tile should use ~full height");
}

#[test]
fn compose_reserves_strip_when_minimized_present() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert_eq!(out.full.len(), 6);
    assert_eq!(out.minimized.len(), 1);
    // The full grid sits entirely above the strip.
    let full_bottom = out.full.iter().map(|(_, r)| r.y + r.h).fold(0.0_f32, f32::max);
    let strip_top = out.minimized[0].1.y;
    assert!(full_bottom <= strip_top + 0.5, "full grid overlaps strip");
    // Strip sits at the bottom of the content area.
    let strip_bottom = out.minimized[0].1.y + out.minimized[0].1.h;
    assert!(strip_bottom <= 600.0 + 0.5);
    assert!(out.minimized[0].1.y > 600.0 - (4.0 * 16.0 + 2.0 * 8.0) - 1.0);
}

#[test]
fn compose_full_indices_sorted_stable() {
    // LRU order is 2,1,0 (most-recent first), but display order is stable by
    // pane index so focusing never moves a tile.
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    let out = compose_grid(content(), &g, 16.0, 8.0);
    let ids: Vec<usize> = out.full.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![0, 1, 2]);
}

#[test]
fn compose_minimized_indices_sorted_stable() {
    // Even though touch() reorders the LRU, the minimized strip stays in
    // stable index order so thumbnails don't jump.
    let mut g = GridLayout::new();
    for idx in 0..8 {
        g.add(idx); // LRU front->back: 7..0; full = 7,6,5,4,3,2 ; min = 1,0
    }
    let out = compose_grid(content(), &g, 16.0, 8.0);
    let ids: Vec<usize> = out.minimized.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![0, 1]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app grid::`
Expected: FAIL — `compose_grid` / `GridRects` / `MINIMIZED_STRIP_ROWS` not found.

- [ ] **Step 3: Implement `compose_grid`**

Create `crates/crew-app/src/grid/compose.rs`:

```rust
use crate::grid::state::{GridLayout, MAX_FULL_TILES};
use crate::layout::{pane_rects_at, Rect};

/// Cell rows reserved for the minimized thumbnail strip when any pane is
/// minimized.
pub const MINIMIZED_STRIP_ROWS: f32 = 4.0;

/// Concrete placement of every tile for one frame.
#[derive(Debug, Clone, Default)]
pub struct GridRects {
    /// Full-size tiles: `(pane_index, rect)`, sorted by pane index (stable
    /// positions — the LRU decides membership, not display order).
    pub full: Vec<(usize, Rect)>,
    /// Minimized thumbnails: `(pane_index, rect)`, sorted by pane index,
    /// left-to-right.
    pub minimized: Vec<(usize, Rect)>,
}

/// Place a `GridLayout` into `content`: grid the full tiles in the main region
/// and — when there are minimized tiles — reserve a bottom strip and lay them
/// out evenly across one row. Both sets render in stable pane-index order, so
/// focusing a pane changes which tiles are full but never reorders them.
pub fn compose_grid(content: Rect, layout: &GridLayout, cell_h: f32, gap: f32) -> GridRects {
    let mut full_ids = layout.full().to_vec();
    let mut min_ids = layout.minimized().to_vec();
    if full_ids.is_empty() && min_ids.is_empty() {
        return GridRects::default();
    }
    debug_assert!(full_ids.len() <= MAX_FULL_TILES);
    // Stable display order regardless of LRU recency.
    full_ids.sort_unstable();
    min_ids.sort_unstable();

    let strip_h = if min_ids.is_empty() {
        0.0
    } else {
        (MINIMIZED_STRIP_ROWS * cell_h + 2.0 * gap).min(content.h)
    };
    let grid_h = (content.h - strip_h).max(0.0);

    let full_rects = pane_rects_at(full_ids.len(), content.x, content.y, content.w, grid_h, gap);
    let full = full_ids.into_iter().zip(full_rects).collect();

    let minimized = if min_ids.is_empty() {
        Vec::new()
    } else {
        let strip_y = content.y + grid_h;
        strip_row(&min_ids, content.x, strip_y, content.w, strip_h, gap)
    };

    GridRects { full, minimized }
}

/// Lay `ids` left-to-right as equal-width tiles across one strip row.
fn strip_row(ids: &[usize], x: f32, y: f32, w: f32, h: f32, gap: f32) -> Vec<(usize, Rect)> {
    let n = ids.len();
    let tile_w = w / n as f32;
    ids.iter()
        .copied()
        .enumerate()
        .map(|(i, idx)| {
            (
                idx,
                Rect {
                    x: x + i as f32 * tile_w + gap,
                    y: y + gap,
                    w: tile_w - 2.0 * gap,
                    h: h - 2.0 * gap,
                },
            )
        })
        .collect()
}
```

Modify `crates/crew-app/src/grid/mod.rs`:

```rust
//! Agent grid LRU: tracks pane indices in most-recently-active order, caps the
//! number of full tiles, and demotes the rest to a minimized strip. Pure and
//! UI-independent; `build_frame` consumes it to place panes.

mod compose;
mod state;

#[cfg(test)]
mod tests;

pub use compose::{compose_grid, GridRects, MINIMIZED_STRIP_ROWS};
pub use state::{GridLayout, MAX_FULL_TILES};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-app grid::`
Expected: PASS — compose tests green.

- [ ] **Step 5: Format, lint, commit**

Run: `cargo fmt && cargo clippy -p crew-app --all-targets`
Expected: warning-free.

```bash
git add crates/crew-app/src/grid/compose.rs crates/crew-app/src/grid/mod.rs crates/crew-app/src/grid/tests.rs
git commit -m "feat(crew): compose_grid — full grid + minimized strip placement"
```

---

### Task 3: Maintain the LRU in `CrewApp` (no visual change)

Add the `GridLayout` field and keep it in sync at the two sites that matter: reconcile + `touch` once per frame in `build_frame`, and `on_close` in `close_pane`. Rendering is untouched in this task, so the app looks identical — but the LRU is now correct and observable.

**Files:**
- Modify: `crates/crew-app/src/app.rs` (add field; call `on_close` in `close_pane`; add a `reconcile_grid` helper + test)
- Modify: `crates/crew-app/src/render.rs` (call `reconcile_grid` at the top of `build_frame`)

**Interfaces:**
- Consumes: `crate::grid::GridLayout` (Task 1).
- Produces: `CrewApp.grid: GridLayout`; `pub(crate) fn reconcile_grid(&mut self)` — adds any pane index `0..panes.len()` missing from the LRU (newly spawned panes), drops any index `>= panes.len()`, then `touch`es `self.focused` when there are panes.

- [ ] **Step 1: Add the field to `CrewApp`**

In `crates/crew-app/src/app.rs`, add the import near the other `crate::` imports:

```rust
use crate::grid::GridLayout;
```

Add the field to the `CrewApp` struct (it derives `Default`, and `GridLayout: Default`, so no constructor change is needed). Place it after `focused`:

```rust
    pub(crate) focused: usize,
    /// LRU of pane indices: which panes are full tiles vs. minimized.
    pub(crate) grid: GridLayout,
```

- [ ] **Step 2: Call `on_close` in `close_pane`**

In `close_pane` (`app.rs`), update the body so the LRU is fixed up whenever a pane is actually removed:

```rust
    pub fn close_pane(&mut self, idx: usize) -> bool {
        if idx < self.panes.len() {
            self.panes.remove(idx);
            self.grid.on_close(idx);
        }
        // Closing a pane returns to the grid; never linger zoomed on it.
        self.zoomed = false;
        if self.panes.is_empty() {
            self.focused = 0;
            self.input.focused = true;
            self.broadcast = false;
            self.input.broadcast = false;
            return false;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        false
    }
```

- [ ] **Step 3: Write the failing test for `reconcile_grid`**

Add to the `#[cfg(test)] mod tests` block in `app.rs` (create one if absent, mirroring the existing test style in `panemanage.rs`). This test drives the helper directly with a hand-built app:

```rust
    #[test]
    fn reconcile_grid_tracks_panes_and_focus() {
        let mut app = CrewApp::default();
        // Simulate two spawned panes by pushing Far panes (no PTY needed).
        app.panes.push(crate::panemanage::tests_far_pane("a"));
        app.panes.push(crate::panemanage::tests_far_pane("b"));
        app.focused = 1;
        app.reconcile_grid();
        // Both panes tracked; focused (1) is most-recently-active.
        assert_eq!(app.grid.len(), 2);
        assert_eq!(app.grid.full()[0], 1);

        // Close pane 0; reconcile must not resurrect a stale index.
        app.close_pane(0);
        app.reconcile_grid();
        assert_eq!(app.grid.len(), 1);
        assert_eq!(app.grid.full(), &[0]);
    }
```

If `panemanage.rs` already has a private `far_pane` test helper (it does — used by its own tests), expose it to `app.rs`'s tests by adding, in `panemanage.rs` inside its `#[cfg(test)] mod tests`, a `pub(crate)` re-export, OR simply duplicate a 3-line `tests_far_pane` constructor in `app.rs`'s test module:

```rust
    fn tests_far_pane(name: &str) -> crate::pane::Pane {
        use crate::pane::{Pane, PaneContent};
        use crew_term::GridSize;
        Pane {
            content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: crate::layout::Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            label: Some(name.into()),
            name: None,
            activity: false,
            bell: false,
        }
    }
```

(Use the local `tests_far_pane` in the test above instead of `crate::panemanage::tests_far_pane` to avoid touching `panemanage.rs`.)

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p crew-app reconcile_grid`
Expected: FAIL — `reconcile_grid` not found.

- [ ] **Step 5: Implement `reconcile_grid`**

Add to `impl CrewApp` in `app.rs` (near `focus_new_pane`):

```rust
    /// Keep the grid LRU in step with `self.panes` and the current focus. Adds
    /// any pane index not yet tracked (newly spawned), drops any index past the
    /// end, and marks the focused pane most-recently-active. Called once per
    /// frame from `build_frame`.
    pub(crate) fn reconcile_grid(&mut self) {
        let n = self.panes.len();
        for idx in 0..n {
            if !self.grid.full().contains(&idx) && !self.grid.minimized().contains(&idx) {
                self.grid.add(idx);
            }
        }
        // Drop any stale indices at/after the end (defensive; close_pane already
        // fixes the common case via on_close).
        while self.grid.len() > n {
            self.grid.on_close(n);
        }
        if n > 0 {
            self.grid.touch(self.focused.min(n - 1));
        }
    }
```

- [ ] **Step 6: Call it from `build_frame`**

In `crates/crew-app/src/render.rs`, at the very top of `build_frame` (right after the `frame_geometry` guard returns early), add the reconcile call:

```rust
    pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> {
        let Some((cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return Vec::new();
        };
        self.reconcile_grid();
        let ih = chrome::input_h(ch);
```

- [ ] **Step 7: Run tests + full build to verify nothing regressed**

Run: `cargo test -p crew-app && cargo clippy --workspace --all-targets`
Expected: PASS, warning-free. The app renders exactly as before (rendering still uses the old path); only the LRU is now maintained.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-app/src/app.rs crates/crew-app/src/render.rs
git commit -m "feat(crew): maintain grid LRU (reconcile per frame, fixup on close)"
```

---

### Task 4: Render the full set via `compose_grid`

Switch the non-zoom render path from "grid every pane" to "grid the full set" (capped at `MAX_FULL_TILES`, rendered in stable pane-index order). Over-cap panes drop out of the main grid but remain reachable (sidebar, `Cmd+number`, `[`/`]`); focusing one promotes it back into the full set next frame. The minimized strip is added in Task 5.

**Files:**
- Modify: `crates/crew-app/src/paneview.rs` (add an index-driven scene builder; keep `build_scenes` working)
- Modify: `crates/crew-app/src/render.rs` (use `compose_grid` for the non-zoom branch)
- Modify: `crates/crew-app/src/pane.rs` (add `relayout_one` for a single pane)

**Interfaces:**
- Consumes: `crate::grid::{compose_grid, GridRects}`; `chrome::content_rect`.
- Produces:
  - `pub fn relayout_one(pane: &mut Pane, rect: Rect, cell_w: f32, cell_h: f32)` in `pane.rs` — the single-pane form of `relayout`.
  - `pub fn full_scenes(panes: &[Pane], placed: &[(usize, crate::layout::Rect)], focused: Option<usize>, broadcast: bool, find: Option<&str>, cw: f32, ch: f32) -> Vec<PaneScene>` in `paneview.rs` — renders the panes named by `placed` (pane index → rect), numbering tiles `1..` in placement order, marking the tile whose pane index equals `focused`.

- [ ] **Step 1: Add `relayout_one` (refactor `relayout` to use it)**

In `crates/crew-app/src/pane.rs`, extract the per-pane body of `relayout` into a reusable function and have `relayout` call it:

```rust
/// Assign one pane's pixel rect and resize its PTY (Terminal only) when the
/// derived grid changes. Reserves a one-cell border ring (fieldset card).
pub fn relayout_one(pane: &mut Pane, rect: Rect, cell_w: f32, cell_h: f32) {
    pane.rect = rect;
    let cols = ((rect.w / cell_w).floor() as u16).saturating_sub(2).max(1);
    let rows = ((rect.h / cell_h).floor() as u16).saturating_sub(2).max(1);
    if cols != pane.grid.cols || rows != pane.grid.rows {
        let new_grid = GridSize { cols, rows };
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.pty.resize(new_grid);
        }
        pane.grid = new_grid;
    }
}

/// Assign pixel rects to panes (zipped in order). Thin wrapper over `relayout_one`.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        relayout_one(pane, rect, cell_w, cell_h);
    }
}
```

(The existing `relayout` tests, if any, still pass — behavior is unchanged.)

- [ ] **Step 2: Add `full_scenes` to `paneview.rs`**

`build_scenes` already renders one pane into two `PaneScene`s. Refactor its loop body into a private helper, then add `full_scenes`. Append to `crates/crew-app/src/paneview.rs` (and have `build_scenes` call the helper to stay DRY):

```rust
/// Render the panes named by `placed` (`(pane_index, rect)`), numbering tiles
/// `1..` in placement order. `focused` is the *pane index* of the focused pane.
pub fn full_scenes(
    panes: &[Pane],
    placed: &[(usize, crate::layout::Rect)],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    cw: f32,
    ch: f32,
) -> Vec<PaneScene> {
    let multi = placed.len() > 1;
    let mut scenes = Vec::with_capacity(placed.len() * 2);
    for (slot, &(idx, _rect)) in placed.iter().enumerate() {
        let p = &panes[idx];
        let foc = focused == Some(idx);
        push_pane_scenes(&mut scenes, p, multi.then_some(slot + 1), foc, broadcast, find, cw, ch);
    }
    scenes
}
```

Extract this private helper from the current `build_scenes` body (it is the existing per-pane code, parameterized by the displayed index and using `p.rect`):

```rust
#[allow(clippy::too_many_arguments)]
fn push_pane_scenes(
    scenes: &mut Vec<PaneScene>,
    p: &Pane,
    index: Option<usize>,
    foc: bool,
    broadcast: bool,
    find: Option<&str>,
    cw: f32,
    ch: f32,
) {
    let mut cells = p.cells(foc);
    let is_term = matches!(&p.content, PaneContent::Terminal(_));
    let scroll = match &p.content {
        PaneContent::Terminal(t) => t.pty.display_offset(),
        _ => 0,
    };
    if is_term {
        crate::linkhl::colorize(&mut cells, p.grid.cols, p.grid.rows);
    }
    if foc && is_term && scroll > 0 {
        if let Some(term) = find {
            crate::findhl::highlight(&mut cells, term, p.grid.cols, p.grid.rows);
        }
    }
    let r = p.rect;
    scenes.push(PaneScene {
        cells,
        x: r.x + cw,
        y: r.y + ch,
        w: (r.w - 2.0 * cw).max(0.0),
        h: (r.h - 2.0 * ch).max(0.0),
        focused: foc,
        bordered: false,
        overlay: false,
    });
    let title = p.title_text();
    scenes.push(PaneScene {
        cells: pane_card(
            p.grid.cols,
            p.grid.rows,
            &Bar {
                index,
                title: &title,
                focused: foc,
                scroll,
                activity: p.activity && !foc,
                bell: p.bell && !foc,
                broadcast: broadcast && is_term,
            },
        ),
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        focused: foc,
        bordered: false,
        overlay: false,
    });
}
```

Then replace the body of `build_scenes`'s loop with a call to `push_pane_scenes` (passing `multi.then_some(i + 1)` and `focused == Some(i)`), leaving `build_scenes`'s public signature unchanged so the zoom branch and any other caller keep working.

- [ ] **Step 3: Use `compose_grid` in `build_frame`'s non-zoom branch**

In `crates/crew-app/src/render.rs`, add to the imports:

```rust
use crate::grid::compose_grid;
use crate::pane::relayout_one;
use crate::paneview::full_scenes;
```

Replace the `else` branch (the non-zoom path) of the `scenes` assignment in `build_frame`:

```rust
        } else {
            let content =
                chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
            let placed = compose_grid(content, &self.grid, ch, GAP);
            for &(idx, rect) in &placed.full {
                relayout_one(&mut self.panes[idx], rect, cw, ch);
            }
            let f = (!self.input.focused).then_some(self.focused);
            full_scenes(&self.panes, &placed.full, f, self.broadcast, self.last_find.as_deref(), cw, ch)
        };
```

(The `self.grid_rects()` helper and the old `relayout(&mut self.panes, …)` call in this branch are no longer used here. Leave `grid_rects()` in place only if other code still calls it — check with `grep -n "grid_rects" crates/crew-app/src`; if nothing else uses it, delete the method to avoid dead code, which clippy will flag.)

- [ ] **Step 4: Verify build + tests**

Run: `cargo fmt && cargo check -p crew-app && cargo test -p crew-app && cargo clippy --workspace --all-targets`
Expected: compiles, tests pass, warning-free (remove `grid_rects` if it became dead).

- [ ] **Step 5: Manual smoke test**

Run: `cargo run -p crew-app` (or the project's run command — check `README.md`). Spawn 7+ panes (`Cmd+T` repeatedly). Expected: at most 6 full tiles; the 7th pushes the least-recently-focused pane out of the grid. Focusing a hidden pane via the sidebar or `Cmd+<number>` brings it back into the full set. No minimized strip yet (Task 5).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/paneview.rs crates/crew-app/src/render.rs crates/crew-app/src/pane.rs
git commit -m "feat(crew): render full set via compose_grid (cap at MAX_FULL_TILES)"
```

---

### Task 5: Render the minimized thumbnail strip

Draw each minimized pane as a small fieldset card in the reserved bottom strip — title + activity/bell glyph — so over-cap panes stay visible at a glance and clickable.

**Files:**
- Create: `crates/crew-app/src/minstrip.rs` (thumbnail card builder)
- Modify: `crates/crew-app/src/render.rs` (push strip scenes after the full grid)
- Modify: `crates/crew-app/src/lib.rs` (declare `mod minstrip;`)
- Modify: `crates/crew-app/src/hit.rs` (click on a thumbnail focuses that pane — see Step 4)

**Interfaces:**
- Consumes: `GridRects.minimized` (Task 2); `panecard::push_card`; `Pane::title_text`.
- Produces: `pub fn push_min_strip(scenes: &mut Vec<PaneScene>, panes: &[Pane], placed: &[(usize, Rect)], cw: f32, ch: f32)` — pushes one fieldset card per minimized pane showing its title and an activity dot.

- [ ] **Step 1: Create the thumbnail builder**

Create `crates/crew-app/src/minstrip.rs`:

```rust
//! Minimized pane thumbnails: the bottom strip of fieldset cards for panes
//! demoted out of the full grid (LRU). Each card shows the pane title and an
//! activity dot — enough to track a pane at a glance and click to restore it.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;
use crate::pane::Pane;
use crate::panecard::{push_card, ACTIVITY};

/// Push one fieldset card per minimized pane into `scenes`.
pub fn push_min_strip(
    scenes: &mut Vec<PaneScene>,
    panes: &[Pane],
    placed: &[(usize, Rect)],
    cw: f32,
    ch: f32,
) {
    for &(idx, rect) in placed {
        let Some(p) = panes.get(idx) else { continue };
        let title = p.title_text();
        let activity = p.activity;
        push_card(scenes, rect, cw, ch, &title, move |cols, _rows| {
            let mut v = Vec::new();
            if activity && cols > 0 {
                v.push(CellView {
                    col: 0,
                    row: 0,
                    c: '●',
                    fg: ACTIVITY,
                    bg: (0, 0, 0),
                    bold: false,
                    italic: false,
                });
            }
            v
        });
    }
}
```

(If `ACTIVITY` is not `pub(crate)` in `panecard.rs`, change its declaration from `pub(crate) const ACTIVITY` — it already is `pub(crate)`, so the import works.)

- [ ] **Step 2: Declare the module**

In `crates/crew-app/src/lib.rs`, add `mod minstrip;` in alphabetical position.

- [ ] **Step 3: Push strip scenes in `build_frame`**

In `render.rs`, after the non-zoom `full_scenes(...)` produces `scenes` (i.e. after the `let mut scenes = if self.zoomed { … } else { … };` block, but only for the non-zoom case), push the strip. The simplest correct placement: keep the `placed` from Task 4 in scope by lifting it out of the `else` branch. Restructure so `placed` is computed before the `if self.zoomed` and the strip is pushed after, guarded to skip when zoomed:

```rust
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let placed = compose_grid(content, &self.grid, ch, GAP);
        let mut scenes = if self.zoomed && !self.panes.is_empty() {
            // ... existing zoom branch unchanged ...
        } else {
            for &(idx, rect) in &placed.full {
                relayout_one(&mut self.panes[idx], rect, cw, ch);
            }
            let f = (!self.input.focused).then_some(self.focused);
            full_scenes(&self.panes, &placed.full, f, self.broadcast, self.last_find.as_deref(), cw, ch)
        };
        if !self.zoomed {
            crate::minstrip::push_min_strip(&mut scenes, &self.panes, &placed.minimized, cw, ch);
        }
```

- [ ] **Step 4: Make thumbnails clickable (focus on click)**

Inspect `crates/crew-app/src/hit.rs` (it maps a click position to a pane index via `point_in` over pane rects). The minimized panes now have rects in `self.panes[idx].rect` only for full panes; minimized panes' rects are in the strip. Extend the hit test: when a click falls in a strip thumbnail rect, set `self.focused = idx` (which promotes it next frame via `reconcile_grid`).

First read the file to match its exact pattern:

Run: `sed -n '1,60p' crates/crew-app/src/hit.rs`

Then add a check, before/after the existing pane loop, that iterates the same `compose_grid(content, &self.grid, ch, GAP).minimized` rects and, on `chrome::point_in(rect, x, y)`, sets `self.focused = idx; self.input.focused = false;`. Keep this helper small; if `hit.rs` approaches 200 lines, move the strip-hit logic into `minstrip.rs` as `pub fn strip_hit(placed: &[(usize, Rect)], x: f32, y: f32) -> Option<usize>` and call it from `hit.rs`.

- [ ] **Step 5: Verify build + tests**

Run: `cargo fmt && cargo test -p crew-app && cargo clippy --workspace --all-targets`
Expected: compiles, tests pass, warning-free.

- [ ] **Step 6: Manual smoke test**

Run the app, spawn 8 panes. Expected: 6 full tiles on top, a bottom strip of 2 thumbnails (title + activity dot). Clicking a thumbnail promotes that pane into the full grid. Closing panes collapses the strip and restores full height when ≤ 6 remain.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app/src/minstrip.rs crates/crew-app/src/render.rs crates/crew-app/src/lib.rs crates/crew-app/src/hit.rs
git commit -m "feat(crew): minimized thumbnail strip for over-cap panes (LRU)"
```

---

### Task 6: Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/CREW.md`

- [ ] **Step 1: Document the behavior**

Add (do not rewrite existing content; match the existing style) a short note to both `README.md` and `docs/CREW.md`: Crew shows up to 6 panes as full tiles in the auto-tiling grid; additional panes are demoted to a minimized thumbnail strip along the bottom (least-recently-active first). Focusing a minimized pane — by clicking its thumbnail, using the sidebar, or `Cmd+<number>` — restores it to the full grid.

- [ ] **Step 2: Final format + check + commit**

Run: `cargo fmt && cargo check --workspace`
Expected: clean.

```bash
git add README.md docs/CREW.md
git commit -m "docs(crew): document LRU full-tile cap + minimized strip"
```

---

## Self-Review

- **Spec coverage:** "cap full tiles" → Task 1 (`MAX_FULL_TILES`, `GridLayout.full/minimized`). "demote least-recently-active (LRU)" → Task 1 (`add`/`touch`/`on_close`) + Task 3 (reconcile + per-frame `touch`). "minimized strip" → Task 2 (`compose_grid` reserves the strip) + Task 5 (renders thumbnails). "decouple running from visible" → Task 4 (over-cap panes keep running, drop out of the full grid, stay reachable). Index-shift correctness on close → Task 1 `on_close` + Task 3 `close_pane` wiring. ✅
- **Placeholder scan:** every code step contains complete code; the two `sed`/`grep` steps (Task 3 Step 1, Task 5 Step 4) are deliberate inspections of existing files whose exact contents drive a small, described edit — not deferred work. ✅
- **Type consistency:** `GridLayout`, `MAX_FULL_TILES`, `compose_grid`, `GridRects`, `MINIMIZED_STRIP_ROWS`, `relayout_one`, `full_scenes`, `push_pane_scenes`, `push_min_strip` used identically across tasks and `mod.rs` re-exports. `Rect` is `crate::layout::Rect` (f32 fields) throughout — never `ratatui::Rect`. `full()`/`minimized()`/`add()`/`touch()`/`on_close()` names stable. ✅
- **File-size limit:** new files — `state.rs` ≈70, `compose.rs` ≈70, `minstrip.rs` ≈45. `grid/tests.rs` is test-only (the 200-line production rule still applies; if it nears 200, split into `tests/state.rs` + `tests/compose.rs`). `render.rs` was 180 lines pre-change; Task 3 adds 1 line, Tasks 4–5 swap the `else` branch and add ~6 lines and 3 imports — **re-check `wc -l crates/crew-app/src/render.rs` after Task 5; if > 200, extract the non-zoom branch into a `crate::frame::build_grid_scenes(self, …)` helper in a new `frame.rs`.** ✅ (flagged for the implementer)
- **No new deps:** reuses `pane_rects_at`, `push_card`, `titled_card`, existing `CellView`/`PaneScene`. ✅

---

## Where this sits in the wider goal

This is the first of three Phase-0 plans from `docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`. It makes "hundreds running, a handful visible" structurally possible. The next two Phase-0 plans — **tokio migration** and **event bus + per-agent telemetry** — build on the same decoupling: telemetry is what the eventual swarm/constellation view (Phase 2) will render for the minimized-and-beyond agents this plan first hides.
```
