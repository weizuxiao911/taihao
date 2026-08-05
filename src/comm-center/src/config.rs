//! 中台调度通信配置 — /etc/taihao-os/comm-center.toml
//!
//! 双链路：远程（SSE/WS/MQTT 任选其一为协议，4G/5G 通道）+ 本地 Mesh（UDP）。
//! 主备关系与健康阈值由配置决定；对接对象（岸基 / 云 / 邻域）由厂商配置决定。

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 远程链路协议：sse_http / websocket / mqtt
    #[serde(default = "default_remote_proto")]
    pub remote_proto: String,
    /// 远程端点（SSE 订阅 + HTTP 上报 base，或 WS URL，或 MQTT broker）
    #[serde(default = "default_remote_endpoint")]
    pub remote_endpoint: String,
    /// MQTT 主题前缀
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,
    /// 本地 Mesh 端口（UDP）
    #[serde(default = "default_mesh_port")]
    pub mesh_port: u16,
    /// 健康检查周期（秒）
    #[serde(default = "default_health_secs")]
    pub health_check_secs: u64,
    /// 周期状态上报间隔（秒）：0 = 关闭周期上报（默认 10s）
    #[serde(default = "default_status_report_secs")]
    pub status_report_secs: u64,
    /// 故障切换阈值：连续失败 N 次切换
    #[serde(default = "default_fail_threshold")]
    pub fail_threshold: u32,
    /// 路由表：topic → 处理方式（script 命令 或 builtin:report）
    #[serde(default)]
    pub routes: Vec<RouteDef>,
    /// 状态上报文件（JSON，供调试/验证读取）
    #[serde(default = "default_status_file")]
    pub status_file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteDef {
    /// 消息主题（支持通配前缀匹配）
    pub topic: String,
    /// 处理方式：script:<命令> 或 builtin:report
    pub action: String,
}

fn default_remote_proto() -> String {
    "sse_http".into()
}
fn default_remote_endpoint() -> String {
    "http://127.0.0.1:9000".into()
}
fn default_topic_prefix() -> String {
    "taihao/".into()
}
fn default_mesh_port() -> u16 {
    41800
}
fn default_health_secs() -> u64 {
    5
}
fn default_status_report_secs() -> u64 {
    10
}
fn default_fail_threshold() -> u32 {
    3
}
fn default_status_file() -> String {
    "/var/lib/taihao-os/comm-center/status.json".into()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            remote_proto: default_remote_proto(),
            remote_endpoint: default_remote_endpoint(),
            topic_prefix: default_topic_prefix(),
            mesh_port: default_mesh_port(),
            health_check_secs: default_health_secs(),
            status_report_secs: default_status_report_secs(),
            fail_threshold: default_fail_threshold(),
            routes: vec![
                RouteDef { topic: "task".into(), action: "builtin:report".into() },
                RouteDef { topic: "command".into(), action: "builtin:report".into() },
            ],
            status_file: default_status_file(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let mut cfg = match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    warnings.push(format!("配置解析失败: {}，使用默认配置", e));
                    Config::default()
                }
            },
            Err(e) => {
                warnings.push(format!("配置 {} 读取失败: {}，使用默认配置", path.display(), e));
                Config::default()
            }
        };
        // 后台端点环境变量覆盖（REMOTE_ENDPOINT，部署时经 systemd EnvironmentFile 注入）
        if let Ok(ep) = std::env::var("REMOTE_ENDPOINT") {
            if !ep.is_empty() {
                warnings.push(format!("REMOTE_ENDPOINT 环境变量覆盖 remote_endpoint: {}", ep));
                cfg.remote_endpoint = ep;
            }
        }
        (cfg, warnings)
    }

    pub fn find_route(&self, topic: &str) -> Option<&RouteDef> {
        self.routes
            .iter()
            .find(|r| topic.starts_with(&r.topic) || topic == r.topic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.remote_proto, "sse_http");
        assert_eq!(cfg.fail_threshold, 3);
    }

    #[test]
    fn parse_toml() {
        let toml = r#"
remote_proto = "mqtt"
remote_endpoint = "tcp://10.0.0.2:1883"
topic_prefix = "fleet/a/"
mesh_port = 42000
fail_threshold = 5

[[routes]]
topic = "task"
action = "builtin:report"

[[routes]]
topic = "ota"
action = "script:/usr/bin/taihao-ota"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.remote_proto, "mqtt");
        assert_eq!(cfg.find_route("task/123").unwrap().action, "builtin:report");
        assert_eq!(cfg.find_route("ota/1").unwrap().action, "script:/usr/bin/taihao-ota");
        assert!(cfg.find_route("unknown").is_none());
    }

    #[test]
    fn missing_file_falls_back() {
        let (cfg, warnings) = Config::load(Path::new("/nonexistent/comm-center.toml"));
        assert_eq!(cfg.mesh_port, 41800);
        assert!(!warnings.is_empty());
    }
}
