use gpui_kit::{App, px, rgb, rgba};
use gpuicn::theme::{Appearance, ThemeMode, UiFonts, UiRadius, UiTheme};
use lingxi_domain::ThemeAppearance;

use crate::fonts::{PRIMARY_MONO_FONT, PRIMARY_UI_FONT};
use crate::tokens::{elevation, lingxi_typography};

/// Re-export `tokens::space` for callers previously importing `crate::theme::tokens`.
pub use crate::tokens::space as tokens;

/// LingXi Light palette: Warm Cream Claude Desktop canvas, elevated white surfaces, Fox Orange restraint.
pub fn lingxi_light_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_light();
    theme.mode = ThemeMode::Light;

    // Level 0: Canvas & Sidebar
    theme.colors.background = rgb(0xFAF7F2);
    theme.colors.foreground = rgb(0x1F1B18);
    theme.colors.sidebar = rgb(0xF3EEE6);
    theme.colors.sidebar_foreground = rgb(0x1F1B18);
    theme.colors.sidebar_primary = rgb(0xE8702A);
    theme.colors.sidebar_primary_foreground = rgb(0xFFFAF5);
    theme.colors.sidebar_accent = rgb(0xE9E1D5);
    theme.colors.sidebar_accent_foreground = rgb(0x1F1B18);
    theme.colors.sidebar_border = rgb(0xE7DFD3);
    theme.colors.sidebar_ring = rgb(0xE8702A);
    theme.colors.sidebar_muted_foreground = Some(rgb(0x756B62));

    // Level 1 & Level 2/3 surfaces
    theme.colors.card = rgb(0xFFFFFF);
    theme.colors.card_foreground = rgb(0x1F1B18);
    theme.colors.popover = rgb(0xFFFFFF);
    theme.colors.popover_foreground = rgb(0x1F1B18);

    // Semantic accents (Fox Orange used sparingly)
    theme.colors.primary = rgb(0xE8702A);
    theme.colors.primary_foreground = rgb(0xFFFAF5);
    theme.colors.secondary = rgb(0xEFE9DF);
    theme.colors.secondary_foreground = rgb(0x1F1B18);
    theme.colors.muted = rgb(0xF2ECE3);
    theme.colors.muted_foreground = rgb(0x756C63);
    theme.colors.accent = rgb(0xFBEFE6);
    theme.colors.accent_foreground = rgb(0x1F1B18);
    theme.colors.border = rgb(0xE6DDD1);
    theme.colors.input = rgb(0xDDD3C6);
    theme.colors.ring = rgb(0xE8702A);
    theme.colors.overlay = rgba(0x1A151157);

    theme.colors.destructive = rgb(0xD93848);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x2B8454);
    theme.colors.success_foreground = rgb(0xFFFFFF);
    theme.colors.warning = rgb(0xC9740E);
    theme.colors.warning_foreground = rgb(0xFFFFFF);
    theme.colors.info = rgb(0x2F66D0);
    theme.colors.info_foreground = rgb(0xFFFFFF);

    theme.fonts = UiFonts {
        body: PRIMARY_UI_FONT.into(),
        heading: PRIMARY_UI_FONT.into(),
        mono: PRIMARY_MONO_FONT.into(),
    };
    theme.typography = lingxi_typography();
    theme.shadows = elevation::ui_shadows(ThemeMode::Light);
    theme.radius = UiRadius::new(px(10.0));
    theme.motion.reduced = true;
    theme
}

