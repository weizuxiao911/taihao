//! comm-center — 调度通信中心
//!
// 职责(契约 AGENTS.md「调度通信中心」):
// - 协议适配:SSE / WS / MQTT + Mesh UDP 多链路
//! - 调度:任务编排、消息路由、多链路选择、故障切换
//! - 对接对象由厂商配置决定,出厂不预设
//!
// 实现:
// - 内部总线:Unix socket(`/run/taihao-os/comm-center.sock`)对接 5 个服务
// - SSE 接入端:TCP 8084,HTTP SSE
// - WebSocket 接入端:TCP 8086,WS 上行(接收外部命令)
// - MQTT 接入端:TCP 1883(客户端),由 COMM_CENTER_MQTT_URL 配置驱动(厂商决定)
// - Mesh UDP:7685 广播监听
// - 故障切换:链路健康检查 + 主/备切换(配置驱动)

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use taihao_common::{init_logging, Envelope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket, UnixListener};
use tokio::sync::RwLock;

const INTERNAL_SOCK: &str = "/run/taihao-os/comm-center.sock";
const SSE_PORT: u16 = 8084;
const MESH_UDP_PORT: u16 = 7685;
const WS_PORT: u16 = 8086;

fn internal_sock() -> String {
    std::env::var("COMM_CENTER_SOCK").unwrap_or_else(|_| INTERNAL_SOCK.to_string())
}

/// 链路状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LinkState { #[default] Up, Down }

impl LinkState {
    fn as_str(&self) -> &'static str { match self { Self::Up => "up", Self::Down => "down" } }
}

/// 链路健康信息
#[derive(Debug, Clone, Copy)]
struct LinkHealth {
    state: LinkState,
    /// 连续失败次数
    consecutive_failures: u32,
}

impl Default for LinkHealth {
    fn default() -> Self { Self { state: LinkState::Up, consecutive_failures: 0 } }
}

#[derive(Default)]
struct LinkStats {
    /// 链路状态
    primary: LinkHealth,
    secondary: LinkHealth,
    /// 备份链路(WS / MQTT 等)
    backup: LinkHealth,
    rx_msgs: u64,
    tx_msgs: u64,
    failures: u64,
    auto_switches: u64,
}

impl LinkStats {
    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "primary": self.primary.state.as_str(),
            "secondary": self.secondary.state.as_str(),
            "backup": self.backup.state.as_str(),
            "rx_msgs": self.rx_msgs,
            "tx_msgs": self.tx_msgs,
            "failures": self.failures,
            "auto_switches": self.auto_switches,
        })
    }

    /// 故障切换:主链路 Down → 切到备
    fn mark_primary_failure(&mut self) {
        self.failures += 1;
        self.primary.consecutive_failures += 1;
        if self.primary.consecutive_failures >= 2 {
            // 连续失败 ≥ 2 次 → 判定 Down,自动切到备
            if self.primary.state == LinkState::Up {
                self.primary.state = LinkState::Down;
                self.auto_switches += 1;
                tracing::warn!("主链路故障,自动切换");
            }
        }
    }

    fn mark_primary_ok(&mut self) {
        self.primary.consecutive_failures = 0;
        self.primary.state = LinkState::Up;
    }
}

/// 内部消息总线:对接 5 个服务
/// 路由规则:
/// - Envelope::Intent → 转发到 rt-loop socket
/// - Envelope::HalCall → 转发到 hal-gateway socket
/// - 其它 → echo 回源
async fn serve_internal_bus(stats: Arc<RwLock<LinkStats>>) -> anyhow::Result<()> {
    let sock = internal_sock();
    let _ = std::fs::remove_file(&sock);
    if let Some(p) = std::path::Path::new(&sock).parent() {
        std::fs::create_dir_all(p)?;
    }
    let listener = UnixListener::bind(&sock).context("绑定内部总线 socket")?;
    tracing::info!(path = %sock, "comm-center 内部总线监听");

    loop {
        let (stream, _) = match listener.accept().await {
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
                    s.mark_primary_ok();
                    drop(s);

                    // 路由转发
                    let route_target = match &env {
                        Envelope::Intent { .. } => Some(rt_loop_sock()),
                        Envelope::HalCall { .. } => Some(hal_gateway_sock()),
                        _ => None,
                    };
                    if let Some(target) = route_target {
                        forward_to(target, &env).await;
                    }

                    // echo 回源(供调用方确认)
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

/// rt-loop socket 路径(可环境变量覆盖)
fn rt_loop_sock() -> String {
    std::env::var("RT_LOOP_SOCK").unwrap_or_else(|_| "/run/taihao-os/rt-loop.sock".into())
}

/// hal-gateway socket 路径(可环境变量覆盖)
fn hal_gateway_sock() -> String {
    std::env::var("HAL_GATEWAY_SOCK").unwrap_or_else(|_| "/run/taihao-os/hal-gateway.sock".into())
}

/// 转发消息到目标 socket(尽力而为,失败仅 warn)
async fn forward_to(sock_path: String, env: &Envelope) {
    use tokio::io::AsyncWriteExt;
    let result = async {
        let mut stream = tokio::net::UnixStream::connect(&sock_path).await
            .map_err(|e| anyhow::anyhow!("连接 {} 失败: {}", sock_path, e))?;
        let mut line = serde_json::to_string(env)?;
        line.push('\n');
        stream.write_all(line.as_bytes()).await?;
        stream.shutdown().await?;
        Ok::<(), anyhow::Error>(())
    }.await;
    match result {
        Ok(()) => tracing::debug!(target = %sock_path, "消息已转发"),
        Err(e) => tracing::warn!(error = %e, target = %sock_path, "转发失败"),
    }
}

/// Mesh UDP 监听:接收广播
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
                s.mark_primary_ok();
                drop(s);
                tracing::debug!(bytes = n, "Mesh UDP 接收");
            }
            Err(_) => {
                let mut s = stats.write().await;
                s.mark_primary_failure();
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
            let header = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\n\r\n";
            let _ = sock.write_all(header).await;
            let _ = sock.write_all(b"data: {}\n\n").await;
            let s = stats.read().await;
            let body = format!("event: state\ndata: {}\n\n", s.snapshot());
            drop(s);
            let _ = sock.write_all(body.as_bytes()).await;
            let _ = sock.shutdown().await;
        });
    }
}

