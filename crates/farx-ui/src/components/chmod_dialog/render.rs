use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

use super::state::{format_rwx, ChmodDialogState};

/// Render the chmod dialog.
pub fn render_chmod_dialog(frame: &mut Frame, state: &ChmodDialogState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 44u16.min(area.width.saturating_sub(4));
    let dialog_height = 12u16.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let filename = state
        .file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let title = format!(" Permissions: {} ", filename);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if inner.height < 6 || inner.width < 20 {
        return;
    }

    render_header(frame, inner);
    render_permission_rows(frame, state, inner);
    render_octal(frame, state, inner);
    render_hint(frame, inner);
}

fn render_header(frame: &mut Frame, inner: Rect) {
    let labels = ["Read", "Write", "Execute"];
    let mut header_spans = vec![Span::styled(
        format!("{:<10}", ""),
        Style::default().fg(Color::White).bg(Color::Indexed(236)),
    )];
    for label in &labels {
        header_spans.push(Span::styled(
            format!(" {:<9}", label),
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(header_spans)),
        Rect {
            y: inner.y,
            height: 1,
            ..inner
        },
    );
}

fn render_permission_rows(frame: &mut Frame, state: &ChmodDialogState, inner: Rect) {
    let groups = ["Owner", "Group", "Other"];
    for (row, group) in groups.iter().enumerate() {
        let mut spans = vec![Span::styled(
            format!(" {:<9}", group),
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        )];

        for col in 0..3 {
            let bit_idx = row * 3 + col;
            let is_focused = state.cursor == bit_idx;
            let is_set = state.bits[bit_idx];

            let checkbox = if is_set { "[x]" } else { "[ ]" };

            let style = if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_set {
                Style::default().fg(Color::Green).bg(Color::Indexed(236))
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Indexed(236))
            };

            spans.push(Span::styled(format!("  {:<7}", checkbox), style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                y: inner.y + 1 + row as u16,
                height: 1,
                ..inner
            },
        );
    }
}

fn render_octal(frame: &mut Frame, state: &ChmodDialogState, inner: Rect) {
    let mode = state.to_mode();
    let octal_line = Line::from(vec![
        Span::styled(
            " Octal: ",
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        ),
        Span::styled(
            format!("{:04o}", mode),
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", format_rwx(mode)),
            Style::default().fg(Color::DarkGray).bg(Color::Indexed(236)),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(octal_line),
        Rect {
            y: inner.y + 5,
            height: 1,
            ..inner
        },
    );
}

fn render_hint(frame: &mut Frame, inner: Rect) {
    let hint = Line::from(vec![
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw("=Toggle  "),
        Span::styled("Arrows", Style::default().fg(Color::Yellow)),
        Span::raw("=Move  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw("=Apply  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw("=Cancel"),
    ]);
    frame.render_widget(
        Paragraph::new(hint),
        Rect {
            y: inner.y + inner.height.saturating_sub(1),
            height: 1,
            ..inner
        },
    );
}
