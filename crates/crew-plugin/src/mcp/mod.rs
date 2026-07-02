//! A minimal MCP (Model Context Protocol) client: stdio transport,
//! line-delimited JSON-RPC 2.0. Servers are declared in `mcp.json` (the same
//! `mcpServers` schema other coding tools use), connected lazily, and exposed
//! to the `/crew` relay as callable tools.
mod client;
mod config;

use std::collections::BTreeMap;

pub use client::McpClient;
pub use config::ServerConfig;

/// One callable tool on a connected server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTool {
    pub server: String,
    pub name: String,
    pub description: String,
}

/// All configured MCP servers: lazy connections plus a per-server tool cache,
/// so listing tools costs one `tools/list` per server per session.
#[derive(Default)]
pub struct McpHost {
    servers: BTreeMap<String, ServerConfig>,
    clients: BTreeMap<String, McpClient>,
    cache: BTreeMap<String, Vec<McpTool>>,
}

impl McpHost {
    /// A host over an explicit server map (used by tests).
    pub fn new(servers: BTreeMap<String, ServerConfig>) -> Self {
        Self {
            servers,
            ..Self::default()
        }
    }

    /// A host over the merged `mcp.json` config. Empty under
    /// `CREW_BROKER_MOCK_REPLY` so mock-driven broker tests stay deterministic
    /// on machines that have real servers configured.
    pub fn from_config() -> Self {
        if std::env::var("CREW_BROKER_MOCK_REPLY").is_ok() {
            return Self::default();
        }
        Self::new(config::load())
    }

    /// Whether any server is configured at all.
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    /// The connected client for `server`, connecting on first use.
    fn client(&mut self, server: &str) -> Result<&mut McpClient, String> {
        let Some(cfg) = self.servers.get(server) else {
            let known: Vec<&str> = self.servers.keys().map(|s| s.as_str()).collect();
            return Err(format!(
                "unknown MCP server \u{201c}{server}\u{201d} \u{2014} configured: {}",
                if known.is_empty() {
                    "(none)".into()
                } else {
                    known.join(", ")
                }
            ));
        };
        if !self.clients.contains_key(server) {
            let c = McpClient::connect(cfg)?;
            self.clients.insert(server.to_string(), c);
        }
        Ok(self.clients.get_mut(server).expect("just inserted"))
    }

    /// One server's tools, fetched once and cached for the session.
    fn fetch(&mut self, server: &str) -> Result<Vec<McpTool>, String> {
        if let Some(t) = self.cache.get(server) {
            return Ok(t.clone());
        }
        let list = self.client(server)?.tools()?;
        let tools: Vec<McpTool> = list
            .into_iter()
            .map(|(name, description)| McpTool {
                server: server.to_string(),
                name,
                description,
            })
            .collect();
        self.cache.insert(server.to_string(), tools.clone());
        Ok(tools)
    }

    /// Every tool on every configured server (cached after the first fetch).
    /// A server that fails to connect or list contributes nothing here — the
    /// failure is visible in [`McpHost::report`].
    pub fn tools(&mut self) -> Vec<McpTool> {
        let names: Vec<String> = self.servers.keys().cloned().collect();
        names
            .iter()
            .flat_map(|s| self.fetch(s).unwrap_or_default())
            .collect()
    }

    /// Call `tool` on `server` with JSON `args` (empty = `{}`). A failed call
    /// drops the connection so the next call reconnects fresh.
    pub fn call(&mut self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        let args = args.trim();
        let value: serde_json::Value = if args.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(args)
                .map_err(|e| format!("tool arguments are not valid JSON: {e}"))?
        };
        let res = self.client(server)?.call(tool, value);
        if res.is_err() {
            self.clients.remove(server);
        }
        res
    }

    /// The `/mcp` listing: each server with its tools, or its failure.
    pub fn report(&mut self) -> String {
        if self.is_empty() {
            return "No MCP servers configured. Declare them in \
                    ~/.config/crew/mcp.json or ./.crew/mcp.json as \
                    {\"mcpServers\": {\"name\": {\"command\": \"\u{2026}\", \
                    \"args\": [\u{2026}]}}} \u{2014} agents can then call their \
                    tools mid-relay."
                .into();
        }
        let names: Vec<String> = self.servers.keys().cloned().collect();
        let mut lines = Vec::new();
        for server in names {
            match self.fetch(&server) {
                Ok(list) => {
                    let tools: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
                    lines.push(format!(
                        "\u{25aa} {server} \u{2014} {} tool(s): {}",
                        tools.len(),
                        tools.join(", ")
                    ));
                }
                Err(e) => lines.push(format!("\u{25aa} {server} \u{2014} error: {e}")),
            }
        }
        lines.join("\n")
    }
}
