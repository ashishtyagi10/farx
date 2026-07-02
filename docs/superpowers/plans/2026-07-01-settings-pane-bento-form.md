# Settings Pane Bento Form Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the settings pane's flat label/value list with a two-column bento form (Appearance / Window / Notifications cards, boxed inputs, notify-patterns text area) saved with Cmd+S / Alt+S and cancelled with Esc.

**Architecture:** Everything stays inside the existing ratatui `Buffer` → `tui::to_cells` pipeline. A new `settingspane/form.rs` holds pure layout geometry (field → `Rect` in virtual coordinates) plus drawing helpers for cards, boxed inputs (label as fieldset legend), checkboxes, and the text area. `render.rs` draws into a virtual buffer of the layout's full height, then blits the scrolled window into the pane-sized buffer with the button row pinned at the bottom. Cmd+S is wired in `chords.rs` (overriding broadcast-toggle only when a settings pane is focused); Alt+S in global `keys.rs` matched on the physical `KeyS`.

**Tech Stack:** Rust, ratatui (rendering), winit (input), existing crew-app crate.

**Spec:** `docs/superpowers/specs/2026-07-01-settings-pane-bento-form-design.md`

## Global Constraints

- On-disk config format is unchanged (`notify_patterns: Vec<String>`).
- Esc behavior is unchanged: closes the font dropdown if open, else cancels.
- Tab/Shift-Tab focus order, wheel scroll semantics, and per-field parse/clamp commit logic are unchanged.
- Cards stack single-column below 64 pane columns.
- Every commit must pass the repo pre-commit hook (`cargo fmt` check + `cargo check`).
- Commit messages end with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

### Task 1: Notify patterns become newline-separated (text-area data model)

**Files:**
- Modify: `crates/crew-app/src/settingspane/commit.rs:46-54` (split on `\n`), `:80` (join with `\n`)
- Modify: `crates/crew-app/src/settingspane/mod.rs:97` (join with `\n`), `:82-83` (doc comment)
- Modify: `crates/crew-app/src/settingspane/keys.rs:58-79` (Enter inserts newline in the patterns field)
- Test: `crates/crew-app/src/settingspane/mod_tests.rs`

**Interfaces:**
- Consumes: existing `SettingsPane`, `commit_field`, `refresh_bufs`.
- Produces: `patterns_buf` is newline-separated (one pattern per line); `commit_field` splits it on `\n`. Task 3's text-area rendering relies on this.

- [ ] **Step 1: Update the patterns commit test to expect newline semantics**

Replace `commit_patterns_splits_and_drops_blanks` in `mod_tests.rs`:

```rust
#[test]
fn commit_patterns_splits_lines_and_drops_blanks() {
    let mut p = pane();
    focus(&mut p, Field::NotifyPatterns);
    p.patterns_buf = " error \n\n DONE ".into();
    commit_field(&mut p);
    assert_eq!(
        p.draft.notify_patterns,
        vec!["error".to_string(), "DONE".to_string()]
    );
    assert_eq!(p.patterns_buf, "error\nDONE"); // normalized display
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app commit_patterns_splits_lines -- --nocapture`
Expected: FAIL — commit still splits on commas, so `notify_patterns` comes back as one string `"error \n\n DONE"` trimmed weirdly / display joins with `", "`.

- [ ] **Step 3: Implement newline semantics**

In `commit.rs`, `Field::NotifyPatterns` arm of `commit_field`:

```rust
        Field::NotifyPatterns => {
            p.draft.notify_patterns = p
                .patterns_buf
                .split('\n')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
```

In `commit.rs::refresh_bufs`:

```rust
    p.patterns_buf = p.draft.notify_patterns.join("\n");
```

In `mod.rs::new`:

```rust
        let patterns_buf = cfg.notify_patterns.join("\n");
```

And the field's doc comment in `mod.rs`:

