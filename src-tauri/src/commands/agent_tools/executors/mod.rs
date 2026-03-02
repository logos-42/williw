/// Agent tool executors module
///
/// This module contains specialized executors for different categories of tools:
/// - system: System information and checks
/// - shell: Shell command execution
/// - http: HTTP endpoint checks and waiting
/// - filesystem: File and directory operations
/// - network: Network diagnosis and process management
/// - model: AI model downloads and inference server management
/// - search: File searching
/// - plan: Task planning and todo management

pub mod system;
pub mod shell;
pub mod http;
pub mod filesystem;
pub mod network;
pub mod model;
pub mod search;
pub mod plan;
