//! Agent Setup 模块
//!
//! AI Agent 设置流程相关辅助函数

use serde_json::Value;
use std::process::Command;

/// 解析 LLM 响应，提取文本和工具调用
pub fn parse_llm_response(resp: &Value, provider: &str) -> (Option<String>, Vec<Value>) {
    let mut text_parts: Vec<String> = vec![];
    let mut tool_calls: Vec<Value> = vec![];

    if provider == "anthropic" {
        if let Some(content) = resp.get("content").and_then(|v| v.as_array()) {
            for block in content {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(text.to_string());
                        }
                    }
                    Some("tool_use") => {
                        tool_calls.push(serde_json::json!({
                            "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            "name": block.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                            "arguments": block.get("input").cloned().unwrap_or(serde_json::json!({}))
                        }));
                    }
                    _ => {}
                }
            }
        }
    } else {
        // OpenAI 格式
        if let Some(choices) = resp.get("choices").and_then(|v| v.as_array()) {
            if let Some(first) = choices.first() {
                let default_msg = serde_json::json!({});
                let msg = first.get("message").unwrap_or(&default_msg);

                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        text_parts.push(content.to_string());
                    }
                }

                if let Some(calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                    for call in calls {
                        let args_str = call
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}");
                        let args: Value = serde_json::from_str(args_str)
                            .unwrap_or(serde_json::json!({}));

                        tool_calls.push(serde_json::json!({
                            "id": call.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            "name": call.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
                            "arguments": args
                        }));
                    }
                }
            }
        }
    }

    let text = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };
    (text, tool_calls)
}

/// 找到 ollama 二进制文件路径
pub fn find_ollama_bin() -> Option<String> {
    let extra_paths = [
        "/Applications/Ollama.app/Contents/Resources/ollama",
        "/usr/local/bin/ollama",
        "/opt/homebrew/bin/ollama",
    ];
    // 先检查 PATH
    let in_path = Command::new("sh")
        .arg("-c")
        .arg("command -v ollama 2>/dev/null")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(p) = in_path {
        return Some(p);
    }
    extra_paths.iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
}

/// 根据本机 RAM 从已有模型中选最合适的
pub fn select_best_model(models: &[String]) -> String {
    // 获取内存
    let ram_gb: u64 = Command::new("sh")
        .arg("-c")
        .arg("sysctl -n hw.memsize 2>/dev/null || free -b 2>/dev/null | awk '/Mem:/{print $2}' || echo 0")
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
        .map(|b| b / (1024 * 1024 * 1024))
        .unwrap_or(8);

    // 优先顺序（越往前越好）
    let preference = if ram_gb >= 16 {
        vec!["qwen2.5:3b", "llama3.2:3b", "qwen2.5:1.5b", "qwen2.5:0.5b"]
    } else {
        vec!["qwen2.5:1.5b", "qwen2.5:0.5b", "qwen2.5:3b", "llama3.2:3b"]
    };

    for pref in preference {
        if let Some(m) = models.iter().find(|m| m.starts_with(pref)) {
            return m.clone();
        }
    }
    // 如果没有匹配偏好，返回第一个
    models.first().cloned().unwrap_or_else(|| "qwen2.5:1.5b".to_string())
}