```rust
    /// Watched output substrings, one per line (text area).
    pub(crate) patterns_buf: String,
```

In `keys.rs::edit_key`, make Enter insert a newline in the patterns field instead of committing and advancing (Tab still leaves the field):

```rust
    match &key.logical_key {
        Key::Named(NamedKey::Enter) if f == Field::NotifyPatterns => {
            // The patterns field is a text area: Enter starts a new pattern.
            if let Some(buf) = buf_of(p, f) {
                buf.push('\n');
            }
        }
        Key::Named(NamedKey::Enter) => {
            commit_field(p);
            move_focus(p, false);
        }
```

(only the first arm is new; the plain-Enter arm keeps its existing body).

- [ ] **Step 4: Run the settingspane tests**

Run: `cargo test -p crew-app settingspane`
Expected: PASS (all existing + the updated test).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/settingspane
git commit -m "feat(crew-app): notify patterns edit as one-per-line text area buffer

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: `form.rs` — bento layout geometry and form-control drawing

**Files:**
- Create: `crates/crew-app/src/settingspane/form.rs`
- Create: `crates/crew-app/src/settingspane/form_tests.rs`
- Modify: `crates/crew-app/src/settingspane/mod.rs:5-7` (add `mod form;`)

**Interfaces:**
- Consumes: `Field` from `mod.rs`, `crate::palette::accent_color`, `crew_theme::theme`.
- Produces (used by Task 3):
  - `form::layout(cols: u16) -> FormLayout` — pure geometry; `FormLayout { cards: Vec<Card>, rects: Vec<(Field, Rect)>, height: u16 }`, `FormLayout::rect_of(Field) -> Option<Rect>`; `Card { title: &'static str, rect: Rect }`.
  - `form::scroll_for(rect: Rect, total: u16, viewport: u16) -> u16`.
  - Drawing: `card(&mut Buffer, &Card, active: bool)`, `input_box(&mut Buffer, Rect, label, value, focused, cursor)`, `checkbox(&mut Buffer, Rect, label, on, focused)`, `text_area(&mut Buffer, Rect, label, value, focused)`.
  - Colors: `pub(crate) fn dim() -> Color`, `pub(crate) fn ink() -> Color`.
  - Constants: `STACK_BELOW: u16 = 64`, `TEXTAREA_ROWS: u16 = 4`.

- [ ] **Step 1: Write the failing geometry tests**

Create `crates/crew-app/src/settingspane/form_tests.rs`:

```rust
use ratatui::layout::Rect;

use super::form::{layout, scroll_for, STACK_BELOW};
use super::Field;

#[test]
fn wide_pane_lays_out_two_columns() {
    let lay = layout(80);
    assert_eq!(lay.cards.len(), 3);
    let appearance = &lay.cards[0];
    let window = &lay.cards[1];
    let notifications = &lay.cards[2];
    assert_eq!(appearance.title, "APPEARANCE");
    // Window sits in the right column, Notifications stacked below it.
    assert!(window.rect.x > appearance.rect.x);
    assert_eq!(notifications.rect.x, window.rect.x);
    assert!(notifications.rect.y >= window.rect.y + window.rect.height);
    assert_eq!(lay.height, lay.cards.iter().map(|c| c.rect.y + c.rect.height).max().unwrap());
}

#[test]
fn narrow_pane_stacks_single_column() {
    let lay = layout(STACK_BELOW - 1);
    let xs: Vec<u16> = lay.cards.iter().map(|c| c.rect.x).collect();
    assert!(xs.iter().all(|&x| x == xs[0]), "same x: {xs:?}");
    for w in lay.cards.windows(2) {
        assert!(w[1].rect.y >= w[0].rect.y + w[0].rect.height);
    }
}

#[test]
fn every_form_field_has_a_rect() {
    let lay = layout(80);
    for f in super::FIELDS.iter().take(super::FIELDS.len() - 2) {
        assert!(lay.rect_of(*f).is_some(), "{f:?} missing a rect");
    }
    // Buttons are pinned outside the scrolled form.
    assert!(lay.rect_of(Field::Save).is_none());
}

#[test]
fn field_rects_stay_inside_their_card() {
    let lay = layout(80);
    for (f, r) in &lay.rects {
        assert!(
            lay.cards.iter().any(|c| c.rect.contains(ratatui::layout::Position::new(r.x, r.y))
                && r.y + r.height <= c.rect.y + c.rect.height),
            "{f:?} rect {r:?} escapes every card"
        );
    }
}

#[test]
fn scroll_for_keeps_the_focused_rect_visible() {
    // Everything fits → no scroll.
    assert_eq!(scroll_for(Rect::new(0, 5, 10, 3), 20, 30), 0);
    // Focus near the bottom → scrolls just enough to show its bottom edge.
    assert_eq!(scroll_for(Rect::new(0, 20, 10, 3), 25, 10), 13);
    // Focus at the top → back to zero.
    assert_eq!(scroll_for(Rect::new(0, 0, 10, 3), 25, 10), 0);
    // Never past the end.
    assert_eq!(scroll_for(Rect::new(0, 24, 10, 1), 25, 10), 15);
}
```

