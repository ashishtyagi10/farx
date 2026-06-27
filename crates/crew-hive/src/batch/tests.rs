use super::*;
use crate::graph::{ModelTier, TaskId};

fn job(p: &str, tier: ModelTier) -> Job {
    Job {
        title: p.into(),
        prompt: p.into(),
        tier,
    }
}

#[test]
fn batch_graph_is_flat_and_independent() {
    let g = batch_graph(vec![
        job("a", ModelTier::Cheap),
        job("b", ModelTier::Standard),
        job("c", ModelTier::Capable),
    ])
    .unwrap();
    assert_eq!(g.len(), 3);
    // all tasks are roots (no deps) -> all ready immediately
    let ready = g.ready(&std::collections::HashSet::new());
    assert_eq!(ready, vec![TaskId(0), TaskId(1), TaskId(2)]);
    assert_eq!(g.get(TaskId(2)).unwrap().model, ModelTier::Capable);
}

#[test]
fn batch_graph_empty_ok() {
    let g = batch_graph(vec![]).unwrap();
    assert!(g.is_empty());
}
