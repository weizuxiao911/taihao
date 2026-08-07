//! hal-gateway — 硬件访问网关
//!
// 职责(契约 AGENTS.md「开放接入位 → HAL 集」):
// - 白名单:HAL 调用(设备 / 操作)必须登记,未登记拒绝
//! - 审计:每次调用写入 /var/log/taihao-os/hal-audit.log(JSON Lines)
// - mock 驱动:仿真下无真硬件,返回 mock 数据
//!
// 不做:协议适配(交给 comm-center)、MCN 实时控制(交给 rt-loop)、扩展协议(交给 extension-bridge)

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use taihao_common::{init_logging, paths, AuditDecision, Envelope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::RwLock;

const SOCK_PATH: &str = "/run/taihao-os/hal-gateway.sock";
const AUDIT_LOG: &str = "/var/log/taihao-os/hal-audit.log";

/// 白名单:登记的 (device, operation) 对
#[derive(Default)]
struct Whitelist {
    /// device -> 允许的 operations 集合
    entries: std::collections::HashMap<String, HashSet<String>>,
}

impl Whitelist {
    fn new() -> Self { Self::default() }

    /// 装载白名单(默认登记 SPI / I2C / GPIO 的常见操作)
    fn builtin(&mut self) {
        let devs: &[(&str, &[&str])] = &[
            ("spi0", &["read", "write"]),
            ("i2c0", &["read", "write"]),
            ("gpio0", &["read", "write"]),
            ("pwm0", &["set_duty", "set_freq"]),
            ("uart0", &["read", "write"]),
            ("can0", &["send", "recv"]),
        ];
        for &(dev, ops) in devs {
            let set: HashSet<String> = ops.iter().map(|s| s.to_string()).collect();
            self.entries.insert(dev.into(), set);
        }
    }

    fn check(&self, device: &str, operation: &str) -> bool {
        self.entries.get(device).map(|ops| ops.contains(operation)).unwrap_or(false)
    }
}

/// HAL 调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HalRequest {
    id: String,
    actor: String,
    device: String,
    operation: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HalResponse {
    id: String,
    ok: bool,
    payload: Option<serde_json::Value>,
    error: Option<String>,
}

/// mock 驱动:按 device / operation 返回模拟数据
fn mock_hal(req: &HalRequest) -> HalResponse {
    let payload = match (req.device.as_str(), req.operation.as_str()) {
        ("spi0", "read") => serde_json::json!({"bytes": [0x01, 0x02, 0x03]}),
        ("i2c0", "read") => serde_json::json!({"reg": 0x42, "value": 0xab}),
        ("gpio0", "read") => serde_json::json!({"pin": 17, "value": 1}),
        ("pwm0", _) => serde_json::json!({"channel": 0, "duty": 50}),
        ("can0", "recv") => serde_json::json!({"id": 0x100, "data": [0xde, 0xad]}),
        _ => serde_json::json!({"echo": req.params}),
    };
    HalResponse { id: req.id.clone(), ok: true, payload: Some(payload), error: None }
}

/// 写入审计日志(异步,行缓冲)
async fn write_audit(env: &Envelope) -> anyhow::Result<()> {
    if let Some(parent) = std::path::Path::new(AUDIT_LOG).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    let line = serde_json::to_string(env)?;
    use tokio::io::AsyncWriteExt;
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(AUDIT_LOG).await?;
    f.write_all(line.as_bytes()).await?;
    f.write_all(b"\n").await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("hal-gateway");

    let wl = Arc::new(RwLock::new({
        let mut w = Whitelist::new();
        w.builtin();
        w
    }));

    let _ = paths::config_dir();

    // 健康 HTTP
    let wl_health = wl.clone();
    tokio::spawn(async move {
        let port: u16 = std::env::var("HAL_GATEWAY_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8083);
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(l) => l,
            Err(_) => return,
        };
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p, Err(_) => continue,
            };
            let w = wl_health.read().await;
            let body = format!("{{\"whitelisted_devices\":{}}}\n", w.entries.len());
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

    // Unix socket 服务端
    let _ = std::fs::remove_file(SOCK_PATH);
    if let Some(p) = std::path::Path::new(SOCK_PATH).parent() {
        std::fs::create_dir_all(p)?;
    }
    let listener = UnixListener::bind(SOCK_PATH).context("绑定 hal-gateway Unix socket")?;
    tracing::info!(path = SOCK_PATH, "hal-gateway Unix socket 监听");

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
                let resp = match serde_json::from_str::<HalRequest>(line.trim()) {
                    Err(e) => HalResponse { id: String::new(), ok: false, payload: None, error: Some(format!("json: {}", e)) },
                    Ok(req) => {
                        let w = wl.read().await;
                        let allow = w.check(&req.device, &req.operation);
                        drop(w);
                        let env = Envelope::Audit {
                            actor: req.actor.clone(),
                            action: format!("{}:{}", req.device, req.operation),
                            target: req.device.clone(),
                            decision: if allow { AuditDecision::Allow } else { AuditDecision::Deny },
                            reason: if allow { None } else { Some("未在白名单".into()) },
                        };
                        let _ = write_audit(&env).await;
                        if allow {
                            mock_hal(&req)
                        } else {
                            HalResponse {
                                id: req.id.clone(),
                                ok: false,
                                payload: None,
                                error: Some(format!("白名单拒绝: {} / {}", req.device, req.operation)),
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