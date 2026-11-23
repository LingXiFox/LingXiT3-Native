# Phase 0 — LingXi T3 现有项目审计报告

## 1. 基线信息

- **参考本地仓库**：`/Volumes/LingXiWorkspace/Projects/LingXi-T3Code`（严格只读，不修改）
- **参考远端仓库**：`https://github.com/LingXiFox/t3code`（分支 `lingxifox`）
- **上游参考仓库**：`https://github.com/pingdotgg/t3code`
- **基线 Commit SHA**：`5e8e265d1e4571187f2777b20c92074e9157ce3f`
- **基线提交信息**：`fix(ci): sync pnpm-lock.yaml and localize README to Chinese`

## 2. 现有架构与目录审计

### 2.1 应用层 (`apps/`)
- `apps/desktop`：Electron 主进程外壳（窗口管理、系统菜单、macOS/Linux/Windows 权限与 URL Scheme `lingxit3://`、安全存储 `ElectronSafeStorage`、自动更新 `ElectronUpdater`、原生能力桥接、内嵌 Node server 启动与关闭回收）。
- `apps/web`：React + Vite 渲染端界面（TanStack Router 路由、工作区/会话树侧边栏、Tiptap 富文本 Composer、流式 Markdown、Diff 审阅、终端 xterm/ghostty-vt、Git/PR 面板、6 平台额度与上下文缓存监控中心、设置中心、命令面板、LingXi Night Sky 主题）。
- `apps/server`：基于 TypeScript + Effect 的后端控制平面：
  - `orchestration-v2/`：纯决策 `Orchestrator` + `EventSink`（事件 + 投影 + 命令回执 + Outbox 同事务）+ `EffectWorker` + 崩溃恢复 + Checkpoint/Worktree 协调 + 后台任务/定时任务/自动结算。
  - `persistence/`：SQLite 存储（107 个 V1 迁移 + 11 个 `OrchestrationV2/` 迁移，启用增量 `auto_vacuum` 与 WAL）。
  - `provider/`：Provider 注册表、状态缓存、使用量/额度读取器、12 个内置驱动接入。
  - `ws.ts` & `http.ts`：WebSocket/RPC 服务与 HTTP 路由，支撑桌面、Web、Mobile 三端同步与配对授权。
  - `checkpointing/`、`git/`、`vcs/`、`terminal/`、`mcp/`、`preview/`、`resourceTelemetry/`、`usage/`。
- `apps/mobile`：React Native 客户端，通过 WebSocket/RPC 连接本地或远端 T3 Server。

### 2.2 共享包与驱动 (`packages/`)
- `packages/contracts`：所有跨端 RPC、Orchestration V2 命令/事件、Provider、Git、Terminal、Usage、PeerLink 类型与 Schema 契约。
- `packages/client-runtime`：Web 与 Mobile 共享连接运行时、状态同步、VCS/工作区客户端逻辑。
- `packages/effect-acp` & `packages/provider-acp` & `packages/provider-acp-registry`：ACP JSON-RPC 2.0 客户端运行时、能力发现 (`configOptions` / `modeState`)、权限策略与自定义 ACP 注册表。
- `packages/effect-codex-app-server`：Codex App Server 协议封装。
- **全部 12 个桌面 Provider 驱动**：
  1. `CodexDriver` (`apps/server/src/provider/Drivers/CodexDriver.ts`)
  2. `ClaudeDriver` (`apps/server/src/provider/Drivers/ClaudeDriver.ts`)
  3. `AntigravityDriver` (`apps/server/src/provider/Drivers/AntigravityDriver.ts` + `AntigravityAcpSupport.ts`)
  4. `DshDriver` (`packages/provider-dsh`) — DeepSeek Harness ACP (`dsh --profile acp`)
  5. `CursorDriver` (`packages/provider-cursor`)
  6. `GrokDriver` (`packages/provider-grok`)
  7. `OpenCodeDriver` (`packages/provider-opencode`)
  8. `PiDriver` (`packages/provider-pi`)
  9. `MuseDriver` (`packages/provider-muse`)
  10. `KiroDriver` (`packages/provider-kiro`)
  11. `FxDriver` (`packages/provider-fx`)
  12. `AcpRegistryDriver` (`packages/provider-acp-registry`)
- `packages/ssh` & `packages/tailscale` & `packages/source-control-*` & `packages/issue-tracker-linear` & `packages/secret-provider-*`。

### 2.3 原生模块 (`native/`)
- `native/resource-monitor`（已是 Rust crate，用于进程组 CPU/内存采集）
- `native/hyprland-snap-shot` / `native/kde-snap-shot`（Rust）
- `native/apple-speech`（Swift）
- `native/browser-secret`（C）
- `native/libghostty-vt`（终端 VT 库）

