use std::sync::Arc;

use gpui_kit::{BoxShadow, Pixels, Rgba, px, rgb, rgba};
use gpuicn::theme::{ThemeMode, UiShadows, UiTextStyle, UiTheme, UiTypography};

/// Centralized typography scale (Nunito Sans + MiSans + JetBrains Mono).
pub mod typo {
    use super::{Pixels, px};

    pub const PAGE_TITLE: Pixels = px(22.0);
    pub const PAGE_TITLE_LH: Pixels = px(30.0);

    pub const HERO_TITLE: Pixels = px(24.0);
    pub const HERO_TITLE_LH: Pixels = px(32.0);

    pub const SECTION_TITLE: Pixels = px(15.5);
    pub const SECTION_TITLE_LH: Pixels = px(22.0);

    pub const CHAT_PROSE: Pixels = px(14.5);
    pub const CHAT_PROSE_LH: Pixels = px(23.0);

    pub const UI_BODY: Pixels = px(13.5);
    pub const UI_BODY_LH: Pixels = px(20.0);

    pub const CAPTION: Pixels = px(12.5);
    pub const CAPTION_LH: Pixels = px(18.0);

    pub const CODE: Pixels = px(13.0);
    pub const CODE_LH: Pixels = px(20.0);
}

/// Centralized spacing and layout dimension tokens.
pub mod space {
    use super::{Pixels, px};

    pub const CONTROL_HEIGHT_SM: Pixels = px(28.0);
    pub const CONTROL_HEIGHT_MD: Pixels = px(32.0);
    pub const CONTROL_HEIGHT_LG: Pixels = px(36.0);
    pub const THREAD_HEADER_HEIGHT: Pixels = px(48.0);
    pub const SIDEBAR_ROW_HEIGHT: Pixels = px(32.0);

    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 256.0;
    pub const SIDEBAR_MIN_WIDTH: f32 = 216.0;
    pub const SIDEBAR_MAX_WIDTH: f32 = 340.0;
    pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 52.0;

    pub const INSPECTOR_DEFAULT_WIDTH: f32 = 420.0;
    pub const INSPECTOR_MIN_WIDTH: f32 = 320.0;
    pub const INSPECTOR_MAX_WIDTH: f32 = 640.0;

    pub const CHAT_COLUMN_MAX_WIDTH: Pixels = px(780.0);
    pub const WELCOME_COMPOSER_MAX_WIDTH: Pixels = px(680.0);
}

/// Centralized corner radius tokens.
pub mod radius {
    use super::{Pixels, px};

    pub const CONTROL: Pixels = px(8.0);
    pub const CARD: Pixels = px(12.0);
    pub const MODAL: Pixels = px(14.0);
    pub const COMPOSER: Pixels = px(16.0);
    pub const BUBBLE: Pixels = px(14.0);
}

/// Semantic surface helpers not already directly mapped by `UiColors`.
pub mod surface {
    use super::{Rgba, ThemeMode, UiTheme, rgb};

    /// Soft tinted surface for user chat messages (no hard outer border).
    pub fn user_bubble(theme: &UiTheme) -> Rgba {
        match theme.mode {
            ThemeMode::Light => rgb(0xF1EBE1),
            ThemeMode::Dark => rgb(0x25211E),
        }
    }

    /// Quiet surface for collapsed/expanded tool calls & inline status blocks.
    pub fn quiet_panel(theme: &UiTheme) -> Rgba {
        match theme.mode {
            ThemeMode::Light => rgb(0xF5F0E8),
            ThemeMode::Dark => rgb(0x201D1A),
        }
    }

    /// Subtle footer/legend bar surface inside Command Palette or sheets.
    pub fn subtle_footer(theme: &UiTheme) -> Rgba {
        match theme.mode {
            ThemeMode::Light => rgb(0xF7F3EC),
            ThemeMode::Dark => rgb(0x1E1B18),
        }
    }
}

