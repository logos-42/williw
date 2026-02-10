//! 文档驱动的AI自主工作流模块
//!
//! 核心理念：把工具交给AI，让AI通过阅读文档自主完成任务
//!
//! 使用方式：
//! 1. 人写文档（身份、任务、工具说明）
//! 2. AI读文档（理解角色和目标）
//! 3. AI用工具（自主执行）
//! 4. 形成闭环（Ralph Loop驱动）

use super::super::AsyncWorkflowExecutor;
use crate::agent::workflow::RalphLoopConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI身份定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// 身份ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 角色描述
    pub role: String,
    /// 专业领域
    pub expertise: Vec<String>,
    /// 工作原则
    pub principles: Vec<String>,
    /// 行为准则
    pub behavior_guidelines: Vec<String>,
    /// 核心工具
    pub core_tools: Vec<String>,
}

impl AgentIdentity {
    /// 创建去中心化算力专家身份
    pub fn decentralized_compute_expert() -> Self {
        Self {
            id: "decentralized_compute_expert".to_string(),
            name: "去中心化算力专家".to_string(),
            role: "专注于去中心化算力网络的模型切分、分发和节点布置".to_string(),
            expertise: vec![
                "模型切分和分片".to_string(),
                "算力节点管理".to_string(),
                "P2P网络协调".to_string(),
                "任务调度分发".to_string(),
                "模型聚合".to_string(),
            ],
            principles: vec![
                "切分粒度适中，平衡计算和通信开销".to_string(),
                "优先选择网络延迟低的节点".to_string(),
                "保持分片一致性".to_string(),
                "失败时自动重试和迁移".to_string(),
            ],
            behavior_guidelines: vec![
                "切分前先分析模型结构和大小".to_string(),
                "布置时考虑节点能力和网络状况".to_string(),
                "记录每个分片的位置和状态".to_string(),
                "定期同步和验证分片完整性".to_string(),
            ],
            core_tools: vec![
                "DecentralizedModel".to_string(),
                "IrohComms".to_string(),
                "Plan".to_string(),
                "FileSystem".to_string(),
                "Search".to_string(),
            ],
        }
    }

    /// 转换为系统prompt
    pub fn to_system_prompt(&self) -> String {
        format!(
            r#"# 身份定义

你是：**{name}**

## 角色
{role}

## 专业领域
{expertise}

## 工作原则
{principles}

## 行为准则
{guidelines}

## 核心工具
{tools}

---
请记住你的身份！在每次决策时，基于你的专业领域和行为准则做出判断。
"#,
            name = self.name,
            role = self.role,
            expertise = self.expertise.iter().enumerate()
                .map(|(i, e)| format!("{}. {}", i + 1, e))
                .collect::<Vec<_>>().join("\n"),
            principles = self.principles.iter().enumerate()
                .map(|(i, p)| format!("{}. {}", i + 1, p))
                .collect::<Vec<_>>().join("\n"),
            guidelines = self.behavior_guidelines.iter().enumerate()
                .map(|(i, g)| format!("{}. {}", i + 1, g))
                .collect::<Vec<_>>().join("\n"),
            tools = self.core_tools.join(", "),
        )
    }
}

/// 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// 任务ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 目标
    pub goal: String,
    /// 验收标准
    pub acceptance_criteria: Vec<String>,
    /// 执行步骤
    pub steps: Vec<TaskStep>,
    /// 输入参数
    pub inputs: HashMap<String, String>,
    /// 约束条件
    pub constraints: TaskConstraints,
}

/// 任务步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: String,
    pub description: String,
    pub tool_hint: Option<String>,
    pub validation: Option<String>,
}

/// 任务约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConstraints {
    pub max_iterations: u32,
    pub timeout_secs: u64,
    pub max_memory_mb: u64,
}

impl Default for TaskConstraints {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            timeout_secs: 3600,
            max_memory_mb: 8192,
        }
    }
}

