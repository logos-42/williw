//! Skills 加载器
//!
//! 从全局 williw 目录加载 Skills

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Skills 清单索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsManifest {
    pub version: String,
    pub skills: Vec<SkillEntry>,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

/// Skills 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub path: String,
}

/// Skill 定义（从 SKILL.md 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

/// Skills 加载器
pub struct SkillsLoader {
    /// Skills 根目录
    skills_dir: PathBuf,
}

impl SkillsLoader {
    /// 创建新的加载器
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }

    /// 加载清单
    pub async fn load_manifest(&self) -> Result<SkillsManifest, String> {
        let manifest_path = self.skills_dir.join("manifest.json");
        
        if !manifest_path.exists() {
            return Err("Skills manifest not found".to_string());
        }

        let content = fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    /// 加载单个 Skill
    pub async fn load_skill(&self, skill_path: &str) -> Result<SkillDefinition, String> {
        let skill_dir = self.skills_dir.join(skill_path);
        let skill_md = skill_dir.join("SKILL.md");

        if !skill_md.exists() {
            return Err(format!("Skill file not found: {:?}", skill_md));
        }

        let content = fs::read_to_string(&skill_md)
            .await
            .map_err(|e| format!("Failed to read skill: {}", e))?;

        // 简单解析 YAML frontmatter
        self.parse_skill_md(&content)
    }

    /// 解析 SKILL.md 文件
    fn parse_skill_md(&self, content: &str) -> Result<SkillDefinition, String> {
        // 提取 frontmatter
        if !content.starts_with("---") {
            return Err("Invalid SKILL.md format: missing frontmatter".to_string());
        }

        let end_idx = content[3..].find("---")
            .ok_or("Invalid SKILL.md format: missing closing ---")?;
        
        let frontmatter = &content[3..3 + end_idx];

        // 解析 YAML
        let mut name = String::new();
        let mut display_name = String::new();
        let mut description = String::new();
        let mut category = String::new();
        let mut version = String::new();
        let mut author = None;
        let mut tags = vec![];

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "name" => name = value.to_string(),
                    "display_name" => display_name = value.to_string(),
                    "description" => description = value.to_string(),
                    "category" => category = value.to_string(),
                    "version" => version = value.to_string(),
                    "author" => author = Some(value.to_string()),
                    "tags" => {
                        // 解析 tags 数组
                        if value.starts_with('[') && value.ends_with(']') {
                            let tags_str = &value[1..value.len()-1];
                            tags = tags_str.split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(SkillDefinition {
            name,
            display_name,
            description,
            category,
            version,
            author,
            tags,
        })
    }

    /// 获取所有可用 Skills
    pub async fn list_skills(&self) -> Result<Vec<SkillDefinition>, String> {
        let manifest = self.load_manifest().await?;
        
        let mut skills = vec![];
        
        for entry in manifest.skills {
            match self.load_skill(&entry.path).await {
                Ok(skill) => skills.push(skill),
                Err(e) => {
                    log::warn!("Failed to load skill {}: {}", entry.id, e);
                }
            }
        }

        Ok(skills)
    }

    /// 根据类别过滤 Skills
    pub async fn get_skills_by_category(&self, category: &str) -> Result<Vec<SkillDefinition>, String> {
        let all_skills = self.list_skills().await?;
        
        Ok(all_skills
            .into_iter()
            .filter(|s| s.category == category)
            .collect())
    }
}

impl Default for SkillsLoader {
    fn default() -> Self {
        Self::new(PathBuf::from("skills"))
    }
}
