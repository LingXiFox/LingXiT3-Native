# LingXiT3-Native 性能基准测试报告 (Phase 7)

## 测试环境

- **设备**：macOS Apple Silicon (aarch64-apple-darwin)
- **Rust 工具链**：`rustc 1.100.0` / `cargo 1.100.0` (`--release` 优化配置：`opt-level = "s"`, `lto = "thin"`, `strip = "debuginfo"`)
- **测试命令**：
  - `target/release/lingxi-t3-native --check-startup --data-dir /tmp/lingxi-bench-check`
  - `target/release/lingxi-t3-native --bench`

## 实测结果

| 指标 | LingXiT3-Native (Rust + GPUI) 实测 | 说明 / 对比参考 |
| --- | --- | --- |
| **原生 Release 二进制体积** | **28 MiB** (`target/release/lingxi-t3-native`) | 零 Electron / Chromium / Node.js 运行时；原版 `apps/desktop/dist-electron` 仅打包构建产物已占 `29 MiB`（不含 Electron.app 本体） |
| **进程冷启动自检耗时 (`--check-startup`)** | **0.64s wall** (`0.00s user`, `0.01s sys`) | 包含 SQLite 初始化、Schema 迁移、单实例锁及 Tokio 多线程运行时就绪 |
| **基准测试后进程 RSS 内存** | **21.30 MiB** (`22,331,392 bytes`) | 含 SQLite 内存投影、Tokio 线程池与 10,000 条会话结构 |
| **10,000 历史会话过滤耗时** | **1.15 ms** | 从 10,000 个会话中按关键词与置顶排序过滤 |
| **500 次流式 Markdown 增量更新 (35 KB)** | **6.82 ms** | 使用 `StreamingMarkdownCache`，已闭合段落与代码块不重复解析 |
| **SQLite 1,000 条消息事件溯源写入** | **164.64 ms** | 含 `orchestration_events` + `orchestration_command_receipts` + `messages` + `turns` 同事务提交 |
| **SQLite 1,000 条消息投影读取** | **0.44 ms** | 复合索引 `idx_messages_thread_created` 覆盖 |

## 架构级性能优化措施

1. **非阻塞冷启动**：Provider CLI 安装与版本探测在窗口打开后于后台 Tokio 任务中异步执行，不阻塞首帧渲染。
2. **事件驱动零轮询 UI**：GPUI 仅在 `Orchestrator` 广播事件或用户交互触发 `cx.notify()` 时重绘；主题明确关闭多余动画 (`motion.reduced = true`) 与毛玻璃特效。
3. **增量流式 Markdown 缓存**：`StreamingMarkdownCache` 追踪已闭合的双换行边界，仅重解析尾部活跃块。
4. **有界环形终端缓冲**：`TerminalManager` 对每个 PTY 会话强制 `MAX_RING_LINES = 5000` 上限，避免海量终端输出导致内存无限增长。

## 诚实边界说明

- 以上原生 Rust 指标均在当前 Apple Silicon 设备上真实运行 `target/release/lingxi-t3-native` 测得。
- 原版 Electron 应用的实时 GUI 帧率与运行时 RSS 未在本次无头自动化会话中并行拉起实测，故不填写猜测数字，仅列出已核查的磁盘构建体积对比。
