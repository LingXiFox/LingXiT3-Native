# Phase 0 — 协议与数据兼容矩阵

## 1. Provider 与 Agent 协议兼容策略

| Provider | 原版协议 / 驱动 | Rust 原生实现策略 | 兼容边界与约束 |
| --- | --- | --- | --- |
| **Codex** | `codex` CLI / App Server JSON-RPC | `crates/providers/src/codex.rs` | 直接通过 `tokio::process::Command` 启动 `codex` 子进程，支持安装/认证检测、模型发现、reasoning effort、流式事件与会话恢复。 |
| **Claude Code** | `claude` CLI `--output-format stream-json` | `crates/providers/src/claude.rs` | 解析 NDJSON `stream-json` 事件流、思考增量、工具调用与权限模式映射，支持 `--resume` 会话恢复。 |
| **Antigravity** | `agy acp` (ACP JSON-RPC 2.0) | `crates/providers/src/antigravity.rs` + `crates/acp` | 严格复刻 `AntigravityAcpSupport.ts`：版本检测 (`>=1.5.1` clientInfo, `>=1.5.0` prompt steering)、`permissionMode` 映射、保留原生 Orchestrator 预设；Planner/Executor 独立配置仅当真实 `configOptions` 存在对应字段时暴露。 |
| **DSH** | `dsh --profile acp` (ACP JSON-RPC 2.0) | `crates/providers/src/dsh.rs` + `crates/acp` | 严格复刻 `packages/provider-dsh`：`dsh --version >= 0.2.0` 校验、扁平化分组模型与 `["provider","model"]` wire tuple、`reasoning_effort` 动态读取。 |
| **Cursor** | `cursor` / `cursor-agent` CLI | `crates/providers/src/cursor.rs` | 检测安装与认证状态，按真实 CLI 参数执行并翻译事件流。 |
| **Grok** | `grok` CLI | `crates/providers/src/grok.rs` | CLI 安装检测、模型发现与流式输出翻译。 |
| **OpenCode** | `opencode` CLI / ACP | `crates/providers/src/opencode.rs` | 优先 ACP / CLI JSON 流协议适配，支持模型发现与恢复。 |
| **Pi** | `pi` CLI RPC | `crates/providers/src/pi.rs` | 检测 `pi` 可执行文件并通过 stdio RPC 驱动会话。 |
| **Muse Code** | `muse` CLI | `crates/providers/src/muse.rs` | CLI 检测、流式输出与会话管理。 |
| **Kiro** | `kiro` CLI / ACP | `crates/providers/src/kiro.rs` | CLI/ACP 检测与事件流适配。 |
| **fx** | `fx` CLI | `crates/providers/src/fx.rs` | CLI 检测与命令流适配。 |
| **ACP Registry** | 自定义 ACP 命令与参数 | `crates/providers/src/acp_registry.rs` | 完全通过真实 `initialize` + `session/new` 返回的 `configOptions` / `modeState` 动态构建模型与思考等级选项，绝不伪造。 |

## 2. 外部客户端与远程 WebSocket/RPC 协议

| 接口面 | 兼容决策 | 说明 |
| --- | --- | --- |
| **本地桌面 UI ↔ Rust 核心** | **Rust 内部直接通道** | GPUI 与 Tokio 运行在同一进程内，通过 `mpsc` / `broadcast` / `Arc` 通信，零 WebSocket 序列化开销。 |
| **Web / Mobile 客户端 ↔ Rust 后端 (`crates/remote`)** | **保持 T3 Code WS/RPC 协议兼容** | 暴露 `/ws`、`/api/health`、`/api/server-config`、`/api/attachments/*`，支持 JSON-RPC 方法分派、Scope 鉴权 (`read`/`write`/`admin`)、设备配对码及基于 `sequence` 游标的增量重连追赶。 |
| **MCP 工具接口 (`crates/mcp`)** | **标准 MCP JSON-RPC 2.0 (HTTP)** | 暴露标准 `initialize` / `tools/list` / `tools/call`，供外部或子 Agent 调用工作区/会话受控工具。 |

## 3. SQLite 数据与迁移兼容决策

- 原生数据库路径：`~/.lingxi-t3-native/userdata/native.sqlite`（独立，不触碰 `~/.lingxi-t3/userdata/statev2.sqlite`）。
- 原生核心表保留清晰的事件溯源与投影分离结构（`orchestration_events`, `orchestration_command_receipts`, `orchestration_effect_outbox`, `projects`, `threads`, `turns`, `turn_items`, `messages`, `activities`, `pending_approvals`, `checkpoints`, `settings`, `drafts`, `scheduled_tasks`, `remote_peers`）。
- **只读导入器**：以 `SQLITE_OPEN_READ_ONLY` 读取旧版 `statev2.sqlite` 或 `state.sqlite` 的项目、会话、消息与 Turn 投影表，导入前对目标原生库生成时间戳备份以支持回滚。
