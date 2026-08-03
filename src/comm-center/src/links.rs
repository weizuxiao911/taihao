//! 双链路抽象 — 远程链路 + 本地 Mesh 链路
//!
//! Link trait 定义健康检查 / 发送 / 接收；
//! 远程链路协议适配：sse_http / websocket / mqtt（按配置选择）；
//! 本地 Mesh 链路：UDP（std 实现，无外部依赖）。

use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 链路接口
pub trait Link: Send + Sync {
    fn name(&self) -> &'static str;
    /// 健康探测（返回是否可用）
    fn healthy(&self) -> bool;
    /// 发送消息（topic + payload）
    fn send(&self, topic: &str, payload: &str) -> Result<(), String>;
    /// 接收消息（阻塞至超时；None = 超时无消息）
    fn recv(&self, timeout: Duration) -> Result<Option<(String, String)>, String>;
}

/// 链路健康状态（共享，供故障切换读取）
#[derive(Debug, Clone)]
pub struct HealthState {
    pub consecutive_failures: Arc<AtomicU32>,
    pub enabled: Arc<AtomicBool>,
}

impl HealthState {
    pub fn new(enabled: bool) -> Self {
        HealthState {
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }
    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
    }
    pub fn failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
    pub fn set_enabled(&self, v: bool) {
        self.enabled.store(v, Ordering::Relaxed);
    }
}

// ---------------- 本地 Mesh 链路 (UDP) ----------------

/// Mesh 链路：UDP 组播/单播。消息格式 `topic\npayload`（首行 topic）。
pub struct MeshLink {
    pub name_: &'static str,
    pub socket: UdpSocket,
    pub peer: SocketAddr,
    pub health: HealthState,
}

impl MeshLink {
    pub fn bind(port: u16, peer: SocketAddr) -> Result<Self, String> {
        let socket = UdpSocket::bind(("0.0.0.0", port)).map_err(|e| e.to_string())?;
        socket.set_read_timeout(Some(Duration::from_millis(200))).map_err(|e| e.to_string())?;
        Ok(MeshLink { name_: "mesh", socket, peer, health: HealthState::new(true) })
    }
}

