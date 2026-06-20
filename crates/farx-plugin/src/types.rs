use std::path::PathBuf;

/// A registered plugin command.
#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub plugin_file: String,
}

/// Result of executing a plugin command.
#[derive(Debug, Clone)]
pub enum PluginResult {
    /// Display a message to the user.
    Message(String),
    /// Execute a shell command and show output.
    Shell(String),
    /// No visible output.
    None,
}

pub(crate) fn lua_err(e: mlua::Error) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

pub fn plugin_directory() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("farx")
        .join("plugins")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_directory_ends_with_farx_plugins() {
        let dir = plugin_directory();
        assert!(dir.ends_with("farx/plugins"));
    }

    #[test]
    fn lua_err_wraps_message() {
        let err = lua_err(mlua::Error::RuntimeError("boom".to_string()));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn plugin_result_and_command_construct() {
        let cmd = PluginCommand {
            name: "hello".to_string(),
            description: "say hi".to_string(),
            plugin_file: "hello.lua".to_string(),
        };
        assert_eq!(cmd.name, "hello");
        match PluginResult::Message("m".to_string()) {
            PluginResult::Message(m) => assert_eq!(m, "m"),
            _ => panic!("wrong variant"),
        }
        assert!(matches!(PluginResult::None, PluginResult::None));
        assert!(matches!(
            PluginResult::Shell("ls".into()),
            PluginResult::Shell(_)
        ));
    }
}
