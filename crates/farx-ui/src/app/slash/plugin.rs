//! Plugin slash commands: `/plugin` (list or run) plus the fallback that
//! treats an unknown `/foo` as a candidate plugin command.

use super::super::App;

impl App {
    /// Handle `/plugin` and the fallback case for plugin-defined commands.
    /// Returns `true` if the command was recognised (always true for `/plugin`,
    /// only true for unknown commands if a plugin claims them).
    pub(super) fn slash_plugin_or_unknown(&mut self, cmd: &str, args: &str) -> bool {
        if cmd == "/plugin" {
            self.slash_plugin(args);
            return true;
        }
        self.try_plugin_command(cmd)
    }

    fn slash_plugin(&mut self, args: &str) {
        if args.is_empty() {
            if let Some(ref engine) = self.plugin_engine {
                let cmds = engine.list_commands();
                if cmds.is_empty() {
                    self.feedback.info(
                        "No plugins loaded. Place .lua files in ~/.config/farx/plugins/"
                            .to_string(),
                    );
                } else {
                    let lines: Vec<String> = cmds
                        .iter()
                        .map(|c| format!("  /{} — {} ({})", c.name, c.description, c.plugin_file))
                        .collect();
                    self.feedback.show_output("Plugins", lines.join("\n"));
                }
            } else {
                self.feedback
                    .error("Plugin engine not available".to_string());
            }
            return;
        }
        let dir = self.active_tree_ref().root.to_string_lossy().to_string();
        if let Some(ref engine) = self.plugin_engine {
            match engine.execute_command(args, &dir) {
                Ok(farx_plugin::PluginResult::Message(msg)) => self.feedback.info(msg),
                Ok(farx_plugin::PluginResult::Shell(cmd)) => {
                    self.command_line.input = cmd;
                    self.smart_execute_command();
                }
                Ok(farx_plugin::PluginResult::None) => {}
                Err(e) => self.feedback.error(format!("Plugin: {}", e)),
            }
        }
    }

    fn try_plugin_command(&mut self, cmd: &str) -> bool {
        let plugin_cmd = cmd.trim_start_matches('/');
        let has_command = self
            .plugin_engine
            .as_ref()
            .map(|e| e.has_command(plugin_cmd))
            .unwrap_or(false);
        if !has_command {
            return false;
        }
        let dir = self.active_tree_ref().root.to_string_lossy().to_string();
        if let Some(ref engine) = self.plugin_engine {
            match engine.execute_command(plugin_cmd, &dir) {
                Ok(farx_plugin::PluginResult::Message(msg)) => self.feedback.info(msg),
                Ok(farx_plugin::PluginResult::Shell(shell_cmd)) => {
                    self.command_line.input = shell_cmd;
                    self.smart_execute_command();
                }
                Ok(farx_plugin::PluginResult::None) => {}
                Err(e) => self.feedback.error(format!("Plugin: {}", e)),
            }
        }
        true
    }
}