impl AgentTask {
    /// 创建模型切分任务
    pub fn model_split_task(model_path: &str, target_nodes: &[String]) -> Self {
        Self {
            id: format!("task_split_{}", chrono::Utc::now().timestamp()),
            name: format!("切分模型: {}", model_path),
            description: format!("将模型 {} 切分并分发到 {:?} 个节点", model_path, target_nodes.len()),
            goal: "完成模型切分并确保所有分片正确分发".to_string(),
            acceptance_criteria: vec![
                format!("模型被切分为 {} 个分片", target_nodes.len()),
                "所有分片完整且可验证".to_string(),
                "每个分片已分配到目标节点".to_string(),
                "分片校验和验证通过".to_string(),
            ],
            steps: vec![
                TaskStep {
                    id: "1".to_string(),
                    description: format!("分析模型 {} 的结构和大小", model_path),
                    tool_hint: Some("DecentralizedModel::Analyze".to_string()),
                    validation: Some("获取到模型结构信息".to_string()),
                },
                TaskStep {
                    id: "2".to_string(),
                    description: format!("将模型切分为 {} 个分片", target_nodes.len()),
                    tool_hint: Some("DecentralizedModel::Split".to_string()),
                    validation: Some("生成了预期数量的分片文件".to_string()),
                },
                TaskStep {
                    id: "3".to_string(),
                    description: "将分片分发到目标节点".to_string(),
                    tool_hint: Some("DecentralizedModel::Transfer".to_string()),
                    validation: Some("所有分片传输完成且校验通过".to_string()),
                },
                TaskStep {
                    id: "4".to_string(),
                    description: "验证完整分发结果".to_string(),
                    tool_hint: Some("DecentralizedModel::Verify".to_string()),
                    validation: Some("验证通过".to_string()),
                },
            ],
            inputs: {
                let mut map = HashMap::new();
                map.insert("model_path".to_string(), model_path.to_string());
                map.insert("target_nodes".to_string(), target_nodes.join(","));
                map
            },
            constraints: TaskConstraints::default(),
        }
    }

    /// 转换为任务prompt
    pub fn to_task_prompt(&self) -> String {
        let criteria_str = self.acceptance_criteria.iter().enumerate()
            .map(|(i, c)| format!("{}. {}", i + 1, c))
            .collect::<Vec<_>>().join("\n");

        let steps_str = self.steps.iter()
            .map(|s| format!("- [{}] {} (验证: {})", 
                s.id, 
                s.description,
                s.validation.as_deref().unwrap_or("无")
            ))
            .collect::<Vec<_>>().join("\n");

        format!(
            r#"# 任务：{name}

## 目标
{goal}

## 描述
{description}

## 验收标准（必须全部达成）
{criteria}

## 执行步骤
{steps}

## 输入参数
{inputs}

---
你的任务是自主完成以上步骤，确保达成所有验收标准。
每完成一个步骤，请验证是否满足对应的验证条件。
"#,
            name = self.name,
            goal = self.goal,
            description = self.description,
            criteria = criteria_str,
            steps = steps_str,
            inputs = self.inputs.iter()
                .map(|(k, v)| format!("- {}: {}", k, v))
                .collect::<Vec<_>>().join("\n"),
        )
    }
}

/// 文档阅读器
pub struct DocumentReader;

/// 便利函数：使用内嵌文档运行工作流
impl AsyncWorkflowExecutor {
    /// 使用默认内嵌文档启动工作流
    pub async fn run_with_embedded_docs(
        &self,
        execution_id: String,
        api_key: String,
        ralph_config: Option<RalphLoopConfig>,
    ) -> Result<(), String> {
        let config = DocumentDrivenConfig {
            use_embedded_docs: true,
            ..Default::default()
        };

        self.run_document_driven_workflow(
            execution_id,
            config,
            api_key,
            ralph_config.unwrap_or_default(),
        ).await
    }
}

