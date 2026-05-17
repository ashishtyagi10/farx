use super::types::{LayoutNode, PanelLeaf};
use ratatui::layout::Direction;

impl LayoutNode {
    /// Split the leaf at the given index, adding a new terminal.
    /// The new split alternates direction: H -> V -> H -> V...
    /// Returns true if the split was performed.
    pub fn split_leaf(&mut self, leaf_index: usize, terminal_id: usize) -> bool {
        let mut counter = 0usize;
        // The root split is Horizontal, so first child split should be Vertical
        let parent_dir = match self {
            LayoutNode::Split { direction, .. } => Some(*direction),
            LayoutNode::Leaf(_) => None,
        };
        self.split_leaf_inner(leaf_index, terminal_id, &mut counter, parent_dir)
    }

    fn split_leaf_inner(
        &mut self,
        target: usize,
        terminal_id: usize,
        counter: &mut usize,
        parent_dir: Option<Direction>,
    ) -> bool {
        match self {
            LayoutNode::Leaf(_) => {
                if *counter == target {
                    // Alternate direction from parent
                    let new_dir = match parent_dir {
                        Some(Direction::Horizontal) => Direction::Vertical,
                        _ => Direction::Horizontal,
                    };

                    let original = std::mem::replace(
                        self,
                        LayoutNode::Leaf(PanelLeaf::Terminal(0)), // placeholder
                    );
                    *self = LayoutNode::Split {
                        direction: new_dir,
                        first: Box::new(original),
                        second: Box::new(LayoutNode::Leaf(PanelLeaf::Terminal(terminal_id))),
                    };
                    true
                } else {
                    *counter += 1;
                    false
                }
            }
            LayoutNode::Split {
                direction,
                first,
                second,
            } => {
                let dir = Some(*direction);
                if first.split_leaf_inner(target, terminal_id, counter, dir) {
                    return true;
                }
                second.split_leaf_inner(target, terminal_id, counter, dir)
            }
        }
    }

    /// Remove a terminal leaf from the tree, collapsing the split.
    /// Returns true if the terminal was found and removed.
    pub fn remove_terminal(&mut self, terminal_id: usize) -> bool {
        self.remove_terminal_inner(terminal_id)
    }

    fn remove_terminal_inner(&mut self, terminal_id: usize) -> bool {
        match self {
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split { first, second, .. } => {
                // Check if first child is the target terminal
                if matches!(first.as_ref(), LayoutNode::Leaf(PanelLeaf::Terminal(id)) if *id == terminal_id)
                {
                    *self = *second.clone();
                    return true;
                }
                // Check if second child is the target terminal
                if matches!(second.as_ref(), LayoutNode::Leaf(PanelLeaf::Terminal(id)) if *id == terminal_id)
                {
                    *self = *first.clone();
                    return true;
                }
                // Recurse
                if first.remove_terminal_inner(terminal_id) {
                    return true;
                }
                second.remove_terminal_inner(terminal_id)
            }
        }
    }

    /// Update terminal IDs after a terminal is removed (shift IDs down).
    pub fn adjust_terminal_ids(&mut self, removed_id: usize) {
        match self {
            LayoutNode::Leaf(PanelLeaf::Terminal(id)) => {
                if *id > removed_id {
                    *id -= 1;
                }
            }
            LayoutNode::Leaf(_) => {}
            LayoutNode::Split { first, second, .. } => {
                first.adjust_terminal_ids(removed_id);
                second.adjust_terminal_ids(removed_id);
            }
        }
    }
}
