# LingXi T3 Native

基于 Rust + [GPUI](https://www.gpui.rs/) + [gpuicn](https://github.com/karyalabs/gpuicn) + [GPUI Kit](https://github.com/longbridge/gpui-kit) 原生重构的 **LingXi T3** 桌面客户端，完全摆脱 Electron、Chromium 与 Node.js 运行时依赖。

- **独立仓库**：`https://github.com/LingXiFox/LingXiT3-Native`
- **参考基线**：`LingXiFox/t3code` (`lingxifox`) @ `5e8e265d1e4571187f2777b20c92074e9157ce3f`
- **应用身份与数据隔离**：`com.lingxifox.lingxit3.native` / `~/.lingxi-t3-native` / `lingxit3-native://`（绝不覆盖或修改旧版 `~/.lingxi-t3` 与原版仓库）

## 核心特性

- **原生 GPU 渲染与极简视觉**：奶白（Light `#FAF6F0`）/ 炭黑（Dark `#121110`）/ 狐橙（`#F07C38`）纯色面板与 1px 细边框，无毛玻璃、无动态星空、无多余持续动画。
- **Event-Sourced Orchestration V2**：纯决策 `Orchestrator`、SQLite WAL 单事务 `EventSink`（事件 + 投影 + 命令回执 + Outbox）、并发 `EffectWorker` 与异常退出恢复。
- **完整 ACP 协议与 12 个桌面 Provider**：支持 Codex、Claude Code、Antigravity（保留原生 Orchestrator 支持）、DeepSeek Harness (`dsh --profile acp`)、Cursor、Grok、OpenCode、Pi、Muse Code、Kiro、fx 及自定义 ACP Registry。
- **只读旧版数据导入**：严格以 `SQLITE_OPEN_READ_ONLY` 读取旧版 Electron SQLite 数据库，导入前自动创建原生库时间戳备份并支持回滚。
- **完整开发工作台服务**：`portable-pty` 多标签终端、原生 `git`/`gh` 工作流与隐藏 Ref Checkpoint、工作区文件扫描/搜索、内置 MCP HTTP 服务端、6 平台额度与上下文监控中心、T3 兼容 WebSocket/RPC 远程服务端、Stable/Nightly 双通道更新器。

## 快速构建与运行

```bash
# 格式检查、Clippy 与全量测试
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 运行原生桌面端
cargo run --release --bin lingxi-t3-native

# 无头启动自检 / 探查本机 Provider / 基准测试 / 无头远程服务
cargo run --release --bin lingxi-t3-native -- --check-startup
cargo run --release --bin lingxi-t3-native -- --probe-providers
cargo run --release --bin lingxi-t3-native -- --bench
cargo run --release --bin lingxi-t3-native -- --headless-server

# 只读导入旧版 LingXi T3 数据
cargo run --release --bin lingxi-t3-native -- --import-legacy ~/.lingxi-t3/userdata/statev2.sqlite
```

## 文档索引

- [架构说明](docs/architecture.md)
- [Phase 0 审计报告](docs/audit/phase0-audit.md)
- [协议兼容矩阵](docs/audit/protocol-compatibility.md)
- [功能迁移验收矩阵](docs/audit/feature-matrix.md)
- [性能基准测试报告](docs/benchmarks/performance-report.md)
- [Phase 10 最终验收报告](docs/acceptance-report.md)

## 开源协议

[MIT](LICENSE) — Copyright (c) 2026 T3 Tools Inc. (Ping Labs / T3 Code) & LingXiFox。第三方组件声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
