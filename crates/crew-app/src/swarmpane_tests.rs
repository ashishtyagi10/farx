//! Headless tests for the live swarm pane. The engine runs on a std::thread with
//! stub agents — no GPU, no winit, no network — so these are fully deterministic.
use super::{demo_graph, SwarmPane};
use std::time::{Duration, Instant};

#[test]
fn demo_graph_is_fanout_merge() {
    // 1 root + 3 parallel workers + 1 merge.
    assert_eq!(demo_graph().len(), 5);
}

#[test]
fn demo_swarm_runs_to_completion() {
    let mut pane = SwarmPane::demo();
    let mut applied_any = false;
    let start = Instant::now();
    loop {
        if pane.poll() {
            applied_any = true;
        }
        if pane.fleet.totals().done >= 5 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "demo swarm timed out before completing"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let t = pane.fleet.totals();
    assert_eq!(t.done, 5, "all 5 demo tasks should complete");
    assert_eq!(t.failed, 0, "stub agents never fail");
    assert!(applied_any, "poll must report applied events while running");
}

#[test]
fn cells_have_hud_row() {
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
