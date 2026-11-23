# LingXi T3 Native — UI Rework Audit & `gpuicn` Component Reuse Mapping

## 1. v1 UI Audit (`crates/ui/src/shell.rs`)

### Observed Defects in v1
1. **Fake / Hand-rolled Components**:
   - Only `gpuicn::button::Button` and `gpuicn::textarea::textarea` were used.
   - Over 95% of the UI (sidebar, thread list, chat messages, markdown blocks, diff viewer, command palette, settings modal, update modal, progress bars, tabs, badges, keycaps) was hand-rolled using raw `div()` elements with `.text_xs()` and manual borders.
2. **Severe Modal & Overlay Bugs**:
   - Modals used raw `.absolute().inset_0()` overlays driven by 3 independent booleans (`command_palette_open`, `settings_open`, `update_modal_open`) with fixed `w(720).h(500)` dimensions and no scroll viewport.
   - Opening Settings and Update simultaneously caused direct z-layer overlap.
   - Smaller windows (`960×680`, `1280×800`) severely clipped modal content.
   - Missing proper `DialogHandle` focus trapping, backdrop dismissal, and `Escape` key handling.
3. **Layout & Proportion Defects**:
   - Right inspector defaulted to `inspector_open: true` (`380px`), occupying ~30% of the window with blank space even when no diff/terminal/PR existed.
   - Missing `min_h_0()` / `min_w_0()` and `ScrollArea` on panes caused vertical scroll and composer/timeline proportion issues.
   - Title bar did not use `gpuicn::title_bar::TitleBar` + `title_bar_window_options()`.
4. **Typography & Visual Hierarchy**:
   - Global overuse of `text_xs()` made body text, headings, and controls cramped and low-contrast.
   - Missing consistent `32px`/`36px` control heights, `8px`–`12px` surface radii, and Coss UI / Nova surface layering.

---

## 2. Target Native `gpuicn 0.13.4` Component Reuse Inventory

| UI Area | Native `gpuicn` / `gpui_icons` Components | Target Module |
| --- | --- | --- |
| Window Chrome & Traffic Lights | `TitleBar`, `title_bar_window_options()`, `TITLE_BAR_HEIGHT` | `crates/ui/src/views/titlebar.rs` |
| 3-Pane Split Layout | `Resizable`, `PaneLimits` | `crates/ui/src/shell.rs` |
| Left Workspace & Thread Tree | `Sidebar`, `SidebarItem`, `SidebarItemSize`, `sidebar_group_label`, `sidebar_badge`, `sidebar_separator`, `Input`, `InputState` | `crates/ui/src/views/sidebar.rs` |
| Welcome / Empty States | `Empty`, `EmptyHeader`, `EmptyMedia`, `EmptyTitle`, `EmptyDescription`, `EmptyContent` | `crates/ui/src/views/welcome.rs`, `views/inspector.rs` |
| Chat Timeline & Messages | `ScrollArea`, `Message`, `MessageDensity`, `Avatar`, `AvatarSize`, `markdown::markdown`, `collapsible`, `collapsible_trigger`, `collapsible_content` | `crates/ui/src/views/chat.rs` |
| Chat Composer & Controls | `Composer`, `ComposerState`, `ComposerEvent`, `ComposerSuggestion`, `Select`, `SelectState`, `SelectItem` | `crates/ui/src/views/composer.rs` |
| Right Auxiliary Inspector | `Tabs`, `Tab`, `TabsVariant`, `DiffView`, `DiffState`, `DiffFile`, `Meter`, `Progress`, `Badge`, `BadgeVariant` | `crates/ui/src/views/inspector.rs` |
| Command Palette (`⌘K`) | `dialog_with`, `command_dialog_popup`, `Command`, `CommandState`, `CommandItem`, `CommandEvent`, `Kbd` | `crates/ui/src/views/modals.rs` |
| Settings Center (`⌘,`) | `dialog_with`, `dialog_popup`, `dialog_close`, `Sidebar`, `SidebarItem`, `ScrollArea`, `Switch`, `Select`, `Input`, `Badge`, `Kbd`, `Separator` | `crates/ui/src/views/settings.rs` |
| Update Dialog | `dialog_with`, `dialog_popup`, `dialog_header`, `dialog_title`, `dialog_description`, `dialog_footer`, `dialog_close`, `Progress` | `crates/ui/src/views/modals.rs` |
| Permission Approval Modal | `alert_dialog_with`, `alert_dialog_action`, `alert_dialog_cancel`, `Badge` | `crates/ui/src/views/modals.rs` |
| Icons & Typography | `gpui_icons::{lucide, LucideIcon}`, `gpuicn::theme::primitives::{text, surface}` | Global across `crates/ui/src/` |
