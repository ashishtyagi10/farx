use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// The function-key labels shown at the bottom of the screen.
const FN_ITEMS: &[(u8, &str)] = &[
    (1, "Help"),
    (2, "Menu"),
    (3, "View"),
    (4, "Edit"),
    (5, "Copy"),
    (6, "Move"),
    (7, "MkDir"),
    (8, "Del"),
    (9, "Menu"),
    (10, "Quit"),
];

/// Render the function key bar.
/// Format: " F1 Help  F2 Menu  F3 View ..." with key highlighted and label in normal style.
pub fn render_fn_bar(frame: &mut Frame, area: Rect, theme: &Theme) {
    let total_width = area.width as usize;
    let item_count = FN_ITEMS.len();
    let slot_width = if item_count > 0 {
        total_width / item_count
    } else {
        0
    };

    let mut spans: Vec<Span<'_>> = Vec::with_capacity(item_count * 3);

    for (i, &(num, label)) in FN_ITEMS.iter().enumerate() {
        let key_text = format!(" F{} ", num);
        let label_text = format!("{} ", label);

        let this_slot = if i < item_count - 1 {
            slot_width
        } else {
            total_width.saturating_sub(slot_width * (item_count - 1))
        };

        // Key + label both highlighted
        spans.push(Span::styled(key_text.clone(), theme.fn_bar_key));
        spans.push(Span::styled(label_text.clone(), theme.fn_bar_key));

        // Gap between items
        let used = key_text.len() + label_text.len();
        let gap = this_slot.saturating_sub(used);
        if gap > 0 {
            spans.push(Span::styled(
                " ".repeat(gap),
                Style::default().bg(theme.fn_bar_bg),
            ));
        }
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
