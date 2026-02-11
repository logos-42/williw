// Tauri command modules
pub mod training_commands;
pub mod model_device_commands;
pub mod api_key_commands;
pub mod node_commands;
pub mod workers_commands;
pub mod gpu_commands;
pub mod workflow_commands;
pub mod external_api_commands;

// Re-export all commands for easy access
pub use training_commands::*;
pub use model_device_commands::*;
pub use api_key_commands::*;
pub use node_commands::*;
pub use workers_commands::*;
pub use gpu_commands::*;
pub use workflow_commands::*;
pub use external_api_commands::*;