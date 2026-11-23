# LingXiT3-Native 功能迁移验收矩阵

> 基线参考：`LingXiFox/t3code` (`lingxifox`) @ `5e8e265d1e4571187f2777b20c92074e9157ce3f`
> 状态说明：`Pending`（待实现） / `In Progress`（进行中） / `Verified`（代码+测试已验证） / `Partial`（部分完成或受外部环境限制） / `Blocked`（受阻）

| ID | 类别 | 功能项 | 目标 Crate | 验收方式 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| F-01 | 基础工程 | 15-Crate Cargo Workspace 与锁定 `Cargo.lock` | `workspace` | `cargo check --workspace` | Verified |
| F-02 | 基础工程 | GPUI 原生窗口、应用生命周期与 Tokio 桥接 | `app` | `--check-startup` + 手动编译运行 | Verified |
| F-03 | 视觉主题 | `gpuicn` + `gpui-kit` 集成与奶白/炭黑/狐橙 Light/Dark 主题 | `ui` | 主题对比度/token 单元测试 | Verified |
| F-04 | 平台抽象 | macOS/Windows/Linux 路径、单实例锁、通知、剪贴板、密钥链 | `platform` | 跨平台抽象与路径单元测试 | Verified |
| F-05 | UI 布局 | 工作区树状侧边栏（文件夹 + 活跃/历史会话 + Worktree） | `ui`, `app` | UI 状态与过滤单元测试 | Verified |
| F-06 | UI 布局 | 多标签页与会话切换 | `ui`, `app` | Tab 状态切换测试 | Verified |
| F-07 | UI 聊天 | Agent 聊天视图、流式 Markdown、代码块高亮、思考折叠 | `ui` | Markdown 分块与高亮测试 | Verified |
| F-08 | UI 聊天 | 富文本 Composer（IME 中文输入、`@file`、图片附件、剪贴板粘贴） | `ui`, `app` | `@` 补全与附件状态测试 | Verified |
| F-09 | UI 聊天 | 模型 & 思考等级 & 运行时权限模式选择器 | `ui`, `app` | 能力映射与选择测试 | Verified |
| F-10 | UI 交互 | Tool Call 卡片、内联权限确认、Checkpoint 回滚入口 | `ui`, `app` | 权限状态联动测试 | Verified |
| F-11 | UI 面板 | 内嵌 PTY 终端、Code Diff、只读文件预览、Git & PR 面板 | `ui`, `terminal`, `git` | 面板状态与 Diff 解析测试 | Verified |
| F-12 | UI 监控 | 上下文窗口分解与 6 平台/Antigravity 多号池额度监控 | `ui`, `telemetry` | 指标聚合测试 | Verified |
| F-13 | UI 全局 | 设置中心、快捷键自定义、`Cmd+K` 命令面板、全局更新弹窗 | `ui`, `app` | 模糊搜索与快捷键测试 | Verified |
| F-14 | 编排核心 | 纯决策 `Orchestrator`、幂等 `command_id` 去重、单调事件序列 | `orchestration` | 编排器核心行为测试 | Verified |
| F-15 | 编排核心 | `EventSink` 单事务写入 + `EffectWorker` 并发执行与优雅取消 | `orchestration`, `storage` | 事务一致性与并发测试 | Verified |
| F-16 | 编排核心 | 上下文压缩、任务交接、Checkpoint 捕获/回滚、崩溃恢复 | `orchestration`, `git` | 故障恢复与回滚测试 | Verified |
| F-17 | ACP 协议 | 完整 ACP JSON-RPC 2.0（`initialize`, `session/*`, `configOptions`, 权限） | `acp` | Mock ACP 进程契约测试 | Pending |
| F-18 | Providers | 12 个桌面 Provider 适配器（Codex/Claude/Antigravity/DSH 等） | `providers` | 契约测试 + 本机真实 CLI 探测 | Pending |
| F-19 | 数据存储 | SQLite Schema、版本迁移、草稿/设置/投影持久化 | `storage` | 迁移与 CRUD 集成测试 | Pending |
| F-20 | 数据迁移 | 独立只读旧版数据导入器与时间戳备份回滚 | `storage` | 只读 SHA256 零变更 + 回滚测试 | Pending |
| F-21 | 系统服务 | 本地文件扫描/搜索、Worktree 管理、原生 Git/Diff/PR | `workspace`, `git` | 临时 Git 仓库集成测试 | Pending |
| F-22 | 系统服务 | 跨平台 PTY 终端引擎、MCP HTTP Server、Skills 发现、定时任务 | `terminal`, `mcp`, `telemetry` | PTY/MCP/Cron 集成测试 | Pending |
| F-23 | 远程兼容 | T3 兼容 WS/RPC 服务端、多端配对、SSH 工作区、断线重连 | `remote` | WS/RPC 端到端集成测试 | Pending |
| F-24 | 性能优化 | 启动/内存/虚拟列表/流式 Markdown/终端输出基准对比 | `app`, `docs` | 本机实测基准报告 | Pending |
| F-25 | 构建发行 | Stable/Nightly 更新器、macOS `.app`/`.dmg` 打包与 3 平台 CI | `updater`, `.github` | 本机 macOS 打包 + 更新测试 | Pending |
| F-26 | 上游追踪 | 自动化 T3 Code 上游差异审计脚本与 GitHub Actions | `scripts`, `.github` | `--dry-run` 实测通过 | Pending |
| F-27 | 最终验收 | 全量测试、IME/中文字体、许可证检查、交付文档与验收报告 | `docs` | Phase 10 验收清单 | Pending |