impl DocumentReader {
    /// 从Markdown读取身份
    pub async fn read_identity(path: &str) -> Result<AgentIdentity, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::parse_identity(&content)
    }

    /// 解析身份文档
    pub fn parse_identity(content: &str) -> Result<AgentIdentity, Box<dyn std::error::Error>> {
        let mut identity = AgentIdentity {
            id: "default".to_string(),
            name: "AI助手".to_string(),
            role: "通用AI助手".to_string(),
            expertise: vec![],
            principles: vec![],
            behavior_guidelines: vec![],
            core_tools: vec![],
        };

        let mut current_section = "";

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("# ") {
                identity.name = line[2..].to_string();
            } else if line.starts_with("## ") {
                current_section = &line[3..];
            } else if line.starts_with("### ") {
                let subsection = &line[4..];
                if subsection == "角色" {
                    current_section = "role";
                }
            } else if line.starts_with("- ") || line.starts_with("* ") {
                let item = line[2..].trim().to_string();
                match current_section {
                    "专业领域" => identity.expertise.push(item),
                    "工作原则" => identity.principles.push(item),
                    "行为准则" => identity.behavior_guidelines.push(item),
                    "核心工具" => identity.core_tools.push(item),
                    _ => {}
                }
            } else if current_section == "role" && !line.is_empty() {
                identity.role = line.to_string();
                current_section = "";
            }
        }

        Ok(identity)
    }

    /// 从Markdown读取任务
    pub async fn read_task(path: &str) -> Result<AgentTask, Box<dyn std::error::Error>> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::parse_task(&content)
    }

    /// 解析任务文档
    pub fn parse_task(content: &str) -> Result<AgentTask, Box<dyn std::error::Error>> {
        let mut task = AgentTask {
            id: format!("task_{}", chrono::Utc::now().timestamp()),
            name: "未命名任务".to_string(),
            description: String::new(),
            goal: String::new(),
            acceptance_criteria: vec![],
            steps: vec![],
            inputs: HashMap::new(),
            constraints: TaskConstraints::default(),
        };

        let mut current_section = "";
        let mut current_step: Option<TaskStep> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("# ") {
                task.name = line[2..].to_string();
            } else if line.starts_with("## ") {
                // 保存当前步骤
                if let Some(step) = current_step.take() {
                    task.steps.push(step);
                }
                current_section = &line[3..];
            } else if line.starts_with("### ") {
                // 子节
            } else if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
                let criterion = line[6..].to_string();
                if current_section == "验收标准" {
                    task.acceptance_criteria.push(criterion);
                }
            } else if line.starts_with("- ") || line.starts_with("* ") {
                let item = line[2..].to_string();
                match current_section {
                    "描述" => task.description = item,
                    "目标" => task.goal = item,
                    "步骤" => {
                        let id = format!("{}", task.steps.len() + 1);
                        current_step = Some(TaskStep {
                            id,
                            description: item,
                            tool_hint: None,
                            validation: None,
                        });
                    }
                    _ => {}
                }
            } else if current_section == "目标" && !line.is_empty() && task.goal.is_empty() {
                task.goal = line.to_string();
            }
        }

        // 保存最后一个步骤
        if let Some(step) = current_step {
            task.steps.push(step);
        }

        Ok(task)
    }
}

/// 文档驱动的Ralph Loop配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDrivenConfig {
    /// 身份文档路径
    pub identity_doc_path: Option<String>,
    /// 任务文档路径
    pub task_doc_path: Option<String>,
    /// 工具文档目录
    pub tools_doc_dir: Option<String>,
    /// 是否使用内嵌文档（优先于路径）
    pub use_embedded_docs: bool,
    /// 是否启用文档阅读
    pub enable_doc_reading: bool,
    /// 每次迭代重新阅读文档
    pub re_read_docs_per_iteration: bool,
}

impl Default for DocumentDrivenConfig {
    fn default() -> Self {
        Self {
            identity_doc_path: None,
            task_doc_path: None,
            tools_doc_dir: None,
            use_embedded_docs: true,  // 默认使用内嵌文档
            enable_doc_reading: true,
            re_read_docs_per_iteration: false,
        }
    }
}

/// 自主执行上下文
#[derive(Debug, Clone)]
pub struct AutonomousContext {
    /// AI身份
    pub identity: AgentIdentity,
    /// 当前任务
    pub task: AgentTask,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 当前步骤
    pub current_step: Option<String>,
}

