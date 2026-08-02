//! 调度核心 — 双链路选择 + 消息路由 + 故障切换
//!
//! 主链路（远程）健康失败累计 ≥ 阈值 → 切换备用（Mesh）；Mesh 失败回切远程。
//! 路由：消息 topic 按配置表匹配（前缀），动作 = 内置 report / 外部脚本。

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::links::{HealthState, Link};

/// 调度器状态
pub struct Dispatcher {
    pub primary: Box<dyn Link>,
    pub backup: Box<dyn Link>,
    pub primary_health: HealthState,
    pub backup_health: HealthState,
    pub config: Config,
    pub current_primary_is_remote: Arc<AtomicBool>,
}

impl Dispatcher {
    pub fn new(primary: Box<dyn Link>, backup: Box<dyn Link>, config: Config) -> Self {
        let current_primary_is_remote = Arc::new(AtomicBool::new(true));
        // 共享健康状态引用（链路内部也更新，调度器在此读取决策）
        Dispatcher {
            primary,
            backup,
            primary_health: HealthState::new(true),
            backup_health: HealthState::new(true),
            config,
            current_primary_is_remote,
        }
    }

    /// 当前主链路
    pub fn active_link(&self) -> &dyn Link {
        if self.current_primary_is_remote.load(Ordering::Relaxed) {
            self.primary.as_ref()
        } else {
            self.backup.as_ref()
        }
    }

    /// 故障切换决策：返回是否发生切换
    pub fn maybe_failover(&self) -> bool {
        let threshold = self.config.fail_threshold.max(1);
        let remote_failures = self.primary_health.failures();
        let mesh_failures = self.backup_health.failures();
        let remote_is_primary = self.current_primary_is_remote.load(Ordering::Relaxed);

        if remote_is_primary && remote_failures >= threshold {
            self.current_primary_is_remote.store(false, Ordering::Relaxed);
            log::warn!("远程链路失败 {} 次，切换到本地 Mesh", remote_failures);
            return true;
        }
        if !remote_is_primary && mesh_failures >= threshold {
            self.current_primary_is_remote.store(true, Ordering::Relaxed);
            log::warn!("Mesh 链路失败 {} 次，回切远程链路", mesh_failures);
            return true;
        }
        false
    }

    /// 发送（走当前主链路，失败记录）
    pub fn send(&self, topic: &str, payload: &str) -> Result<(), String> {
        let link = self.active_link();
        match link.send(topic, payload) {
            Ok(()) => {
                if link.name() == "remote-sse" {
                    self.primary_health.record_success();
                } else {
                    self.backup_health.record_success();
                }
                Ok(())
            }
            Err(e) => {
                if link.name() == "remote-sse" {
                    self.primary_health.record_failure();
                } else {
                    self.backup_health.record_failure();
                }
                Err(e)
            }
        }
    }

    /// 接收（轮询主链路 + 备用，超时返回 None）
    pub fn recv(&self, timeout: Duration) -> Option<(String, String)> {
        let started = std::time::Instant::now();
        loop {
            for link in [self.primary.as_ref(), self.backup.as_ref()] {
                match link.recv(Duration::from_millis(50)) {
                    Ok(Some(msg)) => {
                        if link.name() == "remote-sse" {
                            self.primary_health.record_success();
                        } else {
                            self.backup_health.record_success();
                        }
                        return Some(msg);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        if link.name() == "remote-sse" {
                            self.primary_health.record_failure();
                        } else {
                            self.backup_health.record_failure();
                        }
                    }
                }
            }
            self.maybe_failover();
            if started.elapsed() >= timeout {
                return None;
            }
        }
    }

    /// 路由处理：按 topic 匹配动作
    pub fn route(&self, topic: &str, payload: &str) -> Result<String, String> {
        let route = match self.config.find_route(topic) {
            Some(r) => r,
            None => return Err(format!("无路由匹配: {}", topic)),
        };
        if route.action == "builtin:report" {
            Ok(format!("report:{}", payload))
        } else if let Some(cmd) = route.action.strip_prefix("script:") {
            let out = Command::new("sh")
                .arg("-c")
                .arg(format!("{} '{}'", cmd, payload.replace('\'', "'\\''")))
                .output()
                .map_err(|e| format!("脚本执行失败: {}", e))?;
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                Err(format!("脚本退出码: {}", out.status))
            }
        } else {
            Err(format!("未知动作: {}", route.action))
        }
    }

    /// 写状态文件（调试 / 验证）
    pub fn write_status(&self, path: &std::path::Path) -> Result<(), String> {
        let status = serde_json::json!({
            "active_primary": if self.current_primary_is_remote.load(Ordering::Relaxed) { "remote" } else { "mesh" },
            "remote_failures": self.primary_health.failures(),
            "mesh_failures": self.backup_health.failures(),
            "fail_threshold": self.config.fail_threshold,
        });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&status).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::{MeshLink, SseHttpLink, spawn_echo_server};

    fn test_config() -> Config {
        let mut cfg = Config::default();
        cfg.fail_threshold = 2;
        cfg
    }

    #[test]
    fn failover_switches_to_mesh_after_threshold() {
        // 远程链路连不存在的端点（必然失败），Mesh 链路可用
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        assert!(d.current_primary_is_remote.load(Ordering::Relaxed));

        // 发送失败 2 次（阈值）
        for _ in 0..2 {
            let _ = d.send("status", "{}");
        }
        assert!(d.maybe_failover());
        assert!(!d.current_primary_is_remote.load(Ordering::Relaxed));
    }

    #[test]
    fn no_failover_below_threshold() {
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        let _ = d.send("status", "{}");
        assert!(!d.maybe_failover());
    }

    #[test]
    fn route_builtin_report() {
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        assert_eq!(d.route("task/1", "{\"a\":1}").unwrap(), "report:{\"a\":1}");
        assert!(d.route("unknown", "{}").is_err());
    }

    #[test]
    fn route_script_action() {
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let mut cfg = test_config();
        cfg.routes = vec![crate::config::RouteDef {
            topic: "echo".into(),
            action: "script:echo".into(),
        }];
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), cfg);
        assert_eq!(d.route("echo/1", "hi").unwrap(), "hi");
    }

    #[test]
    fn recv_picks_any_link_message() {
        let (base, _h) = spawn_echo_server();
        let remote = SseHttpLink::new(base, "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        let msg = d.recv(Duration::from_millis(3000));
        assert!(msg.is_some());
    }

    #[test]
    fn recv_timeout_returns_none() {
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        assert_eq!(d.recv(Duration::from_millis(200)), None);
    }

    #[test]
    fn status_file_written() {
        let remote = SseHttpLink::new("http://127.0.0.1:1".into(), "t/".into());
        let mesh = MeshLink::bind(0, "127.0.0.1:0".parse().unwrap()).unwrap();
        let d = Dispatcher::new(Box::new(remote), Box::new(mesh), test_config());
        let path = std::path::Path::new("/tmp/comm-center-test/status.json");
        d.write_status(path).unwrap();
        assert!(path.exists());
        let raw = std::fs::read_to_string(path).unwrap();
        assert!(raw.contains("active_primary"));
    }
}
