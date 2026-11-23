use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("mcp server error: {0}")]
    Server(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub name: String,
    pub description: String,
    pub path: String,
}

pub type ToolHandler = Arc<dyn Fn(Value) -> Result<Value, String> + Send + Sync>;

#[derive(Clone, Default)]
pub struct McpRegistry {
    tools: Arc<RwLock<HashMap<String, (McpToolDefinition, ToolHandler)>>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_tool(&self, def: McpToolDefinition, handler: ToolHandler) {
        self.tools
            .write()
            .await
            .insert(def.name.clone(), (def, handler));
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/mcp", post(handle_mcp_rpc))
            .with_state(self)
    }
}

async fn handle_mcp_rpc(
    State(registry): State<McpRegistry>,
    Json(req): Json<Value>,
) -> Json<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": { "name": "lingxi-t3-native-mcp", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": { "tools": {} }
        })),
        "tools/list" => {
            let guard = registry.tools.read().await;
            let list: Vec<Value> = guard
                .values()
                .map(|(def, _)| {
                    json!({
                        "name": def.name,
                        "description": def.description,
                        "inputSchema": def.input_schema,
                    })
                })
                .collect();
            Ok(json!({ "tools": list }))
        }
        "tools/call" => {
            let name = req
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let args = req
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let guard = registry.tools.read().await;
            match guard.get(name) {
                Some((_, handler)) => match handler(args) {
                    Ok(out) => Ok(json!({
                        "content": [{ "type": "text", "text": out.to_string() }]
                    })),
                    Err(e) => Err((-32000, e)),
                },
                None => Err((-32601, format!("unknown tool: {name}"))),
            }
        }
        other => Err((-32601, format!("method not found: {other}"))),
    };

    match result {
        Ok(res) => Json(json!({ "jsonrpc": "2.0", "id": id, "result": res })),
        Err((code, message)) => Json(
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
        ),
    }
}

/// Discovers `SKILL.md` skills in a directory (reads only frontmatter metadata, never credentials).
pub fn discover_skills(skills_root: &Path) -> Vec<SkillDescriptor> {
    let mut skills = Vec::new();
    let Ok(entries) = std::fs::read_dir(skills_root) else {
        return skills;
    };
    for entry in entries.flatten() {
        let skill_md = entry.path().join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&skill_md) else {
            continue;
        };
        let (name, description) = parse_skill_frontmatter(&content).unwrap_or_else(|| {
            (
                entry.file_name().to_string_lossy().to_string(),
                String::new(),
            )
        });
        skills.push(SkillDescriptor {
            name,
            description,
            path: skill_md.to_string_lossy().to_string(),
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn parse_skill_frontmatter(content: &str) -> Option<(String, String)> {
    let rest = content.strip_prefix("---")?;
    let end = rest.find("---")?;
    let yaml = &rest[..end];
    let mut name = None;
    let mut desc = None;
    for line in yaml.lines() {
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            desc = Some(v.trim().trim_matches('"').to_string());
        }
    }
    Some((name?, desc.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mcp_http_server_lists_and_calls_registered_tools() {
        let reg = McpRegistry::new();
        reg.register_tool(
            McpToolDefinition {
                name: "ping".into(),
                description: "Ping tool".into(),
                input_schema: json!({"type":"object"}),
            },
            Arc::new(|_| Ok(json!({"pong": true}))),
        )
        .await;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            axum::serve(listener, reg.router()).await.ok();
        });

        let client = reqwest::Client::new();
        let resp: Value = client
            .post(format!("http://{addr}/mcp"))
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ping","arguments":{}}}))
            .send()
            .await
            .expect("send")
            .json()
            .await
            .expect("json");
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("pong")
        );
        server.abort();
    }

    #[test]
    fn discover_skills_reads_frontmatter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let skill_dir = dir.path().join("demo-skill");
        std::fs::create_dir_all(&skill_dir).expect("mkdir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo-skill\ndescription: A test skill\n---\n# Demo\n",
        )
        .expect("write");
        let found = discover_skills(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "demo-skill");
    }
}
