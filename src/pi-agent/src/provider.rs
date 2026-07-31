//! Provider — OpenAI 兼容端点客户端
//!
//! M2 只做远端 OpenAI 兼容 REST 调用；设备端 NPU 推理（llama.cpp / RKNN）留待后续。
//! API key 不写入配置，从环境变量读取。

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

/// Provider 配置（来自 /etc/taihao-os/pi-agent.toml 的 [provider] 段）
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    /// OpenAI 兼容端点 base URL，如 http://192.168.1.10:8000/v1
    pub base_url: String,
    pub model: String,
    /// API key 环境变量名（默认 OPENAI_API_KEY）
    #[serde(default = "default_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_key_env() -> String {
    "OPENAI_API_KEY".to_string()
}

fn default_timeout() -> u64 {
    10
}

/// ChatCompletion 响应（只取需要的字段）
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}

/// 决策请求：system 上下文 + 当前状态
#[derive(Debug, Clone)]
pub struct DecisionRequest {
    pub system: String,
    pub user: String,
}

/// 调用 LLM 返回决策文本
pub fn request_decision(cfg: &ProviderConfig, req: &DecisionRequest) -> Result<String, String> {
    let key = std::env::var(&cfg.api_key_env)
        .map_err(|_| format!("环境变量 {} 未设置", cfg.api_key_env))?;

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let body = json!({
        "model": cfg.model,
        "messages": [
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user}
        ],
        "temperature": 0.2,
        "max_tokens": 512,
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build();

    let resp = agent
        .post(&url)
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    let chat: ChatResponse = resp.into_json().map_err(|e| format!("响应解析失败: {}", e))?;
    chat.choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or("LLM 响应无 choices".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_config_defaults() {
        let cfg: ProviderConfig = serde_yaml::from_str(
            "base_url: http://localhost:8000/v1\nmodel: qwen2.5-7b",
        )
        .unwrap();
        assert_eq!(cfg.api_key_env, "OPENAI_API_KEY");
        assert_eq!(cfg.timeout_secs, 10);
    }

    #[test]
    fn url_join_no_double_slash() {
        let cfg = ProviderConfig {
            base_url: "http://localhost:8000/v1/".into(),
            model: "m".into(),
            api_key_env: "K".into(),
            timeout_secs: 1,
        };
        let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
        assert_eq!(url, "http://localhost:8000/v1/chat/completions");
    }
}
