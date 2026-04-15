use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use crate::tree::TreeState;

/// A group of tabs, each containing a TreeState.
/// Derefs to the active tab's TreeState for transparent usage.
pub struct TabGroup {
    tabs: Vec<TreeState>,
    active: usize,
}

impl TabGroup {
    /// Create a new TabGroup with a single tab.
    pub fn new(tree: TreeState) -> Self {
        Self {
            tabs: vec![tree],
            active: 0,
        }
    }

    /// Number of tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Active tab index.
    pub fn active_tab(&self) -> usize {
        self.active
    }

    /// Get tab labels: (directory_name, is_active).
    pub fn tab_labels(&self) -> Vec<(String, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let name = t
                    .root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| t.root.to_string_lossy().to_string());
                (name, i == self.active)
            })
            .collect()
    }

    /// Open a new tab at the given directory.
    pub fn new_tab(&mut self, root: PathBuf, show_hidden: bool) {
        let mut tree = TreeState::new(root);
        tree.show_hidden = show_hidden;
        tree.rebuild();
        self.tabs.push(tree);
        self.active = self.tabs.len() - 1;
    }

    /// Close the active tab. Returns false if this is the last tab (won't close).
    pub fn close_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        true
    }

    /// Switch to a tab by index.
    pub fn switch_to(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// Switch to the next tab (wraps around).
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    /// Switch to the previous tab (wraps around).
    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active = if self.active == 0 {
                self.tabs.len() - 1
            } else {
                self.active - 1
            };
        }
    }
}

impl Deref for TabGroup {
    type Target = TreeState;
    fn deref(&self) -> &TreeState {
        &self.tabs[self.active]
    }
}

impl DerefMut for TabGroup {
    fn deref_mut(&mut self) -> &mut TreeState {
        &mut self.tabs[self.active]
    }
}
