//! Constellation layout: depth-placed nodes and dependency edges.
use crate::graph::{TaskGraph, TaskId, TaskState};
use crate::telemetry::Fleet;
use crate::view::{state_color, Rgb};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub task: TaskId,
    pub x: f32,
    pub y: f32,
    pub color: Rgb,
    pub state: TaskState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: TaskId,
    pub to: TaskId,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Constellation {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

fn compute_depths(graph: &TaskGraph) -> HashMap<TaskId, usize> {
    let tasks = graph.tasks();
    let mut depths: HashMap<TaskId, usize> = tasks.iter().map(|t| (t.id, 0)).collect();
    for _ in 0..tasks.len() {
        let mut changed = false;
        for t in tasks {
            for dep in &t.deps {
                let dep_d = *depths.get(dep).unwrap_or(&0);
                let cur_d = *depths.get(&t.id).unwrap_or(&0);
                if dep_d + 1 > cur_d {
                    depths.insert(t.id, dep_d + 1);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    depths
}

pub fn constellation(graph: &TaskGraph, fleet: &Fleet) -> Constellation {
    let tasks = graph.tasks();
    let depths = compute_depths(graph);
    let max_depth = depths.values().copied().max().unwrap_or(0);

    let mut by_depth: HashMap<usize, Vec<TaskId>> = HashMap::new();
    for t in tasks {
        by_depth
            .entry(*depths.get(&t.id).unwrap_or(&0))
            .or_default()
            .push(t.id);
    }
    for layer in by_depth.values_mut() {
        layer.sort_unstable();
    }

    let mut nodes: Vec<Node> = tasks
        .iter()
        .map(|t| {
            let depth = *depths.get(&t.id).unwrap_or(&0);
            let x = if max_depth == 0 {
                0.5
            } else {
                depth as f32 / max_depth as f32
            };
            let layer = &by_depth[&depth];
            let i = layer.iter().position(|&id| id == t.id).unwrap();
            let y = (i + 1) as f32 / (layer.len() + 1) as f32;
            let state = fleet
                .agents()
                .find(|a| a.task == t.id)
                .map(|a| a.state)
                .unwrap_or(TaskState::Pending);
            let color = state_color(state);
            Node {
                task: t.id,
                x,
                y,
                color,
                state,
            }
        })
        .collect();
    nodes.sort_by_key(|n| n.task);

    let edges: Vec<Edge> = tasks
        .iter()
        .flat_map(|t| {
            t.deps.iter().map(move |&dep| Edge {
                from: dep,
                to: t.id,
            })
        })
        .collect();

    Constellation { nodes, edges }
}
