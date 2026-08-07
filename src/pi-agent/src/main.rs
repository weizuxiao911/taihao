//! pi-agent — 认知决策层服务
//!
// 职责(契约 AGENTS.md「终端架构认知」):
// - Agent Loop:感知 → 理解 → 决策 → 动作,亚秒级(本服务降到 ~10Hz)
// - SKILL Registry:注册/加载/签名校验(ed25519 验签,白名单公钥)
// - Provider:OpenAI 兼容远端端点抽象(Stub 或真实 HTTP)
// - Hooks:决策前后的扩展点(通过 extension-bridge 接入)
// - Extension 加载:扩展由 extension-bridge 提供 JSON-RPC
// - 意图链路:决策产出 Intent → 发送到 comm-center 内部总线 → 路由到 rt-loop
//!
// 不做(职责边界):
// - 底层实时微操(交给 rt-loop)
// - 硬件直接访问(交给 hal-gateway)
// - 协议适配 / 路由(交给 comm-center)
// - 越权调用 / MCP 网关(交给 extension-bridge)

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use taihao_common::{init_logging, paths, Envelope};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::RwLock;

/// SKILL 注册表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub version: String,
    pub description: String,
    /// ed25519 签名(base64)
    pub signature: String,
    /// 执行入口(可由 extension-bridge 注入)
    pub entrypoint: String,
    /// 输入参数 schema(JSON Schema 字符串)
    #[serde(default)]
    pub input_schema: String,
}

/// SKILL 注册表
#[derive(Default)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 加载系统级 SKILL 目录
    pub async fn load_from(&mut self, dir: &Path) -> anyhow::Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(e) = entries.next_entry().await? {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let text = tokio::fs::read_to_string(&p).await?;
            match toml::from_str::<Skill>(&text) {
                Ok(skill) => {
                    tracing::info!(name = %skill.name, "加载 SKILL");
                    self.skills.insert(skill.name.clone(), skill);
                }
                Err(e) => tracing::warn!(file = %p.display(), error = %e, "SKILL 解析失败"),
            }
        }
        Ok(())
    }

    /// 校验 SKILL 签名(ed25519 公钥验签)
    pub fn verify(&self, skill: &Skill, pubkey: &ed25519_dalek::VerifyingKey) -> bool {
        let sig = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &skill.signature) {
            Ok(s) if s.len() == 64 => match ed25519_dalek::Signature::from_slice(&s) {
                Ok(sig) => sig,
                Err(_) => {
                    tracing::warn!(skill = %skill.name, "SKILL 签名解码失败");
                    return false;
                }
            },
            _ => {
                tracing::warn!(skill = %skill.name, sig_raw = %skill.signature, sig_len = skill.signature.len(), "SKILL 签名长度非法");
                return false;
            }
        };
        let msg = format!("{}:{}:{}", skill.name, skill.version, skill.entrypoint);
        let ok = pubkey.verify_strict(msg.as_bytes(), &sig).is_ok();
        if !ok {
            tracing::warn!(skill = %skill.name, msg = %msg, sig = %skill.signature, "SKILL 验签失败");
        }
        ok
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

/// OpenAI 兼容 Provider 抽象
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, req: CompletionRequest) -> anyhow::Result<CompletionResponse>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub usage: HashMap<String, u64>,
}

/// 占位 Provider — 不发实际请求,返回固定响应(mock 测试用)
pub struct StubProvider {
    name: String,
}

impl StubProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl Provider for StubProvider {
    fn name(&self) -> &str { &self.name }

    async fn complete(&self, req: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let content = req.messages.iter()
            .filter(|m| m.role == "user")
            .last()
            .map(|m| format!("[{} stub] ack: {}", self.name, m.content.chars().take(60).collect::<String>()))
            .unwrap_or_else(|| "[stub] empty".to_string());
        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".into(), 1);
        usage.insert("completion_tokens".into(), 1);
        Ok(CompletionResponse { content, usage })
    }
}

/// 真实 OpenAI 兼容 Provider — 通过 HTTP 调用远端端点
pub struct OpenAiProvider {
    name: String,
    endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: "openai-compat".into(),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

/// OpenAI /v1/chat/completions 响应结构
#[derive(Debug, Deserialize)]
struct OpenAiResp {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsg,
}

#[derive(Debug, Deserialize)]
struct OpenAiMsg {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str { &self.name }

