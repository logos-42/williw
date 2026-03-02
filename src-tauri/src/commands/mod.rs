// Tauri command modules
pub mod training_commands;
pub mod model_device_commands;
pub mod api_key_commands;
pub mod node_commands;
pub mod workers_commands;
pub mod gpu_commands;
pub mod workflow_commands;
pub mod external_api_commands;
pub mod ai_decision_commands;
pub mod model_commands;
pub mod agent_commands;
pub mod autonomous_commands;

// New modular agent commands (refactored from agent_commands.rs)
pub mod agent;           // Agent chat, setup, tools
pub mod tools;           // Tool definitions and registry
pub mod task;            // Task execution system
pub mod agent_tools;     // Tool executors
// pub mod agent_orchestration; // Agent workflow orchestration - TODO: fix compilation errors
pub mod global_skills;   // Global skills management

// Re-export all commands for easy access
pub use training_commands::*;
pub use model_device_commands::*;
pub use api_key_commands::*;
pub use node_commands::*;
pub use workers_commands::*;
pub use gpu_commands::*;
pub use workflow_commands::*;
pub use external_api_commands::*;
pub use ai_decision_commands::*;
pub use model_commands::*;
pub use agent_commands::*;
pub use autonomous_commands::*;

// Re-export agent commands from modular components
// (These were moved from agent_commands.rs to agent/ module)
pub use agent::{warmup_local_model, quick_start_local_inference, chat_with_local_endpoint};