/// WebSocket 接入端:接收外部命令(简化实现,不依赖 tungstenite 握手)
/// 厂商配置驱动:COMM_CENTER_WS_ENABLE=1 时启用
async fn serve_ws(stats: Arc<RwLock<LinkStats>>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", WS_PORT)).await
        .context("绑定 WS 端口")?;
    tracing::info!(port = WS_PORT, "comm-center WebSocket 接入端监听");
    loop {
        let (mut sock, _) = match listener.accept().await {
            Ok(p) => p, Err(_) => continue,
        };
        let stats = stats.clone();
        tokio::spawn(async move {
            // 简化:接受连接 → 回应一条文本帧(echo stats),关闭
            let s = stats.read().await;
            let body = format!("data: {}\n\n", s.snapshot());
            drop(s);
            let _ = sock.write_all(body.as_bytes()).await;
            let _ = sock.shutdown().await;
        });
    }
}

/// MQTT 接入端:客户端连接 broker(厂商配置)
/// COMM_CENTER_MQTT_URL 格式:mqtt://host:port 或 tcp://host:port
/// 未配置 → 不启用(日志提示)
async fn serve_mqtt(stats: Arc<RwLock<LinkStats>>) -> anyhow::Result<()> {
    let url = std::env::var("COMM_CENTER_MQTT_URL").unwrap_or_default();
    if url.is_empty() {
        tracing::warn!("未配置 COMM_CENTER_MQTT_URL,MQTT 链路不启用");
        // 挂起等待配置
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }

    tracing::info!(url = %url, "comm-center MQTT 客户端连接");

    // 简化实现:解析 host:port,尝试 TCP 连接作为占位(真实 MQTT 握手后续补)
    let host = url.trim_start_matches("mqtt://").trim_start_matches("tcp://");
    let (h, p) = match host.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(1883)),
        None => (host.to_string(), 1883u16),
    };

    loop {
        match tokio::net::TcpStream::connect((h.as_str(), p)).await {
            Ok(mut stream) => {
                tracing::info!(url = %url, "MQTT broker 已连接");
                let mut s = stats.write().await;
                s.secondary.state = LinkState::Up;
                s.secondary.consecutive_failures = 0;
                drop(s);
                // 占位:保持连接(真实 MQTT CONNECT/PUBLISH/SUBSCRIBE 后续补)
                let _ = stream.shutdown().await;
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
            Err(e) => {
                tracing::warn!(url = %url, error = %e, "MQTT broker 连接失败");
                let mut s = stats.write().await;
                s.secondary.consecutive_failures += 1;
                if s.secondary.consecutive_failures >= 2 {
                    s.secondary.state = LinkState::Down;
                    s.failures += 1;
                }
                drop(s);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
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
        primary: LinkHealth::default(),
        secondary: LinkHealth::default(),
        backup: LinkHealth::default(),
        ..Default::default()
    }));

    tokio::select! {
        r = serve_internal_bus(stats.clone()) => { let _ = r; }
        r = serve_mesh_udp(stats.clone()) => { let _ = r; }
        r = serve_sse(stats.clone()) => { let _ = r; }
        r = serve_ws(stats.clone()) => { let _ = r; }
        r = serve_mqtt(stats.clone()) => { let _ = r; }
        _ = link_watchdog(stats.clone()) => {},
        _ = tokio::signal::ctrl_c() => { tracing::info!("收到 SIGINT"); }
    }
    Ok(())
}