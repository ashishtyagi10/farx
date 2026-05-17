#[cfg(test)]
mod tests {
    use super::super::{LayoutNode, PanelLeaf};
    use crate::PanelSide;
    use ratatui::layout::{Direction, Rect};

    #[test]
    fn default_layout_has_two_file_panels() {
        let layout = LayoutNode::default_layout();
        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );
    }

    #[test]
    fn split_leaf_adds_terminal_and_alternates_direction() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 7));

        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::Terminal(7),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );

        match &layout {
            LayoutNode::Split { first, .. } => match first.as_ref() {
                LayoutNode::Split { direction, .. } => {
                    assert_eq!(*direction, Direction::Vertical);
                }
                _ => panic!("expected first child to be split after splitting first leaf"),
            },
            _ => panic!("expected root split"),
        }
    }

    #[test]
    fn remove_terminal_collapses_split() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 2));
        assert!(layout.remove_terminal(2));

        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );
    }

    #[test]
    fn adjust_terminal_ids_shifts_higher_ids() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 1));
        assert!(layout.split_leaf(1, 3));
        layout.adjust_terminal_ids(1);

        let leaves = layout.leaves();
        assert!(leaves.contains(&PanelLeaf::Terminal(2)));
        assert!(!leaves.contains(&PanelLeaf::Terminal(3)));
    }

    #[test]
    fn compute_rects_returns_area_for_each_leaf() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(1, 5));

        let rects = layout.compute_rects(Rect::new(0, 0, 120, 40));
        assert_eq!(rects.len(), layout.leaves().len());
        assert!(rects.iter().all(|(_, r)| r.width > 0 && r.height > 0));
    }
}