    async fn complete(&self, req: CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": req.messages,
            "temperature": req.temperature,
        });
        let resp = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("OpenAI 端点请求失败")?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI 端点返回 {}: {}", status, text));
        }
        let data: OpenAiResp = resp.json().await.context("OpenAI 响应解析失败")?;
        let content = data.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
        let mut usage = HashMap::new();
        if let Some(u) = data.usage {
            if let Some(v) = u.prompt_tokens { usage.insert("prompt_tokens".into(), v); }
            if let Some(v) = u.completion_tokens { usage.insert("completion_tokens".into(), v); }
            if let Some(v) = u.total_tokens { usage.insert("total_tokens".into(), v); }
        }
        Ok(CompletionResponse { content, usage })
    }
}

/// Hooks:决策前后扩展点
#[derive(Default)]
pub struct Hooks {
    pre_decision: Vec<String>,
    post_decision: Vec<String>,
}

impl Hooks {
    pub fn new() -> Self { Self::default() }
    pub fn register_pre(&mut self, hook: impl Into<String>) { self.pre_decision.push(hook.into()); }
    pub fn register_post(&mut self, hook: impl Into<String>) { self.post_decision.push(hook.into()); }
}

/// 意图发送器:pi-agent → comm-center 内部总线(Unix socket)
pub struct IntentSender {
    sock_path: String,
    intents_emitted: Arc<std::sync::atomic::AtomicU64>,
}

impl IntentSender {
    pub fn new(sock_path: impl Into<String>) -> Self {
        Self { sock_path: sock_path.into(), intents_emitted: Arc::new(std::sync::atomic::AtomicU64::new(0)) }
    }

    /// 发送 Intent 到 comm-center 内部总线
    pub async fn send(&self, id: String, skill: String, params: serde_json::Value) -> anyhow::Result<()> {
        let mut stream = UnixStream::connect(&self.sock_path).await
            .with_context(|| format!("连接 {} 失败", self.sock_path))?;
        let env = Envelope::Intent {
            id: id.clone(),
            skill: skill.clone(),
            params: params.clone(),
            issued_at: chrono::Utc::now(),
        };
        let mut line = serde_json::to_string(&env)?;
        line.push('\n');
        stream.write_all(line.as_bytes()).await?;
        stream.shutdown().await?;
        self.intents_emitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        tracing::debug!(id = %id, skill = %skill, "Intent 已发送到 comm-center");
        Ok(())
    }

