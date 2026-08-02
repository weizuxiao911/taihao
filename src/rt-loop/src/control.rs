//! 模式 → 执行器指令映射
//!
//! 安全语义：紧急模式（emergency_*）在本周期立即生效（写入执行器输出），
//! 非紧急模式按目标平滑映射。映射表是硬编码边界，不随 LLM 输出变化。

use serde_json::{json, Value};

/// 一次决策（与 Pi Agent decision.json 对齐）
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Decision {
    pub timestamp: u64,
    pub mode: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub rationale: String,
}

/// 执行器指令（mock 语义：推进器功率 + 方向，浮力调节量）
#[derive(Debug, Clone, PartialEq)]
pub struct ActuatorCmd {
    pub thruster_power: u8,
    pub thruster_direction: String,
    pub buoyancy_delta: i8,
}

/// 模式 → 执行器指令
pub fn map_mode(mode: &str, target: Option<&str>) -> ActuatorCmd {
    match mode {
        "emergency_stop" => ActuatorCmd { thruster_power: 0, thruster_direction: "halt".into(), buoyancy_delta: 0 },
        "emergency_surface" => ActuatorCmd { thruster_power: 0, thruster_direction: "halt".into(), buoyancy_delta: 60 },
        "emergency_avoidance" => ActuatorCmd { thruster_power: 30, thruster_direction: "veer".into(), buoyancy_delta: 0 },
        "return" => ActuatorCmd { thruster_power: 70, thruster_direction: "reverse".into(), buoyancy_delta: 10 },
        "survey" => ActuatorCmd { thruster_power: 50, thruster_direction: "forward".into(), buoyancy_delta: 0 },
        "idle" | _ => ActuatorCmd { thruster_power: 0, thruster_direction: "idle".into(), buoyancy_delta: 0 },
    }
}

/// 执行器输出文件内容
pub fn actuator_json(cmd: &ActuatorCmd, mode: &str) -> Value {
    json!({
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
        "mode": mode,
        "thruster_power": cmd.thruster_power,
        "thruster_direction": cmd.thruster_direction,
        "buoyancy_delta": cmd.buoyancy_delta,
    })
}

/// 判断决策是否紧急（紧急模式立即生效）
pub fn is_emergency(mode: &str) -> bool {
    mode.starts_with("emergency_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_stop_halts() {
        let cmd = map_mode("emergency_stop", None);
        assert_eq!(cmd.thruster_power, 0);
        assert_eq!(cmd.thruster_direction, "halt");
    }

    #[test]
    fn emergency_surface_buoyancy() {
        let cmd = map_mode("emergency_surface", None);
        assert_eq!(cmd.buoyancy_delta, 60);
    }

    #[test]
    fn unknown_mode_falls_to_idle() {
        let cmd = map_mode("hacked_mode", None);
        assert_eq!(cmd, map_mode("idle", None));
    }

    #[test]
    fn emergency_flag() {
        assert!(is_emergency("emergency_stop"));
        assert!(!is_emergency("survey"));
    }

    #[test]
    fn decision_parse() {
        let raw = r#"{"timestamp":1,"mode":"return","target":"dock","rationale":"低电量返航"}"#;
        let d: Decision = serde_json::from_str(raw).unwrap();
        assert_eq!(d.mode, "return");
        assert_eq!(d.target.as_deref(), Some("dock"));
    }
}