impl AsyncWorkflowExecutor {
    /// 启动文档驱动的自主工作流
    pub async fn run_document_driven_workflow(
        &self,
        execution_id: String,
        config: DocumentDrivenConfig,
        api_key: String,
        ralph_config: RalphLoopConfig,
    ) -> Result<(), String> {
        println!("📚 [DOC-DRIVEN] 启动文档驱动的自主工作流");

        // 1. 阅读身份文档
        let identity = if config.use_embedded_docs {
            println!("👤 [DOC-DRIVEN] 使用内嵌身份文档");
            match DocumentReader::parse_identity(super::IDENTITY_COMPUTE_EXPERT) {
                Ok(id) => id,
                Err(e) => {
                    println!("⚠️ [DOC-DRIVEN] 解析内嵌身份文档失败: {}", e);
                    AgentIdentity::decentralized_compute_expert()
                }
            }
        } else if let Some(ref path) = config.identity_doc_path {
            println!("👤 [DOC-DRIVEN] 阅读身份文档: {}", path);
            match DocumentReader::read_identity(path).await {
                Ok(id) => id,
                Err(e) => {
                    println!("⚠️ [DOC-DRIVEN] 读取身份文档失败: {}", e);
                    AgentIdentity::decentralized_compute_expert()
                }
            }
        } else {
            println!("👤 [DOC-DRIVEN] 使用默认去中心化算力专家身份");
            AgentIdentity::decentralized_compute_expert()
        };

        println!("🎭 [DOC-DRIVEN] AI身份: {}", identity.name);
        println!("   角色: {}", identity.role);
        println!("   专业: {}", identity.expertise.join(", "));

        // 2. 阅读任务文档
        let task = if config.use_embedded_docs {
            println!("📋 [DOC-DRIVEN] 使用内嵌任务文档");
            match DocumentReader::parse_task(super::TASK_SPLIT_MODEL_EXAMPLE) {
                Ok(t) => t,
                Err(e) => {
                    println!("⚠️ [DOC-DRIVEN] 解析内嵌任务文档失败: {}", e);
                    return Err(format!("无法解析任务文档: {}", e));
                }
            }
        } else if let Some(ref path) = config.task_doc_path {
            println!("📋 [DOC-DRIVEN] 阅读任务文档: {}", path);
            match DocumentReader::read_task(path).await {
                Ok(t) => t,
                Err(e) => {
                    println!("⚠️ [DOC-DRIVEN] 读取任务文档失败: {}", e);
                    return Err(format!("无法读取任务文档: {}", e));
                }
            }
        } else {
            println!("❌ [DOC-DRIVEN] 未提供任务文档");
            return Err("必须提供任务文档".to_string());
        };

        println!("🎯 [DOC-DRIVEN] 任务: {}", task.name);
        println!("   目标: {}", task.goal);
        println!("   步骤数: {}", task.steps.len());

        // 3. 创建自主上下文
        let autonomous_context = AutonomousContext {
            identity: identity.clone(),
            task: task.clone(),
            completed_steps: vec![],
            current_step: None,
        };

        // 4. 将身份和任务信息添加到agent_info
        let agent_info = serde_json::json!({
            "identity": identity,
            "task": task,
            "mode": "document_driven",
            "embedded_docs": config.use_embedded_docs,
        });

        // 5. 构建Ralph Loop工作流
        let workflow = self.build_workflow_from_task(&task)?;

        // 6. 启动Ralph Loop执行
        println!("🚀 [DOC-DRIVEN] 启动Ralph Loop自主执行...");
        self.execute_workflow_with_ralph_loop(
            execution_id,
            workflow,
            api_key,
            Some(agent_info),
            ralph_config,
        ).await
    }

    /// 从任务构建工作流
    fn build_workflow_from_task(&self, task: &AgentTask) -> Result<super::super::Workflow, String> {
        let steps: Vec<super::super::WorkflowStep> = task.steps.iter()
            .map(|s| super::super::WorkflowStep {
                id: s.id.clone(),
                name: s.description.clone(),
                tool: s.tool_hint.clone().unwrap_or_else(|| "auto".to_string()),
                args: serde_json::json!({}),
                depends_on: vec![],
                status: None,
                result: None,
                error: None,
            })
            .collect();

        Ok(super::super::Workflow {
            id: task.id.clone(),
            name: task.name.clone(),
            description: task.description.clone(),
            steps,
            status: "pending".to_string(),
            created_at: chrono::Utc::now().timestamp(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_to_prompt() {
        let identity = AgentIdentity::decentralized_compute_expert();
        let prompt = identity.to_system_prompt();
        assert!(prompt.contains("去中心化算力专家"));
        assert!(prompt.contains("模型切分和分片"));
    }

    #[test]
    fn test_task_to_prompt() {
        let task = AgentTask::model_split_task("/model.bin", &["node1".to_string(), "node2".to_string()]);
        let prompt = task.to_task_prompt();
        assert!(prompt.contains("切分模型"));
        assert!(prompt.contains("验收标准"));
    }

    #[test]
    fn test_parse_identity() {
        let markdown = r#"# 算力专家

## 角色
专业的去中心化算力工程师

## 专业领域
- 模型切分
- 节点管理

## 工作原则
- 安全第一
- 效率优先
"#;

        let identity = DocumentReader::parse_identity(markdown).unwrap();
        assert_eq!(identity.name, "算力专家");
        assert_eq!(identity.expertise.len(), 2);
    }

    #[test]
    fn test_parse_task() {
        let markdown = r#"# 模型切分任务

## 目标
将模型切分到多个节点

## 描述
这是一个测试任务

## 验收标准
- [ ] 标准1
- [ ] 标准2

## 步骤
- 分析模型
- 执行切分
- 验证结果
"#;

        let task = DocumentReader::parse_task(markdown).unwrap();
        assert_eq!(task.name, "模型切分任务");
        assert_eq!(task.acceptance_criteria.len(), 2);
        assert_eq!(task.steps.len(), 3);
    }
}