    pub fn count(&self) -> u64 {
        self.intents_emitted.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Agent Loop 入口
pub struct AgentLoop {
    skills: Arc<RwLock<SkillRegistry>>,
    provider: Arc<dyn Provider>,
    pubkey: Arc<ed25519_dalek::VerifyingKey>,
    hooks: Arc<RwLock<Hooks>>,
    sender: IntentSender,
}

impl AgentLoop {
    pub fn new(
        provider: Arc<dyn Provider>,
        pubkey: Arc<ed25519_dalek::VerifyingKey>,
        sock_path: impl Into<String>,
    ) -> Self {
        Self {
            skills: Arc::new(RwLock::new(SkillRegistry::new())),
            provider,
            pubkey,
            hooks: Arc::new(RwLock::new(Hooks::new())),
            sender: IntentSender::new(sock_path),
        }
    }

    pub async fn load_skills(&self, dir: &Path) -> anyhow::Result<()> {
        self.skills.write().await.load_from(dir).await
    }

    /// Agent Loop 单次迭代
    /// 感知输入 → Provider 推理 → 决策 → 发送 Intent
    pub async fn tick(&self, perception: &str) -> anyhow::Result<String> {
        // pre hooks
        for h in self.hooks.read().await.pre_decision.iter() {
            tracing::debug!(hook = %h, "pre-decision hook");
        }
        // Provider 推理
        let req = CompletionRequest {
            model: "taihao-default".into(),
            messages: vec![ChatMessage { role: "user".into(), content: perception.into() }],
            temperature: 0.2,
        };
        let resp = self.provider.complete(req).await?;

        // 解析 SKILL 名(感知输入 "skill:<name>" 或默认 echo)
        let skill_name = if perception.starts_with("skill:") {
            perception.trim_start_matches("skill:").split_whitespace().next().unwrap_or("").to_string()
        } else {
            "echo".to_string()
        };

        // SKILL 签名校验
        let valid = {
            let skills = self.skills.read().await;
            skills.get(&skill_name)
                .map(|s| skills.verify(s, &self.pubkey))
                .unwrap_or(true)
        };

        // 决策 → 发送 Intent(通过 comm-center 路由到 rt-loop)
        let decision = if valid { "approved" } else { "denied" };
        if valid {
            let params = serde_json::json!({
                "perception": perception,
                "skill": skill_name,
                "provider": self.provider.name(),
            });
            // 尝试发送;失败不致命(打印 warn 继续)
            if let Err(e) = self.sender.send(
                format!("intent-{}", chrono::Utc::now().timestamp_millis()),
                skill_name.clone(),
                params,
            ).await {
                tracing::warn!(error = %e, "Intent 发送失败");
            }
        }

        // post hooks
        for h in self.hooks.read().await.post_decision.iter() {
            tracing::debug!(hook = %h, "post-decision hook");
        }

        Ok(format!("{} [skill={},decision={},intents={}]", resp.content, skill_name, decision, self.sender.count()))
    }

    pub fn intents_count(&self) -> u64 {
        self.sender.count()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("pi-agent");

    // 公钥:从环境变量 TAIHAO_SKILL_PUBKEY(base64 ed25519 公钥)读取
    // 未提供时退化为"不验签"(仅日志警告)
    let pubkey_raw = std::env::var("TAIHAO_SKILL_PUBKEY").ok();
    let pubkey: Arc<ed25519_dalek::VerifyingKey> = match pubkey_raw {
        Some(b64) => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &b64)
                .context("TAIHAO_SKILL_PUBKEY 不是合法 base64")?;
            if bytes.len() != 32 {
                return Err(anyhow::anyhow!("TAIHAO_SKILL_PUBKEY 长度应为 32 字节"));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            let k = ed25519_dalek::VerifyingKey::from_bytes(&arr)
                .context("TAIHAO_SKILL_PUBKEY 不是合法 ed25519 公钥")?;
            Arc::new(k)
        }
        None => {
            tracing::warn!("未设置 TAIHAO_SKILL_PUBKEY,SKILL 签名校验降级为不验签");
            // 占位公钥(全零,不可能验签通过任何真实签名)
            let k = ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32]).unwrap();
            Arc::new(k)
        }
    };
    tracing::info!("SKILL 公钥已加载: {}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pubkey.as_bytes()));

    // Provider:优先真实 OpenAI 兼容端点(环境变量),否则 Stub
    let provider: Arc<dyn Provider> = if let Ok(endpoint) = std::env::var("PI_AGENT_LLM_ENDPOINT") {
        let api_key = std::env::var("PI_AGENT_LLM_API_KEY").unwrap_or_default();
        let model = std::env::var("PI_AGENT_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
        tracing::info!(endpoint = %endpoint, model = %model, "使用 OpenAI 兼容 Provider");
        Arc::new(OpenAiProvider::new(endpoint, api_key, model))
    } else {
        tracing::warn!("未设置 PI_AGENT_LLM_ENDPOINT,使用 StubProvider");
        Arc::new(StubProvider::new("stub"))
    };

    // comm-center 内部总线 socket
    let comm_sock = std::env::var("COMM_CENTER_SOCK")
        .unwrap_or_else(|_| "/run/taihao-os/comm-center.sock".into());

    let loop_ = Arc::new(AgentLoop::new(provider, pubkey, comm_sock));

    // 加载 SKILL(若目录存在)
    let skills_dir = paths::skills_system();
    if let Err(e) = loop_.load_skills(&skills_dir).await {
        tracing::warn!(error = %e, "SKILL 加载跳过");
    }

    // 内置 echo SKILL 仅兜底:若文件加载的 SKILL 已存在同名(如带签名 echo),不覆盖
    {
        let mut r = loop_.skills.write().await;
        if !r.skills.contains_key("echo") {
            let demo = Skill {
                name: "echo".into(),
                version: "0.1.0".into(),
                description: "内置 echo SKILL(占位,无签名)".into(),
                signature: String::new(),
                entrypoint: "echo".into(),
                input_schema: String::new(),
            };
            r.skills.insert(demo.name.clone(), demo);
        }
    }

    // 健康探活 HTTP(仿真下无 httpx,直接 std net)
    let port: u16 = std::env::var("PI_AGENT_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8081);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await.with_context(|| format!("绑定端口 {} 失败", port))?;
    tracing::info!(addr = %format!("127.0.0.1:{}", port), "pi-agent 探活端口");

    // Agent Loop 周期(10Hz,实际 ~100ms)
    let period = std::time::Duration::from_millis(100);
    let loop_tick = loop_.clone();

    tokio::select! {
        _ = async {
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(p) => p, Err(_) => continue,
                };
                use tokio::io::AsyncWriteExt;
                let body = format!("{{\"ok\":true,\"intents\":{}}}\n", loop_tick.intents_count());
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        } => {},
        _ = async {
            let mut ticker = tokio::time::interval(period);
            let mut i = 0u64;
            loop {
                ticker.tick().await;
                let p = format!("tick#{}", i);
                if let Ok(out) = loop_tick.tick(&p).await {
                    if i % 10 == 0 {
                        tracing::debug!(output = %out, "Agent Loop 决策");
                    }
                }
                i += 1;
            }
        } => {},
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("收到 SIGINT,退出");
        }
    }

    Ok(())
}