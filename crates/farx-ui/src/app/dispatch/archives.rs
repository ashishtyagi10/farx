//! Archive and remote operations: list/extract/compress + SSH browse.

use farx_core::Action;

use super::super::helpers::format_size_human;
use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_archives(&mut self, action: &Action) -> bool {
        match action {
            Action::ViewArchive => self.view_archive(),
            Action::ExtractArchive => self.extract_archive(),
            Action::CompressSelection => self.compress_selection(),
            Action::SshBrowse(target) => self.ssh_browse(target.clone()),
            _ => return false,
        }
        true
    }

    fn view_archive(&mut self) {
        let Some(node) = self.active_tree_ref().current_node() else {
            return;
        };
        let path = node.entry.path.clone();
        let title_name = node.entry.name.clone();
        match farx_fs::list_archive(&path) {
            Ok(entries) => {
                let lines: Vec<String> = entries
                    .iter()
                    .map(|e| {
                        if e.is_dir {
                            format!("[DIR]  {}", e.name)
                        } else {
                            format!("{:>8}  {}", format_size_human(e.size), e.name)
                        }
                    })
                    .collect();
                let title = format!("Archive: {} ({} entries)", title_name, entries.len());
                self.feedback.show_output(&title, lines.join("\n"));
            }
            Err(e) => self.feedback.error(format!("Archive: {}", e)),
        }
    }

    fn extract_archive(&mut self) {
        let Some(node) = self.active_tree_ref().current_node() else {
            return;
        };
        let path = node.entry.path.clone();
        if !farx_fs::is_archive(&path) {
            self.feedback.error("Not a supported archive".to_string());
            return;
        }
        let dest = self.inactive_tree_root();
        match farx_fs::extract_archive(&path, &dest) {
            Ok(count) => {
                self.feedback
                    .success(format!("Extracted {} entries to {}", count, dest.display()));
                self.left_tree.rebuild();
                self.right_tree.rebuild();
            }
            Err(e) => self.feedback.error(format!("Extract: {}", e)),
        }
    }

    fn compress_selection(&mut self) {
        let paths = self.collect_selected_paths();
        if paths.is_empty() {
            self.feedback
                .error("No files selected to compress".to_string());
            return;
        }
        let names = self.collect_selected_names();
        let archive_name = if names.len() == 1 {
            format!("{}.zip", names[0])
        } else {
            "archive.zip".to_string()
        };
        let dest = self.active_tree_ref().root.join(&archive_name);
        let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
        match farx_fs::compress_to_zip(&refs, &dest) {
            Ok(count) => {
                self.feedback
                    .success(format!("Compressed {} files to {}", count, archive_name));
                self.active_tree().rebuild();
            }
            Err(e) => self.feedback.error(format!("Compress: {}", e)),
        }
    }

    fn ssh_browse(&mut self, target: String) {
        let (host_part, remote_path) = if let Some(idx) = target.find(':') {
            (&target[..idx], &target[idx + 1..])
        } else {
            (target.as_str(), "~")
        };

        self.feedback
            .info(format!("Connecting to {}...", host_part));

        let cmd = format!(
            "ssh -o ConnectTimeout=5 -o BatchMode=yes {} ls -lahF {}",
            host_part, remote_path
        );
        let output = std::process::Command::new("sh").args(["-c", &cmd]).output();

        match output {
            Ok(out) if out.status.success() => {
                let listing = String::from_utf8_lossy(&out.stdout).to_string();
                let title = format!("SSH: {}:{}", host_part, remote_path);
                self.feedback.show_output(&title, listing);
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                self.feedback
                    .error(format!("SSH failed: {}", stderr.trim()));
            }
            Err(e) => self.feedback.error(format!("SSH: {}", e)),
        }
    }
}
