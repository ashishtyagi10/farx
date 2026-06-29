//! Headless tests for the live swarm pane. The planner and engine run on
//! std::threads with stub implementations — no GPU, no winit, no network — so
//! these are fully deterministic.
use super::{demo_graph, SwarmPane, SwarmState, GOAL_FANOUT};
use std::time::{Duration, Instant};

/// Drive `pane.poll()` until `done` predicate holds or the deadline passes.
fn pump_until(pane: &mut SwarmPane, deadline: Duration, done: impl Fn(&SwarmPane) -> bool) {
    let start = Instant::now();
    while !done(pane) {
        pane.poll();
        assert!(start.elapsed() < deadline, "swarm pane timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The number of completed tasks, or 0 unless the pane is running.
fn done_count(pane: &SwarmPane) -> usize {
    match &pane.state {
        SwarmState::Running { fleet, .. } => fleet.totals().done,
        _ => 0,
    }
}

#[test]
fn demo_graph_is_fanout_merge() {
    // 1 root + 3 parallel workers + 1 merge.
    assert_eq!(demo_graph().len(), 5);
}

#[test]
fn demo_swarm_runs_to_completion() {
    let mut pane = SwarmPane::demo();
    assert!(
        matches!(pane.state, SwarmState::Running { .. }),
        "demo runs at once"
    );
    pump_until(&mut pane, Duration::from_secs(5), |p| done_count(p) >= 5);
    match &pane.state {
        SwarmState::Running { fleet, .. } => {
            let t = fleet.totals();
            assert_eq!(t.done, 5, "all 5 demo tasks should complete");
            assert_eq!(t.failed, 0, "stub agents never fail");
        }
        _ => panic!("expected Running state"),
    }
}

#[test]
fn goal_pane_plans_then_runs() {
    let mut pane = SwarmPane::for_goal("build a thing".into());
    // Starts in Planning, showing the goal in its banner.
    assert!(matches!(pane.state, SwarmState::Planning { .. }));
    let banner: String = pane.cells(60, 12).iter().map(|c| c.c).collect();
    assert!(
        banner.contains("build a thing"),
        "planning banner echoes the goal"
    );

    // The plan arrives, the pane transitions to Running, and the graph completes.
    pump_until(&mut pane, Duration::from_secs(5), |p| {
        matches!(p.state, SwarmState::Running { .. })
    });
    // StubPlanner { fanout: N } makes N leaves + 1 merge.
    let expected = GOAL_FANOUT + 1;
    pump_until(&mut pane, Duration::from_secs(5), |p| {
        done_count(p) >= expected
    });
    assert_eq!(done_count(&pane), expected, "all planned tasks complete");
}

#[test]
fn cells_have_hud_row_when_running() {
    let pane = SwarmPane::demo();
    let cells = pane.cells(60, 12);
    assert!(
        cells.iter().any(|c| c.row == 0 && c.bg == (20, 20, 40)),
        "row 0 must carry the dark-navy HUD"
    );
}

#[test]
fn cells_empty_for_zero_dims() {
    let pane = SwarmPane::demo();
    assert!(pane.cells(0, 12).is_empty());
    assert!(pane.cells(60, 0).is_empty());
}