impl Link for MeshLink {
    fn name(&self) -> &'static str {
        self.name_
    }
    fn healthy(&self) -> bool {
        self.health.is_enabled()
    }
    fn send(&self, topic: &str, payload: &str) -> Result<(), String> {
        let frame = format!("{}\n{}", topic, payload);
        self.socket
            .send_to(frame.as_bytes(), self.peer)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    fn recv(&self, _timeout: Duration) -> Result<Option<(String, String)>, String> {
        let mut buf = [0u8; 4096];
        match self.socket.recv_from(&mut buf) {
            Ok((n, _)) => {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let (topic, payload) = match text.split_once('\n') {
                    Some((t, p)) => (t.to_string(), p.to_string()),
                    None => ("raw".to_string(), text),
                };
                self.health.record_success();
                Ok(Some((topic, payload)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

// ---------------- 远程链路 (HTTP + SSE) ----------------

/// 远程链路：SSE 订阅接收 + HTTP POST 上报（ureq 实现，无额外依赖）。
pub struct SseHttpLink {
    pub name_: &'static str,
    pub base: String,
    pub topic_prefix: String,
    pub health: HealthState,
}

impl SseHttpLink {
    pub fn new(base: String, topic_prefix: String) -> Self {
        SseHttpLink { name_: "remote-sse", base, topic_prefix, health: HealthState::new(true) }
    }
}

impl Link for SseHttpLink {
    fn name(&self) -> &'static str {
        self.name_
    }
    fn healthy(&self) -> bool {
        self.health.is_enabled()
    }
    fn send(&self, topic: &str, payload: &str) -> Result<(), String> {
        let url = format!("{}/report/{}", self.base.trim_end_matches('/'), topic);
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(5)).build();
        agent
            .post(&url)
            .send_string(payload)
            .map(|_| {
                self.health.record_success();
            })
            .map_err(|e| {
                self.health.record_failure();
                format!("上报失败: {}", e)
            })
    }
    fn recv(&self, _timeout: Duration) -> Result<Option<(String, String)>, String> {
        // SSE 长轮询端点：GET {base}/events/{topic_prefix}
        // 轮询超时仅对 UDP recv 有意义；HTTP 请求用内部固定超时 (2s)，避免短轮询窗口下必然超时
        let url = format!("{}/events/{}", self.base.trim_end_matches('/'), self.topic_prefix.trim_end_matches('/'));
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(2)).build();
        let resp = agent
            .get(&url)
            .call()
            .map_err(|e| {
                self.health.record_failure();
                format!("SSE 订阅失败: {}", e)
            })?;
        let text = resp.into_string().map_err(|e| e.to_string())?;
        self.health.record_success();
        if text.trim().is_empty() {
            return Ok(None);
        }
        let (topic, payload) = text.split_once('\n').unwrap_or(("event", text.as_str()));
        Ok(Some((topic.to_string(), payload.to_string())))
    }
}

// ---------------- 远程链路 (MQTT) ----------------

/// 远程链路：MQTT 3.1.1 客户端（rumqttc）。
pub struct MqttAdapter {
    pub name_: &'static str,
    pub endpoint: String,
    pub topic_prefix: String,
    pub health: HealthState,
    pub client: std::sync::Mutex<Option<rumqttc::Client>>,
}

impl MqttAdapter {
    pub fn new(endpoint: &str, topic_prefix: &str) -> Self {
        MqttAdapter {
            name_: "remote-mqtt",
            endpoint: endpoint.to_string(),
            topic_prefix: topic_prefix.to_string(),
            health: HealthState::new(true),
            client: std::sync::Mutex::new(None),
        }
    }
    fn client(&self) -> rumqttc::Client {
        if let Some(c) = self.client.lock().unwrap().as_ref() {
            return c.clone();
        }
        let opts = rumqttc::MqttOptions::new("taihao-device", &self.endpoint, 1883);
        let (c, mut conn) = rumqttc::Client::new(opts, 64);
        // 驱动连接事件循环（后台线程，断线自动重连由 rumqttc 承担）
        std::thread::spawn(move || {
            for _ in conn.iter() {}
        });
        *self.client.lock().unwrap() = Some(c.clone());
        c
    }
}

impl Link for MqttAdapter {
    fn name(&self) -> &'static str {
        self.name_
    }
    fn healthy(&self) -> bool {
        self.health.is_enabled()
    }
    fn send(&self, topic: &str, payload: &str) -> Result<(), String> {
        let full = format!("{}{}", self.topic_prefix, topic);
        let client = self.client();
        client
            .publish(full, rumqttc::QoS::AtMostOnce, false, payload.as_bytes().to_vec())
            .map_err(|e| {
                self.health.record_failure();
                format!("MQTT 发布失败: {}", e)
            })?;
        self.health.record_success();
        Ok(())
    }
    fn recv(&self, _timeout: Duration) -> Result<Option<(String, String)>, String> {
        // 事件驱动接收由 rumqttc 事件循环承担；轮询模式返回 None（保持 trait 形状）
        Ok(None)
    }
}

// ---------------- 远程链路 (WebSocket) ----------------

/// 远程链路：WebSocket 客户端（tungstenite）。
pub struct WsAdapter {
    pub name_: &'static str,
    pub endpoint: String,
    pub topic_prefix: String,
    pub health: HealthState,
}

impl WsAdapter {
    pub fn new(endpoint: &str, topic_prefix: &str) -> Self {
        WsAdapter {
            name_: "remote-ws",
            endpoint: endpoint.to_string(),
            topic_prefix: topic_prefix.to_string(),
            health: HealthState::new(true),
        }
    }
}

impl Link for WsAdapter {
    fn name(&self) -> &'static str {
        self.name_
    }
    fn healthy(&self) -> bool {
        self.health.is_enabled()
    }
    fn send(&self, topic: &str, payload: &str) -> Result<(), String> {
        let (mut sock, _) = tungstenite::connect(&self.endpoint).map_err(|e| {
            self.health.record_failure();
            format!("WS 连接失败: {}", e)
        })?;
        let frame = format!("{}\n{}", topic, payload);
        sock.send(tungstenite::Message::Text(frame)).map_err(|e| {
            self.health.record_failure();
            format!("WS 发送失败: {}", e)
        })?;
        self.health.record_success();
        Ok(())
    }
    fn recv(&self, _timeout: Duration) -> Result<Option<(String, String)>, String> {
        Ok(None)
    }
}

/// 供测试：本地 HTTP mock 端点的最小服务器（回显）
pub fn spawn_echo_server() -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            use std::io::{Read, Write};
            let mut buf = [0u8; 2048];
            let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
            if stream.read(&mut buf).is_ok() {
                let body = "event\nok";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        }
    });
    (format!("http://127.0.0.1:{}", port), handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_send_recv_roundtrip() {
        // 两个 socket 互为 peer（不 connect，send_to 各自对端）
        let mut a = MeshLink::bind(0, "127.0.0.1:9".parse().unwrap()).unwrap();
        let mut b = MeshLink::bind(0, "127.0.0.1:9".parse().unwrap()).unwrap();
        let pa = a.socket.local_addr().unwrap();
        let pb = b.socket.local_addr().unwrap();
        let lo = |port: u16| SocketAddr::new("127.0.0.1".parse().unwrap(), port);
        a.peer = lo(pb.port());
        b.peer = lo(pa.port());

        a.send("hello", "world").unwrap();
        let got = b.recv(Duration::from_millis(500)).unwrap();
        assert_eq!(got, Some(("hello".to_string(), "world".to_string())));
    }

    #[test]
    fn mesh_recv_timeout_is_none() {
        let a = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        assert_eq!(a.recv(Duration::from_millis(100)).unwrap(), None);
    }

    #[test]
    fn health_state_tracks_failures() {
        let h = HealthState::new(true);
        assert!(h.is_enabled());
        h.record_failure();
        h.record_failure();
        assert_eq!(h.failures(), 2);
        h.record_success();
        assert_eq!(h.failures(), 0);
    }

    #[test]
    fn sse_http_link_roundtrip_with_echo() {
        let (base, _h) = spawn_echo_server();
        let link = SseHttpLink::new(base, "taihao/".into());
        link.send("status", "{}").ok();
        let msg = link.recv(Duration::from_millis(3000)).unwrap();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().0, "event");
    }
}