Add to `mod.rs` below the other module declarations:

```rust
mod form;
```

and next to the other test includes at the bottom:

```rust
#[cfg(test)]
#[path = "form_tests.rs"]
mod form_tests;
```

- [ ] **Step 2: Run to verify it fails to compile**

Run: `cargo test -p crew-app form_`
Expected: compile error — `form` module missing.

- [ ] **Step 3: Implement `form.rs`**

Create `crates/crew-app/src/settingspane/form.rs`:

```rust
//! Form controls for the settings pane: bento cards, boxed inputs with the
//! label as a fieldset legend, checkboxes, and a multi-line text area — plus
//! the pure two-column layout geometry shared by the renderer and tests.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use super::Field;
use crate::palette::accent_color;

/// Pane width below which the two card columns stack vertically.
pub(crate) const STACK_BELOW: u16 = 64;
/// Content rows inside the notify-patterns text area.
pub(crate) const TEXTAREA_ROWS: u16 = 4;

/// One bento card: a legend plus the frame the fields are drawn inside.
pub(crate) struct Card {
    pub(crate) title: &'static str,
    pub(crate) rect: Rect,
}

/// Computed form geometry, in virtual rows (y may exceed the pane height).
pub(crate) struct FormLayout {
    pub(crate) cards: Vec<Card>,
    pub(crate) rects: Vec<(Field, Rect)>,
    pub(crate) height: u16,
}

impl FormLayout {
    pub(crate) fn rect_of(&self, f: Field) -> Option<Rect> {
        self.rects.iter().find(|(g, _)| *g == f).map(|&(_, r)| r)
    }
}

/// Bento layout: two columns when the pane is wide enough (Appearance left;
/// Window + Notifications right), otherwise one stacked column.
pub(crate) fn layout(cols: u16) -> FormLayout {
    let mut rects = Vec::new();
    let mut cards = Vec::new();
    if cols >= STACK_BELOW {
        let col_w = (cols - 4) / 2; // 1-col margins + 2-col gutter
        let (lx, rx) = (1, 1 + col_w + 2);
        let ah = appearance(&mut rects, lx, 0, col_w);
        cards.push(Card { title: "APPEARANCE", rect: Rect::new(lx, 0, col_w, ah) });
        let wh = window(&mut rects, rx, 0, col_w);
        cards.push(Card { title: "WINDOW", rect: Rect::new(rx, 0, col_w, wh) });
        let ny = wh + 1;
        let nh = notifications(&mut rects, rx, ny, col_w);
        cards.push(Card { title: "NOTIFICATIONS", rect: Rect::new(rx, ny, col_w, nh) });
        FormLayout { cards, rects, height: ah.max(ny + nh) }
    } else {
        let w = cols.saturating_sub(2);
        let mut y = 0;
        for (title, build) in [
            ("APPEARANCE", appearance as fn(&mut Vec<(Field, Rect)>, u16, u16, u16) -> u16),
            ("WINDOW", window),
            ("NOTIFICATIONS", notifications),
        ] {
            let h = build(&mut rects, 1, y, w);
            cards.push(Card { title, rect: Rect::new(1, y, w, h) });
            y += h + 1;
        }
        FormLayout { cards, rects, height: y - 1 }
    }
}

/// Appearance card fields; returns the card height (content + border).
fn appearance(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::FontFamily, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::FontSize, Rect::new(ix, cy, half, 3)));
    rects.push((Field::PaperGrain, Rect::new(ix + half + 2, cy, half, 3)));
    cy += 3;
    rects.push((Field::Theme, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::Accent, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::PaperTexture, Rect::new(ix, cy, iw, 1)));
    cy += 1;
    cy + 1 - y
}

/// Window card fields; returns the card height.
fn window(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::NavWidth, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    for f in [Field::ShowNav, Field::Maximized] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    cy + 1 - y
}

/// Notifications card fields; returns the card height.
fn notifications(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    for f in [
        Field::Notify,
        Field::NotifyAgentDone,
        Field::NotifyBell,
        Field::NotifyExit,
    ] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::NotifyMinSecs, Rect::new(ix, cy, half, 3)));
    cy += 3;
    rects.push((Field::NotifyPatterns, Rect::new(ix, cy, iw, 2 + TEXTAREA_ROWS)));
    cy += 2 + TEXTAREA_ROWS;
    cy + 1 - y
}

/// Content inset inside a card border: x + 2, width − 4.
fn inner(x: u16, w: u16) -> (u16, u16) {
    (x + 2, w.saturating_sub(4))
}

/// Scroll offset keeping `rect` fully inside a `viewport`-row window over
/// `total` virtual rows (0 when everything fits).
pub(crate) fn scroll_for(rect: Rect, total: u16, viewport: u16) -> u16 {
    if viewport == 0 || total <= viewport {
        return 0;
    }
    (rect.y + rect.height)
        .saturating_sub(viewport)
        .min(rect.y)
        .min(total - viewport)
}

pub(crate) fn dim() -> Color {
    let t = crew_theme::theme();
    Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2)
}

pub(crate) fn ink() -> Color {
    let t = crew_theme::theme();
    Color::Rgb(t.ink.0, t.ink.1, t.ink.2)
}

/// A bento card: rounded border, legend on the top edge (accent while the
/// focused field lives inside it).
pub(crate) fn card(buf: &mut Buffer, c: &Card, active: bool) {
    let legend = if active { accent_color() } else { dim() };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(dim()))
        .title(Span::styled(format!(" {} ", c.title), Style::new().fg(legend)))
        .render(c.rect, buf);
}

/// A boxed input: rounded border with the label as legend; the focused box
/// gets an accent border and, for typed fields, a trailing block cursor.
pub(crate) fn input_box(
    buf: &mut Buffer,
    rect: Rect,
    label: &str,
    value: &str,
    focused: bool,
    cursor: bool,
) {
    frame(buf, rect, label, focused);
    let mut text = value.to_string();
    if focused && cursor {
        text.push('\u{2588}');
    }
    let iw = rect.width.saturating_sub(2);
    let line = Line::styled(tail(&text, iw as usize), Style::new().fg(ink()));
    buf.set_line(rect.x + 1, rect.y + 1, &line, iw);
}

/// `[x] Label` single-row toggle; `› ` marker + accent bold when focused.
pub(crate) fn checkbox(buf: &mut Buffer, rect: Rect, label: &str, on: bool, focused: bool) {
    let mark = if on { "[x]" } else { "[ ]" };
    let lead = if focused { "\u{203a} " } else { "  " };
    let mut style = Style::new().fg(if focused { accent_color() } else { ink() });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    let line = Line::styled(format!("{lead}{mark} {label}"), style);
    buf.set_line(rect.x, rect.y, &line, rect.width);
}

/// Multi-line boxed text area (one entry per line); shows the tail when the
/// content overflows, cursor on the final line while focused.
pub(crate) fn text_area(buf: &mut Buffer, rect: Rect, label: &str, value: &str, focused: bool) {
    frame(buf, rect, label, focused);
    let ih = rect.height.saturating_sub(2) as usize;
    let iw = rect.width.saturating_sub(2);
    let mut lines: Vec<String> = value.split('\n').map(str::to_string).collect();
    if focused {
        if let Some(last) = lines.last_mut() {
            last.push('\u{2588}');
        }
    }
    let skip = lines.len().saturating_sub(ih);
    for (i, l) in lines.iter().skip(skip).take(ih).enumerate() {
        let line = Line::styled(tail(l, iw as usize), Style::new().fg(ink()));
        buf.set_line(rect.x + 1, rect.y + 1 + i as u16, &line, iw);
    }
}

/// Rounded input frame with the label as legend, accent while focused.
fn frame(buf: &mut Buffer, rect: Rect, label: &str, focused: bool) {
    let col = if focused { accent_color() } else { dim() };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(col))
        .title(Span::styled(format!(" {label} "), Style::new().fg(col)))
        .render(rect, buf);
}

/// The last `w` chars of `s`, so the cursor end stays visible while typing.
fn tail(s: &str, w: usize) -> String {
    let n = s.chars().count();
    s.chars().skip(n.saturating_sub(w)).collect()
}
```

