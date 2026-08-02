//! HAL 网关核心 — 硬件调用的唯一入口
//!
//! 安全边界（对应设计两条核心安全准则的第二条）：
//! 1. 白名单校验：工具必须已注册，参数名必须落在工具白名单内；
//! 2. 审计日志：每次调用（含被拒绝的调用）落盘，可追溯；
//! 3. 调用方标识：SKILL 调用必须声明调用方，审计留痕。

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::config::Config;
use crate::driver::{call_mock, DriverResult};

/// 一次 HAL 调用的审计记录
#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub caller: String,
    pub tool: String,
    pub args: BTreeMap<String, String>,
    pub allowed: bool,
    pub result: String,
}

/// 调用结果
#[derive(Debug, Clone)]
pub struct CallOutcome {
    pub allowed: bool,
    pub denied_reason: Option<String>,
    pub result: Option<DriverResult>,
}

/// 执行一次 HAL 调用（库 API）
pub fn call(cfg: &Config, caller: &str, tool: &str, args: &BTreeMap<String, String>) -> CallOutcome {
    let audit = |allowed: bool, result: String| AuditEntry {
        timestamp: now_secs(),
        caller: caller.to_string(),
        tool: tool.to_string(),
        args: args.clone(),
        allowed,
        result,
    };

    // 1. 白名单校验：工具存在
    let def = match cfg.find_tool(tool) {
        Some(d) => d,
        None => {
            let entry = audit(false, "tool not in whitelist".into());
            append_audit(cfg, &entry);
            return CallOutcome { allowed: false, denied_reason: Some(format!("工具不在白名单: {}", tool)), result: None };
        }
    };

    // 2. 参数白名单校验
    let disallowed = crate::driver::disallowed_args(&def.args, args);
    if !disallowed.is_empty() {
        let entry = audit(false, format!("args not allowed: {:?}", disallowed));
        append_audit(cfg, &entry);
        return CallOutcome {
            allowed: false,
            denied_reason: Some(format!("参数不在白名单: {:?}", disallowed)),
            result: None,
        };
    }

    // 3. 执行（mock 驱动；真机驱动按同接口替换）
    let result = call_mock(&def.driver, tool, args);
    let entry = audit(result.ok, result.detail.clone());
    append_audit(cfg, &entry);
    CallOutcome { allowed: true, denied_reason: None, result: Some(result) }
}

/// 追加审计日志（append 模式，审计不可覆盖）
fn append_audit(cfg: &Config, entry: &AuditEntry) {
    let path = Path::new(&cfg.audit_log);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(entry) {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// 供测试：审计日志最后一行
pub fn last_audit_line(cfg: &Config) -> Option<String> {
    let path = Path::new(&cfg.audit_log);
    let content = fs::read_to_string(path).ok()?;
    content.lines().last().map(|s| s.to_string())
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::args_from;

    fn test_cfg() -> Config {
        let mut cfg = Config::default();
        // 每个测试独立审计文件，避免并行测试交错
        let uid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        cfg.audit_log = format!("/tmp/hal-gateway-test-{}-{}/audit.log", uid, nanos);
        cfg
    }

    #[test]
    fn allowed_tool_executes_and_audits() {
        let cfg = test_cfg();
        let out = call(&cfg, "skill:emergency-avoidance", "thruster_emergency_stop", &args_from(&[]));
        assert!(out.allowed);
        assert!(out.result.unwrap().ok);
        let line = last_audit_line(&cfg).unwrap();
        assert!(line.contains("thruster_emergency_stop"));
        assert!(line.contains("\"allowed\":true"));
    }

    #[test]
    fn unknown_tool_denied_and_audited() {
        let cfg = test_cfg();
        let out = call(&cfg, "skill:fault-recovery", "motor_detonate", &args_from(&[]));
        assert!(!out.allowed);
        assert!(out.denied_reason.unwrap().contains("白名单"));
        let line = last_audit_line(&cfg).unwrap();
        assert!(line.contains("\"allowed\":false"));
    }

    #[test]
    fn disallowed_arg_denied() {
        let cfg = test_cfg();
        let out = call(&cfg, "skill:test", "thruster_set", &args_from(&[("power", "90"), ("hidden", "1")]));
        assert!(!out.allowed);
        assert!(out.denied_reason.unwrap().contains("hidden"));
    }

    #[test]
    fn audit_keeps_caller() {
        let cfg = test_cfg();
        call(&cfg, "skill:zone-partition", "sensor_read", &args_from(&[("sensor", "distance")]));
        let line = last_audit_line(&cfg).unwrap();
        assert!(line.contains("skill:zone-partition"));
    }
}
