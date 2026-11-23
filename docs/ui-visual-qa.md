# LingXi T3 Native — UI Rework Visual QA & Parity Report

## 1. Native `gpuicn 0.13.4` Component Reuse Inventory

| UI Surface | Native `gpuicn` / `gpui_icons` Components Used | Implementation File |
| --- | --- | --- |
| Window Chrome & Tabs | `TitleBar`, `title_bar_window_options()`, `Button`, `Kbd`, `Badge` | [titlebar.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/titlebar.rs) |
| 3-Pane Split Layout | `Resizable`, `PaneLimits` | [shell.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/shell.rs) |
| Left Workspace & Thread Tree | `Sidebar`, `SidebarItem`, `SidebarItemSize`, `sidebar_group_label`, `sidebar_badge`, `sidebar_separator`, `Input`, `InputState`, `Kbd` | [sidebar.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/sidebar.rs) |
| Welcome / Empty States | `Empty`, `EmptyHeader`, `EmptyMedia`, `EmptyTitle`, `EmptyDescription`, `EmptyContent`, `Badge`, `Kbd` | [welcome.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/welcome.rs), [inspector.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/inspector.rs) |
| Chat Timeline | `ScrollArea`, `Message`, `MessageDensity`, `Avatar`, `AvatarSize`, `markdown::markdown`, `collapsible`, `collapsible_trigger`, `collapsible_content`, `Badge` | [chat.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/chat.rs) |
| Chat Composer | `Composer`, `ComposerState`, `ComposerEvent`, `ComposerSuggestion`, `Select`, `SelectState`, `SelectItem`, `Badge`, `Button` | [composer.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/composer.rs) |
| Right Auxiliary Inspector | `Tabs`, `Tab`, `TabsVariant`, `TabsSize`, `DiffView`, `DiffState`, `DiffFile`, `Meter`, `Progress`, `Badge`, `Input`, `ScrollArea` | [inspector.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/inspector.rs) |
| Command Palette (`⌘K`) | `dialog_with`, `command_dialog_popup`, `CommandState`, `CommandItem`, `CommandEvent`, `Kbd` | [modals.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/modals.rs) |
| Settings Center (`⌘,`) | `dialog_with`, `dialog_popup`, `Sidebar`, `SidebarItem`, `ScrollArea`, `Switch`, `Select`, `Input`, `Badge`, `Kbd`, `Separator`, `Button` | [settings.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/settings.rs) |
| Update & Permission Dialogs | `dialog_with`, `dialog_popup`, `dialog_header`, `dialog_title`, `dialog_description`, `Progress`, `Badge`, `Button` | [modals.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/modals.rs) |

---

## 2. Side-by-Side Visual Comparison Matrix (Native vs Original LingXi T3)

| Page / Surface | Original LingXi T3 (Electron) | Reworked LingXi T3 Native (GPUI + `gpuicn`) | Status |
| --- | --- | --- | --- |
| Welcome / Home | `orig_home.png`: Onboarding hero + agent status + project import | `native_welcome_{light,dark}_1440x900.png` + `1280x800` + `960x680`: `Empty` hero, 2x2 action cards, live CLI status pills | Verified |
| Main 3-Pane Chat | Workspace sidebar + thread timeline + bottom composer + collapsible right tools | `native_chat_{light,dark}_1440x900.png` + `1280x800` + `960x680`: Right inspector collapsed by default, `Message` + `markdown` + `collapsible` tool cards + `Composer` | Verified |
| Right Inspector (Diff/Term/Git/Metrics) | On-demand right panel | `native_inspector_{diff,terminal,git,metrics}_*.png`: Native `DiffView`, real PTY terminal, Git status & checkpoints, `sysinfo` + `Meter`/`Progress` + 6-platform quota | Verified |
| Command Palette (`⌘K`) | `orig_command.png`: Centered dialog, search, grouped actions, shortcuts, footer legend | `native_command_palette_{light,dark}.png`: Centered `command_dialog_popup`, Chinese placeholder, groups, `Kbd` shortcuts, bottom keyboard legend | Verified |
| Settings — General | `orig_settings_general.png`: 2-col modal with left nav + right grouped cards | `native_settings_general_{light,dark}.png` + `960x680`: Responsive modal, single close button, crisp borders, independent `ScrollArea` | Verified |
| Settings — Appearance | Light / Dark / System selector | `native_settings_appearance_light.png`: 3 interactive preview cards + Coss UI visual spec & reduced motion | Verified |
| Settings — Providers | 12 provider statuses + custom config | `native_settings_providers_light.png`: Live detection of 12 drivers + editable custom ACP command | Verified |
| Settings — Keybindings | Shortcut table | `native_settings_keybindings_light.png`: Structured rows with native `Kbd` keycaps | Verified |
| Settings — MCP & Skills | MCP / Skills configuration | `native_settings_mcp_light.png`: Axum `/mcp` server status + 38 discovered local skills | Verified |
| Settings — Remote | Remote server & pairing | `native_settings_remote_light.png`: `Switch` toggle, port `Input`, Scoped Token status | Verified |
| Settings — Legacy Import | Migration & maintenance | `native_settings_import_light.png`: Auto-detected `~/.lingxi-t3/userdata/statev2.sqlite`, read-only import button | Verified |
| Settings — About & Updates | Version & updates | `native_settings_about_light.png` + `native_update_dialog_light.png`: Channel `Select`, update check modal | Verified |
| Permission Confirmation | Approval prompt | `native_approval_dialog_dark.png`: Centered 500px modal with command preview & 3 decision buttons | Verified |

