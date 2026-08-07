//! 太昊 OS 服务层公共库
//!
// 跨模块共享类型 / 日志 / 路径 / 错误约定。
// 跨模块通信约定见各模块文档。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 太昊 OS 运行时路径(契约 AGENTS.md「目录职责」)
pub mod paths {
    use std::path::PathBuf;

    /// 全局配置(只读,出厂时由厂商写入)
    pub fn config_dir() -> PathBuf {
        PathBuf::from("/etc/taihao-os")
    }

    /// 系统级 SKILL
    pub fn skills_system() -> PathBuf {
        PathBuf::from("/usr/share/taihao-os/skills")
    }

    /// 厂商级 SKILL(可写)
    pub fn skills_vendor() -> PathBuf {
        PathBuf::from("/etc/taihao-os/skills")
    }

    /// 运行时持久化
    pub fn state() -> PathBuf {
        PathBuf::from("/var/lib/taihao-os")
    }

    /// 审计 / 日志
    pub fn logs() -> PathBuf {
        PathBuf::from("/var/log/taihao-os")
    }
}

/// 服务层日志初始化
pub fn init_logging(service: &str) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_ansi(false)
        .try_init();
    tracing::info!(service = service, "太昊 OS 服务启动");
}

/// 统一错误类型
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("白名单拒绝: {0}")]
    WhitelistDenied(String),
    #[error("权限越界: {0}")]
    PermissionDenied(String),
    #[error("未实现: {0}")]
    NotImplemented(String),
    #[error("其它: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 服务层通用消息(用于 comm-center / extension-bridge 等的 IPC)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Envelope {
    /// 高层意图(pi-agent → rt-loop)
    Intent {
        id: String,
        skill: String,
        params: serde_json::Value,
        issued_at: chrono::DateTime<chrono::Utc>,
    },
    /// 状态上报(rt-loop → pi-agent / comm-center)
    StateReport {
        source: String,
        ts: chrono::DateTime<chrono::Utc>,
        payload: serde_json::Value,
    },
    /// HAL 调用(pi-agent → hal-gateway)
    HalCall {
        id: String,
        device: String,
        operation: String,
        params: serde_json::Value,
    },
    /// HAL 响应
    HalResult {
        id: String,
        ok: bool,
        payload: serde_json::Value,
        error: Option<String>,
    },
    /// 审计事件
    Audit {
        actor: String,
        action: String,
        target: String,
        decision: AuditDecision,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Allow,
    Deny,
}

/// 让 thiserror 可用(workspace 公共依赖)
use thiserror::Error;