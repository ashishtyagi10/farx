use crew_plugin::{Plugin, PluginCommand, PluginEvent};
use std::time::{Duration, Instant};

fn drain_until<F: Fn(&PluginEvent) -> bool>(p: &Plugin, pred: F) -> bool {
    let end = Instant::now() + Duration::from_secs(3);
    while Instant::now() < end {
        if p.try_recv().iter().any(&pred) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn echo_roundtrip() {
    let mut p = Plugin::spawn(env!("CARGO_BIN_EXE_crew-echo-plugin"), &[]).unwrap();
    p.send(&PluginCommand::Hello { v: 1 }).unwrap();
    assert!(drain_until(&p, |e| matches!(
        e,
        PluginEvent::Ready { provider, .. } if provider == "echo"
    )));
    p.send(&PluginCommand::Send {
        channel: "general".into(),
        text: "ping".into(),
    })
    .unwrap();
    assert!(drain_until(&p, |e| matches!(
        e,
        PluginEvent::Message { text, .. } if text == "ping"
    )));
}
