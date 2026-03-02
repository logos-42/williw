/// Search tools executor
///
/// Provides file searching capabilities by filename or content pattern.

use std::process::Command;
use serde_json;

/// Search for files by name or search within files for content patterns.
pub async fn search_files(
    path: &str,
    pattern: &str,
    search_type: &str,
    max_results: usize,
) -> serde_json::Value {
    log::info!("[Agent] 搜索文件：path={}, pattern={}, type={}", path, pattern, search_type);

    let search_type = if search_type == "content" { "content" } else { "filename" };

    let output = if search_type == "content" {
        Command::new("sh")
            .arg("-c")
            .arg(format!("grep -r -l -- '{}' '{}' 2>/dev/null | head -{}", pattern, path, max_results))
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("find '{}' -name '*{}*' -type f 2>/dev/null | head -{}", path, pattern, max_results))
            .output()
    };

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

            serde_json::json!({
                "success": true,
                "pattern": pattern,
                "search_type": search_type,
                "results": files,
                "count": files.len()
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("搜索失败：{}", e)
        })
    }
}
