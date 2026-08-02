//! 桥配置 — /etc/taihao-os/extension-bridge.toml
//!
//! 工具白名单是硬边界：MCP 客户端只能调用已注册的工具，
//! 每个工具对应一个扩展命令（脚本 / 可执行文件），参数白名单受控。

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDef {
    /// 工具名（JSON-RPC method 名，tools/call 的 params.name）
    pub name: String,
    /// 扩展命令（sh -c 执行；参数按白名单注入）
    pub command: String,
    /// 允许注入的参数名
    #[serde(default)]
    pub args: Vec<String>,
    /// 安全分级：L1 最高权限，L3 只读查询
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_level() -> String {
    "L2".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 请求目录（JSON-RPC 文件协议）
    #[serde(default = "default_request_dir")]
    pub request_dir: String,
    /// 响应目录
    #[serde(default = "default_response_dir")]
    pub response_dir: String,
    /// 轮询间隔（毫秒）
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    /// 工具白名单
    #[serde(default)]
    pub tools: Vec<ToolDef>,
}

fn default_request_dir() -> String {
    "/var/lib/taihao-os/extension-bridge/requests".into()
}
fn default_response_dir() -> String {
    "/var/lib/taihao-os/extension-bridge/responses".into()
}
fn default_poll_ms() -> u64 {
    200
}

impl Default for Config {
    fn default() -> Self {
        Config {
            request_dir: default_request_dir(),
            response_dir: default_response_dir(),
            poll_ms: default_poll_ms(),
            tools: vec![
                ToolDef {
                    name: "device_status".into(),
                    command: "cat /var/lib/taihao-os/rt-loop/loop-state.json".into(),
                    args: vec![].into(),
                    level: "L3".into(),
                },
                ToolDef {
                    name: "report".into(),
                    command: "echo report".into(),
                    args: vec!["message".into()].into(),
                    level: "L2".into(),
                },
            ],
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!("配置 {} 读取失败: {}，使用默认配置", path.display(), e));
                return (Config::default(), warnings);
            }
        };
        match toml::from_str::<Config>(&content) {
            Ok(cfg) => (cfg, warnings),
            Err(e) => {
                warnings.push(format!("配置解析失败: {}，使用默认配置", e));
                (Config::default(), warnings)
            }
        }
    }

    pub fn find_tool(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_tools() {
        let cfg = Config::default();
        assert!(cfg.find_tool("device_status").is_some());
        assert!(cfg.find_tool("hack").is_none());
    }

    #[test]
    fn parse_toml() {
        let toml = r#"
request_dir = "/tmp/ext/req"
response_dir = "/tmp/ext/resp"

[[tools]]
name = "ping"
command = "echo pong"
args = []
level = "L3"

[[tools]]
name = "run"
command = "/usr/bin/custom-run"
args = ["arg1", "arg2"]
level = "L1"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.tools.len(), 2);
        assert_eq!(cfg.find_tool("run").unwrap().args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn missing_file_falls_back() {
        let (cfg, warnings) = Config::load(Path::new("/nonexistent/extension-bridge.toml"));
        assert_eq!(cfg.tools.len(), 2);
        assert!(!warnings.is_empty());
    }
}
