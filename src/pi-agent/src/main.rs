//! pi-agent — 认知决策层服务
//!
// 职责(契约 AGENTS.md「终端架构认知」):
// - Agent Loop:感知 → 理解 → 决策 → 动作,亚秒级(本服务降到 ~10Hz)
// - SKILL Registry:注册/加载/签名校验(白名单签名 skill 才执行)
// - Provider:OpenAI 兼容远端端点抽象(本服务占位,实际请求可转发到远端)
// - Hooks:决策前后的扩展点(通过 extension-bridge 接入)
// - Extension 加载:扩展由 extension-bridge 提供 JSON-RPC
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
use taihao_common::{init_logging, paths};
use tokio::sync::RwLock;

/// SKILL 注册表条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub version: String,
    pub description: String,
    /// SHA-256 摘要,用于签名校验
    pub sha256: String,
    /// 执行入口(可由 extension-bridge 注入)
    pub entrypoint: String,
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

    /// 校验 SKILL 签名(只接受白名单摘要)
    pub fn verify(&self, skill: &Skill) -> bool {
        // 仿真占位:实际生产用公钥验签
        !skill.sha256.is_empty()
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
    /// 名称
    fn name(&self) -> &str;
    /// 推理:OpenAI /v1/chat/completions 语义
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

/// 占位 Provider — 不发实际请求,返回固定响应
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
        // 占位实现:返回最后一个 user message 的前缀作为"推理"结果
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

/// Agent Loop 入口
pub struct AgentLoop {
    skills: Arc<RwLock<SkillRegistry>>,
    provider: Arc<dyn Provider>,
    hooks: Arc<RwLock<Hooks>>,
    intents_emitted: Arc<std::sync::atomic::AtomicU64>,
}

impl AgentLoop {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            skills: Arc::new(RwLock::new(SkillRegistry::new())),
            provider,
            hooks: Arc::new(RwLock::new(Hooks::new())),
            intents_emitted: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn load_skills(&self, dir: &Path) -> anyhow::Result<()> {
        self.skills.write().await.load_from(dir).await
    }

    /// Agent Loop 单次迭代
    pub async fn tick(&self, perception: &str) -> anyhow::Result<String> {
        // pre hooks
        for h in self.hooks.read().await.pre_decision.iter() {
            tracing::debug!(hook = %h, "pre-decision hook");
        }
        // Provider 推理
        let req = CompletionRequest {
            model: "taihao-stub".into(),
            messages: vec![ChatMessage { role: "user".into(), content: perception.into() }],
            temperature: 0.2,
        };
        let resp = self.provider.complete(req).await?;
        let skill_name = if perception.starts_with("skill:") {
            perception.trim_start_matches("skill:").split_whitespace().next().unwrap_or("").to_string()
        } else {
            "echo".to_string()
        };
        // 校验 SKILL 签名
        let skills = self.skills.read().await;
        let valid = skills.get(&skill_name).map(|s| skills.verify(s)).unwrap_or(true);
        // post hooks
        for h in self.hooks.read().await.post_decision.iter() {
            tracing::debug!(hook = %h, "post-decision hook");
        }
        // 发出 Intent(由 comm-center 路由到 rt-loop)
        self.intents_emitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let decision = if valid { "approved" } else { "denied" };
        Ok(format!("{} [skill={},decision={}]", resp.content, skill_name, decision))
    }

    pub fn intents_count(&self) -> u64 {
        self.intents_emitted.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging("pi-agent");

    let provider: Arc<dyn Provider> = Arc::new(StubProvider::new("stub"));
    let loop_ = Arc::new(AgentLoop::new(provider));

    // 加载 SKILL(若目录存在)
    let skills_dir = paths::skills_system();
    if let Err(e) = loop_.load_skills(&skills_dir).await {
        tracing::warn!(error = %e, "SKILL 加载跳过");
    }

    // 注册内置 SKILL 列表
    {
        let mut r = loop_.skills.write().await;
        let demo = Skill {
            name: "echo".into(),
            version: "0.1.0".into(),
            description: "内置 echo SKILL(占位)".into(),
            sha256: "demo-sha256-stub".into(),
            entrypoint: "echo".into(),
        };
        r.skills.insert(demo.name.clone(), demo);
    }

    // 健康探活 HTTP(仿真下没有 httpx,直接 std net)
    let port: u16 = std::env::var("PI_AGENT_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8081);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await.with_context(|| format!("绑定端口 {} 失败", port))?;
    tracing::info!(addr = %format!("127.0.0.1:{}", port), "pi-agent 探活端口");

    // Agent Loop 周期(10Hz,实际 ~100ms)
    let period = std::time::Duration::from_millis(100);
    let loop_tick = loop_.clone();

    tokio::select! {
        _ = async {
            // 健康端点
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(p) => p, Err(_) => continue,
                };
                use tokio::io::{AsyncWriteExt};
                let body = format!("{{\"ok\":true,\"intents\":{}}}\n", loop_tick.intents_count());
                let _ = sock.write_all(body.as_bytes()).await;
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