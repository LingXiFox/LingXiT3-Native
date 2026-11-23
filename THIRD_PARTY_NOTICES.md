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

A complete machine-generated license inventory is verified before final release in Phase 10.
