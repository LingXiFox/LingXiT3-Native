# LingXiT3-Native 架构说明

## 1. 15-Crate Cargo Workspace 分层

```text
                  ┌──────────────────────────────────────┐
                  │             crates/app               │
                  │  GPUI Window + Tokio Bridge + CLI    │
                  └──────┬───────────────┬───────────────┘
                         │               │
           ┌─────────────▼───┐       ┌───▼────────────────────────┐
           │    crates/ui    │       │   crates/orchestration     │
           │ gpuicn + Theme  │       │ Orchestrator + EffectWorker│
           └────────┬────────┘       └───┬──────────┬─────────┬───┘
                    │                    │          │         │
                    │        ┌───────────▼──┐  ┌────▼─────┐ ┌─▼──────────┐
                    │        │crates/storage│  │providers │ │ crates/git │
                    │        │SQLite+Import │  │12 Drivers│ │CLI+Ref CP  │
                    │        └───────┬──────┘  └────┬─────┘ └──────┬─────┘
                    │                │              │              │
                    │                │         ┌────▼─────┐ ┌──────▼─────┐
                    │                │         │crates/acp│ │ workspace  │
                    │                │         │JSON-RPC  │ │Files+Watch │
                    │                │         └──────────┘ └────────────┘
                    │                │
    ┌───────────────▼────────────────▼──────────────────────────────────┐
    │                           crates/domain                           │
    │   Commands, Events, Provider/Quota/Settings/Git/Terminal Models   │
    └───────────────────────────────────────────────────────────────────┘

  横向基础设施 Crates（均只依赖 domain / storage / orchestration）：
  - crates/platform   (跨平台路径、单实例锁、剪贴板、密钥链、通知)
  - crates/terminal   (portable-pty 多会话 + 有界环形缓冲)
  - crates/mcp        (axum MCP HTTP JSON-RPC 2.0 Server + SKILL.md 发现)
  - crates/telemetry  (sysinfo 资源采样 + 上下文分解 + 6 平台诚实额度)
  - crates/remote     (axum WS/RPC 服务端 + Scope 鉴权 + SSH + 重连客户端)
  - crates/updater    (Stable/Nightly 更新检查 + SHA-256 + 原子回滚)
```

## 2. 线程与渲染模型

- **GPUI 主线程**：负责窗口渲染、`LingXiShellView` 状态响应与中文 IME/键盘快捷键处理；严禁在 `Render` 热路径执行同步文件 I/O、子进程调用或阻塞查询。
- **Tokio 后台多线程运行时**：承载 `Orchestrator` 串行命令提交、`EffectWorker` 并发 Turn 执行、ACP/CLI 子进程 stdio 流读取、`axum` MCP/Remote HTTP 服务及文件监听。
- **事件驱动刷新**：`Orchestrator` 事务提交后通过 `broadcast` 推送事件，`App` 异步协程通过 `AsyncApp::update_entity` 触发精准重绘。

## 3. 数据安全与兼容边界

- 原生数据库 `~/.lingxi-t3-native/userdata/native.sqlite` 开启 `WAL`、`foreign_keys = ON` 与 `auto_vacuum = INCREMENTAL`。
- `legacy_importer` 严格使用 `SQLITE_OPEN_READ_ONLY` 读取旧版 Electron 数据库，导入前先备份原生库。
- `crates/remote` 保持 T3 Code `/ws`、`/api/health`、`/api/server-config` 与 `sequence` 增量重连协议兼容。
