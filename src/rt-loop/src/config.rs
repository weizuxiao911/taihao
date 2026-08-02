//! 实时回路配置 — /etc/taihao-os/rt-loop.toml
//!
//! 频率 10~100Hz；决策文件来自 Pi Agent（≤1Hz 下发目标/模式），
//! 本回路把模式映射为执行器指令（10ms 级周期，不经 LLM）。

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// 回路频率（Hz），10~100，默认 50
    #[serde(default = "default_freq")]
    pub frequency_hz: u64,
    /// Pi Agent 决策文件（JSON: mode/target）
    #[serde(default = "default_decision")]
    pub decision_file: String,
    /// 状态输入文件（无则用内置 mock 状态）
    #[serde(default)]
    pub state_file: Option<String>,
    /// 执行器输出文件（JSON）
    #[serde(default = "default_actuator")]
    pub actuator_file: String,
    /// 控制周期状态文件（JSON）
    #[serde(default = "default_loop_state")]
    pub loop_state_file: String,
    /// 允许的模式集合（超出视为非法决策，降级 idle）
    #[serde(default = "default_modes")]
    pub allowed_modes: Vec<String>,
}

fn default_freq() -> u64 {
    50
}
fn default_decision() -> String {
    "/var/lib/taihao-os/pi-agent/decision.json".into()
}
fn default_actuator() -> String {
    "/var/lib/taihao-os/rt-loop/actuator.json".into()
}
fn default_loop_state() -> String {
    "/var/lib/taihao-os/rt-loop/loop-state.json".into()
}
fn default_modes() -> Vec<String> {
    vec![
        "idle".into(),
        "survey".into(),
        "return".into(),
        "emergency_stop".into(),
        "emergency_surface".into(),
        "emergency_avoidance".into(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            frequency_hz: default_freq(),
            decision_file: default_decision(),
            state_file: None,
            actuator_file: default_actuator(),
            loop_state_file: default_loop_state(),
            allowed_modes: default_modes(),
        }
    }
}

impl Config {
    /// 从路径加载；失败返回默认（带告警）
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
}

/// 回路运行时的路径集合
pub struct Paths {
    pub decision: PathBuf,
    pub actuator: PathBuf,
    pub loop_state: PathBuf,
    pub state: Option<PathBuf>,
}

impl From<&Config> for Paths {
    fn from(c: &Config) -> Self {
        Paths {
            decision: PathBuf::from(&c.decision_file),
            actuator: PathBuf::from(&c.actuator_file),
            loop_state: PathBuf::from(&c.loop_state_file),
            state: c.state_file.as_ref().map(PathBuf::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.frequency_hz, 50);
        assert!(cfg.allowed_modes.contains(&"emergency_stop".to_string()));
    }

    #[test]
    fn parse_toml() {
        let toml = r#"
frequency_hz = 100
decision_file = "/tmp/decision.json"
actuator_file = "/tmp/actuator.json"
loop_state_file = "/tmp/loop.json"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.frequency_hz, 100);
    }

    #[test]
    fn missing_file_falls_back() {
        let (cfg, warnings) = Config::load(Path::new("/nonexistent/rt-loop.toml"));
        assert_eq!(cfg.frequency_hz, 50);
        assert!(!warnings.is_empty());
    }
}
