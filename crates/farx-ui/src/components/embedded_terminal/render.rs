use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::session::TerminalSession;

/// Convert a vt100 color to a ratatui color.
fn vt100_to_ratatui_color(color: vt100::Color, default: Color) -> Color {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Render a terminal session into a frame area.
pub fn render_terminal(frame: &mut Frame, area: Rect, session: &TerminalSession, is_focused: bool) {
    let border_color = if is_focused {
        Color::Cyan
    } else if session.has_attention {
        Color::Yellow
    } else if !session.alive {
        Color::Red
    } else {
        Color::Indexed(240)
    };

    let title = if !session.alive {
        format!(" {} (exited) ", session.title)
    } else {
        format!(" {} ", session.title)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let screen = session.screen();
    let screen_rows = inner.height as usize;
    let screen_cols = inner.width as usize;

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(screen_rows);

    for row in 0..screen_rows {
        let mut spans: Vec<Span<'_>> = Vec::new();
        let mut col = 0usize;

        while col < screen_cols {
            let cell = screen.cell(row as u16, col as u16);
            match cell {
                Some(cell) => {
                    let fg = vt100_to_ratatui_color(cell.fgcolor(), Color::White);
                    let bg = vt100_to_ratatui_color(cell.bgcolor(), Color::Black);
                    let mut style = Style::default().fg(fg).bg(bg);
                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    if cell.inverse() {
                        style = style.add_modifier(Modifier::REVERSED);
                    }

                    let contents = cell.contents();
                    if contents.is_empty() {
                        spans.push(Span::styled(" ", style));
                    } else {
                        spans.push(Span::styled(contents.to_string(), style));
                    }
                    col += 1;
                }
                None => {
                    spans.push(Span::styled(
                        " ",
                        Style::default().fg(Color::White).bg(Color::Black),
                    ));
                    col += 1;
                }
            }
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // Show cursor if terminal is focused and cursor is visible
    if is_focused {
        let cursor = screen.cursor_position();
        let cx = inner.x + cursor.1;
        let cy = inner.y + cursor.0;
        if cx < inner.x + inner.width && cy < inner.y + inner.height {
            frame.set_cursor_position((cx, cy));
        }
    }
}
