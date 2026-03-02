use serde::{Deserialize, Serialize};
use crate::commands::tools::definitions::{ToolDefinition, ToolType};

/// Tool registry for managing all available tools
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
    }

    pub fn register_tools(&mut self, tools: Vec<ToolDefinition>) {
        self.tools.extend(tools);
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn get_all_tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    pub fn get_tools_by_type(&self, tool_type: &ToolType) -> Vec<&ToolDefinition> {
        self.tools.iter().filter(|t| &t.tool_type == tool_type).collect()
    }

    pub fn get_tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name.clone()).collect()
    }

    pub fn get_tool_count(&self) -> usize {
        self.tools.len()
    }

    pub fn is_tool_registered(&self, name: &str) -> bool {
        self.get_tool(name).is_some()
    }

    pub fn get_tools_by_category(&self, category: &str) -> Vec<&ToolDefinition> {
        self.tools.iter().filter(|t| t.description.contains(category)).collect()
    }

    /// Get tools as JSON for AI consumption
    pub fn get_tools_as_json(&self) -> serde_json::Value {
        let mut tools_json = Vec::new();
        
        for tool in &self.tools {
            let mut tool_json = serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            });

            // Add parameters
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for param in &tool.parameters {
                let mut param_json = serde_json::Map::new();
                param_json.insert("type".to_string(), serde_json::json!(param.param_type));
                param_json.insert("description".to_string(), serde_json::json!(param.description));
                
                if let Some(default_value) = &param.default_value {
                    param_json.insert("default".to_string(), default_value.clone());
                }

                properties.insert(param.name.clone(), serde_json::Value::Object(param_json));

                if param.required {
                    required.push(serde_json::json!(param.name));
                }
            }

            if !properties.is_empty() {
                if let Some(function_obj) = tool_json.get_mut("function").and_then(|v| v.as_object_mut()) {
                    if let Some(parameters_obj) = function_obj.get_mut("parameters").and_then(|v| v.as_object_mut()) {
                        parameters_obj.insert("properties".to_string(), serde_json::Value::Object(properties));
                        parameters_obj.insert("required".to_string(), serde_json::Value::Array(required));
                    }
                }
            }

            tools_json.push(tool_json);
        }

        serde_json::Value::Array(tools_json)
    }

    /// Get tools grouped by type
    pub fn get_tools_grouped_by_type(&self) -> std::collections::HashMap<String, Vec<&ToolDefinition>> {
        let mut groups = std::collections::HashMap::new();
        
        for tool in &self.tools {
            let type_name = match tool.tool_type {
                ToolType::System => "system",
                ToolType::File => "file",
                ToolType::Network => "network",
                ToolType::Model => "model",
                ToolType::Inference => "inference",
                ToolType::Process => "process",
                ToolType::Utility => "utility",
            };
            
            groups.entry(type_name.to_string()).or_insert_with(Vec::new).push(tool);
        }

        groups
    }

    /// Validate tool definition
    pub fn validate_tool(&self, tool: &ToolDefinition) -> Result<(), String> {
        if tool.name.is_empty() {
            return Err("Tool name cannot be empty".to_string());
        }

        if tool.description.is_empty() {
            return Err("Tool description cannot be empty".to_string());
        }

        // Check for duplicate parameter names
        let mut param_names = std::collections::HashSet::new();
        for param in &tool.parameters {
            if param.name.is_empty() {
                return Err("Parameter name cannot be empty".to_string());
            }
            
            if param_names.contains(&param.name) {
                return Err(format!("Duplicate parameter name: {}", param.name));
            }
            
            param_names.insert(param.name.clone());
        }

        Ok(())
    }

    /// Register a tool with validation
    pub fn register_tool_with_validation(&mut self, tool: ToolDefinition) -> Result<(), String> {
        self.validate_tool(&tool)?;
        self.register_tool(tool);
        Ok(())
    }

    /// Register multiple tools with validation
    pub fn register_tools_with_validation(&mut self, tools: Vec<ToolDefinition>) -> Result<(), String> {
        for tool in &tools {
            self.validate_tool(tool)?;
        }
        self.register_tools(tools);
        Ok(())
    }

    /// Remove a tool by name
    pub fn remove_tool(&mut self, name: &str) -> bool {
        let initial_len = self.tools.len();
        self.tools.retain(|t| t.name != name);
        self.tools.len() < initial_len
    }

    /// Update an existing tool
    pub fn update_tool(&mut self, tool: ToolDefinition) -> Result<(), String> {
        self.validate_tool(&tool)?;
        
        if let Some(existing_tool) = self.tools.iter_mut().find(|t| t.name == tool.name) {
            *existing_tool = tool;
            Ok(())
        } else {
            Err(format!("Tool '{}' not found", tool.name))
        }
    }

    /// Get tool usage statistics
    pub fn get_statistics(&self) -> ToolRegistryStats {
        let mut stats = ToolRegistryStats::default();
        
        for tool in &self.tools {
            stats.total_tools += 1;
            stats.total_parameters += tool.parameters.len();
            
            match tool.tool_type {
                ToolType::System => stats.system_tools += 1,
                ToolType::File => stats.file_tools += 1,
                ToolType::Network => stats.network_tools += 1,
                ToolType::Model => stats.model_tools += 1,
                ToolType::Inference => stats.inference_tools += 1,
                ToolType::Process => stats.process_tools += 1,
                ToolType::Utility => stats.utility_tools += 1,
            }
        }

        stats
    }
}

/// Tool registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistryStats {
    pub total_tools: usize,
    pub total_parameters: usize,
    pub system_tools: usize,
    pub file_tools: usize,
    pub network_tools: usize,
    pub model_tools: usize,
    pub inference_tools: usize,
    pub process_tools: usize,
    pub utility_tools: usize,
}

impl Default for ToolRegistryStats {
    fn default() -> Self {
        Self {
            total_tools: 0,
            total_parameters: 0,
            system_tools: 0,
            file_tools: 0,
            network_tools: 0,
            model_tools: 0,
            inference_tools: 0,
            process_tools: 0,
            utility_tools: 0,
        }
    }
}

/// Tool registry builder for fluent API
pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn register_tool(mut self, tool: ToolDefinition) -> Result<Self, String> {
        self.registry.register_tool_with_validation(tool)?;
        Ok(self)
    }

    pub fn register_tools(mut self, tools: Vec<ToolDefinition>) -> Result<Self, String> {
        self.registry.register_tools_with_validation(tools)?;
        Ok(self)
    }

    pub fn build(self) -> ToolRegistry {
        self.registry
    }
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default tool registry with all tools registered
pub fn create_default_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    
    // Register all tool definitions
    let tools = crate::commands::tools::definitions::get_tool_definitions();
    registry.register_tools(tools);
    
    registry
}

/// Get all tool definitions as JSON for AI consumption (backward compatibility)
pub fn get_tool_definitions_json() -> serde_json::Value {
    crate::commands::tools::definitions::get_tool_definitions_json()
}