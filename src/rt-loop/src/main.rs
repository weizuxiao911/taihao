//! 实时回路主循环 — 10~100Hz 周期驱动
//!
//! 每周期：读决策文件（模式/目标）→ 映射执行器指令 → 写执行器输出 + 回路状态。
//! 决策文件缺失/非法 → 降级 idle（安全默认）。
//! 紧急模式本周期立即写入执行器输出（不等平滑）。

mod config;
mod control;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rt-loop", version, about = "太昊 OS 高频实时回路 — 10~100Hz 运动控制")]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "/etc/taihao-os/rt-loop.toml")]
    config: String,
    /// 运行指定周期数后退出（验证用）
    #[arg(long)]
    ticks: Option<u64>,
}

/// 回路单周期：读决策 → 映射 → 写输出
fn tick(cfg: &config::Config, paths: &config::Paths) -> String {
    let decision = read_decision(paths);
    let mode = decision
        .as_ref()
        .map(|d| d.mode.clone())
        .filter(|m| cfg.allowed_modes.iter().any(|x| x == m))
        .unwrap_or_else(|| "idle".to_string());
    let cmd = control::map_mode(&mode, decision.as_ref().and_then(|d| d.target.as_deref()));
    let out = control::actuator_json(&cmd, &mode);
    let state = json_state(&mode, decision.as_ref().map(|d| d.timestamp).unwrap_or(0));

    let write = |path: &PathBuf, v: &serde_json::Value| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(v).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    };
    if let Err(e) = write(&paths.actuator, &out) {
        log::error!("执行器输出写入失败: {}", e);
    }
    if let Err(e) = write(&paths.loop_state, &state) {
        log::error!("回路状态写入失败: {}", e);
    }
    mode
}

fn read_decision(paths: &config::Paths) -> Option<control::Decision> {
    let raw = std::fs::read_to_string(&paths.decision).ok()?;
    serde_json::from_str::<control::Decision>(&raw).ok()
}

fn json_state(mode: &str, decision_ts: u64) -> serde_json::Value {
    serde_json::json!({
        "timestamp_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0),
        "mode": mode,
        "decision_timestamp": decision_ts,
        "loop_hz": "10~100",
    })
}

/// 阻塞运行回路（stop 置位后退出）
pub fn run_loop(cfg: &config::Config, stop: Arc<AtomicBool>, max_ticks: Option<u64>) {
    let paths = config::Paths::from(cfg);
    let period = Duration::from_secs_f64(1.0 / cfg.frequency_hz.clamp(10, 100) as f64);
    log::info!("实时回路启动: {}Hz (周期 {:?})", cfg.frequency_hz.clamp(10, 100), period);

    let mut ticks: u64 = 0;
    loop {
        if stop.load(Ordering::Relaxed) {
            log::info!("实时回路收到停止信号");
            break;
        }
        let start = Instant::now();
        let mode = tick(cfg, &paths);
        if ticks % 50 == 0 {
            log::debug!("回路 tick={} mode={}", ticks, mode);
        }
        ticks += 1;
        if let Some(max) = max_ticks {
            if ticks >= max {
                log::info!("达到指定周期数 {}，退出", max);
                break;
            }
        }
        let elapsed = start.elapsed();
        if elapsed < period {
            std::thread::sleep(period - elapsed);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let (cfg, warnings) = config::Config::load(&std::path::Path::new(&cli.config));
    for w in &warnings {
        log::warn!("{}", w);
    }
    run_loop(&cfg, Arc::new(AtomicBool::new(false)), cli.ticks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_cfg(dir: &str) -> config::Config {
        let mut cfg = config::Config::default();
        cfg.decision_file = format!("{}/decision.json", dir);
        cfg.actuator_file = format!("{}/actuator.json", dir);
        cfg.loop_state_file = format!("{}/loop.json", dir);
        cfg.frequency_hz = 100;
        cfg
    }

    #[test]
    fn tick_writes_actuator_idle_when_no_decision() {
        let dir = format!("/tmp/rt-loop-test-{}", std::process::id());
        let cfg = test_cfg(&dir);
        let paths = config::Paths::from(&cfg);
        let mode = tick(&cfg, &paths);
        assert_eq!(mode, "idle");
        let raw = std::fs::read_to_string(&paths.actuator).unwrap();
        assert!(raw.contains("\"thruster_power\": 0"));
    }

    #[test]
    fn tick_follows_emergency_decision() {
        let dir = format!("/tmp/rt-loop-test-em-{}", std::process::id());
        let cfg = test_cfg(&dir);
        let paths = config::Paths::from(&cfg);
        std::fs::create_dir_all(paths.decision.parent().unwrap()).ok();
        std::fs::write(
            &paths.decision,
            r#"{"timestamp":1,"mode":"emergency_stop","target":null,"rationale":"碰撞"}"#,
        )
        .ok();
        let mode = tick(&cfg, &paths);
        assert_eq!(mode, "emergency_stop");
        let raw = std::fs::read_to_string(&paths.actuator).unwrap();
        assert!(raw.contains("\"thruster_direction\": \"halt\""));
    }

    #[test]
    fn invalid_decision_degrades_to_idle() {
        let dir = format!("/tmp/rt-loop-test-bad-{}", std::process::id());
        let cfg = test_cfg(&dir);
        let paths = config::Paths::from(&cfg);
        std::fs::create_dir_all(paths.decision.parent().unwrap()).ok();
        std::fs::write(&paths.decision, "not json").ok();
        let mode = tick(&cfg, &paths);
        assert_eq!(mode, "idle");
    }

    #[test]
    fn run_loop_ticks_bounded() {
        let dir = format!("/tmp/rt-loop-test-run-{}", std::process::id());
        let cfg = test_cfg(&dir);
        run_loop(&cfg, Arc::new(AtomicBool::new(false)), Some(3));
        let paths = config::Paths::from(&cfg);
        assert!(Path::new(&cfg.loop_state_file).exists());
        assert!(paths.actuator.exists());
    }
}
