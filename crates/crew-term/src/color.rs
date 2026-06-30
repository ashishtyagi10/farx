use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, Rgb};

/// The active theme's terminal default foreground.
pub(crate) fn default_fg() -> (u8, u8, u8) {
    crew_theme::theme().term_fg
}

/// The active theme's terminal default background.
pub(crate) fn default_bg() -> (u8, u8, u8) {
    crew_theme::theme().term_bg
}

pub(crate) fn resolve_color(color: Color, palette: &Colors, default: (u8, u8, u8)) -> (u8, u8, u8) {
    let ansi = &crew_theme::theme().ansi;
    match color {
        Color::Spec(Rgb { r, g, b }) => (r, g, b),
        Color::Named(named) => {
            let idx = named as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
        Color::Indexed(i) => {
            let idx = i as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::term::color::Colors;
    use alacritty_terminal::vte::ansi::{Color, NamedColor};

    #[test]
    fn named_red_resolves_to_active_theme_ansi() {
        crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
        let palette = Colors::default(); // all slots unset → fall back to theme
        let got = resolve_color(
            Color::Named(NamedColor::Red),
            &palette,
            crew_theme::theme().term_fg,
        );
        assert_eq!(got, crew_theme::PAPER_LIGHT.ansi[1]);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }
}
