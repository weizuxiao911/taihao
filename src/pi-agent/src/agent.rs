//! Agent Loop — 唯一智能决策层主循环
//!
//! 每 tick（默认 1s，≤1Hz）读取状态 → 构造决策请求 → 调 LLM →
//! 校验输出格式 → 下发目标/模式到决策文件。
//! 安全约束：LLM 输出仅为目标/模式，不直接控硬件（硬件走 HAL 网关，M2 未含）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::provider::{request_decision, DecisionRequest, ProviderConfig};
use crate::skill::{skills_to_context, Registry};

/// Agent Loop 配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// 决策周期（秒），必须 ≥1（≤1Hz 约束）
    pub tick_secs: u64,
    /// 状态输入文件（无则用内置默认状态）
    pub state_file: Option<PathBuf>,
    /// 决策输出文件（JSON）
    pub decision_file: PathBuf,
    /// 决策模式白名单：LLM 输出必须落在此集合内
    pub allowed_modes: Vec<String>,
    /// 决策前 Hooks（sh -c 命令，失败仅告警）
    pub hooks_before: Vec<String>,
    /// 决策后 Hooks（sh -c 命令，失败仅告警）
    pub hooks_after: Vec<String>,
    /// 已加载扩展的工具名（进决策上下文，实际调用走 Extension·MCP 桥）
    pub extension_tools: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            tick_secs: 1,
            state_file: None,
            decision_file: PathBuf::from("/var/lib/taihao-os/pi-agent/decision.json"),
            allowed_modes: vec![
                "idle".into(),
                "survey".into(),
                "return".into(),
                "emergency_stop".into(),
                "emergency_surface".into(),
                "emergency_avoidance".into(),
            ],
            hooks_before: vec![],
            hooks_after: vec![],
            extension_tools: vec![],
        }
    }
}

/// 决策输出（下发目标/模式）
#[derive(Debug, serde::Serialize)]
pub struct Decision {
    pub timestamp: u64,
    pub mode: String,
    pub target: Option<String>,
    pub rationale: String,
}

/// 一次决策的产物
pub struct TickResult {
    pub decision: Option<Decision>,
    pub llm_error: Option<String>,
}

/// 运行 Agent Loop（阻塞）
pub fn run_loop(
    registry: &Registry,
    provider: &ProviderConfig,
    agent_cfg: &AgentConfig,
    stop: Arc<AtomicBool>,
) {
    log::info!(
        "Agent Loop 启动: tick={}s, skills={}, provider={}",
        agent_cfg.tick_secs,
        registry.len(),
        provider.model
    );
    loop {
        if stop.load(Ordering::Relaxed) {
            log::info!("Agent Loop 收到停止信号");
            break;
        }
        match tick(registry, provider, agent_cfg) {
            TickResult { decision: Some(d), .. } => {
                log::info!("决策: mode={} target={:?} {}", d.mode, d.target, d.rationale);
                if let Err(e) = write_decision(agent_cfg, &d) {
                    log::error!("决策写入失败: {}", e);
                }
            }
            TickResult { decision: None, llm_error: Some(e) } => {
                log::warn!("本 tick 无决策: {}", e);
                // LLM 故障降级：不重复刷错误日志，等待下个 tick
            }
            TickResult { decision: None, llm_error: None } => {}
        }
        std::thread::sleep(std::time::Duration::from_secs(agent_cfg.tick_secs));
    }
}

/// 单次决策：读状态 → 构造请求 → 调 LLM → 校验 → 产出决策
fn tick(
    registry: &Registry,
    provider: &ProviderConfig,
    agent_cfg: &AgentConfig,
) -> TickResult {
    run_hooks(&agent_cfg.hooks_before, "before");
    let result = tick_inner(registry, provider, agent_cfg);
    run_hooks(&agent_cfg.hooks_after, "after");
    result
}

fn tick_inner(
    registry: &Registry,
    provider: &ProviderConfig,
    agent_cfg: &AgentConfig,
) -> TickResult {
    let state = read_state(agent_cfg);
    let system = build_system_prompt(registry, agent_cfg);
    let user = format!("当前设备状态:\n{}\n\n请输出决策（JSON）：mode 从 {} 中选择，target 为目标参数，rationale 为一句中文理由。", 
        state,
        agent_cfg.allowed_modes.join(" / "));

    let req = DecisionRequest { system, user };
    let raw = match request_decision(provider, &req) {
        Ok(s) => s,
        Err(e) => return TickResult { decision: None, llm_error: Some(e) },
    };
    match parse_decision(&raw, agent_cfg) {
        Ok(d) => TickResult { decision: Some(d), llm_error: None },
        Err(e) => TickResult { decision: None, llm_error: Some(e) },
    }
}

/// 执行 Hooks（sh -c）；失败仅告警，不中断决策
fn run_hooks(hooks: &[String], when: &str) {
    for cmd in hooks {
        match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
            Ok(out) if out.status.success() => log::debug!("Hook[{}] 成功: {}", when, cmd),
            Ok(out) => log::warn!("Hook[{}] 失败 (status={}): {}", when, out.status, cmd),
            Err(e) => log::warn!("Hook[{}] 执行错误: {} err={}", when, cmd, e),
        }
    }
}

/// 读取状态文件；缺失或解析失败时返回内置默认状态（不中断循环）
fn read_state(agent_cfg: &AgentConfig) -> String {
    let path = match &agent_cfg.state_file {
        Some(p) => p.clone(),
        None => return "默认状态：无状态输入文件，模式 idle".to_string(),
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => s.trim().to_string(),
        Err(e) => format!("状态读取失败: {}（使用默认状态 idle）", e),
    }
}

