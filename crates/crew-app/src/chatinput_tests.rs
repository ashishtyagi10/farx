use super::*;

fn agents(names: &[&str]) -> Vec<AgentInfo> {
    names
        .iter()
        .map(|n| AgentInfo {
            name: (*n).into(),
            role: String::new(),
            model: String::new(),
        })
        .collect()
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    // Preserve gaps so left/right alignment is visible.
    let mut s = String::new();
    for (col, c) in v {
        while s.chars().count() < col as usize {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

#[test]
fn tall_pane_gets_bar_and_prompt() {
    let cells = composer_cells("hi", &agents(&["planner", "coder"]), 60, 10);
    let bar = row_text(&cells, 8);
    assert!(bar.contains("@planner"), "bar: {bar}");
    assert!(bar.contains("@coder"), "bar: {bar}");
    assert!(bar.ends_with("Enter send \u{00b7} Esc close"), "bar: {bar}");
    assert!(row_text(&cells, 9).starts_with("\u{276f} hi"));
}

#[test]
fn short_pane_gets_prompt_only() {
    let cells = composer_cells("hi", &agents(&["planner"]), 60, 5);
    assert!(cells.iter().all(|c| c.row == 4));
    assert!(row_text(&cells, 4).starts_with("\u{276f} hi"));
}

#[test]
fn valid_mention_is_highlighted_in_agent_colour() {
    let a = agents(&["coder"]);
    let cells = composer_cells("@coder fix", &a, 60, 10);
    let ink = crew_theme::theme().ink;
    let at = |col: u16| cells.iter().find(|c| c.row == 9 && c.col == col).unwrap();
    assert_ne!(at(2).fg, ink, "@ of the mention takes the agent colour");
    assert!(at(2).bold && at(7).bold, "mention renders bold");
    assert_eq!(at(9).fg, ink, "text after the mention stays ink");
}

#[test]
fn unknown_mention_stays_plain() {
    let cells = composer_cells("@ghost hi", &agents(&["coder"]), 60, 10);
    let ink = crew_theme::theme().ink;
    assert!(cells
        .iter()
        .filter(|c| c.row == 9 && c.col >= 2 && c.c != '\u{258f}')
        .all(|c| c.fg == ink));
}

#[test]
fn caret_follows_the_input() {
    let cells = composer_cells("ab", &[], 60, 10);
    let caret = cells.iter().find(|c| c.c == '\u{258f}').unwrap();
    assert_eq!((caret.col, caret.row), (4, 9));
}

#[test]
fn everything_clips_to_width() {
    let cells = composer_cells(
        "a very long input line that overflows",
        &agents(&["planner", "coder", "reviewer"]),
        12,
        10,
    );
    assert!(cells.iter().all(|c| c.col < 12));
}
