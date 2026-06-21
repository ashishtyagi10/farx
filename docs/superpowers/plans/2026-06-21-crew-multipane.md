# Crew Multi-Pane + Focus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn Crew from a single pane into an auto-tiling **agent grid**: multiple shell panes packed near-square, each rendered in its own sub-rect, with a focus model (click-to-focus + `Cmd`-chord pane management) that keeps every non-reserved key flowing to the focused pane.

**Architecture:** `crew-app` owns a `Vec<Pane>` (each = `PtyTerm` + single-use writer + its own grid size + pixel rect). A pure `layout::pane_rects` computes near-square tile rects. `crew-render`'s `Renderer::frame` is generalized from one cell-set to a *scene* of panes: per-pane background quads + a per-pane border (bright on the focused pane) + per-pane glyphon `TextArea`s. Keyboard routing: reserved `Cmd`-chords (Super modifier — available because Crew owns the OS keyboard) manage panes; everything else goes to the focused pane's PTY.

**Tech Stack:** Rust 2021, `alacritty_terminal` =0.26.0, `glyphon` =0.11.0, `wgpu` =29.0.3, `winit` =0.30.13, `portable-pty` =0.9.0.

## Global Constraints

- **`crew-render` must NEVER import `crew-term`** — panes cross as `crew-render`'s own value types (`CellView`, and a new `PaneScene`); `crew-app` maps.
- **Every `.rs` file ≤ 200 lines (HARD, no exceptions).** Split into submodules before crossing.
- **Reserved keys are `Cmd`/Super chords ONLY** (`Cmd+T`, `Cmd+W`, `Cmd+1..9`, `Cmd+[`/`Cmd+]`). Every non-Super key — including all `Ctrl`/`Alt`/function keys — passes through to the focused pane untouched. Do NOT steal non-Super keys from panes.
- Pinned MIT/Apache deps. `cargo fmt` + `cargo clippy --workspace --all-targets` clean; no dead code / no `#[allow(dead_code)]`.
- GPU tasks: gate = compile + clippy clean + `cargo test` + timeboxed non-panic launch (`timeout 6 cargo run -p crew-app` → exit 124). Visual confirmation is the human's. Reconcile any from-memory `wgpu`/`glyphon`/`winit` API against the real pinned crate.

---

### Task 1: `crew-app` — pane tiling geometry (TDD)

**Files:**
- Create: `crates/crew-app/src/layout.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod layout;`)

**Interfaces:**
- Produces: `pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }`; `pub fn pane_rects(n: usize, width: f32, height: f32, gap: f32) -> Vec<Rect>` — near-square packing: `cols = ceil(sqrt(n))`, `rows = ceil(n / cols)`; cells laid row-major; each tile inset by `gap` on all sides. Returns exactly `n` rects (empty vec if `n == 0`). Last row may be partially filled (fewer than `cols`); those tiles still use the same tile width.

