use ratatui::layout::Direction;

/// What occupies a leaf node in the panel tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelLeaf {
    /// A file browser panel (Left or Right).
    FilePanel(crate::PanelSide),
    /// An embedded terminal session (index into App's terminals vec).
    Terminal(usize),
}

/// Recursive layout tree for panel splitting.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A single panel (file browser or terminal).
    Leaf(PanelLeaf),
    /// Two panels split in a direction.
    Split {
        direction: Direction,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    /// Create the default two-panel layout.
    pub fn default_layout() -> Self {
        LayoutNode::Split {
            direction: Direction::Horizontal,
            first: Box::new(LayoutNode::Leaf(PanelLeaf::FilePanel(
                crate::PanelSide::Left,
            ))),
            second: Box::new(LayoutNode::Leaf(PanelLeaf::FilePanel(
                crate::PanelSide::Right,
            ))),
        }
    }
}
