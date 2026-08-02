//! 中台调度通信模块入口
//!
//! 常驻服务：启动远程 + Mesh 双链路 → 调度循环（接收 → 路由 → 处理 → 上报）。
//! 对接对象（岸基 / 云 / 邻域）由配置决定，OS 出厂不预设。

mod config;
mod dispatcher;
mod links;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "comm-center", version, about = "太昊 OS 中台调度通信模块 — 双链路 + 路由 + 故障切换")]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "/etc/taihao-os/comm-center.toml")]
    config: String,
    /// 运行 N 秒后退出（验证用，0 = 常驻）
    #[arg(long)]
    seconds: Option<u64>,
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let (cfg, warnings) = config::Config::load(&std::path::Path::new(&cli.config));
    for w in &warnings {
        log::warn!("{}", w);
    }

    // 远程链路：按协议构建
    let remote: Box<dyn links::Link> = match cfg.remote_proto.as_str() {
        "mqtt" => Box::new(links::MqttAdapter::new(&cfg.remote_endpoint, &cfg.topic_prefix)),
        "websocket" => Box::new(links::WsAdapter::new(&cfg.remote_endpoint, &cfg.topic_prefix)),
        _ => Box::new(links::SseHttpLink::new(cfg.remote_endpoint.clone(), cfg.topic_prefix.clone())),
    };
    // 本地 Mesh 链路
    let mesh_peer = "255.255.255.255".to_string();
    let mesh = match links::MeshLink::bind(cfg.mesh_port, format!("{}:{}", mesh_peer, cfg.mesh_port).parse().unwrap()) {
        Ok(m) => Box::new(m) as Box<dyn links::Link>,
        Err(e) => {
            log::warn!("Mesh 链路绑定失败: {}（仅远程链路运行）", e);
            Box::new(links::SseHttpLink::new(cfg.remote_endpoint.clone(), cfg.topic_prefix.clone()))
        }
    };

    let dispatch = dispatcher::Dispatcher::new(remote, mesh, cfg.clone());
    log::info!(
        "comm-center 启动: 远程协议={} 端点={} Mesh端口={} 故障阈值={}",
        cfg.remote_proto, cfg.remote_endpoint, cfg.mesh_port, cfg.fail_threshold
    );

    let started = std::time::Instant::now();
    let stop = Arc::new(AtomicBool::new(false));
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Some(secs) = cli.seconds {
            if started.elapsed() >= Duration::from_secs(secs) {
                log::info!("达到运行时长，退出");
                break;
            }
        }
        match dispatch.recv(Duration::from_millis(500)) {
            Some((topic, payload)) => match dispatch.route(&topic, &payload) {
                Ok(out) => {
                    log::info!("消息路由: topic={} → {}", topic, out);
                    // 执行结果回传（经当前主链路）
                    let report_topic = format!("report/{}", topic);
                    let _ = dispatch.send(&report_topic, &out);
                }
                Err(e) => log::warn!("消息路由失败: {}", e),
            },
            None => {}
        }
        if let Ok(status) = dispatch.write_status(&PathBuf::from(&cfg.status_file)) {
            let _ = status;
        }
        dispatch.maybe_failover();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let (cfg, warnings) = config::Config::load(std::path::Path::new("/nonexistent/comm.toml"));
        assert!(cfg.remote_proto == "sse_http" || !warnings.is_empty());
    }
}
