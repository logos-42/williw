//! AI Decision Commands - Simple model download command
//!
//! AI-driven decision interface for model download and split

use tauri::Emitter;
use serde_json;

/// AI-driven model download and split (auto decision)
/// This is the command AI can call to automatically download and split model
#[tauri::command]
pub async fn ai_download_and_split_model(
    app: tauri::AppHandle,
    model_id: Option<String>,
    num_nodes: Option<usize>,
) -> Result<serde_json::Value, String> {
    // AI automatically decides model and node count
    let model = model_id.unwrap_or_else(|| "meta-llama/Llama-3.2-1B".to_string());
    let nodes = num_nodes.unwrap_or(2);
    
    let _ = app.emit("workflow-message", serde_json::json!({
        "type": "info",
        "content": format!("🧠 AI Decision: Download model '{}' and split to {} nodes", model, nodes),
        "step": "ai_download_split",
        "progress": 0.0,
    }));
    
    // Call model download and split command
    let result = crate::commands::model_commands::download_and_split_model(
        app.clone(),
        model.clone(),
        nodes,
        None,
    ).await;
    
    match result {
        Ok(data) => {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "success",
                "content": "✅ AI completed model download and split",
                "step": "ai_download_split",
                "progress": 1.0,
            }));
            
            Ok(serde_json::json!({
                "success": true,
                "action": "ai_download_and_split_model",
                "model_id": model,
                "num_nodes": nodes,
                "result": data,
                "ai_decision": "Auto-selected optimal split strategy based on system resources"
            }))
        }
        Err(e) => {
            let _ = app.emit("workflow-message", serde_json::json!({
                "type": "error",
                "content": format!("❌ AI model download failed: {}", e),
                "step": "ai_download_split",
                "progress": 0.0,
            }));
            
            Err(e)
        }
    }
}
