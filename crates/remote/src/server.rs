use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use lingxi_domain::OrchestrationCommand;
use lingxi_orchestration::Orchestrator;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{RpcScope, verify_token};

#[derive(Clone)]
pub struct RemoteServerState {
    pub orchestrator: Arc<Orchestrator>,
}

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
    pub since: Option<u64>,
}

pub fn build_router(state: RemoteServerState) -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/server-config", get(server_config_handler))
        .route("/ws", get(ws_upgrade_handler))
        .with_state(state)
}

pub async fn serve_on_port(
    orchestrator: Arc<Orchestrator>,
    port: u16,
) -> Result<(), crate::RemoteError> {
    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| crate::RemoteError::Transport(e.to_string()))?;
    tracing::info!(%addr, "LingXi T3 Native remote WS/RPC server listening");
    axum::serve(listener, build_router(RemoteServerState { orchestrator }))
        .await
        .map_err(|e| crate::RemoteError::Transport(e.to_string()))
}

async fn health_handler() -> Json<Value> {
    Json(json!({ "status": "ok", "runtime": "lingxi-t3-native-rust" }))
}

async fn server_config_handler() -> Json<Value> {
    Json(json!({
        "protocolVersion": 2,
        "capabilities": {
            "orchestrationV2": true,
            "threadPullRequests": true,
            "acp": true,
        },
        "identity": "com.lingxifox.lingxit3.native"
    }))
}

async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsAuthQuery>,
    State(state): State<RemoteServerState>,
) -> impl IntoResponse {
    let token = query.token.unwrap_or_default();
    if let Err(err) = verify_token(state.orchestrator.db(), &token, RpcScope::Read) {
        return (StatusCode::UNAUTHORIZED, err.to_string()).into_response();
    }
    let since = query.since.unwrap_or(0);
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state, token, since))
}

async fn handle_ws_connection(
    socket: WebSocket,
    state: RemoteServerState,
    token: String,
    since: u64,
) {
    let (mut sender, mut receiver) = socket.split();

    // Replay missed events since cursor
    if let Ok(missed) = state.orchestrator.db().events_since(since, 500) {
        for ev in missed {
            let payload = json!({
                "type": "orchestration.event",
                "event": ev,
            });
            if sender
                .send(Message::Text(payload.to_string().into()))
                .await
                .is_err()
            {
                return;
            }
        }
    }

    let mut broadcast_rx = state.orchestrator.subscribe();

    loop {
        tokio::select! {
            Ok(ev) = broadcast_rx.recv() => {
                let payload = json!({
                    "type": "orchestration.event",
                    "event": ev,
                });
                if sender.send(Message::Text(payload.to_string().into())).await.is_err() {
                    break;
                }
            }
            Some(Ok(msg)) = receiver.next() => {
                let Message::Text(text) = msg else {
                    if matches!(msg, Message::Close(_)) {
                        break;
                    }
                    continue;
                };
                let resp = dispatch_ws_rpc(&state, &token, &text).await;
                if sender.send(Message::Text(resp.to_string().into())).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
}

async fn dispatch_ws_rpc(state: &RemoteServerState, token: &str, raw: &str) -> Value {
    let Ok(req) = serde_json::from_str::<Value>(raw) else {
        return json!({ "error": { "code": -32700, "message": "parse error" } });
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let result = match method {
        "orchestration.v2.getShellSnapshot" => {
            if let Err(e) = verify_token(state.orchestrator.db(), token, RpcScope::Read) {
                Err((-32001, e.to_string()))
            } else {
                let projects = state.orchestrator.db().list_projects().unwrap_or_default();
                let threads = state.orchestrator.db().list_threads().unwrap_or_default();
                Ok(json!({ "projects": projects, "threads": threads }))
            }
        }
        "orchestration.v2.dispatchCommand" => {
            if let Err(e) = verify_token(state.orchestrator.db(), token, RpcScope::Write) {
                Err((-32001, e.to_string()))
            } else {
                let params = req.get("params").cloned().unwrap_or(Value::Null);
                match serde_json::from_value::<OrchestrationCommand>(params) {
                    Ok(cmd) => match state.orchestrator.dispatch(cmd).await {
                        Ok(outcome) => Ok(json!({
                            "deduplicated": outcome.deduplicated,
                            "events": outcome.events,
                        })),
                        Err(e) => Err((-32000, e.to_string())),
                    },
                    Err(e) => Err((-32602, format!("invalid command params: {e}"))),
                }
            }
        }
        other => Err((-32601, format!("unknown RPC method: {other}"))),
    };

    match result {
        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    }
}