---

## 3. Defects Caught & Fixed During Real Screenshot Visual QA

1. **macOS Traffic Light Overlap in TitleBar**: Added `pl(px(74.0))` on macOS in `views/titlebar.rs` so traffic lights sit cleanly beside the sidebar toggle and Fox emblem.
2. **Approval Modal Clipping at 384px**: `gpuicn::alert_dialog` enforces `max_w(space(96.))` (`384px`). Migrated approval modal to `dialog_with` + `dialog_popup` (`w(px(500.0))`) so preview and 3 buttons render without truncation.
3. **Update Modal Footer Button Overlap**: Replaced `dialog_close` + `dialog_footer` with two standard `Button`s inside a right-aligned flex row in `views/modals.rs`.
4. **Settings Double `× ×` Close Button**: Removed redundant `LucideIcon::X` child / replaced `dialog_close` with a clean `Button` (`IconSm`) wired to `UiAction::CloseModal`.
5. **Backdrop Contrast & Modal Borders**: Added explicit `overlay` alpha colors in `theme.rs`, `1px` borders on all modal popups, `1px` sidebar separator in Settings, and responsive `176px` nav width below `1080px` viewport width.
6. **Command Palette Polish**: Set Chinese placeholder (`"输入命令、工作区或会话名称…"`) and added bottom keyboard legend (`↑ ↓ 导航 · Enter 选择 · Esc 关闭`).
7. **Tool Call Trigger Alignment**: Added `.w_full().justify_start()` to `collapsible_trigger` in `views/chat.rs`.

---

## 4. Verification Summary

- `cargo fmt --all -- --check`: Passed
- `cargo clippy --workspace --all-targets -- -D warnings`: Passed
- `cargo test --workspace`: All unit & integration tests passed (0 failures)
- Real GUI Window Screenshot QA: 27 Native PNGs + 6 Isolated Original PNGs inspected visually via `read_file`
- Release Build & macOS App/DMG Packaging: `cargo build --release --bin lingxi-t3-native && ./scripts/package-macos.sh`

---

## 5. v2 Visual Refinement & Design System Overhaul (Claude Desktop Level)

### 5.1 Design Tokens & Typography Architecture

- **Centralized Design Tokens** ([tokens.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/tokens.rs), [theme.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/theme.rs)):
  - **Typography Scale**: Page Title `22px`, Section Title `15.5px`, Chat Prose `14.5px` (`lh 23px`), Regular UI `13.5px` (`lh 20px`), Caption `12.5px` (`lh 18px`), Code/Diff/Terminal `13px` (`lh 20px`); unified weights `400 / 500 / 600`.
  - **3-Font Stack & Licensing Compliance** ([fonts.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/fonts.rs), [font-licensing.md](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/docs/font-licensing.md)):
    - Latin & numerals: **Nunito Sans** (SIL OFL 1.1, embedded in `assets/fonts/`).
    - Code / Diff / Terminal: **JetBrains Mono** (SIL OFL 1.1, embedded in `assets/fonts/`).
    - Simplified Chinese: **MiSans** (local-only in `.gitignore`d `assets/fonts/local/`, fetched via `scripts/fetch-local-fonts.sh`, embedded into internal `.app` bundle at `Contents/Resources/fonts/` with PingFang SC / Microsoft YaHei / Noto Sans CJK SC fallbacks).
