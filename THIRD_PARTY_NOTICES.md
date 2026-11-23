# Third-Party Notices and Attributions

`LingXiT3-Native` is a Rust + GPUI native desktop rewrite of [LingXi T3](https://github.com/LingXiFox/t3code), which itself is derived from [T3 Code](https://github.com/pingdotgg/t3code).

## Upstream & Reference Projects

- **T3 Code** (`pingdotgg/t3code`) — MIT License, Copyright (c) 2026 T3 Tools Inc.
- **LingXi T3** (`LingXiFox/t3code`, branch `lingxifox`) — MIT License, Copyright (c) 2026 LingXiFox.

## Core Rust & UI Dependencies

All Rust crate dependencies are resolved and locked in `Cargo.lock` and remain subject to their respective open-source licenses (MIT / Apache-2.0 / BSD-3-Clause / MPL-2.0):

- `gpuicn` (`karyalabs/gpuicn`) — MIT License
- `gpui-kit` / `gpui-base` (`longbridge/gpui-kit`) — Apache-2.0
- `gpui-pre-platform` / `gpui` (Zed Industries GPUI ecosystem) — Apache-2.0 / GPL-3.0-or-later (per upstream crate license)
- `gpui-icons` — MIT / Apache-2.0
- `tokio`, `axum`, `tokio-tungstenite`, `reqwest`, `serde`, `serde_json`, `rusqlite`, `portable-pty`, `sysinfo`, `keyring`, `arboard`, `notify`, `tracing`, `clap`, `thiserror`, `anyhow` — MIT / Apache-2.0

## Typography & Font Attributions

- **Nunito Sans** — SIL Open Font License 1.1 (Copyright © The Nunito Project Authors). See [NunitoSans-OFL.txt](assets/fonts/NunitoSans-OFL.txt).
- **JetBrains Mono** — SIL Open Font License 1.1 (Copyright © 2020 The JetBrains Mono Project Authors). See [JetBrainsMono-OFL.txt](assets/fonts/JetBrainsMono-OFL.txt).
- **MiSans** — Xiaomi MiSans Font Intellectual Property License Agreement (Copyright © Beijing Xiaomi Mobile Software Co., Ltd.). Embedded locally inside packaged application bundles only; not redistributed standalone. See [docs/font-licensing.md](docs/font-licensing.md).

A complete machine-generated license inventory is verified before final release in Phase 10.
