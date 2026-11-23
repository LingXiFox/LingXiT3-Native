pub mod antigravity;
pub mod claude;
pub mod cli_adapters;
pub mod codex;
pub mod driver;
pub mod dsh;

pub use driver::{ProviderDriver, ProviderRegistry, ProviderTurnEvent, TurnRequest};
use lingxi_domain::ProviderKind;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider {0:?} is not installed")]
    NotInstalled(ProviderKind),
    #[error("provider {0:?} execution failed: {1}")]
    Execution(ProviderKind, String),
    #[error("acp error: {0}")]
    Acp(#[from] lingxi_acp::AcpError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use lingxi_acp::AcpSessionConfigOption;

    #[test]
    fn registry_registers_all_12_providers() {
        let reg = ProviderRegistry::with_all_builtin();
        for kind in ProviderKind::ALL {
            assert!(reg.get(kind).is_some(), "missing driver for {kind:?}");
        }
    }

    #[test]
    fn dsh_version_and_config_options_parsing_work() {
        assert!(!dsh::is_dsh_version_compatible(Some("dsh 0.1.9")));
        assert!(dsh::is_dsh_version_compatible(Some("dsh 0.2.2-rc.5")));

        let raw: Vec<AcpSessionConfigOption> = serde_json::from_value(serde_json::json!([
            {
                "id": "model",
                "name": "Model",
                "category": "model",
                "type": "select",
                "currentValue": "[\"deepseek\",\"deepseek-chat\"]",
                "options": [
                    {
                        "groupId": "deepseek",
                        "name": "DeepSeek",
                        "options": [
                            {"value": "[\"deepseek\",\"deepseek-chat\"]", "name": "Chat"}
                        ]
                    },
                    {
                        "groupId": "openai",
                        "name": "OpenAI",
                        "options": [
                            {"value": "[\"openai\",\"gpt-5\"]", "name": "Chat"}
                        ]
                    }
                ]
            },
            {
                "id": "reasoning_effort",
                "name": "Reasoning Effort",
                "type": "select",
                "currentValue": "high",
                "options": [
                    {"value": "low", "name": "Low"},
                    {"value": "high", "name": "High"}
                ]
            }
        ]))
        .expect("parse dsh options");

        let models = dsh::parse_dsh_models_from_config_options(&raw);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].slug, "[\"deepseek\",\"deepseek-chat\"]");
        assert_eq!(models[0].name, "Chat (DeepSeek)");
        assert!(models[0].is_default);
        assert!(models[0].capabilities.supports_thinking);
    }

    #[test]
    fn antigravity_never_fabricates_planner_executor_split_without_config_options() {
        let raw: Vec<AcpSessionConfigOption> = serde_json::from_value(serde_json::json!([
            {
                "id": "model",
                "name": "Model",
                "type": "select",
                "currentValue": "orchestrator",
                "options": [
                    {"value": "orchestrator", "name": "Antigravity Orchestrator"}
                ]
            }
        ]))
        .expect("parse");
        let models = antigravity::parse_antigravity_models(&raw);
        assert_eq!(models.len(), 1);
        assert!(models[0].capabilities.supports_orchestrator);
        assert!(!models[0].capabilities.supports_planner_executor_split);

        assert_eq!(
            antigravity::antigravity_client_info(Some("1.4.9")).name,
            "zed"
        );
        assert_eq!(
            antigravity::antigravity_client_info(Some("1.5.2")).name,
            "lingxi-t3-native"
        );
    }

    #[test]
    fn claude_and_codex_stream_parsers_translate_deltas_and_completion() {
        let claude_events = claude::parse_claude_stream_json_line(
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"plan"},{"type":"text","text":"done"}]}}"#,
        );
        assert_eq!(claude_events.len(), 2);

        let codex_ev =
            codex::parse_codex_jsonl_line(r#"{"type":"turn.completed","thread_id":"codex-th-1"}"#);
        assert!(matches!(
            codex_ev,
            Some(ProviderTurnEvent::Completed {
                provider_session_id: Some(ref s)
            }) if s == "codex-th-1"
        ));
    }

    #[tokio::test]
    async fn local_provider_detection_runs_without_panics() {
        let reg = ProviderRegistry::with_all_builtin();
        let statuses = reg.detect_all(None).await;
        assert_eq!(statuses.len(), 12);
    }
}