- [ ] **Step 4: Run the form tests**

Run: `cargo test -p crew-app form_`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/settingspane
git commit -m "feat(crew-app): settings form controls + bento layout geometry

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Rewrite `render.rs` as the two-column bento form

**Files:**
- Modify: `crates/crew-app/src/settingspane/render.rs` (full rewrite of `render`, `buttons`; `label_of`, `value_of`, `button_span`, `dropdown` mostly kept; `scroll_offset` and `LABEL_W` deleted)
- Modify: `crates/crew-app/src/settingspane/render_tests.rs` (rewrite layout-dependent tests)
- Modify: `crates/crew-app/src/settingspane/mod.rs:1-4` (module doc comment)

**Interfaces:**
- Consumes: everything `form.rs` produces (Task 2), `value_of`/`label_of`, `tui::to_cells`.
- Produces: `render(p, cols, rows) -> Vec<CellView>` (signature unchanged — `SettingsPane::cells` untouched). Shorter labels: `PaperGrain` → `"Grain (0-2)"`, `NotifyMinSecs` → `"Min secs"`, `NotifyPatterns` → `"Patterns (one per line)"` (they must fit half-width box legends at 80 cols).

- [ ] **Step 1: Rewrite the render tests**

Replace `render_tests.rs` tests (keep the `pane()` and `row_text` helpers):