/// LingXi Dark palette: Independently crafted layered Charcoal & deep warm gray surfaces with subtle edge highlights.
pub fn lingxi_dark_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_dark();
    theme.mode = ThemeMode::Dark;

    // Level 0: Sidebar (#141312) vs Canvas (#1A1816) — clear spatial separation without flat black
    theme.colors.background = rgb(0x1A1816);
    theme.colors.foreground = rgb(0xF0EAE1);
    theme.colors.sidebar = rgb(0x141312);
    theme.colors.sidebar_foreground = rgb(0xF0EAE1);
    theme.colors.sidebar_primary = rgb(0xF07C38);
    theme.colors.sidebar_primary_foreground = rgb(0x170D07);
    theme.colors.sidebar_accent = rgb(0x26221F);
    theme.colors.sidebar_accent_foreground = rgb(0xF0EAE1);
    theme.colors.sidebar_border = rgb(0x25221F);
    theme.colors.sidebar_ring = rgb(0xF07C38);
    theme.colors.sidebar_muted_foreground = Some(rgb(0x9A9086));

    // Level 1 (#221F1C) & Level 2 (#272320) elevated surfaces
    theme.colors.card = rgb(0x221F1C);
    theme.colors.card_foreground = rgb(0xF0EAE1);
    theme.colors.popover = rgb(0x272320);
    theme.colors.popover_foreground = rgb(0xF0EAE1);

    theme.colors.primary = rgb(0xF07C38);
    theme.colors.primary_foreground = rgb(0x170D07);
    theme.colors.secondary = rgb(0x282420);
    theme.colors.secondary_foreground = rgb(0xF0EAE1);
    theme.colors.muted = rgb(0x24201D);
    theme.colors.muted_foreground = rgb(0x9A9086);
    theme.colors.accent = rgb(0x2D2018);
    theme.colors.accent_foreground = rgb(0xF2ECE3);
    theme.colors.border = rgb(0x312C27);
    theme.colors.input = rgb(0x3B342E);
    theme.colors.ring = rgb(0xF07C38);
    theme.colors.overlay = rgba(0x00000094);

    theme.colors.destructive = rgb(0xF45861);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x3CD096);
    theme.colors.success_foreground = rgb(0x0B1A13);
    theme.colors.warning = rgb(0xF2A024);
    theme.colors.warning_foreground = rgb(0x1A1104);
    theme.colors.info = rgb(0x6BA8FA);
    theme.colors.info_foreground = rgb(0x0A1526);

    theme.fonts = UiFonts {
        body: PRIMARY_UI_FONT.into(),
        heading: PRIMARY_UI_FONT.into(),
        mono: PRIMARY_MONO_FONT.into(),
    };
    theme.typography = lingxi_typography();
    theme.shadows = elevation::ui_shadows(ThemeMode::Dark);
    theme.radius = UiRadius::new(px(10.0));
    theme.motion.reduced = true;
    theme
}

pub fn to_gpuicn_appearance(appearance: ThemeAppearance) -> Appearance {
    match appearance {
        ThemeAppearance::Light => Appearance::Light,
        ThemeAppearance::Dark => Appearance::Dark,
        ThemeAppearance::System => Appearance::System,
    }
}

pub fn apply_appearance(cx: &mut App, appearance: ThemeAppearance) {
    UiTheme::set_pair(cx, lingxi_light_theme(), lingxi_dark_theme());
    UiTheme::set_appearance(cx, to_gpuicn_appearance(appearance));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lingxi_palettes_use_fox_orange_soft_fonts_and_layered_elevation() {
        let light = lingxi_light_theme();
        let dark = lingxi_dark_theme();
        assert_eq!(light.colors.primary, rgb(0xE8702A));
        assert_eq!(dark.colors.primary, rgb(0xF07C38));
        assert_ne!(light.colors.background, dark.colors.background);
        assert_ne!(dark.colors.sidebar, dark.colors.background);
        assert_ne!(dark.colors.card, dark.colors.background);
        assert_ne!(dark.colors.popover, dark.colors.card);
        assert_eq!(light.fonts.body.as_ref(), PRIMARY_UI_FONT);
        assert_eq!(light.fonts.mono.as_ref(), PRIMARY_MONO_FONT);
        assert_eq!(light.shadows.sm.len(), 2);
        assert_eq!(light.shadows.md.len(), 2);
        assert_eq!(light.shadows.lg.len(), 2);
        assert_eq!(light.radius.lg, px(10.0));
    }
}