### 2.4 LingXi T3 定制特性清单 (`main..lingxifox`)
1. **身份与数据隔离**：`com.lingxifox.lingxit3`、`~/.lingxi-t3`、`~/Library/Application Support/lingxi-t3`、`lingxit3://`。
2. **LingXi 视觉与主题**：奶白 (`#F7F1E8`) / 炭黑 / 狐橙 (`#F07C38`) 品牌色；原版包含 `LingXi Night Sky` 网页 CSS 特效，**原生 Rust + GPUI 重构按项目书规定去除毛玻璃、动态星空与多余动画，保留极简线条、纯色面板、细边框、Light/Dark 双模式**。
3. **6 平台额度与上下文缓存监控中心**：Claude、Codex、Grok、Cursor、Muse、Antigravity（含多号池视图与会话上下文分解条）。
4. **DSH (`dsh --profile acp`) 与 Antigravity Orchestrator 支持**：动态读取 ACP `configOptions`，保留 Antigravity 原生 Orchestrator 预设且仅在真实接口支持时启用 Planner/Executor 独立配置。
5. **双通道 CI/CD 与签名**：`.github/workflows/lingxi-{build-desktop,nightly,stable,upstream-sync,verify}.yml`，Stable (`latest`) / Nightly (`nightly`) 双频道与 macOS Developer ID 签名公证。

## 3. Rust 原生重构模块映射 (15 Crates)

| 原有 TS 模块 | 原生 Rust Crate | 职责说明 |
| --- | --- | --- |
| `packages/contracts` | `crates/domain` | 核心领域模型、命令/事件、错误、配置、DTO |
| `apps/server/src/orchestration-v2` | `crates/orchestration` | 纯决策 Orchestrator、EventSink、EffectWorker、恢复服务 |
| `packages/effect-acp`, `provider-acp` | `crates/acp` | ACP JSON-RPC 2.0 客户端、能力发现、双向工具/权限调用 |
| `packages/provider-*`, `server/provider` | `crates/providers` | 统一 `ProviderDriver` trait 与全部 12 个 Provider 适配器 |
| `apps/server/src/persistence` | `crates/storage` | SQLite schema、版本迁移、投影持久化与只读旧库导入器 |
| `apps/server/src/workspace`, `filesystem` | `crates/workspace` | 工作区扫描、文件搜索、Worktree/TaskFolder、附件、监听 |
| `apps/server/src/git`, `vcs`, `checkpointing` | `crates/git` | 原生 `git`/`gh` CLI 封装、Diff、Checkpoint 隐藏 Ref 与 PR |
| `apps/server/src/terminal` | `crates/terminal` | `portable-pty` PTY 会话管理、环形缓冲与并发读写泵 |
| `apps/server/src/mcp`, `skills` | `crates/mcp` | 内置 MCP HTTP/JSON-RPC 服务端、工具授权、Skills 发现 |
| `apps/server/src/usage`, `resourceTelemetry` | `crates/telemetry` | 上下文分解、真实用量/额度采集、系统资源与定时任务 |
| `apps/server/src/ws.ts`, `peer/`, `ssh/` | `crates/remote` | T3 兼容 WS/RPC 服务端、多端配对、SSH 工作区与断线重连 |
| `apps/desktop/src/electron`, `shell`, `speech` | `crates/platform` | macOS/Windows/Linux 路径、单实例、通知、剪贴板、密钥链 |
| `apps/desktop/src/updates` | `crates/updater` | Stable/Nightly 双通道更新检查、校验、下载与回滚 |
| `apps/web/src/components` | `crates/ui` | `gpuicn` + `gpui-kit` 原生 UI 组件、LingXi Light/Dark 主题 |
| `apps/desktop/src/main.ts` + `apps/web` | `crates/app` | GPUI 应用入口、窗口、Tokio 桥接、状态绑定与 CLI |

## 4. 数据安全与隔离承诺

- 新应用独立数据目录：`~/.lingxi-t3-native`（可通过 `--data-dir` 覆盖，测试使用临时目录）。
- 绝对不修改、覆盖或写入 `/Volumes/LingXiWorkspace/Projects/LingXi-T3Code`、`~/.lingxi-t3`、`~/Library/Application Support/lingxi-t3` 或 `~/.t3`。
- 旧版 SQLite 数据导入器严格使用 `SQLITE_OPEN_READ_ONLY` 只读打开，并在导入前对目标原生数据库创建时间戳备份。
