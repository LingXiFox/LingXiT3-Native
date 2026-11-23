use gpui_kit::{App, px, rgb};
use gpuicn::theme::{Appearance, UiRadius, UiTheme};
use lingxi_domain::ThemeAppearance;

/// LingXi Light palette: Warm Cream canvas, Charcoal text, Fox Orange accent.
/// Solid panels, 1px hairline borders, zero frosted glass or animated backgrounds.
pub fn lingxi_light_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_light();
    theme.colors.background = rgb(0xFAF6F0);
    theme.colors.foreground = rgb(0x1A1614);
    theme.colors.card = rgb(0xFFFDF9);
    theme.colors.card_foreground = rgb(0x1A1614);
    theme.colors.popover = rgb(0xFFFDF9);
    theme.colors.popover_foreground = rgb(0x1A1614);
    theme.colors.primary = rgb(0xF07C38);
    theme.colors.primary_foreground = rgb(0xFFFAF5);
    theme.colors.secondary = rgb(0xEFE8DD);
    theme.colors.secondary_foreground = rgb(0x1A1614);
    theme.colors.muted = rgb(0xF1EAE0);
    theme.colors.muted_foreground = rgb(0x6B6058);
    theme.colors.accent = rgb(0xFDF0E6);
    theme.colors.accent_foreground = rgb(0x1A1614);
    theme.colors.border = rgb(0xE5DDD2);
    theme.colors.input = rgb(0xDFD5C8);
    theme.colors.ring = rgb(0xF07C38);
    theme.colors.destructive = rgb(0xDC3545);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x2E8B57);
    theme.colors.success_foreground = rgb(0xFFFFFF);
    theme.colors.warning = rgb(0xD97706);
    theme.colors.warning_foreground = rgb(0xFFFFFF);
    theme.colors.info = rgb(0x2563EB);
    theme.colors.info_foreground = rgb(0xFFFFFF);

    theme.colors.sidebar = rgb(0xF4EFE7);
    theme.colors.sidebar_foreground = rgb(0x1A1614);
    theme.colors.sidebar_primary = rgb(0xF07C38);
    theme.colors.sidebar_primary_foreground = rgb(0xFFFAF5);
    theme.colors.sidebar_accent = rgb(0xEAE2D6);
    theme.colors.sidebar_accent_foreground = rgb(0x1A1614);
    theme.colors.sidebar_border = rgb(0xE5DDD2);
    theme.colors.sidebar_ring = rgb(0xF07C38);
    theme.colors.sidebar_muted_foreground = Some(rgb(0x6B6058));

    theme.radius = UiRadius::new(px(6.0));
    theme.motion.reduced = true;
    theme
}

/// LingXi Dark palette: Deep Charcoal canvas, Warm Cream text, Fox Orange accent.
/// Solid panels, 1px hairline borders, zero frosted glass or animated backgrounds.
pub fn lingxi_dark_theme() -> UiTheme {
    let mut theme = UiTheme::neutral_dark();
    theme.colors.background = rgb(0x121110);
    theme.colors.foreground = rgb(0xF7F1E8);
    theme.colors.card = rgb(0x181614);
    theme.colors.card_foreground = rgb(0xF7F1E8);
    theme.colors.popover = rgb(0x1C1917);
    theme.colors.popover_foreground = rgb(0xF7F1E8);
    theme.colors.primary = rgb(0xF07C38);
    theme.colors.primary_foreground = rgb(0x140B06);
    theme.colors.secondary = rgb(0x211D1A);
    theme.colors.secondary_foreground = rgb(0xF7F1E8);
    theme.colors.muted = rgb(0x211D1A);
    theme.colors.muted_foreground = rgb(0xB8ABA0);
    theme.colors.accent = rgb(0x291A12);
    theme.colors.accent_foreground = rgb(0xF7F1E8);
    theme.colors.border = rgb(0x2A2521);
    theme.colors.input = rgb(0x322B26);
    theme.colors.ring = rgb(0xF07C38);
    theme.colors.destructive = rgb(0xFB414A);
    theme.colors.destructive_foreground = rgb(0xFFFFFF);
    theme.colors.success = rgb(0x34D399);
    theme.colors.success_foreground = rgb(0x0B1A13);
    theme.colors.warning = rgb(0xFE9A00);
    theme.colors.warning_foreground = rgb(0x1A1104);
    theme.colors.info = rgb(0x60A5FA);
    theme.colors.info_foreground = rgb(0x0A1526);

    theme.colors.sidebar = rgb(0x0D0C0B);
    theme.colors.sidebar_foreground = rgb(0xF7F1E8);
    theme.colors.sidebar_primary = rgb(0xF07C38);
    theme.colors.sidebar_primary_foreground = rgb(0x140B06);
    theme.colors.sidebar_accent = rgb(0x221B16);
    theme.colors.sidebar_accent_foreground = rgb(0xF7F1E8);
    theme.colors.sidebar_border = rgb(0x241F1C);
    theme.colors.sidebar_ring = rgb(0xF07C38);
    theme.colors.sidebar_muted_foreground = Some(rgb(0xAB9E93));

    theme.radius = UiRadius::new(px(6.0));
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
    }
}
