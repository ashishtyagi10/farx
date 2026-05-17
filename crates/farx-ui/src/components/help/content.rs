use ratatui::prelude::*;

fn section_header(title: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn entry(key: &'static str, desc: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(key, Style::default().fg(Color::White)),
        Span::raw(desc),
    ])
}

const NAVIGATION: &[(&str, &str)] = &[
    ("  F4           ", "Next panel (cycle files / terminals)"),
    ("  Ctrl+W       ", "Close focused terminal"),
    ("  Enter        ", "Enter directory / open file"),
    ("  Ctrl+PgUp    ", "Go to parent directory"),
    ("  Ctrl+PgDn    ", "Enter directory"),
    ("  Ctrl+\\       ", "Go to root directory"),
    ("  Alt+F1/F2    ", "Change drive/root (left/right panel)"),
];

const FILE_OPS: &[(&str, &str)] = &[
    ("  F3           ", "Edit file"),
    ("  F5           ", "Copy file(s) to other panel"),
    ("  F6           ", "Move/rename file(s)"),
    ("  F7           ", "Create directory"),
    ("  F8           ", "Delete file(s)"),
    ("  Shift+F4     ", "Create new file"),
    ("  Shift+F5     ", "Copy to same directory"),
    ("  Shift+F6     ", "Rename file"),
];

const SELECTION: &[(&str, &str)] = &[
    ("  Insert       ", "Select/deselect file"),
    ("  Gray +       ", "Select by mask"),
    ("  Gray -       ", "Deselect by mask"),
];

const PANELS: &[(&str, &str)] = &[
    ("  Ctrl+O       ", "Toggle panels (show console)"),
    ("  Ctrl+L       ", "Info panel"),
];

const AI: &[(&str, &str)] = &[
    ("  Ctrl+Space   ", "Open AI command bar"),
    (
        "  Ctrl+E       ",
        "AI coding tools (Claude, Codex, Copilot, Gemini)",
    ),
    ("  /claude      ", "Launch Claude Code"),
    ("  /codex       ", "Launch Codex"),
    ("  /copilot     ", "Launch GitHub Copilot"),
    ("  /gemini      ", "Launch Gemini"),
];

const OTHER: &[(&str, &str)] = &[
    ("  F1           ", "Help (this screen)"),
    ("  F9           ", "Menu"),
    ("  F10          ", "Quit"),
    ("  F11          ", "Plugin commands"),
];

fn push_section(
    lines: &mut Vec<Line<'static>>,
    title: &'static str,
    items: &[(&'static str, &'static str)],
) {
    lines.push(section_header(title));
    lines.push(Line::from(""));
    for (k, d) in items {
        lines.push(entry(k, d));
    }
    lines.push(Line::from(""));
}

pub fn build_help_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    push_section(&mut lines, "  NAVIGATION", NAVIGATION);
    push_section(&mut lines, "  FILE OPERATIONS", FILE_OPS);
    push_section(&mut lines, "  SELECTION", SELECTION);
    push_section(&mut lines, "  PANELS", PANELS);
    push_section(&mut lines, "  AI ASSISTANT", AI);
    push_section(&mut lines, "  OTHER", OTHER);
    lines.push(Line::from(Span::styled(
        "  Press Esc or F1 to close help",
        Style::default().fg(Color::DarkGray),
    )));
    lines
}