- [ ] **Step 1: Write the failing tests** (`layout.rs`, `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) { assert!((a - b).abs() < 0.5, "{a} != {b}"); }

    #[test]
    fn one_pane_fills_minus_gap() {
        let r = pane_rects(1, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 1);
        approx(r[0].x, 0.0); approx(r[0].y, 0.0); approx(r[0].w, 800.0); approx(r[0].h, 600.0);
    }

    #[test]
    fn two_panes_side_by_side() {
        let r = pane_rects(2, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 2);
        approx(r[0].w, 400.0); approx(r[1].x, 400.0); approx(r[0].h, 600.0);
    }

    #[test]
    fn four_panes_two_by_two() {
        let r = pane_rects(4, 800.0, 600.0, 0.0);
        assert_eq!(r.len(), 4);
        approx(r[0].w, 400.0); approx(r[0].h, 300.0);
        approx(r[3].x, 400.0); approx(r[3].y, 300.0);
    }

    #[test]
    fn zero_panes_empty() {
        assert!(pane_rects(0, 800.0, 600.0, 4.0).is_empty());
    }
}
```

- [ ] **Step 2: Run, verify failure** — `cargo test -p crew-app pane_rects` (or module) → FAIL (`pane_rects` not found).

- [ ] **Step 3: Implement `Rect` + `pane_rects`**

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Pack `n` tiles near-square into `width`x`height`, each inset by `gap`.
pub fn pane_rects(n: usize, width: f32, height: f32, gap: f32) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let tile_w = width / cols as f32;
    let tile_h = height / rows as f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let c = i % cols;
        let r = i / cols;
        out.push(Rect {
            x: c as f32 * tile_w + gap,
            y: r as f32 * tile_h + gap,
            w: tile_w - 2.0 * gap,
            h: tile_h - 2.0 * gap,
        });
    }
    out
}
```

- [ ] **Step 4: Run tests, verify pass** — `cargo test -p crew-app`.
- [ ] **Step 5: clippy + commit** — `cargo clippy -p crew-app --all-targets`.
```bash
git add crates/crew-app && git commit -m "feat(crew-app): near-square pane tiling geometry"
```

---

### Task 2: `crew-render` — render a SCENE of panes (build-run-observe)

**Files:**
- Modify: `crates/crew-render/src/cellgrid.rs`, `crates/crew-render/src/celltext.rs`, `crates/crew-render/src/renderer.rs`, `crates/crew-render/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct PaneScene { pub cells: Vec<CellView>, pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub focused: bool }` — one pane's cells (cols/rows relative to the pane) plus its pixel rect + focus flag.
  - `Renderer::frame(&mut self, panes: &[PaneScene])` (replaces the single-cells signature).
  - `CellGrid::set_scene(&mut self, gpu: &Gpu, panes: &[PaneScene])` and an updated `prepare`/`draw` that handle multiple panes.

- [ ] **Step 1: Generalize `celltext`** — change the per-pane text builder so it can build one cosmic-text `Buffer` per pane from that pane's `cells` (rich-text spans as today). Keep `cell_w`/`cell_h` (font-derived, shared). Export a helper that, given a pane's cells + cols/rows, returns a built `Buffer`. (cols/rows derive from the pane rect: `cols = floor(w/cell_w)`, `rows = floor(h/cell_h)` — compute in `set_scene`.)

- [ ] **Step 2: `CellGrid::set_scene`** — for each pane: (a) push background `Quad`s for cells whose `bg != DEFAULT_BG`, offset by the pane origin (`x + col*cell_w`, `y + row*cell_h`); (b) push a **border**: 4 thin `Quad`s outlining the pane rect — color `(80,80,90)` normally, bright `(90,140,220)` when `pane.focused` (so the focused pane is obvious); (c) build the pane's text `Buffer`, stored in a `Vec<(Buffer, f32, f32)>` (buffer + origin). Feed all quads to the single `QuadLayer`.

- [ ] **Step 3: `prepare`/`draw` for multiple panes** — `prepare`: update quad viewport + glyphon `Viewport`; build a `TextArea` per pane positioned at its origin (`left: x, top: y`, bounds clipped to the pane rect so text doesn't bleed across panes), and call `text_renderer.prepare(... [areas] ...)` with all of them. `draw`: `quad_layer.draw` (all backgrounds + borders) then `text_renderer.render` (all panes). Order unchanged (quads behind text).

> Reconcile glyphon 0.11: `prepare` accepts an iterator/slice of `TextArea`; `TextBounds` clips per pane. Keep one `FontSystem`/`SwashCache`/`TextAtlas`/`TextRenderer` shared across panes (efficient). Split `cellgrid.rs` if it crosses 200 lines (e.g. move scene-building into a `scene.rs`).

- [ ] **Step 4: `Renderer::frame(panes)`** — set_scene → prepare → acquire → clear pass → cell_grid.draw → submit → present. Update `lib.rs` to `pub use cellgrid::PaneScene` (and keep `CellView`, `GridMetrics` if still used; drop `GridMetrics` if `frame` no longer needs it).

- [ ] **Step 5: Gate** — `cargo build -p crew-render` + `cargo clippy -p crew-render --all-targets` clean; every `.rs` ≤ 200. (Not wired to app yet — that's Task 3.)
- [ ] **Step 6: Commit**
```bash
git add crates/crew-render && git commit -m "feat(crew-render): render a scene of bordered panes"
```

---

### Task 3: `crew-app` — multi-pane state, spawn/close, render all (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/app.rs`, `crates/crew-app/src/session.rs`
- Create: `crates/crew-app/src/pane.rs`

**Interfaces:**
- Produces: `struct Pane { pty: PtyTerm, input: Box<dyn Write+Send>, grid: GridSize, rect: Rect }` and app state holding `panes: Vec<Pane>` + `focused: usize`. Helpers: spawn a pane (shell), close a pane, relayout (recompute rects from `pane_rects` + per-pane grid from rect/cell size, resize each pty on change).

- [ ] **Step 1: `pane.rs`** — `Pane` struct + `Pane::spawn(shell, grid) -> Result<Pane>` (spawns `PtyTerm`, takes its single-use `writer` once into `input`). A `relayout(panes, rects, cell_w, cell_h)` free fn that assigns each pane its `rect` and resizes its grid/pty when the derived cols/rows change.
- [ ] **Step 2: app state** — replace the single `pty`/`input`/`grid` with `panes: Vec<Pane>` + `focused: usize`. In `resumed`, spawn ONE initial pane (bash→sh) sized to the full surface. Keep the `about_to_wait` loop: drain `try_read()` for EVERY pane; redraw if any produced bytes.
- [ ] **Step 3: build the scene + render** — each redraw: `pane_rects(panes.len(), w, h, GAP)` → `relayout` → for each pane build a `PaneScene { cells: to_cellviews(pty.cells()), x,y,w,h from rect, focused: i==focused }` → `renderer.frame(&scenes)`.
- [ ] **Step 4: spawn/close plumbing (no keybind yet — Task 4 binds them)** — `fn spawn_pane(&mut self)` (push a new `Pane`, focus it, relayout) and `fn close_pane(&mut self, idx)` (remove, fix `focused`, relayout; if last pane closed, exit the app). Keep these methods ready for Task 4.
- [ ] **Step 5: Gate** — build + `cargo clippy --workspace --all-targets` clean; `cargo test -p crew-app` (layout tests) green; every `.rs` ≤ 200 (split `app.rs` if needed — e.g. move the frame/scene building into `pane.rs` or a `render.rs`); `timeout 6 cargo run -p crew-app` exit 124.
- [ ] **Step 6: Commit**
```bash
git add crates/crew-app && git commit -m "feat(crew-app): multi-pane state and scene rendering"
```

