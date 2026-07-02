//! Fleet → CellViews renderer: a legible task list over live fleet telemetry.
//! Row 0 is a HUD of fleet totals; each row below is one task — state glyph,
//! title, and (while running or after failing) the agent's last output line —
//! so a swarm pane shows *what* is happening, not just how much.
use std::collections::HashMap;

use crew_hive::{Fleet, TaskGraph, TaskId, TaskState};
use crew_render::CellView;

/// Glyph, colour, and bold flag for a task state.
fn state_style(state: TaskState) -> (char, (u8, u8, u8), bool) {
    let t = crew_theme::theme();
    match state {
        TaskState::Pending | TaskState::Ready => ('\u{25cb}', t.text_muted, false), // ○
        TaskState::Running => ('\u{25cf}', crate::palette::accent(), true),         // ●
        TaskState::Done => ('\u{2713}', t.ansi[2], false),                          // ✓
        TaskState::Failed => ('\u{2717}', t.ansi[9], true),                         // ✗
        TaskState::Cancelled => ('\u{2013}', t.text_muted, false),                  // –
    }
}

/// Append `s` at `(row, col..)` in `fg`, clipped to `cols`; returns the next column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    let mut x = col;
    for ch in s.chars() {
        if x >= cols {
            break;
        }
        cells.push(CellView {
            col: x,
            row,
            c: ch,
            fg,
            bg,
            bold,
            italic: false,
        });
        x += 1;
    }
    x
}

/// Map a `Fleet` to a `Vec<CellView>` for the given terminal grid.
///
/// Row 0 is a HUD showing live/done/failed/cost totals. Rows 1‥rows-1 list the
/// graph's tasks in order, one per row, with a trailing `… +N more` overflow
/// row when the pane is too short for them all.
///
/// Returns an empty vec when `cols == 0 || rows == 0`.
pub fn swarm_cells(graph: &TaskGraph, fleet: &Fleet, cols: u16, rows: u16) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return vec![];
    }
    let t = crew_theme::theme();
    let mut cells = Vec::new();

    // HUD row: live/done/failed + cost in dollars.
    let totals = fleet.totals();
    let hud = format!(
        " live:{} done:{} failed:{} cost:${:.4}",
        totals.live,
        totals.done,
        totals.failed,
        totals.micros_usd as f64 / 1_000_000.0,
    );
    push(&mut cells, 0, 0, cols, &hud, t.ink, false);

    // Task rows below the HUD. A task with no spawned agent yet is Pending.
    let by_task: HashMap<TaskId, _> = fleet.agents().map(|a| (a.task, a)).collect();
    let avail = rows.saturating_sub(1) as usize;
    let tasks = graph.tasks();
    // Keep one row for the overflow note when the list doesn't fit.
    let shown = if tasks.len() > avail {
        avail.saturating_sub(1)
    } else {
        tasks.len()
    };
    for (i, spec) in tasks.iter().take(shown).enumerate() {
        let row = (i + 1) as u16;
        let agent = by_task.get(&spec.id);
        let state = agent.map_or(TaskState::Pending, |a| a.state);
        let (glyph, color, bold) = state_style(state);
        let mut x = push(&mut cells, row, 1, cols, &glyph.to_string(), color, bold);
        x = push(&mut cells, row, x + 1, cols, &spec.title, color, bold);
        // The live tail: what the agent last printed (or the failure reason).
        let tail = agent
            .filter(|_| matches!(state, TaskState::Running | TaskState::Failed))
            .map(|a| a.last_line.as_str())
            .unwrap_or_default();
        if !tail.is_empty() {
            x = push(&mut cells, row, x, cols, " \u{2014} ", t.text_muted, false);
            push(&mut cells, row, x, cols, tail, t.text_muted, false);
        }
    }
    if tasks.len() > shown && avail > 0 {
        let note = format!("\u{2026} +{} more", tasks.len() - shown);
        push(
            &mut cells,
            (shown + 1) as u16,
            1,
            cols,
            &note,
            t.text_muted,
            false,
        );
    }
    cells
}

/// An amber notice on the last row when the budget governor stopped a swarm, so
/// a cancelled run doesn't just look "done".
pub fn cancelled_notice(cols: u16, rows: u16) -> Vec<CellView> {
    let last = rows.saturating_sub(1);
    let t = crew_theme::theme();
    "budget exceeded — swarm cancelled"
        .chars()
        .take(cols as usize)
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: last,
            c,
            fg: t.status_fg,
            bg: t.page_bg,
            bold: true,
            italic: false,
        })
        .collect()
}
