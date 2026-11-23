# Font Licensing & Fallback Policy

LingXi T3 Native uses a three-font typographic system tuned for calm readability across Latin, numerals, Simplified Chinese, and code:

| Role | Family | License | Distribution Strategy |
| --- | --- | --- | --- |
| Latin & Numerals UI | **Nunito Sans** | SIL Open Font License 1.1 (`assets/fonts/NunitoSans-OFL.txt`) | Embedded directly in `assets/fonts/NunitoSans.ttf` |
| Code, Terminal & Diff | **JetBrains Mono** | SIL Open Font License 1.1 (`assets/fonts/JetBrainsMono-OFL.txt`) | Embedded directly in `assets/fonts/JetBrainsMono-{Regular,Medium}.ttf` |
| Simplified Chinese UI | **MiSans** | Xiaomi MiSans Intellectual Property License Agreement | Downloaded locally via `scripts/fetch-local-fonts.sh` into gitignored `assets/fonts/local/` and embedded inside the packaged `.app` bundle (`Contents/Resources/fonts/`); never redistributed as standalone font files in the public repository |

## Cross-Platform Fallback Chain

If MiSans is not present in a source checkout or custom environment, GPUI resolves CJK glyphs cleanly using platform-native system fonts:

- **Primary UI stack**: `"Nunito Sans"` -> `["MiSans", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", "Noto Sans CJK SC"]`
- **Monospace stack**: `"JetBrains Mono"` -> `["MiSans", "PingFang SC", "Microsoft YaHei UI", "Noto Sans CJK SC"]`
- **macOS**: `PingFang SC` / `Hiragino Sans GB`
- **Windows**: `Microsoft YaHei UI`
- **Linux**: `Noto Sans CJK SC`

## Attributions

- **Nunito Sans**: Copyright © The Nunito Project Authors. Licensed under the SIL Open Font License, Version 1.1.
- **JetBrains Mono**: Copyright © 2020 The JetBrains Mono Project Authors. Licensed under the SIL Open Font License, Version 1.1.
- **MiSans**: Copyright © Beijing Xiaomi Mobile Software Co., Ltd. Used in accordance with the MiSans Font Intellectual Property License Agreement.