```rust
fn dump(cells: &[CellView], rows: u16) -> String {
    (0..rows).map(|r| row_text(cells, r) + "\n").collect()
}

#[test]
fn every_field_renders_on_a_tall_pane() {
    let cells = pane().cells(80, 30);
    let all = dump(&cells, 30);
    for f in FIELDS.iter().take(FIELDS.len() - 2) {
        assert!(
            all.contains(label_of(*f)),
            "missing field '{}' in:\n{all}",
            label_of(*f)
        );
    }
    assert!(all.contains("[ Save \u{2318}S ]"), "save button: {all}");
    assert!(all.contains("[ Cancel esc ]"), "cancel button: {all}");
}

#[test]
fn cards_have_legends() {
    let all = dump(&pane().cells(80, 30), 30);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn focused_input_carries_cursor() {
    // Focus starts on FontFamily; its box content row carries the cursor.
    let all = dump(&pane().cells(80, 30), 30);
    assert!(all.contains('\u{2588}'), "cursor missing:\n{all}");
}

#[test]
fn short_pane_scrolls_to_keep_focus_visible() {
    let mut p = pane();
    p.focus = FIELDS.iter().position(|&f| f == Field::NotifyPatterns).unwrap();
    let cells = p.cells(80, 12);
    let all = dump(&cells, 12);
    assert!(all.contains("Patterns"), "focused field visible:\n{all}");
    assert!(all.contains('\u{2191}'), "up hint expected:\n{all}");
}

#[test]
fn narrow_pane_still_renders_all_cards() {
    let all = dump(&pane().cells(48, 60), 60);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn tiny_pane_renders_nothing() {
    assert!(pane().cells(10, 4).is_empty());
}

#[test]
fn theme_value_names_the_current_theme() {
    let (v, cursor) = value_of(&pane(), Field::Theme);
    assert!(v.contains("paper-dark"), "got: {v}");
    assert!(!cursor, "theme is a picker, not a text field");
}
```

