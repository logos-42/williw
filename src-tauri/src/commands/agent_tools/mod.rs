/// AI Agent Tools Module
///
/// This module contains tool-related functionality for the AI agent system,
/// including tool definitions, metadata, and execution logic.
///
/// # Submodules
/// - [`definitions`]: Tool definitions, metadata structures, and enums
/// - [`executors`]: Tool execution implementations organized by category
pub mod definitions;
pub mod executors;

// Re-export commonly used items from definitions for convenience
pub use definitions::{
    get_all_tool_metadata, get_tool_definitions, get_tool_metadata_by_name, get_tools_by_category,
    get_tools_by_priority, ToolCategory, ToolMetadata, ToolPriority,
};
