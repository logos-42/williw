//! Global Skills Manager Module
//!
//! Provides centralized skill management for agent orchestration.
//! This module wraps the task::SkillsLoader functionality.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::task::SkillsLoader;

/// Result of skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Global Skills Manager
///
/// Manages skill loading and execution across the application.
/// Provides a unified interface for agent orchestration.
pub struct GlobalSkillsManager {
    skills_dir: String,
    loaded_skills: Arc<RwLock<Vec<String>>>,
}

impl GlobalSkillsManager {
    /// Create a new GlobalSkillsManager
    pub fn new(skills_dir: String) -> Self {
        Self {
            skills_dir,
            loaded_skills: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Execute a skill by path
    pub async fn execute_skill(
        &self,
        skill_path: &str,
        input: Value,
        _app: &tauri::AppHandle,
    ) -> Result<SkillExecutionResult, String> {
        let start = std::time::Instant::now();

        // Load skill using SkillsLoader
        let loader = SkillsLoader::default();
        
        match loader.load_skill(skill_path).await {
            Ok(skill_def) => {
                // Log skill execution
                log::info!("Executing skill: {} ({})", skill_def.display_name, skill_path);
                
                // For now, return a placeholder result
                // In a full implementation, this would execute the skill
                let result = SkillExecutionResult {
                    success: true,
                    output: serde_json::json!({
                        "skill_name": skill_def.name,
                        "display_name": skill_def.display_name,
                        "description": skill_def.description,
                        "input": input,
                        "message": "Skill loaded successfully (execution not implemented)"
                    }),
                    error: None,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                };

                // Track loaded skill
                {
                    let mut loaded = self.loaded_skills.write().await;
                    if !loaded.contains(&skill_path.to_string()) {
                        loaded.push(skill_path.to_string());
                    }
                }

                Ok(result)
            }
            Err(e) => {
                let result = SkillExecutionResult {
                    success: false,
                    output: Value::Null,
                    error: Some(e),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                };
                Ok(result)
            }
        }
    }

    /// List all available skills
    pub async fn list_skills(&self) -> Result<Vec<String>, String> {
        let loaded = self.loaded_skills.read().await;
        Ok(loaded.clone())
    }

    /// Get the skills directory
    pub fn skills_dir(&self) -> &str {
        &self.skills_dir
    }
}

impl Default for GlobalSkillsManager {
    fn default() -> Self {
        Self::new("skills".to_string())
    }
}

impl Default for SkillExecutionResult {
    fn default() -> Self {
        Self {
            success: false,
            output: Value::Null,
            error: Some("Not initialized".to_string()),
            execution_time_ms: 0,
        }
    }
}
