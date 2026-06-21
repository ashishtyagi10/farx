use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginCommand {
    Hello { v: u32 },
    Subscribe { channel: String },
    Send { channel: String, text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginEvent {
    Ready {
        v: u32,
        provider: String,
        channels: Vec<String>,
    },
    Message {
        channel: String,
        sender: String,
        text: String,
        ts: String,
    },
    Error {
        message: String,
    },
    SpawnPane {
        command: String,
        args: Vec<String>,
        label: String,
    },
    SendPane {
        label: String,
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_serializes_tagged() {
        let s = serde_json::to_string(&PluginCommand::Hello { v: 1 }).unwrap();
        assert_eq!(s, r#"{"type":"hello","v":1}"#);
    }

    #[test]
    fn spawn_pane_serializes_with_type_tag() {
        let ev = PluginEvent::SpawnPane {
            command: "sh".into(),
            args: vec!["-c".into()],
            label: "a".into(),
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""type":"spawn_pane""#), "got: {s}");
    }

    #[test]
    fn send_pane_deserializes_from_json() {
        let line = r#"{"type":"send_pane","label":"a","text":"hi"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::SendPane { label, text } => {
                assert_eq!(label, "a");
                assert_eq!(text, "hi");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn message_event_roundtrips() {
        let line = r#"{"type":"message","channel":"general","sender":"bob","text":"hi","ts":"t"}"#;
        let ev: PluginEvent = serde_json::from_str(line).unwrap();
        match ev {
            PluginEvent::Message {
                channel,
                sender,
                text,
                ts,
            } => {
                assert_eq!(
                    (
                        channel.as_str(),
                        sender.as_str(),
                        text.as_str(),
                        ts.as_str()
                    ),
                    ("general", "bob", "hi", "t")
                );
            }
            _ => panic!("wrong variant"),
        }
    }
}
