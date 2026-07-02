//! Tests for the config/theme plumbing in `spawn.rs`.
use crate::app::CrewApp;
use crate::config::CrewConfig;

#[test]
fn apply_config_adopts_values_without_a_renderer() {
    let mut app = CrewApp::default();
    let cfg = CrewConfig {
        font_size: 19.0,
        show_nav: false,
        ..CrewConfig::default()
    };
    // No renderer in tests: the font calls are skipped, but config is adopted
    // and a relayout/redraw is safe to request.
    app.apply_config(cfg);
    assert_eq!(app.config.font_size, 19.0);
    assert!(!app.config.show_nav);
}

#[test]
fn set_theme_cmd_switches_active_theme() {
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = CrewApp::default();
    app.set_theme_cmd("paper-light");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    assert_eq!(app.config.theme.as_deref(), Some("paper-light"));
    // Unknown name leaves the active theme unchanged.
    app.set_theme_cmd("chartreuse");
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}
