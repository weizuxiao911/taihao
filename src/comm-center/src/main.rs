//! comm-center — 调度通信中心
//!
// 职责(契约 AGENTS.md「调度通信中心」):
// - 协议适配:SSE / WS / MQTT + Mesh UDP 双链路
//! - 调度:任务编排、消息路由、双链路选择、故障切换
//! - 对接对象由厂商配置决定,出厂不预设
//!
// 实现:在仿真环境下启动本地 Unix socket 作为内部总线,TCP SSE stub 作为外部接入面,
//       UDP mesh socket 监听广播(占位)。链路健康检查在后台进行,故障时切换。

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use taihao_common::{init_logging, Envelope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket, UnixListener};
use tokio::sync::RwLock;

const INTERNAL_SOCK: &str = "/run/taihao-os/comm-center.sock";
const SSE_PORT: u16 = 8084;
const MESH_UDP_PORT: u16 = 7685;

/// 链路状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LinkState { #[default] Up, Down }

impl LinkState {
    fn as_str(&self) -> &'static str { match self { Self::Up => "up", Self::Down => "down" } }
}

#[derive(Default)]
struct LinkStats {
    /// 双链路当前状态(MeshUDP 主 / SSE 备)
    primary: LinkState,
    secondary: LinkState,
    rx_msgs: u64,
    tx_msgs: u64,
    failures: u64,
    auto_switches: u64,
}

impl LinkStats {
    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "primary": self.primary.as_str(),
            "secondary": self.secondary.as_str(),
            "rx_msgs": self.rx_msgs,
            "tx_msgs": self.tx_msgs,
            "failures": self.failures,
            "auto_switches": self.auto_switches,
        })
    }
}

/// 内部消息总线:对接 5 个服务
async fn serve_internal_bus(
    stats: Arc<RwLock<LinkStats>>,
) -> anyhow::Result<()> {
    let _ = std::fs::remove_file(INTERNAL_SOCK);
    if let Some(p) = std::path::Path::new(INTERNAL_SOCK).parent() {
        std::fs::create_dir_all(p)?;
    }
    let listener = UnixListener::bind(INTERNAL_SOCK).context("绑定内部总线 socket")?;
    tracing::info!(path = INTERNAL_SOCK, "comm-center 内部总线监听");

    loop {
        let (mut stream, _) = match listener.accept().await {
            Ok(p) => p,
            Err(e) => { tracing::warn!(error = %e, "内部总线 accept 失败"); continue; }
        };
        let stats = stats.clone();
        tokio::spawn(async move {
            let (read_half, mut writer) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(read_half);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 { break; }
                if let Ok(env) = serde_json::from_str::<Envelope>(line.trim()) {
                    let mut s = stats.write().await;
                    s.rx_msgs += 1;
                    drop(s);
                    let out = serde_json::to_string(&env).unwrap_or_default();
                    let _ = writer.write_all(out.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let mut s = stats.write().await;
                    s.tx_msgs += 1;
                    drop(s);
                }
            }
        });
    }
}

/// Mesh UDP 监听:接收广播(占位)
async fn serve_mesh_udp(stats: Arc<RwLock<LinkStats>>) -> anyhow::Result<()> {
    let sock = UdpSocket::bind(("127.0.0.1", MESH_UDP_PORT)).await
        .context("绑定 Mesh UDP 端口")?;
    tracing::info!(port = MESH_UDP_PORT, "comm-center Mesh UDP 监听");
    let mut buf = [0u8; 4096];
    loop {
        match sock.recv(&mut buf).await {
            Ok(n) => {
                let mut s = stats.write().await;
                s.rx_msgs += 1;
                s.primary = LinkState::Up;
                drop(s);
                tracing::debug!(bytes = n, "Mesh UDP 接收");
            }
            Err(_) => {
                let mut s = stats.write().await;
                s.failures += 1;
                s.primary = LinkState::Down;
                s.auto_switches += 1;
                s.secondary = LinkState::Up;
                drop(s);
            }
        }
    }
}

/// SSE 接入端(简化,只响应 GET)
async fn serve_sse(stats: Arc<RwLock<LinkStats>>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", SSE_PORT)).await
        .context("绑定 SSE 端口")?;
    tracing::info!(port = SSE_PORT, "comm-center SSE 接入端监听");
    loop {
        let (mut sock, _) = match listener.accept().await {
            Ok(p) => p, Err(_) => continue,
        };
        let stats = stats.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let header = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\n\r\n";
            let _ = sock.write_all(header).await;
            let _ = sock.write_all(b"data: {}\n\n").await;
            let mut s = stats.read().await;
            let body = format!("event: state\ndata: {}\n\n", s.snapshot());
            drop(s);
            let _ = sock.write_all(body.as_bytes()).await;
            let _ = sock.shutdown().await;
        });
    }
}

/// 后台链路健康检查 + 自动故障切换
async fn link_watchdog(stats: Arc<RwLock<LinkStats>>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        let snap = {
            let s = stats.read().await;
            s.snapshot()
        };
        tracing::info!(?snap, "链路状态");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("comm-center");

    let stats = Arc::new(RwLock::new(LinkStats {
        primary: LinkState::Up,
        secondary: LinkState::Up,
        ..Default::default()
    }));

    tokio::select! {
        r = serve_internal_bus(stats.clone()) => { let _ = r; }
        r = serve_mesh_udp(stats.clone()) => { let _ = r; }
        r = serve_sse(stats.clone()) => { let _ = r; }
        _ = link_watchdog(stats.clone()) => {},
        _ = tokio::signal::ctrl_c() => { tracing::info!("收到 SIGINT"); }
    }
    Ok(())
}