---

### Task 4: `crew-app` — focus model: click-to-focus + Cmd-chord pane management (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/app.rs`, `crates/crew-app/src/session.rs`

**Interfaces:**
- Produces: `fn pane_at(rects: &[Rect], x: f32, y: f32) -> Option<usize>` (hit-test, with a TDD unit test) and keyboard/mouse routing: track `ModifiersState`; `Cmd+T` spawn, `Cmd+W` close focused, `Cmd+1..9` focus N, `Cmd+[`/`Cmd+]` cycle focus; mouse click → `pane_at` → focus; all other keys → focused pane's `input`.

- [ ] **Step 1: hit-test + test** — `pane_at` in session.rs with a unit test (e.g. a click at (410,10) in a 2-pane 800x600 layout returns pane 1).

```rust
pub fn pane_at(rects: &[crate::layout::Rect], x: f32, y: f32) -> Option<usize> {
    rects.iter().position(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
}
```

- [ ] **Step 2: track modifiers + mouse position** — handle `WindowEvent::ModifiersChanged` (store `Modifiers`), `CursorMoved` (store last cursor pos), `MouseInput { state: Pressed, button: Left }` → `pane_at(&rects, x, y)` → set `self.focused` + redraw.
- [ ] **Step 3: Cmd-chord routing in `KeyboardInput`** — if the Super modifier is held, match the logical key: `T` → `spawn_pane()`; `W` → `close_pane(self.focused)`; `1`..`9` → focus that index if it exists; `[` → focus prev (wrap), `]` → focus next (wrap). Consume the event (do NOT forward Super-chords to the pane). Otherwise (no Super) → `key_to_bytes(&event)` → write to `self.panes[self.focused].input`. Request redraw after focus/spawn/close.

> Reconcile winit 0.30: `WindowEvent::ModifiersChanged(Modifiers)`, `Modifiers::state().super_key()` (Super = Cmd on macOS, Win on Windows), `MouseButton::Left`, `ElementState::Pressed`, `CursorMoved { position }`. The Super modifier is the reserved namespace — confirm `super_key()` is the right accessor for the pinned winit.

- [ ] **Step 4: focus highlight** — already handled by Task 2's border (focused pane gets the bright border); just ensure `PaneScene.focused` is set from `self.focused`.
- [ ] **Step 5: Gate** — build + `cargo clippy --workspace --all-targets` clean; `cargo test -p crew-app` (hit-test + layout) green; `.rs` ≤ 200; `timeout 6 cargo run -p crew-app` exit 124.
- [ ] **Step 6: Commit**
```bash
git add crates/crew-app && git commit -m "feat(crew-app): click-to-focus + Cmd-chord pane management"
```

---

### Task 5: Cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` (fix all, no `#[allow]`).
- [ ] **Step 2:** `cargo test -p crew-term -p crew-app` — all green (layout, hit-test, term color/echo).
- [ ] **Step 3:** Every `.rs` in `crates/crew-*/src/` ≤ 200 (`for f in crates/crew-*/src/*.rs; do wc -l "$f"; done`). Split any overflow.
- [ ] **Step 4:** Manual smoke checklist (record in commit body): `Cmd+T` opens a second pane (grid splits near-square); typing goes to the focused pane only; clicking another pane moves focus (border highlights it); `Cmd+W` closes a pane and the grid re-tiles; `Cmd+1/2` jump focus; resize re-tiles. No panics; closing the last pane exits.
- [ ] **Step 5:** Commit milestone.
```bash
git add -A && git commit -m "chore: Crew multi-pane + focus milestone (agent grid)"
```

---

## Notes for the next plan (Plan 4: Orchestrator, or Plan 3.1 polish)

- LRU minimized-thumbnail strip when pane count exceeds the full-tile cap (design doc: cap 6, 7th+ demote least-recently-active) — deferred from this plan's simple near-square tiling.
- Damage-driven redraw via `Term::damage()` to avoid rebuilding every pane's cells each frame.
- Per-pane title bars (cwd / command), and the orchestrator bottom box (Plan 4) that spawns/drives panes as agents.
- Space-cell backgrounds (cells() currently filters spaces).
