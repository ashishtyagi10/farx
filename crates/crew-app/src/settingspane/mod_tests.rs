use super::commit::{build_config, commit_family, commit_field, escape, move_focus};
use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL, FIELDS};
use crate::config::CrewConfig;

fn pane() -> SettingsPane {
    SettingsPane::new(
        CrewConfig::default(),
        vec!["Menlo".into(), "JetBrains Mono".into()],
    )
}

/// Focus the pane on `field` (must be in FIELDS).
fn focus(p: &mut SettingsPane, field: Field) {
    p.focus = FIELDS.iter().position(|&f| f == field).unwrap();
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
    focus(&mut p, Field::FontSize);
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
fn build_config_returns_edited_draft() {
    let mut p = pane();
    focus(&mut p, Field::FontSize);
    p.size_buf = "20".into();
    commit_field(&mut p);
    assert_eq!(build_config(&p).font_size, 20.0);
}

#[test]
fn commit_accent_valid_normalizes_and_sets_draft() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.accent_buf = "#AABBCC".into();
    commit_field(&mut p);
    // Stored canonical lowercase; the buffer mirrors it.
    assert_eq!(p.draft.accent.as_deref(), Some("#aabbcc"));
    assert_eq!(p.accent_buf, "#aabbcc");
}

#[test]
fn commit_accent_invalid_keeps_previous() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "nope".into();
    commit_field(&mut p);
    assert_eq!(p.draft.accent.as_deref(), Some("#001122"));
    assert_eq!(p.accent_buf, "#001122");
}

#[test]
fn commit_accent_empty_clears_to_builtin() {
    let mut p = pane();
    focus(&mut p, Field::Accent);
    p.draft.accent = Some("#001122".into());
    p.accent_buf = "   ".into();
    commit_field(&mut p);
    assert_eq!(p.draft.accent, None);
    assert!(p.accent_buf.is_empty());
}

#[test]
fn commit_grain_clamps_and_formats() {
    let mut p = pane();
    focus(&mut p, Field::PaperGrain);
    p.grain_buf = "9.7".into();
    commit_field(&mut p);
    assert_eq!(p.draft.paper_grain, 2.0);
    assert_eq!(p.grain_buf, "2.0");
}

#[test]
fn commit_min_secs_clamps_up_from_zero() {
    let mut p = pane();
    focus(&mut p, Field::NotifyMinSecs);
    p.minsecs_buf = "0".into();
    commit_field(&mut p);
    assert_eq!(p.draft.notify_min_secs, 1);
}

#[test]
fn commit_patterns_splits_and_drops_blanks() {
    let mut p = pane();
    focus(&mut p, Field::NotifyPatterns);
    p.patterns_buf = " error , , DONE ".into();
    commit_field(&mut p);
    assert_eq!(
        p.draft.notify_patterns,
        vec!["error".to_string(), "DONE".to_string()]
    );
    assert_eq!(p.patterns_buf, "error, DONE"); // normalized display
}

#[test]
fn every_config_property_is_editable_in_the_form() {
    // The goal: all user-configurable properties appear in the settings page.
    // Persisted window state (last_dir, win_w/h) is bookkeeping, not a setting.
    for f in [
        Field::FontFamily,
        Field::FontSize,
        Field::NavWidth,
        Field::ShowNav,
        Field::Theme,
        Field::Accent,
        Field::PaperTexture,
        Field::PaperGrain,
        Field::Maximized,
        Field::Notify,
        Field::NotifyAgentDone,
        Field::NotifyBell,
        Field::NotifyExit,
        Field::NotifyMinSecs,
        Field::NotifyPatterns,
    ] {
        assert!(FIELDS.contains(&f), "{f:?} missing from the form");
    }
}
