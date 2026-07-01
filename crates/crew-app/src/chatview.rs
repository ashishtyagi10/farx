//! Composes the crew pane's full cell view: status header (row 0), agent
//! roster (row 1 when known), role-styled message cards, and the input
//! composer (affordance bar + prompt) on the bottom rows. Tiny panes fall
//! back to the plain layout.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::{layout_cells, CONNECTING_HINT, READY_HINT};

/// Render `pane` into a `cols` × `rows` grid.
pub(crate) fn cells(pane: &ChatPane, cols: u16, rows: u16) -> Vec<CellView> {
    let top = pane.top_rows(rows);
    if top == 0 {
        return layout_cells(
            &pane.messages,
            &pane.input,
            cols,
            rows,
            pane.scroll,
            pane.connected,
        );
    }
    let active = pane.active_status();
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.messages.len(),
        pane.is_busy(),
        active,
        pane.tokens,
    );
    if top > 1 {
        cells.extend(crate::chatroster::roster_cells(
            cols,
            1,
            &pane.agents,
            active.map(|(a, _)| a),
        ));
    }
    let bottom = crate::chatinput::composer_rows(rows);
    if pane.messages.is_empty() {
        cells.extend(hint_cells(pane.connected, cols, top));
    } else {
        let msg_rows = rows.saturating_sub(top + bottom);
        cells.extend(crate::chatmsgs::message_cells(
            &pane.messages,
            cols,
            msg_rows,
            top,
            pane.scroll,
        ));
    }
    cells.extend(crate::chatinput::composer_cells(
        &pane.input,
        &pane.agents,
        cols,
        rows,
    ));
    cells
}

/// The dim one-line hint shown while the pane has no messages yet.
fn hint_cells(connected: bool, cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let hint = if connected {
        READY_HINT
    } else {
        CONNECTING_HINT
    };
    hint.chars()
        .take(cols as usize)
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row,
            c,
            fg: t.hint_fg,
            bg: t.page_bg,
            bold: false,
            italic: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_names_the_state() {
        let ready: String = hint_cells(true, 80, 2).iter().map(|c| c.c).collect();
        assert_eq!(ready, READY_HINT);
        let conn: String = hint_cells(false, 80, 2).iter().map(|c| c.c).collect();
        assert_eq!(conn, CONNECTING_HINT);
        assert!(hint_cells(true, 80, 2).iter().all(|c| c.row == 2));
    }

    #[test]
    fn hint_clips_to_width() {
        assert_eq!(hint_cells(true, 5, 0).len(), 5);
    }
}
