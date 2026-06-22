use super::*;

fn bar(focused: bool) -> Bar<'static> {
    Bar {
        cols: 40,
        index: Some(2),
        title: "~/code",
        focused,
        scroll: 37,
        activity: true,
        bell: true,
        broadcast: false,
    }
}

#[test]
fn title_bar_shows_broadcast_marker() {
    let b = Bar {
        broadcast: true,
        ..bar(true)
    };
    let cells = title_bar(&b);
    assert!(cells.iter().any(|c| c.c == '»' && c.fg == BROADCAST));
    // without broadcast, no marker
    assert!(!title_bar(&bar(true)).iter().any(|c| c.c == '»'));
}

#[test]
fn title_bar_has_index_title_and_glyphs() {
    let cells = title_bar(&bar(true));
    assert_eq!(cells.len(), 40); // full-width bar
    assert!(cells.iter().any(|c| c.c == '2' && c.fg == ACCENT));
    assert!(cells.iter().any(|c| c.c == '~'));
    // scroll indicator renders as `⇡37`
    assert!(cells.iter().any(|c| c.c == '⇡' && c.fg == SCROLL_HINT));
    assert!(cells.iter().any(|c| c.c == '3' && c.fg == SCROLL_HINT));
    assert!(cells.iter().any(|c| c.c == '7' && c.fg == SCROLL_HINT));
    assert!(cells.iter().any(|c| c.c == '●' && c.fg == ACTIVITY));
    assert!(cells.iter().any(|c| c.c == '!' && c.fg == BELL));
    assert!(cells.iter().all(|c| c.row == 0));
}

#[test]
fn title_bar_no_scroll_indicator_at_bottom() {
    let b = Bar {
        scroll: 0,
        activity: false,
        bell: false,
        ..bar(true)
    };
    let cells = title_bar(&b);
    assert!(!cells.iter().any(|c| c.c == '⇡'));
}

#[test]
fn title_bar_bg_differs_by_focus() {
    assert_ne!(title_bar(&bar(true))[0].bg, title_bar(&bar(false))[0].bg);
}
