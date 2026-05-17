//! Read-only analysis: file stats, checksums, duplicate scan, treemap,
//! dir size.

use farx_core::Action;

use super::super::helpers::format_size_human;
use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_analysis(&mut self, action: &Action) -> bool {
        match action {
            Action::ShowFileStats => self.show_file_stats(),
            Action::ShowChecksums => self.show_checksums(),
            Action::FindDuplicates => self.find_duplicates(),
            Action::ShowTreemap => self.show_treemap(),
            Action::CalculateDirSize => self.calculate_dir_size(),
            _ => return false,
        }
        true
    }

    fn show_file_stats(&mut self) {
        let Some(node) = self.active_tree_ref().current_node() else {
            return;
        };
        if node.entry.is_dir || node.entry.name == ".." {
            self.feedback.info("Select a file for stats".to_string());
            return;
        }
        let path = node.entry.path.clone();
        let name = node.entry.name.clone();
        match std::fs::read(&path) {
            Ok(bytes) => {
                let size = bytes.len();
                let is_text = !bytes[..size.min(512)].contains(&0);
                let mut lines = Vec::new();
                lines.push(format!("File: {}", name));
                lines.push(format!("Size: {} bytes", size));
                if is_text {
                    let text = String::from_utf8_lossy(&bytes);
                    let line_count = text.lines().count();
                    let word_count: usize =
                        text.lines().map(|l| l.split_whitespace().count()).sum();
                    let char_count = text.chars().count();
                    lines.push(format!("Lines: {}", line_count));
                    lines.push(format!("Words: {}", word_count));
                    lines.push(format!("Characters: {}", char_count));
                } else {
                    lines.push("Binary file".to_string());
                }
                self.feedback
                    .show_output("File Statistics", lines.join("\n"));
            }
            Err(e) => self.feedback.error(format!("Stats: {}", e)),
        }
    }

    fn show_checksums(&mut self) {
        let tree = self.active_tree_ref();
        let files: Vec<(String, std::path::PathBuf)> = if !tree.selected.is_empty() {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .filter(|n| !n.entry.is_dir)
                .map(|n| (n.entry.name.clone(), n.entry.path.clone()))
                .collect()
        } else if let Some(node) = tree.current_node() {
            if !node.entry.is_dir {
                vec![(node.entry.name.clone(), node.entry.path.clone())]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        if files.is_empty() {
            self.feedback.error("No files selected".to_string());
            return;
        }
        use sha2::{Digest, Sha256};
        let mut lines = Vec::new();
        for (name, path) in &files {
            match std::fs::read(path) {
                Ok(data) => {
                    let mut hasher = Sha256::new();
                    hasher.update(&data);
                    let hash = format!("{:x}", hasher.finalize());
                    lines.push(format!("SHA-256: {}", hash));
                    lines.push(format!("  File: {}", name));
                    lines.push(format!("  Size: {}", format_size_human(data.len() as u64)));
                    lines.push(String::new());
                }
                Err(e) => {
                    lines.push(format!("{}: error — {}", name, e));
                    lines.push(String::new());
                }
            }
        }
        self.feedback.show_output("Checksums", lines.join("\n"));
    }

    fn find_duplicates(&mut self) {
        let root = self.active_tree_ref().root.clone();
        self.feedback.info("Scanning for duplicates...".to_string());
        match farx_fs::find_duplicates(&root, 8) {
            Ok(groups) => {
                if groups.is_empty() {
                    self.feedback.info("No duplicate files found".to_string());
                    return;
                }
                let total_waste: u64 = groups
                    .iter()
                    .map(|g| g.size * (g.paths.len() as u64 - 1))
                    .sum();
                let mut lines = Vec::new();
                lines.push(format!(
                    "{} duplicate groups, {} reclaimable",
                    groups.len(),
                    format_size_human(total_waste)
                ));
                lines.push(String::new());
                for (i, group) in groups.iter().take(50).enumerate() {
                    lines.push(format!(
                        "Group {} — {} each, {} copies:",
                        i + 1,
                        format_size_human(group.size),
                        group.paths.len()
                    ));
                    for path in &group.paths {
                        lines.push(format!("  {}", path.display()));
                    }
                    lines.push(String::new());
                }
                self.feedback
                    .show_output("Duplicate Files", lines.join("\n"));
            }
            Err(e) => self.feedback.error(format!("Duplicates: {}", e)),
        }
    }
}
