//! Blackboard: a concurrent shared store where agents publish their task
//! results and read upstream dependencies' results, so a fan-out of agents can
//! merge results upward. Cheap to clone (shared `Arc<RwLock<_>>`).
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::graph::TaskId;

/// The result an agent publishes for its task.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskResult {
    pub task: TaskId,
    pub output: String,
    pub success: bool,
}

#[derive(Default)]
struct Inner {
    results: HashMap<TaskId, TaskResult>,
    artifacts: HashMap<String, String>,
}

/// Shared, cloneable handle to the blackboard.
#[derive(Clone, Default)]
pub struct Blackboard {
    inner: Arc<RwLock<Inner>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn put_result(&self, result: TaskResult) {
        self.inner.write().await.results.insert(result.task, result);
    }

    pub async fn get_result(&self, task: TaskId) -> Option<TaskResult> {
        self.inner.read().await.results.get(&task).cloned()
    }

    /// Present results for `deps`, in the order given (absent ones skipped).
    pub async fn gather(&self, deps: &[TaskId]) -> Vec<TaskResult> {
        let g = self.inner.read().await;
        deps.iter()
            .filter_map(|d| g.results.get(d).cloned())
            .collect()
    }

    pub async fn result_count(&self) -> usize {
        self.inner.read().await.results.len()
    }
}
