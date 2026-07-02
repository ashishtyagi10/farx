//! Settings form rendering: a scrollable one-row-per-field label/value list —
//! the only layout that fits every configurable property in a tiled pane —
//! plus the font dropdown popup and the Save/Cancel row. Built on ratatui and
//! converted to `CellView`s; Crew draws the GPU pane border around it.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, StatefulWidget, Widget,
};

use super::{Field, SettingsPane, FIELDS};
use crate::palette::accent_color;

/// Columns reserved for the `› ` focus marker + label.
const LABEL_W: usize = 20;

pub(crate) fn label_of(f: Field) -> &'static str {
    match f {
        Field::FontFamily => "Font family",
        Field::FontSize => "Font size",
        Field::NavWidth => "Nav width",
        Field::ShowNav => "Show nav",
        Field::Theme => "Theme",
        Field::Accent => "Accent (#hex)",
        Field::PaperTexture => "Paper texture",
        Field::PaperGrain => "Paper grain (0-2)",
        Field::Maximized => "Launch maximized",
        Field::Notify => "Notifications",
        Field::NotifyAgentDone => "Notify: cmd done",
        Field::NotifyBell => "Notify: bell",
        Field::NotifyExit => "Notify: pane exit",
        Field::NotifyMinSecs => "Notify min secs",
        Field::NotifyPatterns => "Notify patterns",
        Field::Save | Field::Cancel => "",
    }
}

/// The value text shown for a field, and whether it takes a typing cursor.
pub(crate) fn value_of(p: &SettingsPane, f: Field) -> (String, bool) {
    let onoff = |b: bool| (if b { "on" } else { "off" }).to_string();
    match f {
        Field::FontFamily => (p.family_query.clone(), true),
        Field::FontSize => (p.size_buf.clone(), true),
        Field::NavWidth => (p.nav_buf.clone(), true),
        Field::ShowNav => (onoff(p.draft.show_nav), false),
        Field::Theme => (
            format!("\u{2039} {} \u{203a}", p.draft.theme_id().as_str()),
            false,
        ),
        Field::Accent => (p.accent_buf.clone(), true),
        Field::PaperTexture => (onoff(p.draft.paper_texture), false),
        Field::PaperGrain => (p.grain_buf.clone(), true),
        Field::Maximized => (onoff(p.draft.maximized), false),
        Field::Notify => (onoff(p.draft.notify), false),
        Field::NotifyAgentDone => (onoff(p.draft.notify_agent_done), false),
        Field::NotifyBell => (onoff(p.draft.notify_bell), false),
        Field::NotifyExit => (onoff(p.draft.notify_exit), false),
        Field::NotifyMinSecs => (p.minsecs_buf.clone(), true),
        Field::NotifyPatterns => (p.patterns_buf.clone(), true),
        Field::Save | Field::Cancel => (String::new(), false),
    }
}

/// First visible field row so the focused row stays inside a `visible`-row
/// window over `total` field rows.
pub(crate) fn scroll_offset(focus: usize, total: usize, visible: usize) -> usize {
    if visible == 0 || total <= visible {
        return 0;
    }
    let fi = focus.min(total - 1);
    fi.saturating_sub(visible - 1).min(total - visible)
}

/// Render the form into a ratatui buffer, then hand the cells to the GPU.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);

    let n_fields = FIELDS.len() - 2; // Save/Cancel live on the button row
    let list_top = 1u16;
    let visible = rows.saturating_sub(3) as usize; // padding + gap + buttons
    let off = scroll_offset(p.focus, n_fields, visible);
    let focused = p.focused_field();

    let mut family_row: Option<Rect> = None;
    for (row_i, fi) in (off..n_fields).take(visible).enumerate() {
        let f = FIELDS[fi];
        let y = list_top + row_i as u16;
        let is_focused = f == focused;
        let (mut value, cursor) = value_of(p, f);
        if is_focused && cursor {
            value.push('\u{2588}'); // █
        }
        let marker = if is_focused { "\u{203a} " } else { "  " };
        let fg = if is_focused { accent_color() } else { ink };
        let mut label_style = Style::new().fg(if is_focused { accent_color() } else { dim });
        if is_focused {
            label_style = label_style.add_modifier(Modifier::BOLD);
        }
        let line = Line::from(vec![
            Span::styled(format!("{marker}{:<LABEL_W$}", label_of(f)), label_style),
            Span::styled(value, Style::new().fg(fg)),
        ]);
        buf.set_line(2, y, &line, cols.saturating_sub(4));
        if f == Field::FontFamily {
            family_row = Some(Rect::new(2, y, cols.saturating_sub(4), 1));
        }
    }

    // Scroll hints when the list overflows the pane.
    if off > 0 {
        buf.set_line(cols - 4, list_top, &Line::styled("\u{2191}", dim), 1);
    }
    if off + visible < n_fields {
        buf.set_line(
            cols - 4,
            list_top + visible.saturating_sub(1) as u16,
            &Line::styled("\u{2193}", dim),
            1,
        );
    }

    buttons(&mut buf, Rect::new(2, rows - 1, cols - 4, 1), focused);
    if p.family_open {
        if let Some(anchor) = family_row {
            dropdown(&mut buf, p, anchor);
        }
    }
    crate::tui::to_cells(&buf)
}

/// `[ Save ]   [ Cancel ]`, the focused one accent + bold.
fn buttons(buf: &mut Buffer, area: Rect, f: Field) {
    let line = Line::from(vec![
        button_span("[ Save ]", f == Field::Save),
        Span::raw("   "),
        button_span("[ Cancel ]", f == Field::Cancel),
    ]);
    buf.set_line(area.x, area.y, &line, area.width);
}

fn button_span(text: &str, focused: bool) -> Span<'static> {
    let t = crew_theme::theme();
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let mut style = Style::new().fg(if focused { accent_color() } else { dim_col });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text.to_string(), style)
}

/// The type-to-search font list, drawn as a rounded popup anchored below the
/// family row (over the rows beneath it).
fn dropdown(buf: &mut Buffer, p: &SettingsPane, anchor: Rect) {
    let names = p.filtered();
    let want = names.len() as u16 + 2;
    let y0 = anchor.y + anchor.height; // just below the family row
    let max = buf.area.height.saturating_sub(y0);
    if max < 3 {
        return;
    }
    let height = want.clamp(3, max);
    let area = Rect::new(anchor.x, y0, anchor.width, height);
    Clear.render(area, buf);
    let items: Vec<ListItem> = names.into_iter().map(ListItem::new).collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .title(Span::styled(" fonts ", Style::new().fg(accent_color())));
    let t = crew_theme::theme();
    let page_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().fg(page_col).bg(accent_color()))
        .highlight_symbol("\u{203a} ");
    let mut state = ListState::default();
    state.select(Some(p.family_sel));
    StatefulWidget::render(list, area, buf, &mut state);
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
