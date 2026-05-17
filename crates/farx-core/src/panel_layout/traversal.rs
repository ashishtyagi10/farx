use super::types::{LayoutNode, PanelLeaf};
use ratatui::layout::{Constraint, Layout, Rect};

impl LayoutNode {
    /// Collect all leaf nodes in order (left-to-right, top-to-bottom).
    pub fn leaves(&self) -> Vec<PanelLeaf> {
        let mut result = Vec::new();
        self.collect_leaves(&mut result);
        result
    }

    fn collect_leaves(&self, out: &mut Vec<PanelLeaf>) {
        match self {
            LayoutNode::Leaf(leaf) => out.push(*leaf),
            LayoutNode::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    /// Compute the Rect for each leaf node given the total available area.
    pub fn compute_rects(&self, area: Rect) -> Vec<(PanelLeaf, Rect)> {
        let mut result = Vec::new();
        self.compute_rects_inner(area, &mut result);
        result
    }

    fn compute_rects_inner(&self, area: Rect, out: &mut Vec<(PanelLeaf, Rect)>) {
        match self {
            LayoutNode::Leaf(leaf) => {
                out.push((*leaf, area));
            }
            LayoutNode::Split {
                direction,
                first,
                second,
            } => {
                let chunks = Layout::default()
                    .direction(*direction)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);
                first.compute_rects_inner(chunks[0], out);
                second.compute_rects_inner(chunks[1], out);
            }
        }
    }
}
