use lingxi_domain::{ConfigOptionDescriptor, ModelOptionValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcpClientCapabilities {
    pub fs: AcpFsCapabilities,
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AcpFsCapabilities {
    pub read_text_file: bool,
    pub write_text_file: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpInitializeResult {
    #[serde(default)]
    pub protocol_version: Option<Value>,
    #[serde(default)]
    pub agent_info: Option<AcpAgentInfo>,
    #[serde(default)]
    pub agent_capabilities: Option<Value>,
    #[serde(default)]
    pub auth_methods: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpAgentInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionOptionChoice {
    pub value: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionOptionGroup {
    #[serde(default)]
    pub group_id: String,
    pub name: String,
    pub options: Vec<AcpSessionOptionChoice>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AcpSessionSelectEntry {
    Choice(AcpSessionOptionChoice),
    Group(AcpSessionOptionGroup),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionConfigOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(rename = "type")]
    pub option_type: String,
    #[serde(default)]
    pub current_value: Option<String>,
    #[serde(default)]
    pub options: Vec<AcpSessionSelectEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionNewResult {
    pub session_id: String,
    #[serde(default)]
    pub config_options: Vec<AcpSessionConfigOption>,
    #[serde(default)]
    pub modes: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpNormalizedEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    ToolCallUpdate {
        id: String,
        status: String,
        output: Option<String>,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
    },
    Completed {
        stop_reason: Option<String>,
    },
    Error(String),
}

pub fn normalize_config_options(raw: &[AcpSessionConfigOption]) -> Vec<ConfigOptionDescriptor> {
    raw.iter()
        .map(|opt| {
            let mut values = Vec::new();
            for entry in &opt.options {
                match entry {
                    AcpSessionSelectEntry::Choice(c) => values.push(ModelOptionValue {
                        value: c.value.clone(),
                        label: c.name.clone(),
                        group: None,
                    }),
                    AcpSessionSelectEntry::Group(g) => {
                        let group_name = if g.name.trim().is_empty() {
                            if g.group_id.trim().is_empty() {
                                None
                            } else {
                                Some(g.group_id.clone())
                            }
                        } else {
                            Some(g.name.clone())
                        };
                        for c in &g.options {
                            values.push(ModelOptionValue {
                                value: c.value.clone(),
                                label: c.name.clone(),
                                group: group_name.clone(),
                            });
                        }
                    }
                }
            }
            ConfigOptionDescriptor {
                id: opt.id.clone(),
                name: opt.name.clone(),
                category: opt.category.clone(),
                current_value: opt.current_value.clone(),
                options: values,
            }
        })
        .collect()
}

pub fn normalize_session_update(params: &Value) -> Option<AcpNormalizedEvent> {
    let update = params.get("update").unwrap_or(params);
    let kind = update
        .get("sessionUpdate")
        .or_else(|| update.get("type"))
        .and_then(Value::as_str)?;

    match kind {
        "agent_message_chunk" | "text_delta" => {
            let text = update
                .pointer("/content/text")
                .or_else(|| update.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if text.is_empty() {
                None
            } else {
                Some(AcpNormalizedEvent::TextDelta(text))
            }
        }
        "agent_thought_chunk" | "thought_delta" => {
            let text = update
                .pointer("/content/text")
                .or_else(|| update.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if text.is_empty() {
                None
            } else {
                Some(AcpNormalizedEvent::ReasoningDelta(text))
            }
        }
        "tool_call" => Some(AcpNormalizedEvent::ToolCall {
            id: update
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or("tool-unknown")
                .to_string(),
            name: update
                .get("title")
                .or_else(|| update.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string(),
            input: update.get("rawInput").cloned().unwrap_or(Value::Null),
        }),
        "tool_call_update" => Some(AcpNormalizedEvent::ToolCallUpdate {
            id: update
                .get("toolCallId")
                .and_then(Value::as_str)
                .unwrap_or("tool-unknown")
                .to_string(),
            status: update
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("running")
                .to_string(),
            output: update
                .get("rawOutput")
                .and_then(|v| serde_json::to_string(v).ok()),
        }),
        "usage_update" => Some(AcpNormalizedEvent::Usage {
            input_tokens: update
                .get("inputTokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: update
                .get("outputTokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            cache_read_tokens: update
                .get("cacheReadTokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        }),
        _ => None,
    }
}
