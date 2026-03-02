/// Filesystem tools executor
///
/// Provides file and directory operations including read, write, copy, delete, and listing.

use std::fs;
use std::path::Path;
use serde_json;

/// Write content to a file. Creates parent directories if needed.
pub async fn write_file(path: &str, content: &str) -> serde_json::Value {
    log::info!("[Agent] 写文件：{}", path);

    let path_obj = Path::new(path);

    if let Some(parent) = path_obj.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return serde_json::json!({
                    "success": false,
                    "path": path,
                    "error": format!("创建父目录失败：{}", e)
                });
            }
        }
    }

    match fs::write(path, content) {
        Ok(_) => {
            let bytes_written = content.len();
            serde_json::json!({
                "success": true,
                "path": path,
                "bytes_written": bytes_written,
                "message": format!("成功写入 {} 字节", bytes_written)
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("写入失败：{}", e)
        })
    }
}

/// Read content from a file.
pub async fn read_file(path: &str) -> serde_json::Value {
    log::info!("[Agent] 读文件：{}", path);

    match fs::read_to_string(path) {
        Ok(content) => {
            let size = content.len();
            serde_json::json!({
                "success": true,
                "path": path,
                "content": content,
                "size": size,
                "message": format!("成功读取 {} 字节", size)
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("读取失败：{}", e)
        })
    }
}

/// Check if a file or directory exists.
pub fn file_exists(path: &str) -> serde_json::Value {
    log::info!("[Agent] 检查文件是否存在：{}", path);

    let path_obj = Path::new(path);
    let exists = path_obj.exists();
    let is_file = path_obj.is_file();
    let is_dir = path_obj.is_dir();

    serde_json::json!({
        "success": true,
        "path": path,
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "message": if exists {
            if is_file { "是文件" } else if is_dir { "是目录" } else { "存在" }
        } else { "不存在" }
    })
}

/// List files and directories in a path.
pub async fn list_directory(path: &str, include_hidden: bool) -> serde_json::Value {
    log::info!("[Agent] 列出目录：{}, include_hidden={}", path, include_hidden);

    let mut entries: Vec<serde_json::Value> = vec![];

    match fs::read_dir(path) {
        Ok(dir) => {
            for entry in dir.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                if !include_hidden && name.starts_with('.') {
                    continue;
                }

                let file_type = entry.file_type().ok();
                let is_file = file_type.map(|ft| ft.is_file()).unwrap_or(false);
                let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);

                entries.push(serde_json::json!({
                    "name": name,
                    "is_file": is_file,
                    "is_dir": is_dir
                }));
            }

            entries.sort_by(|a, b| {
                let a_is_dir = a.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                let b_is_dir = b.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                if a_is_dir != b_is_dir {
                    b_is_dir.cmp(&a_is_dir)
                } else {
                    a.get("name").and_then(|v| v.as_str()).unwrap_or("").cmp(
                        b.get("name").and_then(|v| v.as_str()).unwrap_or("")
                    )
                }
            });

            serde_json::json!({
                "success": true,
                "path": path,
                "entries": entries,
                "count": entries.len(),
                "message": format!("列出 {} 个条目", entries.len())
            })
        }
        Err(e) => serde_json::json!({
            "success": false,
            "path": path,
            "error": format!("读取目录失败：{}", e)
        })
    }
}

/// Copy a file or directory from source to destination.
pub async fn copy_file(source: &str, destination: &str) -> serde_json::Value {
    log::info!("[Agent] 复制文件：{} -> {}", source, destination);

    let result = if Path::new(source).is_dir() {
        Command::new("sh")
            .arg("-c")
            .arg(format!("cp -r '{}' '{}'", source, destination))
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("cp '{}' '{}'", source, destination))
            .output()
    };

    match result {
        Ok(o) => {
            if o.status.success() {
                serde_json::json!({
                    "success": true,
                    "source": source,
                    "destination": destination
                })
            } else {
                serde_json::json!({
                    "success": false,
                    "error": String::from_utf8_lossy(&o.stderr).trim()
                })
            }
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("复制失败：{}", e)
        })
    }
}

/// Delete a file or directory.
pub async fn delete_file(path: &str, recursive: bool) -> serde_json::Value {
    log::info!("[Agent] 删除文件：{} (recursive: {})", path, recursive);

    let result = if recursive {
        Command::new("sh")
            .arg("-c")
            .arg(format!("rm -rf '{}'", path))
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("rm '{}'", path))
            .output()
    };

    match result {
        Ok(o) => {
            if o.status.success() {
                serde_json::json!({
                    "success": true,
                    "path": path
                })
            } else {
                serde_json::json!({
                    "success": false,
                    "error": String::from_utf8_lossy(&o.stderr).trim()
                })
            }
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("删除失败：{}", e)
        })
    }
}

/// Create a new directory.
pub async fn create_directory(path: &str, parents: bool) -> serde_json::Value {
    log::info!("[Agent] 创建目录：{} (parents: {})", path, parents);

    let result = if parents {
        Command::new("sh")
            .arg("-c")
            .arg(format!("mkdir -p '{}'", path))
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("mkdir '{}'", path))
            .output()
    };

    match result {
        Ok(o) => {
            if o.status.success() {
                serde_json::json!({
                    "success": true,
                    "path": path
                })
            } else {
                serde_json::json!({
                    "success": false,
                    "error": String::from_utf8_lossy(&o.stderr).trim()
                })
            }
        }
        Err(e) => serde_json::json!({
            "success": false,
            "error": format!("创建目录失败：{}", e)
        })
    }
}

/// Get file or directory information including size, modified time, and permissions.
pub fn get_file_info(path: &str) -> serde_json::Value {
    log::info!("[Agent] 获取文件信息：{}", path);

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return serde_json::json!({
                "success": false,
                "error": format!("获取文件信息失败：{}", e)
            });
        }
    };

    let file_type = if metadata.is_dir() {
        "directory"
    } else if metadata.is_symlink() {
        "symlink"
    } else {
        "file"
    };

    let modified = metadata.modified()
        .ok()
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    serde_json::json!({
        "success": true,
        "path": path,
        "file_type": file_type,
        "size_bytes": metadata.len(),
        "size_human": format_size(metadata.len()),
        "modified": modified,
        "readonly": metadata.permissions().readonly()
    })
}

/// Format file size in human-readable format.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// Required for copy_file and delete_file
use std::process::Command;
