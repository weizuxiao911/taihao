//! extension-bridge — 扩展 / MCP 桥
//!
// 职责(契约 AGENTS.md「开放接入位 → Extension·MCP 桥」):
// - JSON-RPC 协议;白名单方法集;权限边界(越权拒绝)
// - 不做:模型推理(交给 pi-agent)、硬件访问(交给 hal-gateway)
//!
// 实现:Unix socket 上提供 line-delimited JSON-RPC 2.0 服务端,白名单方法为:
//   - "system.status" (read-only)
//   - "system.list_skills"
//   - "system.heartbeat"
//   - "intent.submit"(低风险,只触发 Intent 提交)
// 任何超出白名单的方法被拒并出审计。

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use taihao_common::{init_logging, paths, AuditDecision, Envelope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::RwLock;

const SOCK_PATH: &str = "/run/taihao-os/extension-bridge.sock";
const AUDIT_LOG: &str = "/var/log/taihao-os/ext-audit.log";

#[derive(Default)]
struct Whitelist {
    methods: Vec<String>,
}

impl Whitelist {
    fn new() -> Self {
        Self { methods: vec![
            "system.status".into(),
            "system.list_skills".into(),
            "system.heartbeat".into(),
            "intent.submit".into(),
        ] }
    }
    fn allow(&self, method: &str) -> bool { self.methods.iter().any(|m| m == method) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcReq {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResp {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

async fn write_audit(env: &Envelope) -> anyhow::Result<()> {
    if let Some(parent) = std::path::Path::new(AUDIT_LOG).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let line = serde_json::to_string(env)?;
    let mut f = tokio::fs::OpenOptions::new().create(true).append(true).open(AUDIT_LOG).await?;
    use tokio::io::AsyncWriteExt;
    f.write_all(line.as_bytes()).await?;
    f.write_all(b"\n").await?;
    Ok(())
}

/// 方法分派(白名单内)
fn dispatch(req: &JsonRpcReq) -> Result<Value, JsonRpcError> {
    match req.method.as_str() {
        "system.heartbeat" => Ok(serde_json::json!({
            "ok": true,
            "ts": SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        })),
        "system.status" => Ok(serde_json::json!({
            "ok": true,
            "service": "extension-bridge",
            "version": env!("CARGO_PKG_VERSION"),
        })),
        "system.list_skills" => Ok(serde_json::json!({
            "ok": true,
            "skills": ["echo", "advance", "turn", "probe"],
        })),
        "intent.submit" => Ok(serde_json::json!({
            "ok": true,
            "queued": true,
            "intent_id": req.params.get("intent_id").cloned().unwrap_or(Value::Null),
        })),
        // 这是一个"陷阱"方法 — 不在白名单,但被合法调用时会越权
        other => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found or not whitelisted: {}", other),
        }),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("extension-bridge");

    let wl = Arc::new(RwLock::new(Whitelist::new()));
    let _ = paths::config_dir();

    // 健康 HTTP
    let wl_health = wl.clone();
    tokio::spawn(async move {
        let port: u16 = std::env::var("EXT_BRIDGE_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8085);
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(l) => l,
            Err(_) => return,
        };
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p, Err(_) => continue,
            };
            let w = wl_health.read().await;
            let body = format!("{{\"whitelisted_methods\":{},\"service\":\"extension-bridge\"}}\n", w.methods.len());
            drop(w);
            use tokio::io::AsyncWriteExt;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });

    // Unix socket 服务
    let _ = std::fs::remove_file(SOCK_PATH);
    if let Some(p) = std::path::Path::new(SOCK_PATH).parent() {
        std::fs::create_dir_all(p)?;
    }
    let listener = UnixListener::bind(SOCK_PATH).context("绑定 extension-bridge Unix socket")?;
    tracing::info!(path = SOCK_PATH, "extension-bridge 监听");

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(p) => p,
            Err(e) => { tracing::warn!(error = %e, "accept 失败"); continue; }
        };
        let wl = wl.clone();
        tokio::spawn(async move {
            let (read_half, mut writer) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(read_half);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 { break; }
                let resp = match serde_json::from_str::<JsonRpcReq>(line.trim()) {
                    Err(e) => JsonRpcResp {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(JsonRpcError { code: -32700, message: format!("Parse error: {}", e) }),
                        id: Value::Null,
                    },
                    Ok(req) => {
                        let w = wl.read().await;
                        let allowed = w.allow(&req.method);
                        drop(w);
                        let env = Envelope::Audit {
                            actor: "extension-bridge-client".into(),
                            action: req.method.clone(),
                            target: "json-rpc".into(),
                            decision: if allowed { AuditDecision::Allow } else { AuditDecision::Deny },
                            reason: if allowed { None } else { Some("method not whitelisted".into()) },
                        };
                        let _ = write_audit(&env).await;
                        if allowed {
                            match dispatch(&req) {
                                Ok(r) => JsonRpcResp {
                                    jsonrpc: "2.0".into(),
                                    result: Some(r),
                                    error: None,
                                    id: req.id,
                                },
                                Err(e) => JsonRpcResp {
                                    jsonrpc: "2.0".into(),
                                    result: None,
                                    error: Some(e),
                                    id: req.id,
                                },
                            }
                        } else {
                            JsonRpcResp {
                                jsonrpc: "2.0".into(),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32601,
                                    message: format!("白名单拒绝: {}", req.method),
                                }),
                                id: req.id,
                            }
                        }
                    }
                };
                let body = serde_json::to_string(&resp).unwrap_or_default();
                let _ = writer.write_all(body.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
        });
    }
}