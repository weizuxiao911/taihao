//! 配置加载 — /etc/taihao-os/pi-agent.toml
//!
//! 配置缺失时使用内置默认（可无配置文件运行，便于 QEMU 冒烟验证）。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::agent::AgentConfig;
use crate::provider::ProviderConfig;

/// 顶层配置
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub agent: AgentToml,
    #[serde(default)]
    pub provider: ProviderToml,
    /// SKILL 目录列表
    #[serde(default)]
    pub skill_dirs: Vec<String>,
    /// SKILL 签名密钥的环境变量名（未设置则不启用签名校验）
    #[serde(default)]
    pub signing_key_env: Option<String>,
    /// Hooks：决策前后执行的外部命令
    #[serde(default)]
    pub hooks: Vec<HookToml>,
    /// Extension 目录列表（加载 extension.toml 工具清单）
    #[serde(default)]
    pub extension_dirs: Vec<String>,
}

/// Hook 定义
#[derive(Debug, Clone, Deserialize)]
pub struct HookToml {
    /// 触发时机：before（决策前）/ after（决策后）
    pub when: String,
    /// 执行命令（sh -c）
    pub command: String,
}

/// agent 段（与 AgentConfig 对应）
#[derive(Debug, Clone, Deserialize)]
pub struct AgentToml {
    #[serde(default = "default_tick")]
    pub tick_secs: u64,
    #[serde(default)]
    pub state_file: Option<String>,
    #[serde(default = "default_decision_file")]
    pub decision_file: String,
    #[serde(default = "default_modes")]
    pub allowed_modes: Vec<String>,
}

impl Default for AgentToml {
    fn default() -> Self {
        AgentToml {
            tick_secs: 1,
            state_file: None,
            decision_file: "/var/lib/taihao-os/pi-agent/decision.json".into(),
            allowed_modes: default_modes(),
        }
    }
}

fn default_tick() -> u64 {
    1
}
fn default_decision_file() -> String {
    "/var/lib/taihao-os/pi-agent/decision.json".into()
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

/// provider 段
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderToml {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for ProviderToml {
    fn default() -> Self {
        ProviderToml {
            base_url: "http://127.0.0.1:8000/v1".into(),
            model: "taihao-default".into(),
            api_key_env: None,
            timeout_secs: 10,
        }
    }
}

fn default_timeout() -> u64 {
    10
}

impl Config {
    /// 从路径加载；文件不存在或解析失败时返回默认配置（带告警）
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

    /// 签名密钥（从环境变量读取，未设置返回 None = 不启用签名校验）
    pub fn signing_key(&self) -> Option<String> {
        let env = self.signing_key_env.clone().unwrap_or_else(|| "TAIHAO_SKILL_SIGNING_KEY".to_string());
        std::env::var(&env).ok().filter(|k| !k.is_empty())
    }

    /// 转成运行结构
    pub fn to_runtime(&self) -> (AgentConfig, ProviderConfig, Vec<PathBuf>) {
        let (hooks_before, hooks_after): (Vec<String>, Vec<String>) = self
            .hooks
            .iter()
            .filter(|h| h.when == "before" || h.when == "after")
            .fold((Vec::new(), Vec::new()), |(mut b, mut a), h| {
                if h.when == "before" {
                    b.push(h.command.clone());
                } else {
                    a.push(h.command.clone());
                }
                (b, a)
            });

        let agent = AgentConfig {
            tick_secs: self.agent.tick_secs,
            state_file: self.agent.state_file.clone().map(PathBuf::from),
            decision_file: PathBuf::from(&self.agent.decision_file),
            allowed_modes: self.agent.allowed_modes.clone(),
            hooks_before,
            hooks_after,
            // Extension 工具由 main 在加载后注入
            extension_tools: vec![],
        };
        let provider = ProviderConfig {
            base_url: self.provider.base_url.clone(),
            model: self.provider.model.clone(),
            api_key_env: self
                .provider
                .api_key_env
                .clone()
                .unwrap_or_else(|| "OPENAI_API_KEY".to_string()),
            timeout_secs: self.provider.timeout_secs,
        };
        let skill_dirs = if self.skill_dirs.is_empty() {
            vec![
                PathBuf::from("/usr/share/taihao-os/skills"),
                PathBuf::from("/etc/taihao-os/skills"),
            ]
        } else {
            self.skill_dirs.iter().map(PathBuf::from).collect()
        };
        (agent, provider, skill_dirs)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            agent: AgentToml::default(),
            provider: ProviderToml::default(),
            skill_dirs: vec![],
            signing_key_env: None,
            hooks: vec![],
            extension_dirs: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_falls_back_to_default() {
        let (cfg, warnings) = Config::load(Path::new("/nonexistent/pi-agent.toml"));
        assert_eq!(cfg.agent.tick_secs, 1);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn parse_toml() {
        let toml = r#"
skill_dirs = ["/tmp/skills"]

[agent]
tick_secs = 2
[provider]
base_url = "http://10.0.0.2:8000/v1"
model = "qwen2.5-7b"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.agent.tick_secs, 2);
        assert_eq!(cfg.provider.model, "qwen2.5-7b");
        assert_eq!(cfg.skill_dirs, vec!["/tmp/skills"]);
    }

    #[test]
    fn skill_dirs_must_be_top_level() {
        // 顶层字段必须位于 [表] 之前（TOML 语义：表后的裸键属于该表）
        let toml = "skill_dirs = [\"/tmp/skills\"]\n[agent]\ntick_secs = 1\n[provider]\nbase_url = \"http://x/v1\"\nmodel = \"m\"\n";
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.skill_dirs, vec!["/tmp/skills"]);
        let (_, _, dirs) = cfg.to_runtime();
        assert_eq!(dirs, vec![std::path::PathBuf::from("/tmp/skills")]);
    }

    #[test]
    fn hooks_split_before_after() {
        let toml = r#"
[[hooks]]
when = "before"
command = "echo pre"

[[hooks]]
when = "after"
command = "echo post"

[[hooks]]
when = "unknown"
command = "echo skip"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        let (agent, _, _) = cfg.to_runtime();
        assert_eq!(agent.hooks_before, vec!["echo pre"]);
        assert_eq!(agent.hooks_after, vec!["echo post"]);
    }

    #[test]
    fn signing_key_env_default() {
        let cfg = Config::default();
        std::env::remove_var("TAIHAO_SKILL_SIGNING_KEY");
        assert!(cfg.signing_key().is_none());
        std::env::set_var("TAIHAO_SKILL_SIGNING_KEY", "secret");
        assert_eq!(cfg.signing_key().as_deref(), Some("secret"));
        std::env::remove_var("TAIHAO_SKILL_SIGNING_KEY");
    }
}
