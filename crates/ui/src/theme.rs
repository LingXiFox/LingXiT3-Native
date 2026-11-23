use gpui_kit::{App, Pixels, px, rgb};
use gpuicn::theme::{Appearance, UiRadius, UiTheme};
use lingxi_domain::ThemeAppearance;

/// Shared design tokens aligned with gpuicn Nova / Coss UI desktop proportions.
pub mod tokens {
    use super::{Pixels, px};

    pub const CONTROL_HEIGHT_SM: Pixels = px(28.0);
    pub const CONTROL_HEIGHT_MD: Pixels = px(32.0);
    pub const CONTROL_HEIGHT_LG: Pixels = px(36.0);
    pub const THREAD_HEADER_HEIGHT: Pixels = px(48.0);
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 264.0;
    pub const SIDEBAR_MIN_WIDTH: f32 = 220.0;
    pub const SIDEBAR_MAX_WIDTH: f32 = 360.0;
    pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 52.0;
    pub const INSPECTOR_DEFAULT_WIDTH: f32 = 420.0;
    pub const INSPECTOR_MIN_WIDTH: f32 = 300.0;
    pub const INSPECTOR_MAX_WIDTH: f32 = 640.0;
    pub const CHAT_COLUMN_MAX_WIDTH: Pixels = px(860.0);
}

/// LingXi Light palette: Warm Cream canvas, elevated crisp white cards, Charcoal text, Fox Orange accent.
/// Solid panels, 1px hairline borders, zero frosted glass or animated backgrounds.
pub fn lingxi_light_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_light();
    theme.colors.background = rgb(0xFAF6F0);
    theme.colors.foreground = rgb(0x181512);
    theme.colors.card = rgb(0xFFFFFF);
    theme.colors.card_foreground = rgb(0x181512);
    theme.colors.popover = rgb(0xFFFFFF);
    theme.colors.popover_foreground = rgb(0x181512);
    theme.colors.primary = rgb(0xF07C38);
    theme.colors.primary_foreground = rgb(0xFFFAF5);
    theme.colors.secondary = rgb(0xF1EAE0);
    theme.colors.secondary_foreground = rgb(0x181512);
    theme.colors.muted = rgb(0xF3ECE2);
    theme.colors.muted_foreground = rgb(0x6B6058);
    theme.colors.accent = rgb(0xFDF0E6);
    theme.colors.accent_foreground = rgb(0x181512);
    theme.colors.border = rgb(0xE5DDD2);
    theme.colors.input = rgb(0xDDD3C6);
    theme.colors.ring = rgb(0xF07C38);
    theme.colors.overlay = gpui_kit::rgba(0x1815125C);
    theme.colors.destructive = rgb(0xDC3545);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x2E8B57);
    theme.colors.success_foreground = rgb(0xFFFFFF);
    theme.colors.warning = rgb(0xD97706);
    theme.colors.warning_foreground = rgb(0xFFFFFF);
    theme.colors.info = rgb(0x2563EB);
    theme.colors.info_foreground = rgb(0xFFFFFF);

    theme.colors.sidebar = rgb(0xF4EFE7);
    theme.colors.sidebar_foreground = rgb(0x181512);
    theme.colors.sidebar_primary = rgb(0xF07C38);
    theme.colors.sidebar_primary_foreground = rgb(0xFFFAF5);
    theme.colors.sidebar_accent = rgb(0xEAE2D6);
    theme.colors.sidebar_accent_foreground = rgb(0x181512);
    theme.colors.sidebar_border = rgb(0xE5DDD2);
    theme.colors.sidebar_ring = rgb(0xF07C38);
    theme.colors.sidebar_muted_foreground = Some(rgb(0x6B6058));

    theme.radius = UiRadius::new(px(8.0));
    theme.motion.reduced = true;
    theme
}

/// LingXi Dark palette: Deep Charcoal canvas, Warm Cream text, Fox Orange accent.
/// Solid panels, 1px hairline borders, zero frosted glass or animated backgrounds.
pub fn lingxi_dark_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_dark();
    theme.colors.background = rgb(0x121110);
    theme.colors.foreground = rgb(0xF5EFE6);
    theme.colors.card = rgb(0x191715);
    theme.colors.card_foreground = rgb(0xF5EFE6);
    theme.colors.popover = rgb(0x1D1A18);
    theme.colors.popover_foreground = rgb(0xF5EFE6);
    theme.colors.primary = rgb(0xF07C38);
    theme.colors.primary_foreground = rgb(0x140B06);
    theme.colors.secondary = rgb(0x231F1C);
    theme.colors.secondary_foreground = rgb(0xF5EFE6);
    theme.colors.muted = rgb(0x211D1A);
    theme.colors.muted_foreground = rgb(0xA69A8E);
    theme.colors.accent = rgb(0x2A1B12);
    theme.colors.accent_foreground = rgb(0xF5EFE6);
    theme.colors.border = rgb(0x2A2521);
    theme.colors.input = rgb(0x342D28);
    theme.colors.ring = rgb(0xF07C38);
    theme.colors.overlay = gpui_kit::rgba(0x00000099);
    theme.colors.destructive = rgb(0xFB414A);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x34D399);
    theme.colors.success_foreground = rgb(0x0B1A13);
    theme.colors.warning = rgb(0xFE9A00);
    theme.colors.warning_foreground = rgb(0x1A1104);
    theme.colors.info = rgb(0x60A5FA);
    theme.colors.info_foreground = rgb(0x0A1526);

    theme.colors.sidebar = rgb(0x0D0C0B);
    theme.colors.sidebar_foreground = rgb(0xF5EFE6);
    theme.colors.sidebar_primary = rgb(0xF07C38);
    theme.colors.sidebar_primary_foreground = rgb(0x140B06);
    theme.colors.sidebar_accent = rgb(0x231C17);
    theme.colors.sidebar_accent_foreground = rgb(0xF5EFE6);
    theme.colors.sidebar_border = rgb(0x241F1C);
    theme.colors.sidebar_ring = rgb(0xF07C38);
    theme.colors.sidebar_muted_foreground = Some(rgb(0x9E9185));

    theme.radius = UiRadius::new(px(8.0));
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
    fn lingxi_palettes_use_fox_orange_and_high_contrast() {
        let light = lingxi_light_theme();
        let dark = lingxi_dark_theme();
        assert_eq!(light.colors.primary, rgb(0xF07C38));
        assert_eq!(dark.colors.primary, rgb(0xF07C38));
        assert_ne!(light.colors.background, dark.colors.background);
        assert!(light.motion.reduced);
        assert!(dark.motion.reduced);
        assert_eq!(light.radius.lg, px(8.0));
    }
}