(`scroll_offset_windows_the_focus` and `focused_row_carries_marker_and_cursor` are deleted with the code they tested; `Field` needs importing in the test module if not already via `super::*`.)

- [ ] **Step 2: Run to verify the new tests fail**

Run: `cargo test -p crew-app settingspane::render`
Expected: FAIL (old flat-list output has no card legends / new button text).

- [ ] **Step 3: Rewrite `render.rs`**

Keep `value_of` and `dropdown` as they are; shorten three labels in `label_of`:

```rust
        Field::PaperGrain => "Grain (0-2)",
        Field::NotifyMinSecs => "Min secs",
        Field::NotifyPatterns => "Patterns (one per line)",
```

Delete `LABEL_W` and `scroll_offset`. Replace `render` and `buttons`:

```rust
//! Settings form rendering: a two-column bento of fieldset cards
//! (Appearance / Window / Notifications) with boxed inputs, checkboxes and a
//! notify-patterns text area, plus the font dropdown popup and a pinned
//! Save/Cancel row. Built on ratatui and converted to `CellView`s; Crew
//! draws the GPU pane border around it.

/// Render the form into a ratatui buffer, then hand the cells to the GPU.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let lay = form::layout(cols);
    let mut virt = Buffer::empty(Rect::new(0, 0, cols, lay.height.max(1)));
    let focused = p.focused_field();
    let frect = lay.rect_of(focused);
    for c in &lay.cards {
        let active = frect.is_some_and(|r| c.rect.intersects(r));
        form::card(&mut virt, c, active);
    }
    for &(f, r) in &lay.rects {
        control(&mut virt, p, f, r, f == focused);
    }

    let viewport = rows.saturating_sub(2); // gap + button row
    // Save/Cancel live on the pinned button row: keep the form tail visible.
    let tail = Rect::new(0, lay.height.saturating_sub(1), 1, 1);
    let off = form::scroll_for(frect.unwrap_or(tail), lay.height, viewport);

    let mut out = Buffer::empty(Rect::new(0, 0, cols, rows));
    blit(&mut out, &virt, off, viewport);
    hints(&mut out, cols, viewport, off, lay.height);
    buttons(&mut out, cols, rows, focused);
    if p.family_open {
        if let Some(r) = lay.rect_of(Field::FontFamily) {
            if r.y >= off {
                dropdown(&mut out, p, Rect::new(r.x, r.y - off, r.width, r.height));
            }
        }
    }
    crate::tui::to_cells(&out)
}

/// Draw one field's control into the virtual buffer.
fn control(buf: &mut Buffer, p: &SettingsPane, f: Field, r: Rect, focused: bool) {
    let d = &p.draft;
    let check = |buf: &mut Buffer, on| form::checkbox(buf, r, label_of(f), on, focused);
    match f {
        Field::ShowNav => check(buf, d.show_nav),
        Field::PaperTexture => check(buf, d.paper_texture),
        Field::Maximized => check(buf, d.maximized),
        Field::Notify => check(buf, d.notify),
        Field::NotifyAgentDone => check(buf, d.notify_agent_done),
        Field::NotifyBell => check(buf, d.notify_bell),
        Field::NotifyExit => check(buf, d.notify_exit),
        Field::NotifyPatterns => form::text_area(buf, r, label_of(f), &p.patterns_buf, focused),
        Field::Save | Field::Cancel => {}
        _ => {
            let (value, cursor) = value_of(p, f);
            form::input_box(buf, r, label_of(f), &value, focused, cursor);
        }
    }
}

/// Copy `viewport` rows of `src` starting at virtual row `off` into `dst`.
fn blit(dst: &mut Buffer, src: &Buffer, off: u16, viewport: u16) {
    let cols = dst.area.width;
    let rows = viewport.min(src.area.height.saturating_sub(off));
    for y in 0..rows {
        for x in 0..cols {
            if let (Some(s), Some(d)) = (src.cell((x, y + off)), dst.cell_mut((x, y))) {
                *d = s.clone();
            }
        }
    }
}

/// `↑` / `↓` markers at the right edge when the form overflows the viewport.
fn hints(buf: &mut Buffer, cols: u16, viewport: u16, off: u16, total: u16) {
    let style = Style::new().fg(form::dim());
    if off > 0 {
        buf.set_line(cols - 2, 0, &Line::styled("\u{2191}", style), 1);
    }
    if off + viewport < total {
        let y = viewport.saturating_sub(1);
        buf.set_line(cols - 2, y, &Line::styled("\u{2193}", style), 1);
    }
}

/// `[ Save ⌘S ]   [ Cancel esc ]`, pinned bottom-right, focused accent+bold.
fn buttons(buf: &mut Buffer, cols: u16, rows: u16, f: Field) {
    let (save, cancel) = ("[ Save \u{2318}S ]", "[ Cancel esc ]");
    let w = (save.chars().count() + 3 + cancel.chars().count()) as u16;
    let line = Line::from(vec![
        button_span(save, f == Field::Save),
        Span::raw("   "),
        button_span(cancel, f == Field::Cancel),
    ]);
    buf.set_line(cols.saturating_sub(w + 2), rows - 1, &line, w);
}
```

