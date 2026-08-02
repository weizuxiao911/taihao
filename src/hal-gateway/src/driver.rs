//! Mock 驱动注册表 — 硬件调用的执行端
//!
//! 每个驱动类型一个 mock 实现：真机驱动（spidev / i2c / pwm / serial / can / v4l2）
//! 在 HAL 集接入时按同接口替换。mock 只做参数规整与确定性返回，便于测试与仿真。

use std::collections::BTreeMap;

use serde_json::{json, Value};

/// 驱动执行结果
#[derive(Debug, Clone)]
pub struct DriverResult {
    pub ok: bool,
    pub detail: String,
}

/// 按驱动类型执行 mock 调用
pub fn call_mock(driver: &str, tool: &str, args: &BTreeMap<String, String>) -> DriverResult {
    match driver {
        "mock" | "spidev" | "i2c" | "pwm" | "serial" | "can" | "v4l2" => {
            let payload = json!({
                "driver": driver,
                "tool": tool,
                "args": args,
                "ok": true,
            });
            DriverResult { ok: true, detail: payload.to_string() }
        }
        other => DriverResult {
            ok: false,
            detail: format!("未知驱动类型: {}", other),
        },
    }
}

/// 供测试构造参数表
pub fn args_from(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// 白名单外的参数名（内部使用，返回不允许的参数列表）
pub fn disallowed_args(tool_args: &[String], provided: &BTreeMap<String, String>) -> Vec<String> {
    provided
        .keys()
        .filter(|k| !tool_args.contains(k))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_executes() {
        let r = call_mock("mock", "thruster_set", &args_from(&[("power", "80")]));
        assert!(r.ok);
        assert!(r.detail.contains("thruster_set"));
    }

    #[test]
    fn unknown_driver_rejected() {
        let r = call_mock("quantum", "thruster_set", &args_from(&[]));
        assert!(!r.ok);
    }

    #[test]
    fn disallowed_args_detected() {
        let allow = vec!["power".to_string()];
        let provided = args_from(&[("power", "1"), ("hack", "x")]);
        assert_eq!(disallowed_args(&allow, &provided), vec!["hack".to_string()]);
    }
}
