use super::*;
use crate::graph::TaskState;

#[test]
fn state_color_distinguishes_states() {
    assert_eq!(state_color(TaskState::Running), Rgb(0, 220, 140));
    assert_eq!(state_color(TaskState::Failed), Rgb(230, 80, 80));
    assert_ne!(
        state_color(TaskState::Pending),
        state_color(TaskState::Done)
    );
}

#[test]
fn mode_switches_at_threshold() {
    assert_eq!(mode_for_count(1), ViewMode::Constellation);
    assert_eq!(
        mode_for_count(HEATMAP_THRESHOLD - 1),
        ViewMode::Constellation
    );
    assert_eq!(mode_for_count(HEATMAP_THRESHOLD), ViewMode::Heatmap);
    assert_eq!(mode_for_count(5000), ViewMode::Heatmap);
}

#[test]
fn rgb_serde_roundtrip() {
    let c = Rgb(1, 2, 3);
    let j = serde_json::to_string(&c).unwrap();
    assert_eq!(serde_json::from_str::<Rgb>(&j).unwrap(), c);
}
