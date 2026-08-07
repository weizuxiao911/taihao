//! rt-loop — 执行层回路服务
//!
// 职责(契约 AGENTS.md「终端架构认知」):
// - 10-100Hz 闭环循环:接收 Agent 高层意图→指令流下发(模拟 MCU)、回收本体状态→闭环校调
//! - 不做:认知(1-10Hz 交给 pi-agent)、稳控反射(MCU 毫秒级)
//!
// 仿真:在 qemu 测试环境下没有真实 MCU,所有"指令流下发"和"状态回收"都是 mock。
// 接口:Unix socket `/run/taihao-os/rt-loop.sock`,接收 Intent JSON 消息,
//       周期性写 StateReport JSON。

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use taihao_common::{init_logging, paths, Envelope};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;

const SOCK_PATH: &str = "/run/taihao-os/rt-loop.sock";

fn sock_path_env() -> String {
    std::env::var("RT_LOOP_SOCK").unwrap_or_else(|_| SOCK_PATH.to_string())
}

/// 闭环回路状态
#[derive(Default)]
struct LoopState {
    cycles: u64,
    intents_processed: u64,
    last_period_us: u64,
}

impl LoopState {
    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "cycles": self.cycles,
            "intents": self.intents_processed,
            "last_period_us": self.last_period_us,
            "ts": chrono::Utc::now().to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntentMsg {
    id: String,
    skill: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// mock MCU:接收指令,生成虚拟状态
#[derive(Default)]
struct MockMcu {
    pos: f64,
    vel: f64,
}

impl MockMcu {
    fn step(&mut self, intent: &IntentMsg) {
        // 仿真:每次指令触发一次状态变化
        let step = match intent.skill.as_str() {
            "advance" => intent.params.get("distance").and_then(|v| v.as_f64()).unwrap_or(1.0),
            "turn" => intent.params.get("angle").and_then(|v| v.as_f64()).unwrap_or(0.0),
            _ => 0.0,
        };
        self.pos += step;
        self.vel = step;
    }

    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"pos": self.pos, "vel": self.vel})
    }
}

/// Unix socket 服务端:接收 Intent 消息,加入处理队列
async fn serve_unix_socket(
    state: Arc<RwLock<LoopState>>,
    mut mcu: MockMcu,
    tx: tokio::sync::mpsc::UnboundedSender<IntentMsg>,
) -> anyhow::Result<()> {
    let sock = sock_path_env();
    let _ = std::fs::remove_file(&sock);
    if let Some(p) = std::path::Path::new(&sock).parent() {
        std::fs::create_dir_all(p).context("创建 socket 目录")?;
    }
    let listener = UnixListener::bind(&sock).context("绑定 rt-loop Unix socket")?;
    tracing::info!(path = %sock, "rt-loop Unix socket 监听");

    loop {
        let (stream, _addr) = match listener.accept().await {
            Ok(p) => p,
            Err(e) => { tracing::warn!(error = %e, "accept 失败"); continue; }
        };
        let tx2 = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, tx2).await {
                tracing::warn!(error = %e, "连接处理失败");
            }
        });
    }
}

async fn handle_conn(
    stream: UnixStream,
    tx: tokio::sync::mpsc::UnboundedSender<IntentMsg>,
) -> anyhow::Result<()> {
    let (read_half, mut writer) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 { break; }
        match serde_json::from_str::<IntentMsg>(line.trim()) {
            Ok(msg) => {
                let _ = tx.send(msg);
                writer.write_all(b"{\"ok\":true}\n").await?;
            }
            Err(e) => {
                writer.write_all(format!("{{\"ok\":false,\"error\":\"{}\"}}\n", e).as_bytes()).await?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("rt-loop");

    let freq_hz: u32 = std::env::var("RT_LOOP_HZ").ok().and_then(|s| s.parse().ok()).unwrap_or(50);
    let period = Duration::from_micros(1_000_000 / freq_hz as u64);

    let state = Arc::new(RwLock::new(LoopState::default()));
    let mcu = Arc::new(RwLock::new(MockMcu::default()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<IntentMsg>();

    // Unix socket 服务端
    {
        let s = state.clone();
        let m = mcu.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_unix_socket(s, MockMcu::default(), tx).await {
                tracing::error!(error = %e, "Unix socket 服务退出");
            }
        });
    }

    // 健康 HTTP
    let port: u16 = std::env::var("RT_LOOP_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8082);
    let state_health = state.clone();
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
            Ok(l) => l,
            Err(e) => { tracing::error!(error = %e, "健康端口绑定失败"); return; }
        };
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(p) => p, Err(_) => continue,
            };
            let s = state_health.read().await;
            let body = format!("{{\"cycles\":{},\"intents\":{},\"hz\":{}}}\n",
                s.cycles, s.intents_processed, freq_hz);
            drop(s);
            use tokio::io::AsyncWriteExt;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });

    // 10-100Hz 闭环循环
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let _ = paths::state();

    tracing::info!(freq_hz, period_us = period.as_micros() as u64, "rt-loop 闭环循环启动");

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let t0 = std::time::Instant::now();
                // 收状态上报(模拟)
                let mut s = state.write().await;
                s.cycles += 1;
                s.last_period_us = t0.elapsed().as_micros() as u64;
                if s.cycles % (freq_hz as u64 * 5) == 0 {
                    tracing::info!(cycles = s.cycles, period_us = s.last_period_us, "rt-loop 心跳");
                }
                drop(s);
            }
            Some(intent) = rx.recv() => {
                let mut m = mcu.write().await;
                m.step(&intent);
                let state_snap = m.snapshot();
                drop(m);
                let mut s = state.write().await;
                s.intents_processed += 1;
                drop(s);
                tracing::info!(
                    id = %intent.id, skill = %intent.skill,
                    state = %state_snap,
                    "rt-loop 处理 Intent"
                );
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("收到 SIGINT,rt-loop 退出");
                break;
            }
        }
    }
    let _ = std::fs::remove_file(SOCK_PATH);
    Ok(())
}