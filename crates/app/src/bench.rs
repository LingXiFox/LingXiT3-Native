use std::time::Instant;

use chrono::Utc;
use lingxi_domain::{
    AppSettings, CommandId, MessageId, ModelSelection, OrchestrationCommand, ProjectId,
    ProviderKind, RuntimeMode, ThreadId, ThreadRecord, ThreadRunStatus, TurnId,
};
use lingxi_orchestration::Orchestrator;
use lingxi_storage::Database;
use lingxi_telemetry::sample_process_resources;
use lingxi_ui::markdown::StreamingMarkdownCache;
use lingxi_ui::state::AppUiState;

pub fn run_native_benchmarks(rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    // 1. 10,000-thread filter benchmark
    let mut ui = AppUiState::new(AppSettings::default());
    let pid = ProjectId::new();
    let now = Utc::now();
    for idx in 0..10_000usize {
        ui.threads.push(ThreadRecord {
            id: ThreadId::from_string(format!("t-{idx}")),
            project_id: pid.clone(),
            title: format!("Benchmark Thread #{idx}"),
            model_selection: ModelSelection {
                provider: ProviderKind::Codex,
                model_slug: "gpt-5".into(),
                thinking_level: None,
                planner_model: None,
                executor_model: None,
            },
            runtime_mode: RuntimeMode::ApprovalRequired,
            status: ThreadRunStatus::Idle,
            branch: Some("main".into()),
            worktree_path: None,
            provider_session_id: None,
            pinned: idx < 5,
            archived: false,
            unread: false,
            created_at: now,
            updated_at: now,
        });
    }
    ui.sidebar_filter = "999".into();
    let t0 = Instant::now();
    let filtered = ui.filtered_threads_for_project(&pid);
    let filter_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // 2. Streaming Markdown incremental cache benchmark (100KB across 500 chunks)
    let chunk = "Paragraph text with **bold** and `inline`.\n\n```rust\nfn demo() {}\n```\n\n";
    let mut cache = StreamingMarkdownCache::default();
    let mut doc = String::with_capacity(chunk.len() * 500);
    let t1 = Instant::now();
    for _ in 0..500 {
        doc.push_str(chunk);
        let _ = cache.update(&doc);
    }
    let md_ms = t1.elapsed().as_secs_f64() * 1000.0;

    // 3. SQLite 1,000-message dispatch + query benchmark
    let db = Database::open_in_memory()?;
    let orch = Orchestrator::new(db.clone());
    let tid = ThreadId::new();
    let t2 = Instant::now();
    rt.block_on(async {
        orch.dispatch(OrchestrationCommand::CreateProject {
            command_id: CommandId::new(),
            project_id: pid.clone(),
            title: "Bench".into(),
            workspace_root: "/tmp/bench".into(),
        })
        .await?;
        orch.dispatch(OrchestrationCommand::CreateThread {
            command_id: CommandId::new(),
            thread_id: tid.clone(),
            project_id: pid,
            title: "Bench Thread".into(),
            model_selection: ModelSelection {
                provider: ProviderKind::Codex,
                model_slug: "gpt-5".into(),
                thinking_level: None,
                planner_model: None,
                executor_model: None,
            },
            runtime_mode: RuntimeMode::Auto,
            branch: None,
            worktree_path: None,
        })
        .await?;
        for i in 0..500 {
            orch.dispatch(OrchestrationCommand::StartTurn {
                command_id: CommandId::new(),
                thread_id: tid.clone(),
                turn_id: TurnId::new(),
                message_id: MessageId::new(),
                prompt: format!("prompt {i}"),
                attachments: vec![],
            })
            .await?;
            let msgs = db.list_messages(&tid)?;
            let last_id = msgs.last().unwrap().id.clone();
            orch.dispatch(OrchestrationCommand::CompleteTurn {
                command_id: CommandId::new(),
                thread_id: tid.clone(),
                turn_id: TurnId::new(),
                message_id: last_id,
                provider_session_id: None,
                error: None,
            })
            .await?;
        }
        anyhow::Ok(())
    })?;
    let sqlite_write_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let t3 = Instant::now();
    let msgs = db.list_messages(&tid)?;
    let sqlite_read_ms = t3.elapsed().as_secs_f64() * 1000.0;

    let res = sample_process_resources();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "threads_10k_filter_ms": filter_ms,
            "threads_10k_matched": filtered.len(),
            "markdown_500_streaming_chunks_ms": md_ms,
            "markdown_bytes": doc.len(),
            "sqlite_1000_messages_write_ms": sqlite_write_ms,
            "sqlite_1000_messages_read_ms": sqlite_read_ms,
            "sqlite_messages_count": msgs.len(),
            "rss_bytes": res.memory_rss_bytes,
            "rss_mib": res.memory_rss_bytes as f64 / (1024.0 * 1024.0),
        }))?
    );
    Ok(())
}
