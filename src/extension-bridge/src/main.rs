//! Extension·MCP 桥入口
//!
//! serve：轮询请求目录，处理 JSON-RPC（initialize / tools/list / tools/call）→ 写响应。
//! 权限：工具白名单 + 参数白名单 + shell 安全转义。

mod bridge;
mod config;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use clap::Parser;
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[command(name = "extension-bridge", version, about = "太昊 OS Extension·MCP 桥 — JSON-RPC 桥接 + 权限边界")]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "/etc/taihao-os/extension-bridge.toml")]
    config: String,
    /// 子命令
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand, Debug)]
enum Cmd {
    /// 常驻服务：轮询请求目录
    Serve,
    /// 一次性调用（验证用）：method + JSON params
    Call { method: String, params: Option<String> },
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
    log::info!("extension-bridge 启动: {} 个白名单工具", cfg.tools.len());

    match cli.cmd {
        Cmd::Serve => serve(&cfg),
        Cmd::Call { method, params } => {
            let params = params.and_then(|p| serde_json::from_str(&p).ok()).unwrap_or(json!({}));
            let out = handle(&cfg, "cli", method.as_str(), &params);
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
    }
}

/// 处理一次 JSON-RPC 调用，返回响应 JSON（成功或错误）
fn handle(cfg: &config::Config, caller: &str, method: &str, params: &Value) -> Value {
    match method {
        "initialize" => json!({
            "jsonrpc": "2.0", "id": params.get("id").cloned().unwrap_or(Value::Null),
            "result": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "taihao-extension-bridge", "version": "0.1.0"}}
        }),
        "tools/list" => {
            let tools: Vec<Value> = cfg
                .tools
                .iter()
                .map(|t| json!({"name": t.name, "description": format!("level {}", t.level), "inputSchema": {"type": "object", "properties": {}} }))
                .collect();
            json!({"jsonrpc": "2.0", "id": params.get("id").cloned().unwrap_or(Value::Null), "result": {"tools": tools}})
        }
        "tools/call" => {
            let id = params.get("id").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("args").cloned().unwrap_or(json!({}));
            let tool = match cfg.find_tool(name) {
                Some(t) => t,
                None => {
                    log::warn!("MCP 调用被拒: 工具不在白名单 name={} caller={}", name, caller);
                    return json!(bridge::error(id, -32601, &format!("工具不在白名单: {}", name)));
                }
            };
            let filtered = bridge::filter_params(&tool.args, &args);
            // 命令执行（sh -c）：白名单参数以环境变量注入（$<参数名> 在 sh 内展开）
            let cmdline = tool.command.clone();
            match Command::new("sh")
                .arg("-c")
                .arg(&cmdline)
                .envs(&filtered)
                .output() {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    log::info!("MCP 调用成功: tool={} caller={}", name, caller);
                    json!(bridge::ok(id, json!({"content": [{"type": "text", "text": text}]})))
                }
                Ok(out) => {
                    log::warn!("MCP 调用命令失败: tool={} status={}", name, out.status);
                    json!(bridge::error(id, -32000, "扩展命令执行失败"))
                }
                Err(e) => {
                    log::warn!("MCP 调用命令错误: tool={} err={}", name, e);
                    json!(bridge::error(id, -32000, &format!("命令执行错误: {}", e)))
                }
            }
        }
        other => json!(bridge::error(params.get("id").cloned().unwrap_or(Value::Null), -32601, &format!("未知方法: {}", other))),
    }
}

/// 常驻服务：轮询请求目录
fn serve(cfg: &config::Config) {
    let req_dir = PathBuf::from(&cfg.request_dir);
    let resp_dir = PathBuf::from(&cfg.response_dir);
    fs::create_dir_all(&req_dir).ok();
    fs::create_dir_all(&resp_dir).ok();

    loop {
        let entries = match fs::read_dir(&req_dir) {
            Ok(e) => e
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
                .collect::<Vec<_>>(),
            Err(_) => vec![],
        };
        for path in entries {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(req) = serde_json::from_str::<bridge::RpcRequest>(&raw) {
                    // 协议校验：jsonrpc 必须为 2.0（非法请求写错误响应）
                    let resp = match bridge::validate(&req) {
                        Ok(()) => handle(cfg, "mcp-client", req.method.as_str(), &req.params),
                        Err(e) => json!(bridge::error(req.id.clone(), -32600, &e)),
                    };
                    // JSON-RPC id 可为字符串或数字：统一转成文件名
                    let id_str = match &req.id {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => "null".to_string(),
                    };
                    let resp_path = resp_dir.join(format!("{}.json", id_str));
                    if let Ok(json) = serde_json::to_string_pretty(&resp) {
                        fs::write(&resp_path, json).ok();
                    }
                }
            }
            fs::remove_file(&path).ok();
        }
        std::thread::sleep(Duration::from_millis(cfg.poll_ms.max(20)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cfg() -> config::Config {
        let mut cfg = config::Config::default();
        cfg.tools = vec![config::ToolDef {
            name: "echo_tool".into(),
            command: "echo hello-from-extension".into(),
            args: vec!["message".into()].into(),
            level: "L3".into(),
        }];
        cfg
    }

    #[test]
    fn initialize_ok() {
        let cfg = test_cfg();
        let resp = handle(&cfg, "t", "initialize", &json!({"id": 1}));
        assert_eq!(resp["result"]["serverInfo"]["name"], "taihao-extension-bridge");
    }

    #[test]
    fn tools_list_lists_whitelist() {
        let cfg = test_cfg();
        let resp = handle(&cfg, "t", "tools/list", &json!({"id": 2}));
        assert_eq!(resp["result"]["tools"][0]["name"], "echo_tool");
    }

    #[test]
    fn call_unknown_tool_denied() {
        let cfg = test_cfg();
        let resp = handle(&cfg, "t", "tools/call", &json!({"id": 3, "name": "rm_rf", "args": {}}));
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn call_whitelisted_tool_runs_command() {
        let cfg = test_cfg();
        let resp = handle(&cfg, "t", "tools/call", &json!({"id": 4, "name": "echo_tool", "args": {"message": "hi"}}));
        assert_eq!(resp["result"]["content"][0]["text"], "hello-from-extension");
    }

    #[test]
    fn unknown_method_rejected() {
        let cfg = test_cfg();
        let resp = handle(&cfg, "t", "exploit", &json!({"id": 5}));
        assert!(resp["error"].is_object());
    }
}
