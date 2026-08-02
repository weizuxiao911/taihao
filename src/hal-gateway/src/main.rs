//! HAL 网关入口
//!
//! 两种模式：
//! - serve：常驻服务，轮询请求目录的 JSON 文件（SKILL / 厂商扩展经文件协议调用）
//! - call：一次性直调（验证 / 调试用），走同一核心（白名单 + 审计）

mod config;
mod driver;
mod gateway;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use serde::{Deserialize, Serialize};

/// 请求文件协议
#[derive(Debug, Deserialize)]
struct CallRequest {
    id: String,
    caller: String,
    tool: String,
    #[serde(default)]
    args: BTreeMap<String, String>,
}

/// 响应文件协议
#[derive(Debug, Serialize)]
struct CallResponse {
    id: String,
    allowed: bool,
    denied_reason: Option<String>,
    result: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "hal-gateway", version, about = "太昊 OS HAL 网关 — 硬件调用的唯一入口")]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "/etc/taihao-os/hal-gateway.toml")]
    config: String,
    /// 子命令
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand, Debug)]
enum Cmd {
    /// 常驻服务：轮询请求目录
    Serve,
    /// 一次性直调（调试/验证）
    Call {
        /// 调用方标识
        #[arg(long)]
        caller: String,
        /// 工具名
        #[arg(long)]
        tool: String,
        /// 参数 k=v，可重复
        #[arg(long = "arg")]
        args: Vec<String>,
    },
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
    log::info!("HAL 网关启动: {} 个白名单工具", cfg.tools.len());

    match cli.cmd {
        Cmd::Serve => serve(&cfg),
        Cmd::Call { caller, tool, args } => {
            let parsed: BTreeMap<String, String> = args
                .iter()
                .filter_map(|a| a.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let out = gateway::call(&cfg, &caller, &tool, &parsed);
            let response = CallResponse {
                id: "cli".into(),
                allowed: out.allowed,
                denied_reason: out.denied_reason,
                result: out.result.map(|r| r.detail),
            };
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
    }
}

/// 常驻服务：轮询请求目录，处理请求并写响应目录
fn serve(cfg: &config::Config) {
    let req_dir = PathBuf::from(&cfg.request_dir);
    let resp_dir = PathBuf::from(&cfg.response_dir);
    fs::create_dir_all(&req_dir).ok();
    fs::create_dir_all(&resp_dir).ok();
    log::info!("服务模式: {} → {}", req_dir.display(), resp_dir.display());

    loop {
        let entries = match fs::read_dir(&req_dir) {
            Ok(e) => e.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map(|x| x == "json").unwrap_or(false)).collect::<Vec<_>>(),
            Err(_) => vec![],
        };
        for path in entries {
            match process_file(cfg, &path, &resp_dir) {
                Ok(()) => {
                    fs::remove_file(&path).ok();
                    log::debug!("已处理 {}", path.display());
                }
                Err(e) => log::warn!("处理失败 {}: {}", path.display(), e),
            }
        }
        std::thread::sleep(Duration::from_millis(cfg.poll_ms.max(20)));
    }
}

fn process_file(cfg: &config::Config, path: &std::path::Path, resp_dir: &PathBuf) -> Result<(), String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let req: CallRequest = serde_json::from_str(&raw).map_err(|e| format!("请求解析失败: {}", e))?;
    let out = gateway::call(cfg, &req.caller, &req.tool, &req.args);
    let response = CallResponse {
        id: req.id.clone(),
        allowed: out.allowed,
        denied_reason: out.denied_reason,
        result: out.result.map(|r| r.detail),
    };
    let resp_path = resp_dir.join(format!("{}.json", req.id));
    fs::write(&resp_path, serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_call_request() {
        let raw = r#"{"id":"r1","caller":"skill:x","tool":"sensor_read","args":{"sensor":"distance"}}"#;
        let req: CallRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.tool, "sensor_read");
        assert_eq!(req.args.get("sensor").unwrap(), "distance");
    }

    #[test]
    fn parse_call_request_minimal() {
        let raw = r#"{"id":"r2","caller":"c","tool":"t"}"#;
        let req: CallRequest = serde_json::from_str(raw).unwrap();
        assert!(req.args.is_empty());
    }
}
