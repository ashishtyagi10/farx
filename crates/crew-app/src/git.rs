//! Sidebar git section: a `GIT` divider above the current branch and a clean/
//! dirty marker for Crew's working directory. Queried with the `git` CLI (so it
//! honours the user's full git config) and cached/throttled by `StatsPane`.
use std::path::Path;
use std::process::Command;

use crew_render::CellView;

use crate::boxdraw::section_header;

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const LABEL: (u8, u8, u8) = (200, 200, 200);
const DIM: (u8, u8, u8) = (150, 150, 160);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const DIRTY: (u8, u8, u8) = (230, 180, 90);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Branch name and whether the working tree has uncommitted changes.
#[derive(Clone, PartialEq, Eq)]
pub struct GitInfo {
    pub branch: String,
    pub dirty: bool,
}

/// Query `git` for `dir`'s branch and dirty state, or `None` when `dir` isn't a
/// repository (or `git` isn't installed).
pub fn query(dir: &Path) -> Option<GitInfo> {
    let branch = run(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch.is_empty() {
        return None;
    }
    let dirty = !run(dir, &["status", "--porcelain"])
        .unwrap_or_default()
        .is_empty();
    Some(GitInfo { branch, dirty })
}

/// Run `git -C dir <args>`, returning trimmed stdout on success.
fn run(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Render the GIT section: a `GIT` rule on row 0, the branch on row 1, and a
/// clean/dirty marker on row 2.
pub fn git_cells(info: &GitInfo, cols: u16) -> Vec<CellView> {
    let mut out = section_header("GIT", cols, BORDER, ACCENT, BG);
    put(&mut out, &info.branch, 1, cols, LABEL);
    let (marker, fg) = if info.dirty {
        ("● uncommitted", DIRTY)
    } else {
        ("✓ clean", DIM)
    };
    put(&mut out, marker, 2, cols, fg);
    out
}

/// Draw `s` at `row`, indented to align under the section legend, clipped to `cols`.
fn put(out: &mut Vec<CellView>, s: &str, row: u16, cols: u16, fg: (u8, u8, u8)) {
    let max = cols.saturating_sub(4) as usize;
    for (i, c) in s.chars().take(max).enumerate() {
        out.push(CellView {
            col: 3 + i as u16,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_non_repo_is_none() {
        let dir = std::env::temp_dir().join("crew_git_not_a_repo");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(query(&dir).is_none());
    }

    #[test]
    fn git_cells_show_branch_and_marker() {
        let info = GitInfo {
            branch: "main".into(),
            dirty: true,
        };
        let cells = git_cells(&info, 24);
        // GIT divider on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'G' && c.row == 0));
        // branch on row 1
        assert!(cells.iter().any(|c| c.c == 'm' && c.row == 1));
        // dirty marker (amber) on row 2
        assert!(cells
            .iter()
            .any(|c| c.c == '●' && c.row == 2 && c.fg == DIRTY));
    }

    #[test]
    fn git_cells_clean_marker() {
        let info = GitInfo {
            branch: "dev".into(),
            dirty: false,
        };
        let cells = git_cells(&info, 24);
        assert!(cells.iter().any(|c| c.c == '✓' && c.row == 2));
    }
}
