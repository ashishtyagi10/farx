#![cfg(unix)]
use std::collections::BTreeMap;

use super::*;
use crate::mcp::{McpHost, ServerConfig};

/// A canned server: replays `responses` (one JSON-RPC line each) then holds
/// its pipes open, standing in for a real MCP server.
fn canned(tag: &str, responses: &[&str]) -> ServerConfig {
    let path = std::env::temp_dir().join(format!("crew-mcp-{tag}-{}.jsonl", std::process::id()));
    std::fs::write(&path, responses.join("\n")).unwrap();
    ServerConfig {
        command: "sh".into(),
        args: vec!["-c".into(), format!("cat '{}'; sleep 5", path.display())],
        env: BTreeMap::new(),
    }
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake"}}}"#;
const TOOLS: &str = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo text back.\nSecond line ignored.","inputSchema":{}}]}}"#;
const CALL_OK: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"hello"},{"type":"text","text":"world"}],"isError":false}}"#;
const CALL_ERR: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"boom"}],"isError":true}}"#;

#[test]
fn connect_lists_tools_and_calls_one() {
    let cfg = canned("ok", &[INIT, TOOLS, CALL_OK]);
    let mut c = McpClient::connect(&cfg).unwrap();
    let tools = c.tools().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].0, "echo");
    assert_eq!(tools[0].1, "Echo text back."); // first line only
    let out = c.call("echo", serde_json::json!({"text": "hi"})).unwrap();
    assert_eq!(out, "hello\nworld");
}

#[test]
fn an_is_error_result_becomes_err() {
    let cfg = canned("err", &[INIT, TOOLS, CALL_ERR]);
    let mut c = McpClient::connect(&cfg).unwrap();
    c.tools().unwrap();
    let e = c.call("echo", serde_json::json!({})).unwrap_err();
    assert_eq!(e, "boom");
}

#[test]
fn connect_fails_cleanly_when_the_command_is_missing() {
    let cfg = ServerConfig {
        command: "no-such-mcp-server-xyz".into(),
        args: vec![],
        env: BTreeMap::new(),
    };
    assert!(McpClient::connect(&cfg)
        .unwrap_err()
        .contains("failed to launch"));
}

#[test]
fn host_lists_tools_calls_and_reports() {
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("host", &[INIT, TOOLS, CALL_OK]));
    let mut host = McpHost::new(servers);
    assert!(!host.is_empty());
    let tools = host.tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        (tools[0].server.as_str(), tools[0].name.as_str()),
        ("fake", "echo")
    );
    // The tool list is cached — a second read costs no request.
    assert_eq!(host.tools().len(), 1);
    assert_eq!(
        host.call("fake", "echo", r#"{"text":"hi"}"#).unwrap(),
        "hello\nworld"
    );
    let report = host.report();
    assert!(
        report.contains("fake") && report.contains("echo"),
        "got: {report}"
    );
}

#[test]
fn host_rejects_unknown_servers_and_bad_args() {
    let mut host = McpHost::new(BTreeMap::new());
    assert!(host.is_empty());
    assert!(host
        .call("ghost", "t", "{}")
        .unwrap_err()
        .contains("unknown MCP server"));
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("badargs", &[INIT]));
    let mut host = McpHost::new(servers);
    let e = host.call("fake", "echo", "{not json").unwrap_err();
    assert!(e.contains("not valid JSON"), "got: {e}");
}

#[test]
fn host_report_names_a_server_that_fails_to_launch() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "broken".to_string(),
        ServerConfig {
            command: "no-such-mcp-server-xyz".into(),
            args: vec![],
            env: BTreeMap::new(),
        },
    );
    let mut host = McpHost::new(servers);
    assert!(host.tools().is_empty());
    let report = host.report();
    assert!(
        report.contains("broken") && report.contains("error"),
        "got: {report}"
    );
}