/// 4-level elevation & shadow system (Level 0 = flat, Level 1 = Composer, Level 2 = Popover, Level 3 = Modal).
pub mod elevation {
    use super::{Arc, BoxShadow, ThemeMode, UiShadows, UiTheme, px, rgba};

    /// Level 1: Slight lift for floating Composer and key interactive cards.
    pub fn level1(mode: ThemeMode) -> Vec<BoxShadow> {
        match mode {
            ThemeMode::Light => vec![
                BoxShadow::new(px(0.0), px(1.0), rgba(0x1E18120A).into()).blur_radius(px(3.0)),
                BoxShadow::new(px(0.0), px(2.0), rgba(0x1E181212).into()).blur_radius(px(10.0)),
            ],
            ThemeMode::Dark => vec![
                BoxShadow::new(px(0.0), px(2.0), rgba(0x0000004D).into()).blur_radius(px(12.0)),
                BoxShadow::new(px(0.0), px(1.0), rgba(0xFFF8F00D).into())
                    .blur_radius(px(0.0))
                    .inset(),
            ],
        }
    }

    /// Level 2: Floating surfaces (Popover, Select dropdown, Command Palette).
    pub fn level2(mode: ThemeMode) -> Vec<BoxShadow> {
        match mode {
            ThemeMode::Light => vec![
                BoxShadow::new(px(0.0), px(4.0), rgba(0x1E181212).into()).blur_radius(px(12.0)),
                BoxShadow::new(px(0.0), px(8.0), rgba(0x1E181222).into()).blur_radius(px(20.0)),
            ],
            ThemeMode::Dark => vec![
                BoxShadow::new(px(0.0), px(8.0), rgba(0x00000070).into()).blur_radius(px(22.0)),
                BoxShadow::new(px(0.0), px(1.0), rgba(0xFFF8F012).into())
                    .blur_radius(px(0.0))
                    .inset(),
            ],
        }
    }

    /// Level 3: Modal dialogs (Settings, Update, Permission confirmation).
    pub fn level3(mode: ThemeMode) -> Vec<BoxShadow> {
        match mode {
            ThemeMode::Light => vec![
                BoxShadow::new(px(0.0), px(6.0), rgba(0x1E181218).into()).blur_radius(px(18.0)),
                BoxShadow::new(px(0.0), px(14.0), rgba(0x1E18122E).into()).blur_radius(px(36.0)),
            ],
            ThemeMode::Dark => vec![
                BoxShadow::new(px(0.0), px(16.0), rgba(0x00000090).into()).blur_radius(px(38.0)),
                BoxShadow::new(px(0.0), px(1.0), rgba(0xFFF8F014).into())
                    .blur_radius(px(0.0))
                    .inset(),
            ],
        }
    }

    pub fn ui_shadows(mode: ThemeMode) -> UiShadows {
        UiShadows {
            sm: Arc::from(level1(mode)),
            md: Arc::from(level2(mode)),
            lg: Arc::from(level3(mode)),
        }
    }

    pub fn for_level1(theme: &UiTheme) -> Vec<BoxShadow> {
        theme.shadows.sm.to_vec()
    }

    pub fn for_level2(theme: &UiTheme) -> Vec<BoxShadow> {
        theme.shadows.md.to_vec()
    }

    pub fn for_level3(theme: &UiTheme) -> Vec<BoxShadow> {
        theme.shadows.lg.to_vec()
    }
}

pub fn lingxi_typography() -> UiTypography {
    let style = |size: Pixels, line_height: Pixels| UiTextStyle { size, line_height };
    UiTypography {
        xs: style(typo::CAPTION, typo::CAPTION_LH),
        sm: style(typo::UI_BODY, typo::UI_BODY_LH),
        base: style(typo::CHAT_PROSE, typo::CHAT_PROSE_LH),
        lg: style(typo::SECTION_TITLE, typo::SECTION_TITLE_LH),
        xl: style(typo::PAGE_TITLE, typo::PAGE_TITLE_LH),
    }
}
