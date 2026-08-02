//! HAL 网关配置 — /etc/taihao-os/hal-gateway.toml
//!
//! 白名单是硬边界：未注册的工具一律拒绝，注册工具的参数名同样受白名单约束。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 工具注册项
#[derive(Debug, Clone, Deserialize)]
pub struct ToolDef {
    /// 工具名（SKILL 调用时的标识）
    pub name: String,
    /// 驱动类型（mock 为默认；spidev / i2c / pwm / serial / can / v4l2 为真机驱动槽位）
    #[serde(default = "default_driver")]
    pub driver: String,
    /// 允许的参数名（白名单，其余参数拒绝）
    #[serde(default)]
    pub args: Vec<String>,
    /// 安全分级：L1 紧急保护，L2 常规控制，L3 查询
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_driver() -> String {
    "mock".into()
}
fn default_level() -> String {
    "L2".into()
}

/// HAL 网关配置
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 审计日志路径
    #[serde(default = "default_audit_log")]
    pub audit_log: String,
    /// 服务模式：请求目录（JSON 文件协议）
    #[serde(default = "default_request_dir")]
    pub request_dir: String,
    /// 服务模式：响应目录
    #[serde(default = "default_response_dir")]
    pub response_dir: String,
    /// 服务模式：轮询间隔（毫秒）
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    /// 工具白名单
    #[serde(default)]
    pub tools: Vec<ToolDef>,
}

fn default_audit_log() -> String {
    "/var/log/taihao-os/hal-audit.log".into()
}
fn default_request_dir() -> String {
    "/var/lib/taihao-os/hal-gateway/requests".into()
}
fn default_response_dir() -> String {
    "/var/lib/taihao-os/hal-gateway/responses".into()
}
fn default_poll_ms() -> u64 {
    200
}

impl Default for Config {
    fn default() -> Self {
        Config {
            audit_log: default_audit_log(),
            request_dir: default_request_dir(),
            response_dir: default_response_dir(),
            poll_ms: default_poll_ms(),
            tools: vec![
                ToolDef {
                    name: "thruster_set".into(),
                    driver: "mock".into(),
                    args: vec!["power".into(), "channel".into()],
                    level: "L2".into(),
                },
                ToolDef {
                    name: "thruster_emergency_stop".into(),
                    driver: "mock".into(),
                    args: vec![].into(),
                    level: "L1".into(),
                },
                ToolDef {
                    name: "buoyancy_set".into(),
                    driver: "mock".into(),
                    args: vec!["volume".into()],
                    level: "L1".into(),
                },
                ToolDef {
                    name: "sensor_read".into(),
                    driver: "mock".into(),
                    args: vec!["sensor".into()],
                    level: "L3".into(),
                },
            ],
        }
    }
}

impl Config {
    /// 从路径加载；失败时返回默认配置（带告警），保证无配置也可运行
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

    /// 按名字查工具白名单
    pub fn find_tool(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }
}

/// 运行时路径集合（供 CLI 使用）
pub struct Paths {
    pub audit_log: PathBuf,
    pub request_dir: PathBuf,
    pub response_dir: PathBuf,
}

impl From<&Config> for Paths {
    fn from(c: &Config) -> Self {
        Paths {
            audit_log: PathBuf::from(&c.audit_log),
            request_dir: PathBuf::from(&c.request_dir),
            response_dir: PathBuf::from(&c.response_dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_core_tools() {
        let cfg = Config::default();
        assert!(cfg.find_tool("thruster_set").is_some());
        assert!(cfg.find_tool("thruster_emergency_stop").is_some());
        assert!(cfg.find_tool("nope").is_none());
    }

    #[test]
    fn parse_toml_with_tools() {
        let toml = r#"
audit_log = "/tmp/hal/audit.log"
request_dir = "/tmp/hal/req"
response_dir = "/tmp/hal/resp"
poll_ms = 50

[[tools]]
name = "pwm_set"
driver = "pwm"
args = ["channel", "duty"]
level = "L2"

[[tools]]
name = "leak_check"
driver = "mock"
args = []
level = "L1"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.tools.len(), 2);
        let pwm = cfg.find_tool("pwm_set").unwrap();
        assert_eq!(pwm.driver, "pwm");
        assert_eq!(pwm.args, vec!["channel", "duty"]);
        assert_eq!(cfg.find_tool("leak_check").unwrap().level, "L1");
    }

    #[test]
    fn missing_file_falls_back() {
        let (cfg, warnings) = Config::load(Path::new("/nonexistent/hal-gateway.toml"));
        assert_eq!(cfg.tools.len(), 4);
        assert!(!warnings.is_empty());
    }
}
