use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// State for the file operation progress dialog.
pub struct ProgressState {
    /// Operation label (e.g. "Copying", "Moving").
    pub operation: String,
    /// Current file being processed.
    pub current_file: String,
    /// Files completed.
    pub files_done: usize,
    /// Total files.
    pub files_total: usize,
    /// Bytes completed.
    pub bytes_done: u64,
    /// Bytes total.
    pub bytes_total: u64,
    /// Whether the operation is done.
    pub finished: bool,
    /// Error message, if any.
    pub error: Option<String>,
    /// Receiver for progress updates from the background thread.
    pub rx: std::sync::mpsc::Receiver<farx_fs::FileProgress>,
}

impl ProgressState {
    pub fn new(operation: &str, rx: std::sync::mpsc::Receiver<farx_fs::FileProgress>) -> Self {
        Self {
            operation: operation.to_string(),
            current_file: String::new(),
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            finished: false,
            error: None,
            rx,
        }
    }

    /// Poll for progress updates (non-blocking). Returns true if finished.
    pub fn poll(&mut self) -> bool {
        while let Ok(progress) = self.rx.try_recv() {
            self.current_file = progress.current_file;
            self.files_done = progress.files_done;
            self.files_total = progress.files_total;
            self.bytes_done = progress.bytes_done;
            self.bytes_total = progress.bytes_total;
            self.finished = progress.finished;
            self.error = progress.error;
        }
        self.finished
    }

    pub fn percent(&self) -> u16 {
        if self.bytes_total == 0 {
            if self.files_total == 0 {
                return 0;
            }
            return ((self.files_done as f64 / self.files_total as f64) * 100.0) as u16;
        }
        ((self.bytes_done as f64 / self.bytes_total as f64) * 100.0).min(100.0) as u16
    }
}

/// Render the progress dialog.
pub fn render_progress(frame: &mut Frame, state: &ProgressState, _theme: &Theme) {
    let area = frame.area();
    let dialog_width = 60u16.min(area.width.saturating_sub(4));
    let dialog_height = 10u16.min(area.height.saturating_sub(4));

    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    frame.render_widget(Clear, dialog_area);

    let title = format!(" {} ", state.operation);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Indexed(236)))
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::White));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    // Current file
    let file_display: String = state
        .current_file
        .chars()
        .take(inner.width as usize - 2)
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", file_display),
            Style::default().fg(Color::White).bg(Color::Indexed(236)),
        ))),
        Rect {
            y: inner.y,
            height: 1,
            ..inner
        },
    );

    // Files counter
    let counter_text = format!(" Files: {} / {}", state.files_done, state.files_total);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            counter_text,
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
        ))),
        Rect {
            y: inner.y + 1,
            height: 1,
            ..inner
        },
    );

    // Bytes counter
    let bytes_text = format!(
        " Size:  {} / {}",
        format_size(state.bytes_done),
        format_size(state.bytes_total)
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            bytes_text,
            Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
        ))),
        Rect {
            y: inner.y + 2,
            height: 1,
            ..inner
        },
    );

    // Progress bar
    let pct = state.percent();
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Indexed(238))
                .add_modifier(Modifier::BOLD),
        )
        .percent(pct)
        .label(format!("{}%", pct));

    frame.render_widget(
        gauge,
        Rect {
            x: inner.x + 1,
            y: inner.y + 4,
            width: inner.width.saturating_sub(2),
            height: 1,
        },
    );

    // Error or hint
    let hint_y = inner.y + inner.height.saturating_sub(1);
    if let Some(ref err) = state.error {
        let err_display: String = err.chars().take(inner.width as usize - 2).collect();
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" Error: {}", err_display),
                Style::default().fg(Color::Red).bg(Color::Indexed(236)),
            ))),
            Rect {
                x: inner.x,
                y: hint_y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

fn format_size(size: u64) -> String {
    if size < 1_024 {
        format!("{} B", size)
    } else if size < 1_048_576 {
        format!("{:.1} KB", size as f64 / 1_024.0)
    } else if size < 1_073_741_824 {
        format!("{:.1} MB", size as f64 / 1_048_576.0)
    } else {
        format!("{:.1} GB", size as f64 / 1_073_741_824.0)
    }
}
