use super::*;
use crate::graph::TaskId;

fn res(task: u64, out: &str, ok: bool) -> TaskResult {
    TaskResult {
        task: TaskId(task),
        output: out.into(),
        success: ok,
    }
}

#[tokio::test]
async fn put_then_get_result() {
    let b = Blackboard::new();
    b.put_result(res(1, "hello", true)).await;
    assert_eq!(b.get_result(TaskId(1)).await, Some(res(1, "hello", true)));
    assert_eq!(b.get_result(TaskId(2)).await, None);
    assert_eq!(b.result_count().await, 1);
}

#[tokio::test]
async fn put_overwrites_same_task() {
    let b = Blackboard::new();
    b.put_result(res(1, "old", true)).await;
    b.put_result(res(1, "new", false)).await;
    assert_eq!(b.get_result(TaskId(1)).await.unwrap().output, "new");
    assert_eq!(b.result_count().await, 1);
}

#[tokio::test]
async fn gather_returns_present_deps_in_order() {
    let b = Blackboard::new();
    b.put_result(res(2, "two", true)).await;
    b.put_result(res(0, "zero", true)).await;
    // dep 1 absent; expect [0, 2] in the requested order, skipping 1.
    let got = b.gather(&[TaskId(0), TaskId(1), TaskId(2)]).await;
    let tasks: Vec<TaskId> = got.iter().map(|r| r.task).collect();
    assert_eq!(tasks, vec![TaskId(0), TaskId(2)]);
}

#[tokio::test]
async fn clones_share_state() {
    let b = Blackboard::new();
    let b2 = b.clone();
    b.put_result(res(5, "x", true)).await;
    // The clone sees the write — shared Arc.
    assert_eq!(b2.get_result(TaskId(5)).await.unwrap().output, "x");
}

#[tokio::test]
async fn concurrent_writers_all_land() {
    let b = Blackboard::new();
    let mut handles = Vec::new();
    for i in 0..50u64 {
        let bc = b.clone();
        handles.push(tokio::spawn(async move {
            bc.put_result(res(i, "v", true)).await;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(b.result_count().await, 50);
}
