//! Extension 加载 — 扫描厂商扩展目录，汇总工具清单进决策上下文
//!
//! extension.toml 格式：
//! ```toml
//! name = "my-extension"
//! [[tools]]
//! name = "device_status"
//! command = "..."
//! ```
//! 加载器只读元数据（名称 + 工具名），不执行任何命令；
//! 实际工具调用统一走 Extension·MCP 桥（extension-bridge，白名单 + 权限边界）。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 扩展元数据
#[derive(Debug, Clone, Deserialize)]
pub struct ExtensionMeta {
    pub name: String,
    #[serde(default)]
    pub tools: Vec<ExtensionTool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtensionTool {
    pub name: String,
}

/// 扩展加载结果
#[derive(Debug, Clone)]
pub struct ExtensionSet {
    pub extensions: Vec<ExtensionMeta>,
}

impl ExtensionSet {
    /// 扫描目录（目录缺失或解析失败跳过，不中断）
    pub fn load(dirs: &[PathBuf]) -> ExtensionSet {
        let mut extensions = Vec::new();
        for dir in dirs {
            if !dir.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(true, |e| e != "toml") {
                    continue;
                }
                match load_one(&path) {
                    Ok(meta) => {
                        log::info!("Extension 加载: {} ({} 个工具)", meta.name, meta.tools.len());
                        extensions.push(meta);
                    }
                    Err(e) => log::warn!("Extension 跳过 {}: {}", path.display(), e),
                }
            }
        }
        extensions.sort_by(|a, b| a.name.cmp(&b.name));
        ExtensionSet { extensions }
    }

    /// 全部工具名（进 LLM 决策上下文）
    pub fn tool_names(&self) -> Vec<String> {
        self.extensions
            .iter()
            .flat_map(|e| e.tools.iter().map(|t| t.name.clone()))
            .collect()
    }
}

fn load_one(path: &Path) -> Result<ExtensionMeta, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("读取失败: {}", e))?;
    let meta: ExtensionMeta = toml::from_str(&content).map_err(|e| format!("解析失败: {}", e))?;
    if meta.name.is_empty() {
        return Err("name 缺失".into());
    }
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_extension_dir() {
        let dir = std::env::temp_dir().join(format!("ext-load-{}", std::process::id()));
        fs::create_dir_all(&dir).ok();
        fs::write(
            dir.join("my-ext.toml"),
            "name = \"my-ext\"\n[[tools]]\nname = \"device_status\"\n[[tools]]\nname = \"report\"\n",
        )
        .unwrap();
        let set = ExtensionSet::load(&[dir.clone()]);
        assert_eq!(set.extensions.len(), 1);
        assert_eq!(set.extensions[0].name, "my-ext");
        assert_eq!(set.tool_names(), vec!["device_status", "report"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_toml_skipped() {
        let dir = std::env::temp_dir().join(format!("ext-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).ok();
        fs::write(dir.join("bad.toml"), "not toml [[[").unwrap();
        let set = ExtensionSet::load(&[dir.clone()]);
        assert!(set.extensions.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_dir_ok() {
        let set = ExtensionSet::load(&[PathBuf::from("/nonexistent/ext")]);
        assert!(set.extensions.is_empty());
    }
}