- **4-Level Elevation & Surface Hierarchy** (Independent Light Warm Cream & Layered Dark Charcoal palettes):
  - **Level 0** (Canvas `#FAF7F2` / `#1A1816`, Sidebar `#F3EEE6` / `#141312`, borderless Agent prose, soft User bubble): zero shadow.
  - **Level 1** (Floating Composer): soft ambient shadow (`0 2px 10px` light / `0 2px 12px` + `1px` top inset highlight dark).
  - **Level 2** (Popover / Select dropdown / Command Palette): elevated crisp surface (`0 8px 20px` light / `0 8px 22px` + `1px` top inset dark).
  - **Level 3** (Settings / Update / Approval Modals): deepest elevation (`0 14px 36px` light / `0 16px 38px` + `1px` top inset dark) with backdrop scrim.

### 5.2 v2 Visual QA Matrix & Caught Refinements

All 21 v2 screenshots across `1440×900`, `1280×800`, and `960×680` (Light & Dark) were visually inspected via `read_file` (`Welcome`, `Chat`, `Model Selector` open, `Command Palette`, `Settings` General/Appearance/Providers/About, `Diff Inspector`, `Permission Approval`, `Update Check`).

Final polish caught and resolved during v2 visual inspection:
1. **Composer Clipping at `1280×800` Chat**: Added `.h_full().overflow_hidden()` to `center_pane` in [shell.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/shell.rs) and `.overflow_hidden()` to `render_chat_pane` in [chat.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/chat.rs) so `ScrollArea` strictly flexes and keeps `Composer` anchored inside the viewport.
2. **Modal Shadow Bounding-Box Clipping**: `gpuicn::dialog::modal_viewport` uses `.overflow_y_scroll()`, which clipped wide `BoxShadow`s on modal popups. Wrapped each modal popup in `div().px(px(28.0)).py(px(24.0))` in [modals.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/modals.rs) and tuned Level 2/3 blur radii (`20px` / `36px`) in [tokens.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/tokens.rs).
3. **`960×680` Settings Breathing Room**: Adjusted Settings modal height clamp to `(viewport.height - px(120.0)).clamp(px(400.0), px(540.0))` in [settings.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/settings.rs) and `.w_full().max_w(px(780.0))` in [modals.rs](file:///Volumes/LingXiWorkspace/Projects/LingXiT3-Native/crates/ui/src/views/modals.rs).

### 5.3 Release Benchmark Comparison (`--bench`)

| Metric | Baseline (Pre-Refinement) | Post-Refinement (v2 + Fonts & Shadows) | Delta |
| --- | --- | --- | --- |
| `rss_mib` | `21.34375 MiB` (`22,380,544 B`) | `21.796875 MiB` (`22,855,680 B`) | `+0.45 MiB` |
| `threads_10k_filter_ms` | `1.19075 ms` | `1.46575 ms` | `+0.275 ms` |
| `markdown_500_streaming_chunks_ms` | `8.065375 ms` | `8.637875 ms` | `+0.573 ms` |
| `sqlite_1000_messages_write_ms` | `166.383542 ms` | `170.094083 ms` | `+3.711 ms` |
| `sqlite_1000_messages_read_ms` | `0.414292 ms` | `0.429125 ms` | `+0.015 ms` |

- `cargo fmt --all -- --check`: Passed
- `cargo clippy --workspace --all-targets -- -D warnings`: Passed (0 warnings)
- `cargo test --workspace`: Passed (all crates, 0 failures)
- macOS Packaging (`./scripts/package-macos.sh`): Produced `dist/LingXi T3 Native.app` & `dist/LingXiT3-Native-0.1.0-macos-arm64.dmg`
