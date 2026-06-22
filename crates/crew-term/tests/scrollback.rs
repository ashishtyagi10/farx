//! Scrollback behaviour of the headless terminal model.
use crew_term::{GridSize, HeadlessTerm, TermModel};

#[test]
fn scrolling_up_reveals_lines_pushed_into_history() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"TOPLINE\r\n");
    for _ in 0..20 {
        term.feed(b"x\r\n");
    }
    let live: String = term.cells().iter().map(|c| c.c).collect();
    assert!(
        !live.contains('T'),
        "TOPLINE should be off-screen at the live bottom"
    );

    term.scroll(1000); // clamps to the top of the scrollback
    let scrolled: String = term.cells().iter().map(|c| c.c).collect();
    assert!(
        scrolled.contains('T') && scrolled.contains('P'),
        "TOPLINE should reappear after scrolling to the top, got {scrolled:?}"
    );
}
