use super::keys::{build_config, commit_family, commit_field, escape, move_focus};
use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL};
use crate::config::CrewConfig;

fn pane() -> SettingsPane {
    SettingsPane::new(
        CrewConfig::default(),
        vec!["Menlo".into(), "JetBrains Mono".into()],
    )
}

#[test]
fn filtered_leads_with_default_label() {
    assert_eq!(pane().filtered().first().unwrap(), DEFAULT_FAMILY_LABEL);
}

#[test]
fn filtered_narrows_on_query() {
    let mut p = pane();
    p.family_query = "jet".into();
    assert_eq!(p.filtered(), vec!["JetBrains Mono".to_string()]);
}

#[test]
fn commit_font_size_clamps_low() {
    let mut p = pane();
    p.focus = 1; // FontSize
    p.size_buf = "3".into();
    commit_field(&mut p);
    assert_eq!(p.draft.font_size, 12.0);
    assert_eq!(p.size_buf, "12");
}

#[test]
fn commit_family_sets_draft() {
    let mut p = pane();
    p.family_query = "jet".into();
    p.family_sel = 0;
    commit_family(&mut p);
    assert_eq!(p.draft.font_family.as_deref(), Some("JetBrains Mono"));
    assert!(!p.family_open);
}

#[test]
fn move_focus_wraps_backwards_to_cancel() {
    let mut p = pane();
    move_focus(&mut p, true);
    assert_eq!(p.focused_field(), Field::Cancel);
}

#[test]
fn esc_closes_dropdown_then_cancels() {
    let mut p = pane();
    p.family_open = true;
    assert!(escape(&mut p).is_none()); // first Esc closes the dropdown
    assert!(!p.family_open);
    assert!(matches!(escape(&mut p), Some(SettingsAction::Cancel)));
}

#[test]
fn scroll_steps_field_focus_clamped() {
    let mut p = pane();
    p.scroll(-99); // wheel down → last field
    assert_eq!(p.focused_field(), Field::Cancel);
    p.scroll(99); // wheel up → first field
    assert_eq!(p.focused_field(), Field::FontFamily);
}

#[test]
fn scroll_moves_dropdown_selection() {
    let mut p = pane();
    p.family_query = String::new(); // show all families
    p.family_open = true;
    p.scroll(-1); // wheel down → next entry
    assert_eq!(p.family_sel, 1);
    p.scroll(99); // clamps back to top
    assert_eq!(p.family_sel, 0);
}

#[test]
fn build_config_returns_edited_draft() {
    let mut p = pane();
    p.focus = 1; // FontSize
    p.size_buf = "20".into();
    commit_field(&mut p);
    assert_eq!(build_config(&p).font_size, 20.0);
}

#[test]
fn commit_accent_valid_normalizes_and_sets_draft() {
    let mut p = pane();
    p.focus = 4; // Accent
    assert_eq!(p.focused_field(), Field::Accent);
    p.accent_buf = "#AABBCC".into();
    commit_field(&mut p);
    // Stored canonical lowercase; the buffer mirrors it.
    assert_eq!(p.draft.accent.as_deref(), Some("#aabbcc"));
    assert_eq!(p.accent_buf, "#aabbcc");
    assert_eq!(p.draft.accent_rgb(), (0xaa, 0xbb, 0xcc));
}

#[test]
fn commit_accent_invalid_keeps_previous() {
    let mut p = pane();
    p.focus = 4;
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "nope".into();
    commit_field(&mut p);
    // Bad input is rejected; the prior value survives and the buffer is restored.
    assert_eq!(p.draft.accent.as_deref(), Some("#001122"));
    assert_eq!(p.accent_buf, "#001122");
}

#[test]
fn commit_accent_empty_clears_to_builtin() {
    let mut p = pane();
    p.focus = 4;
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "   ".into();
    commit_field(&mut p);
    assert_eq!(p.draft.accent, None);
    assert!(p.accent_buf.is_empty());
}