Imports at the top of `render.rs` become:

```rust
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, StatefulWidget, Widget};

use super::{form, Field, SettingsPane};
use crate::palette::accent_color;
```

(`FIELDS` import drops out of render.rs; `button_span` and `dropdown` keep their existing bodies. `value_of`'s `Field::NotifyPatterns` arm stays — the text area reads `patterns_buf` directly.) Update the `mod.rs` module doc:

```rust
//! Settings form pane: a two-column bento of fieldset cards (Appearance /
//! Window / Notifications) with boxed inputs, checkboxes, and a
//! notify-patterns text area; Tab/wheel navigation, a type-to-search
//! font-family dropdown, and Save (Cmd+S / Alt+S) / Cancel (Esc).
```

- [ ] **Step 4: Run the whole settingspane test suite**

Run: `cargo test -p crew-app settingspane`
Expected: PASS. If a legend is truncated at 80 cols (assertion failure printing the frame), widen the failing box or shorten the label — legends must fit `rect.width - 4`.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/settingspane
git commit -m "feat(crew-app): settings pane renders as a two-column bento form

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Cmd+S / Alt+S save

**Files:**
- Modify: `crates/crew-app/src/settingspane/mod.rs` (public `save()`)
- Modify: `crates/crew-app/src/chords.rs:80` (`"s"` branch)
- Modify: `crates/crew-app/src/keys.rs:100-101` (Alt+S hook) and a new `save_focused_settings` helper
- Test: `crates/crew-app/src/settingspane/mod_tests.rs`

**Interfaces:**
- Consumes: `commit::commit_field`, `commit::build_config`, `CrewApp::{apply_settings, close_pane, toggle_broadcast}`, `PaneContent::Settings`.
- Produces: `SettingsPane::save(&mut self) -> SettingsAction` and `CrewApp::save_focused_settings(&mut self) -> bool`.

- [ ] **Step 1: Write the failing `save()` test**

Add to `mod_tests.rs`:

```rust
#[test]
fn save_commits_the_focused_edit_and_applies() {
    let mut p = pane();
    focus(&mut p, Field::FontSize);
    p.size_buf = "20".into();
    let SettingsAction::Apply(cfg) = p.save() else {
        panic!("save must apply");
    };
    assert_eq!(cfg.font_size, 20.0);
}
```

- [ ] **Step 2: Run to verify it fails to compile**

Run: `cargo test -p crew-app save_commits`
Expected: compile error — `save` not found.

- [ ] **Step 3: Implement the save path**

In `settingspane/mod.rs`, inside `impl SettingsPane`:

```rust
    /// Cmd+S / Alt+S: commit the focused field and save the whole form.
    pub fn save(&mut self) -> SettingsAction {
        commit::commit_field(self);
        SettingsAction::Apply(commit::build_config(self))
    }
```

In `keys.rs`, add to `impl CrewApp` (below `on_key_event`):

```rust
    /// Cmd+S / Alt+S: save-and-close when the focused pane is a settings
    /// form. Returns `false` when it isn't (the chord keeps its old meaning).
    pub(crate) fn save_focused_settings(&mut self) -> bool {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            return false;
        };
        let PaneContent::Settings(s) = &mut pane.content else {
            return false;
        };
        if let SettingsAction::Apply(cfg) = s.save() {
            self.apply_settings(cfg);
        }
        self.close_pane(focused);
        true
    }
```

In `chords.rs`, replace the `"s"` branch:

```rust
            // Cmd+S saves a focused settings form; otherwise toggles broadcast.
            "s" => {
                if !self.save_focused_settings() {
                    self.toggle_broadcast()
                }
            }
```

In `keys.rs::on_key_event`, insert after the super-chord block (after line 100) — matched on the physical key because macOS Option+S types `ß`:

```rust
        // Alt+S saves a focused settings form (physical key: macOS Option+S
        // produces 'ß' as the logical key). Other panes see Alt+S as normal.
        if event.state.is_pressed()
            && mstate.alt_key()
            && matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::KeyS)
            )
            && self.save_focused_settings()
        {
            self.redraw();
            return;
        }
```

and extend the imports:

```rust
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
```

- [ ] **Step 4: Run the tests and build**

Run: `cargo test -p crew-app settingspane && cargo check -p crew-app`
Expected: PASS / clean check.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src
git commit -m "feat(crew-app): Cmd+S / Alt+S save the settings form

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Full verification

**Files:** none new.

- [ ] **Step 1: Full test suite + lints**

Run: `cargo fmt --all && cargo clippy -p crew-app --all-targets -- -D warnings && cargo test -p crew-app`
Expected: all green. Fix anything that isn't; amend the responsible commit if trivial.

- [ ] **Step 2: Drive the app end-to-end**

Run the app (`cargo run`), press Cmd+, to open settings and verify: two-column cards at normal width; boxed inputs with legends; Tab traverses in the declared order; patterns Enter adds a line; Cmd+S saves and closes (config persisted); reopen, Esc cancels; with a terminal pane focused Cmd+S still toggles broadcast (status flash).

- [ ] **Step 3: Commit any fixups**

```bash
git add -A && git commit -m "fix(crew-app): settings bento form polish from end-to-end run

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

(skip if nothing changed).
