// Tools module - provides structured access to all AI agent tools
pub mod definitions;
pub mod implementations;
pub mod executor;
pub mod registry;

// Re-export commonly used types and functions
pub use definitions::{ToolDefinition, ToolParameter, ToolType};
pub use implementations::{ToolExecutor, ToolResult};
pub use registry::ToolRegistry;

/// Initialize the default tool registry with all available tools
pub fn create_default_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    
    // Register all tool definitions
    registry.register_tools(definitions::get_tool_definitions());
    
    registry
}

/// Get all tool definitions as JSON for AI consumption
pub fn get_tool_definitions_json() -> serde_json::Value {
    definitions::get_tool_definitions_json()
}