/// 组装 system prompt：SKILL 上下文 + 扩展工具 + 决策约束
fn build_system_prompt(registry: &Registry, agent_cfg: &AgentConfig) -> String {
    let ext_ctx = if agent_cfg.extension_tools.is_empty() {
        String::new()
    } else {
        format!(
            "可用扩展工具（经 Extension·MCP 桥调用，需在桥白名单内）: {}\n",
            agent_cfg.extension_tools.join(" / ")
        )
    };
    format!(
        "你是设备端智能决策层 Pi Agent。\n\
         只能从可用 SKILL 中选用能力，不得凭空创造动作。\n\
         {} {} \
         安全准则：\n\
         - 你只输出目标与模式，不直接控制任何硬件\n\
         - 紧急情况优先选择 emergency_stop / emergency_surface / emergency_avoidance\n\
         - mode 只允许以下白名单: {}\n\
         - 输出必须是合法 JSON，字段: mode, target, rationale",
        skills_to_context(registry),
        ext_ctx,
        agent_cfg.allowed_modes.join(" / ")
    )
}

/// 从 LLM 输出解析决策；容错：剥离代码块围栏、截取首个 { 到最后一个 }
fn parse_decision(raw: &str, agent_cfg: &AgentConfig) -> Result<Decision, String> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = cleaned.find('{').ok_or("输出无 JSON 对象")?;
    let end = cleaned.rfind('}').ok_or("输出无 JSON 对象")?;
    let json_str = &cleaned[start..=end];

    #[derive(serde::Deserialize)]
    struct Raw {
        mode: String,
        #[serde(default)]
        target: Option<String>,
        #[serde(default)]
        rationale: String,
    }
    let raw: Raw = serde_json::from_str(json_str).map_err(|e| format!("JSON 解析失败: {}", e))?;

    let mode = raw.mode.trim().to_lowercase();
    if !agent_cfg.allowed_modes.iter().any(|m| *m == mode) {
        return Err(format!("mode 不在白名单: {}", mode));
    }
    Ok(Decision {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        mode,
        target: raw.target,
        rationale: raw.rationale,
    })
}

/// 写决策文件（原子替换）
fn write_decision(agent_cfg: &AgentConfig, decision: &Decision) -> Result<(), String> {
    if let Some(parent) = agent_cfg.decision_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(decision).map_err(|e| e.to_string())?;
    let tmp = agent_cfg.decision_file.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &agent_cfg.decision_file).map_err(|e| e.to_string())?;
    log::debug!("决策写入: {}", agent_cfg.decision_file.display());
    Ok(())
}

/// 供测试/服务入口使用的轻量：一次决策并返回
pub fn run_once(registry: &Registry, provider: &ProviderConfig, agent_cfg: &AgentConfig) -> TickResult {
    tick(registry, provider, agent_cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_cfg() -> AgentConfig {
        AgentConfig {
            tick_secs: 1,
            state_file: None,
            decision_file: Path::new("/tmp/pi-agent-test/decision.json").to_path_buf(),
            allowed_modes: vec!["idle".into(), "emergency_stop".into()],
            hooks_before: vec![],
            hooks_after: vec![],
            extension_tools: vec![],
        }
    }

    #[test]
    fn hooks_run_and_fail_soft() {
        // before 成功、after 命令不存在（失败仅告警，不 panic）
        let mut cfg = test_cfg();
        cfg.hooks_before = vec!["echo hook-pre".into()];
        cfg.hooks_after = vec!["/nonexistent-cmd-xyz".into()];
        run_hooks(&cfg.hooks_before, "before");
        run_hooks(&cfg.hooks_after, "after");
    }

    #[test]
    fn prompt_includes_extension_tools() {
        let mut cfg = test_cfg();
        cfg.extension_tools = vec!["device_status".into()];
        let registry = Registry {
            skills: vec![],
            skipped: vec![],
        };
        let prompt = build_system_prompt(&registry, &cfg);
        assert!(prompt.contains("device_status"));
        assert!(prompt.contains("Extension·MCP 桥"));
    }

    #[test]
    fn parse_plain_json() {
        let cfg = test_cfg();
        let d = parse_decision(r#"{"mode": "idle", "target": null, "rationale": "无异常"}"#, &cfg).unwrap();
        assert_eq!(d.mode, "idle");
    }

    #[test]
    fn parse_fenced_json() {
        let cfg = test_cfg();
        let raw = "```json\n{\"mode\": \"emergency_stop\", \"rationale\": \"避障\"}\n```";
        let d = parse_decision(raw, &cfg).unwrap();
        assert_eq!(d.mode, "emergency_stop");
    }

    #[test]
    fn reject_unknown_mode() {
        let cfg = test_cfg();
        assert!(parse_decision(r#"{"mode": "launch_missile"}"#, &cfg).is_err());
    }

    #[test]
    fn reject_not_json() {
        let cfg = test_cfg();
        assert!(parse_decision("抱歉，我无法回答", &cfg).is_err());
    }

    #[test]
    fn write_decision_creates_dir() {
        let cfg = test_cfg();
        let d = Decision {
            timestamp: 1,
            mode: "idle".into(),
            target: None,
            rationale: "测试".into(),
        };
        write_decision(&cfg, &d).unwrap();
        assert!(cfg.decision_file.exists());
        std::fs::remove_dir_all("/tmp/pi-agent-test").ok();
    }
}
