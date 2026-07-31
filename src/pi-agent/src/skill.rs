//! SKILL Registry — 加载 / 校验 / 注册 SKILL.md 包
//!
//! 扫描目录（系统级 + 厂商级），解析 frontmatter，校验通过后注册。
//! 校验失败仅告警跳过，不中断 Agent Loop。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml::Value;

/// 安全分级：L1 最高权限（紧急保护），数值越小越关键
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SecurityLevel {
    L1,
    L2,
    L3,
}

/// SKILL frontmatter schema（与 examples/skills/*.md 一致）
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub version: String,
    pub security_level: SecurityLevel,
    pub description: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub author: String,
}

/// 已注册 SKILL：元信息 + 正文（决策流程 / 触发条件等）
#[derive(Debug, Clone)]
pub struct Skill {
    pub meta: SkillMeta,
    /// 正文（决策流程等），供 Hooks / 调试读取
    #[allow(dead_code)]
    pub body: String,
    #[allow(dead_code)]
    pub path: PathBuf,
}

/// SKILL 加载结果
#[derive(Debug)]
pub struct Registry {
    pub skills: Vec<Skill>,
    pub skipped: Vec<(PathBuf, String)>,
}

impl Registry {
    /// 从目录列表扫描并注册 SKILL（目录不存在时忽略）
    pub fn load(dirs: &[PathBuf]) -> Registry {
        let mut skills = Vec::new();
        let mut skipped = Vec::new();

        for dir in dirs {
            if !dir.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(dir) {
                Ok(e) => e,
                Err(err) => {
                    log::warn!("SKILL 目录不可读 {}: {}", dir.display(), err);
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(true, |e| e != "md") {
                    continue;
                }
                match parse_skill_file(&path) {
                    Ok(skill) => {
                        log::info!("SKILL 注册: {} v{} ({})", skill.meta.name, skill.meta.version, skill.meta.security_level.as_str());
                        skills.push(skill);
                    }
                    Err(err) => {
                        log::warn!("SKILL 跳过 {}: {}", path.display(), err);
                        skipped.push((path, err));
                    }
                }
            }
        }
        skills.sort_by(|a, b| a.meta.name.cmp(&b.meta.name));
        Registry { skills, skipped }
    }

    /// 按名称查找 SKILL
    #[allow(dead_code)]
    pub fn find(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.meta.name == name)
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }
}

impl SecurityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityLevel::L1 => "L1",
            SecurityLevel::L2 => "L2",
            SecurityLevel::L3 => "L3",
        }
    }
}

/// 解析单个 SKILL.md：frontmatter 必须合法，正文为 frontmatter 之后的部分
fn parse_skill_file(path: &Path) -> Result<Skill, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("读取失败: {}", e))?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    // 防注入：frontmatter 只允许已知标量字段
    assert_frontmatter_safe(&frontmatter)?;
    let meta: SkillMeta = serde_yaml::from_str(&frontmatter)
        .map_err(|e| format!("frontmatter 解析失败: {}", e))?;

    validate(&meta)?;

    Ok(Skill {
        meta,
        body: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

/// 拆出 frontmatter（--- 定界）与正文
fn split_frontmatter(content: &str) -> Result<(String, String), String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first() != Some(&"---") {
        return Err("缺少 frontmatter 起始定界符 ---".to_string());
    }
    let end = lines[1..]
        .iter()
        .position(|l| *l == "---")
        .ok_or("缺少 frontmatter 结束定界符 ---")?;
    let frontmatter = lines[1..1 + end].join("\n");
    let body_start = 1 + end + 1;
    let body = lines.get(body_start..).unwrap_or_default().join("\n");
    Ok((frontmatter, body))
}

/// 字段校验
fn validate(meta: &SkillMeta) -> Result<(), String> {
    if meta.name.is_empty() || meta.name.len() > 64 {
        return Err("name 缺失或超长（≤64）".to_string());
    }
    if !meta.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!("name 非法（仅小写字母/数字/连字符）: {}", meta.name));
    }
    if meta.version.is_empty() {
        return Err("version 缺失".to_string());
    }
    if meta.description.is_empty() {
        return Err("description 缺失".to_string());
    }
    Ok(())
}

/// 供前端展示的 SKILL 清单（JSON）
#[allow(dead_code)]
pub fn skills_to_json(registry: &Registry) -> String {
    let list: Vec<serde_json::Value> = registry
        .skills
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.meta.name,
                "version": s.meta.version,
                "security_level": s.meta.security_level.as_str(),
                "description": s.meta.description,
                "author": s.meta.author,
                "path": s.path.display().to_string(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "skills": list })).unwrap_or_default()
}

/// 把 SKILL 描述组装进 LLM 的 system prompt（决策上下文）
pub fn skills_to_context(registry: &Registry) -> String {
    if registry.skills.is_empty() {
        return "当前无可用 SKILL。".to_string();
    }
    let mut out = String::from("可用 SKILL 清单（决策时只能从下列 SKILL 中选用）:\n");
    for s in &registry.skills {
        out.push_str(&format!(
            "- {} (v{}, 安全分级 {})\n  {}\n",
            s.meta.name,
            s.meta.version,
            s.meta.security_level.as_str(),
            s.meta.description
        ));
    }
    out
}

/// 检查 frontmatter 中是否含非法 YAML 类型（防注入，只允许标量/文本块）
pub fn assert_frontmatter_safe(frontmatter: &str) -> Result<(), String> {
    let v: Value = serde_yaml::from_str(frontmatter).map_err(|e| e.to_string())?;
    let map = v.as_mapping().ok_or("frontmatter 必须是映射")?;
    for (key, val) in map {
        let k = key.as_str().unwrap_or("");
        if !matches!(k, "name" | "version" | "security_level" | "description" | "author") {
            return Err(format!("frontmatter 含未知字段: {}", k));
        }
        if !matches!(val, Value::String(_)) {
            return Err(format!("frontmatter 字段 {} 必须是字符串", k));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_skill() {
        let content = "---\nname: emergency-avoidance\nversion: 0.1.0\nsecurity_level: L1\ndescription: 紧急避险\nauthor: taihao-team\n---\n\n# 正文";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert_eq!(body.trim(), "# 正文");
        let meta: SkillMeta = serde_yaml::from_str(&fm).unwrap();
        assert_eq!(meta.name, "emergency-avoidance");
        assert_eq!(meta.security_level, SecurityLevel::L1);
        assert_frontmatter_safe(&fm).unwrap();
    }

    #[test]
    fn parse_invalid_skill_missing_delimiter() {
        assert!(split_frontmatter("no frontmatter").is_err());
        assert!(split_frontmatter("---\nname: x").is_err());
    }

    #[test]
    fn reject_bad_name() {
        let meta = SkillMeta {
            name: "Bad Name!".into(),
            version: "0.1.0".into(),
            security_level: SecurityLevel::L2,
            description: "x".into(),
            author: String::new(),
        };
        assert!(validate(&meta).is_err());
    }

    #[test]
    fn reject_unknown_field() {
        let fm = "name: a\nversion: 0.1.0\nsecurity_level: L1\ndescription: x\ninjected: true";
        assert!(assert_frontmatter_safe(fm).is_err());
    }
}
