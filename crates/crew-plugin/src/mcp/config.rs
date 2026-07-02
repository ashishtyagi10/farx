//! `mcp.json` — which MCP servers exist and how to launch them. The schema is
//! the `mcpServers` map other coding tools already use, so a config can be
//! copied across verbatim. Merged from `~/.config/crew/mcp.json` (user) and
//! `./.crew/mcp.json` (project; wins on a name collision).
use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// How to launch one stdio MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment for the server process (merged over the broker's).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct McpFile {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, ServerConfig>,
}

/// Parse one `mcp.json`; unreadable or malformed content is an empty map.
pub(crate) fn parse(text: &str) -> BTreeMap<String, ServerConfig> {
    serde_json::from_str::<McpFile>(text)
        .map(|f| f.mcp_servers)
        .unwrap_or_default()
}

fn load_file(path: &Path) -> BTreeMap<String, ServerConfig> {
    std::fs::read_to_string(path)
        .map(|t| parse(&t))
        .unwrap_or_default()
}

/// The merged server map: user config first, project entries on top.
pub(crate) fn load() -> BTreeMap<String, ServerConfig> {
    let mut all = dirs::config_dir()
        .map(|d| load_file(&d.join("crew").join("mcp.json")))
        .unwrap_or_default();
    all.extend(load_file(Path::new(".crew/mcp.json")));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_the_mcp_servers_map() {
        let m = parse(
            r#"{"mcpServers":{"fs":{"command":"mcp-fs","args":["--root","."],"env":{"K":"v"}}}}"#,
        );
        let fs = &m["fs"];
        assert_eq!(fs.command, "mcp-fs");
        assert_eq!(fs.args, vec!["--root", "."]);
        assert_eq!(fs.env["K"], "v");
    }

    #[test]
    fn parse_defaults_args_and_env() {
        let m = parse(r#"{"mcpServers":{"x":{"command":"srv"}}}"#);
        assert!(m["x"].args.is_empty());
        assert!(m["x"].env.is_empty());
    }

    #[test]
    fn parse_of_garbage_or_empty_is_empty() {
        assert!(parse("not json").is_empty());
        assert!(parse("{}").is_empty());
    }

    #[test]
    fn load_file_of_missing_path_is_empty() {
        assert!(load_file(Path::new("/nonexistent/mcp.json")).is_empty());
    }
}
