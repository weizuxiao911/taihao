//! 太昊 OS Pi Agent — 唯一智能决策层入口
//!
//! 职责：加载 SKILL → Agent Loop → LLM 决策 → 下发目标/模式。
//! 安全：本进程设计为受限 namespace、无 capabilities；不直接控制硬件。

mod agent;
mod config;
mod provider;
mod skill;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use clap::Parser;

/// Pi Agent 命令行入口
#[derive(Parser, Debug)]
#[command(name = "pi-agent", version, about = "太昊 OS 唯一智能决策层")]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "/etc/taihao-os/pi-agent.toml")]
    config: String,

    /// 单次决策后退出（验证用）
    #[arg(long)]
    once: bool,
}

fn main() {
    let cli = Cli::parse();
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .format_timestamp_secs()
    .init();

    let (cfg, warnings) = config::Config::load(&PathBuf::from(&cli.config));
    for w in &warnings {
        log::warn!("{}", w);
    }
    let (agent_cfg, provider_cfg, skill_dirs) = cfg.to_runtime();

    let registry = skill::Registry::load(&skill_dirs);
    log::info!(
        "SKILL Registry: {} 个已注册, {} 个跳过",
        registry.len(),
        registry.skipped.len()
    );
    for (path, reason) in &registry.skipped {
        log::warn!("跳过 {}: {}", path.display(), reason);
    }

    if registry.len() == 0 {
        log::warn!("无可用 SKILL，Agent Loop 将以空技能集运行");
    }

    if cli.once {
        let result = agent::run_once(&registry, &provider_cfg, &agent_cfg);
        match result.decision {
            Some(d) => {
                let json = serde_json::to_string_pretty(&d).unwrap();
                println!("{}", json);
                std::process::exit(0);
            }
            None => {
                eprintln!("单次决策失败: {:?}", result.llm_error);
                std::process::exit(1);
            }
        }
    }

    // 常驻循环；SIGINT/SIGTERM 优雅停止
    let stop = Arc::new(AtomicBool::new(false));
    unsafe {
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
    }
    agent::run_loop(&registry, &provider_cfg, &agent_cfg, stop);
}

extern "C" fn handle_signal(_: libc::c_int) {
    // 简化实现：立即退出以保证 systemd 可管理
    std::process::exit(0);
}
