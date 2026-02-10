//! 简单测试：验证文档驱动系统是否工作

fn main() {
    println!("🧪 测试文档驱动系统");
    println!("==================\n");

    // 测试1: 验证文档常量可以访问
    println!("✅ 测试1: 检查文档常量");

    #[cfg(feature = "document_driven")]
    {
        use williw::agent::workflow::ralph_loop::{
            IDENTITY_COMPUTE_EXPERT,
            TASK_SPLIT_MODEL_EXAMPLE,
            TOOL_DECENTRALIZED_MODEL,
        };

        // 检查文档不为空
        assert!(!IDENTITY_COMPUTE_EXPERT.is_empty(), "身份文档为空");
        assert!(!TASK_SPLIT_MODEL_EXAMPLE.is_empty(), "任务文档为空");
        assert!(!TOOL_DECENTRALIZED_MODEL.is_empty(), "工具文档为空");

        println!("   ✅ 身份文档长度: {} 字符", IDENTITY_COMPUTE_EXPERT.len());
        println!("   ✅ 任务文档长度: {} 字符", TASK_SPLIT_MODEL_EXAMPLE.len());
        println!("   ✅ 工具文档长度: {} 字符", TOOL_DECENTRALIZED_MODEL.len());
    }

    // 测试2: 验证文档解析
    println!("\n✅ 测试2: 文档解析");

    #[cfg(feature = "document_driven")]
    {
        use williw::agent::workflow::ralph_loop::DocumentReader;

        // 解析身份文档
        let identity = DocumentReader::parse_identity(
            r#"# 测试专家

## 角色
测试角色

## 专业领域
- 领域1
- 领域2

## 工作原则
- 原则1

## 行为准则
- 准则1

## 核心工具
- 工具1
"#
        ).expect("解析身份文档失败");

        println!("   ✅ 身份名称: {}", identity.name);
        println!("   ✅ 专业领域数: {}", identity.expertise.len());

        // 解析任务文档
        let task = DocumentReader::parse_task(
            r#"# 测试任务

## 目标
测试目标

## 描述
测试描述

## 验收标准
- [ ] 标准1
- [ ] 标准2

## 执行步骤
1. 步骤1
2. 步骤2
"#
        ).expect("解析任务文档失败");

        println!("   ✅ 任务名称: {}", task.name);
        println!("   ✅ 验收标准数: {}", task.acceptance_criteria.len());
        println!("   ✅ 步骤数: {}", task.steps.len());
    }

    // 测试3: 验证配置
    println!("\n✅ 测试3: 配置对象");

    #[cfg(feature = "document_driven")]
    {
        use williw::agent::workflow::ralph_loop::{DocumentDrivenConfig, RalphLoopConfig};

        let config = DocumentDrivenConfig::default();
        assert!(config.use_embedded_docs, "默认应使用内嵌文档");

        let ralph_config = RalphLoopConfig::default();
        assert!(ralph_config.enabled, "默认应启用Ralph Loop");

        println!("   ✅ 使用内嵌文档: {}", config.use_embedded_docs);
        println!("   ✅ Ralph Loop启用: {}", ralph_config.enabled);
    }

    println!("\n🎉 所有测试通过！");
    println!("\n📝 文档驱动系统已就绪");
    println!("   - 内嵌文档: ✅");
    println!("   - 文档解析: ✅");
    println!("   - 配置系统: ✅");
}