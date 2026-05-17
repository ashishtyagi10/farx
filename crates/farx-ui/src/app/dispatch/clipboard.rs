//! Clipboard actions: copy current/selected paths or names.

use farx_core::Action;

use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_clipboard(&mut self, action: &Action) -> bool {
        match action {
            Action::CopyPathToClipboard => self.copy_paths_to_clipboard(),
            Action::CopyNameToClipboard => self.copy_names_to_clipboard(),
            _ => return false,
        }
        true
    }

    fn copy_paths_to_clipboard(&mut self) {
        let tree = self.active_tree_ref();
        let paths: Vec<String> = if !tree.selected.is_empty() {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.path.to_string_lossy().to_string())
                .collect()
        } else if let Some(node) = tree.current_node() {
            vec![node.entry.path.to_string_lossy().to_string()]
        } else {
            Vec::new()
        };
        if paths.is_empty() {
            return;
        }
        self.set_clipboard_text(&paths, "Copied", "path", "paths");
    }

    fn copy_names_to_clipboard(&mut self) {
        let tree = self.active_tree_ref();
        let names: Vec<String> = if !tree.selected.is_empty() {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.name.clone())
                .collect()
        } else if let Some(node) = tree.current_node() {
            vec![node.entry.name.clone()]
        } else {
            Vec::new()
        };
        if names.is_empty() {
            return;
        }
        self.set_clipboard_text(&names, "Copied name", "name", "names");
    }

    fn set_clipboard_text(&mut self, items: &[String], verb: &str, single: &str, plural: &str) {
        let text = items.join("\n");
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&text) {
                Ok(()) => {
                    if items.len() == 1 {
                        self.feedback.info(format!("{}: {}", verb, items[0]));
                    } else {
                        let _ = single;
                        self.feedback
                            .info(format!("{} {} {}", verb, items.len(), plural));
                    }
                }
                Err(e) => self.feedback.error(format!("Clipboard: {}", e)),
            },
            Err(e) => self.feedback.error(format!("Clipboard: {}", e)),
        }
    }
}
