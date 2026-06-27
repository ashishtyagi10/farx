use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::TaskId;

use super::{Agent, AgentContext};

/// A deterministic agent for headless tests: emits an output + token event and
/// returns a result whose success depends on whether its task id is in
/// `fail_ids`.
pub struct StubAgent {
    pub fail_ids: HashSet<TaskId>,
}

impl Agent for StubAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let fail = self.fail_ids.contains(&ctx.task.id);
        Box::pin(async move {
            let output = format!("stub:{} deps={}", ctx.task.id.0, ctx.deps.len());
            ctx.bus.publish(HiveEvent::OutputChunk {
                agent: ctx.agent.clone(),
                text: format!("{output}\n"),
            });
            ctx.bus.publish(HiveEvent::TokenDelta {
                agent: ctx.agent.clone(),
                input: 1,
                output: 1,
            });
            if fail {
                ctx.bus.publish(HiveEvent::Failed {
                    agent: ctx.agent,
                    error: "stub failure".into(),
                });
            }
            TaskResult {
                task: ctx.task.id,
                output,
                success: !fail,
            }
        })
    }
}
