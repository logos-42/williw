//! 中间结果缓存
//!
//! 用于缓存分布式推理过程中的中间结果，支持断点续传和容错

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// 缓存的中间结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    /// 分片 ID
    pub shard_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 输出数据
    pub data: Vec<u8>,
    /// 数据形状
    pub shape: Vec<usize>,
    /// 数据类型
    pub dtype: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间（秒）
    pub ttl_secs: u64,
    /// 访问次数
    pub access_count: u64,
    /// 数据大小（字节）
    pub size_bytes: u64,
}

impl CachedResult {
    /// 创建新的缓存结果
    pub fn new(shard_id: String, task_id: String, data: Vec<u8>) -> Self {
        let size_bytes = data.len() as u64;
        Self {
            shard_id,
            task_id,
            data,
            shape: vec![],
            dtype: "float32".to_string(),
            created_at: Utc::now(),
            ttl_secs: 3600, // 默认 1 小时过期
            access_count: 0,
            size_bytes,
        }
    }
    
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let elapsed = (now - self.created_at).num_seconds() as u64;
        elapsed > self.ttl_secs
    }
    
    /// 访问数据（增加访问计数）
    pub fn access(&mut self) -> &[u8] {
        self.access_count += 1;
        &self.data
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// 总大小（字节）
    pub total_size_bytes: u64,
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 驱逐次数
    pub evictions: u64,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// 中间结果缓存
pub struct IntermediateCache {
    /// 缓存存储（任务 ID -> (分片 ID -> 结果)）
    cache: Arc<RwLock<HashMap<String, HashMap<String, CachedResult>>>>,
    /// 最大大小（字节）
    max_size_bytes: u64,
    /// 默认 TTL（秒）
    default_ttl_secs: u64,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

impl IntermediateCache {
    /// 创建新的缓存
    pub fn new(max_size_mb: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size_bytes: max_size_mb * 1024 * 1024,
            default_ttl_secs: 3600,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    /// 存储结果
    pub async fn put(&self, task_id: &str, shard_id: &str, data: Vec<u8>) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        // 检查是否需要驱逐
        let new_size = stats.total_size_bytes + data.len() as u64;
        if new_size > self.max_size_bytes {
            self.evict_lru(&mut cache, &mut stats, data.len() as u64);
        }
        
        // 创建缓存条目
        let result = CachedResult {
            shard_id: shard_id.to_string(),
            task_id: task_id.to_string(),
            data,
            shape: vec![],
            dtype: "float32".to_string(),
            created_at: Utc::now(),
            ttl_secs: self.default_ttl_secs,
            access_count: 0,
            size_bytes: 0, // 稍后计算
        };
        let size_bytes = result.data.len() as u64;
        
        // 插入缓存
        let task_cache = cache.entry(task_id.to_string()).or_insert_with(HashMap::new);
        task_cache.insert(shard_id.to_string(), result);
        
        // 更新统计
        stats.total_entries = cache.values().map(|m| m.len()).sum();
        stats.total_size_bytes += size_bytes;
    }
    
    /// 获取结果
    pub async fn get(&self, task_id: &str, shard_id: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        if let Some(task_cache) = cache.get_mut(task_id) {
            if let Some(result) = task_cache.get_mut(shard_id) {
                if result.is_expired() {
                    // 过期，移除
                    let size = result.data.len() as u64;
                    task_cache.remove(shard_id);
                    stats.total_size_bytes -= size;
                    stats.total_entries -= 1;
                    stats.misses += 1;
                    return None;
                }
                
                stats.hits += 1;
                return Some(result.data.clone());
            }
        }
        
        stats.misses += 1;
        None
    }
    
    /// 检查是否存在
    pub async fn contains(&self, task_id: &str, shard_id: &str) -> bool {
        let cache = self.cache.read().await;
        if let Some(task_cache) = cache.get(task_id) {
            if let Some(result) = task_cache.get(shard_id) {
                return !result.is_expired();
            }
        }
        false
    }
    
    /// 移除任务的所有缓存
    pub async fn remove_task(&self, task_id: &str) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        if let Some(task_cache) = cache.remove(task_id) {
            let size: u64 = task_cache.values().map(|r| r.data.len() as u64).sum();
            stats.total_size_bytes -= size;
            stats.total_entries -= task_cache.len();
        }
    }
    
    /// 清理过期条目
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        let mut total_removed: u64 = 0;
        let mut size_removed: u64 = 0;
        
        for task_cache in cache.values_mut() {
            let expired: Vec<String> = task_cache
                .iter()
                .filter(|(_, r)| r.is_expired())
                .map(|(k, _)| k.clone())
                .collect();
            
            for key in expired {
                if let Some(result) = task_cache.remove(&key) {
                    size_removed += result.data.len() as u64;
                    total_removed += 1;
                }
            }
        }
        
        stats.total_entries -= total_removed as usize;
        stats.total_size_bytes -= size_removed;
        stats.evictions += total_removed;
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
    
    /// 驱逐最少使用的条目
    fn evict_lru(
        &self,
        cache: &mut HashMap<String, HashMap<String, CachedResult>>,
        stats: &mut CacheStats,
        required_space: u64,
    ) {
        let mut freed_space = 0u64;
        
        // 收集所有条目并按访问次数排序
        let mut entries: Vec<(String, String, u64, u64)> = Vec::new();
        for (task_id, task_cache) in cache.iter() {
            for (shard_id, result) in task_cache.iter() {
                entries.push((
                    task_id.clone(),
                    shard_id.clone(),
                    result.access_count,
                    result.data.len() as u64,
                ));
            }
        }
        
        // 按访问次数升序排序
        entries.sort_by_key(|e| e.2);
        
        // 驱逐直到有足够空间
        for (task_id, shard_id, _, size) in entries {
            if freed_space >= required_space {
                break;
            }
            
            if let Some(task_cache) = cache.get_mut(&task_id) {
                if task_cache.remove(&shard_id).is_some() {
                    freed_space += size;
                    stats.evictions += 1;
                    stats.total_entries -= 1;
                    stats.total_size_bytes -= size;
                }
            }
        }
    }
    
    /// 获取缓存大小
    pub async fn size(&self) -> u64 {
        self.stats.read().await.total_size_bytes
    }
    
    /// 获取条目数
    pub async fn len(&self) -> usize {
        self.stats.read().await.total_entries
    }
    
    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = IntermediateCache::new(100); // 100 MB
        
        cache.put("task_1", "shard_0", vec![1, 2, 3, 4]).await;
        
        let result = cache.get("task_1", "shard_0").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
    }
    
    #[tokio::test]
    async fn test_cache_miss() {
        let cache = IntermediateCache::new(100);
        
        let result = cache.get("task_1", "shard_0").await;
        assert!(result.is_none());
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.misses, 1);
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let cache = IntermediateCache::new(100);
        
        cache.put("task_1", "shard_0", vec![1, 2, 3, 4]).await;
        cache.get("task_1", "shard_0").await;
        cache.get("task_1", "shard_1").await; // miss
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_entries, 1);
    }
    
    #[tokio::test]
    async fn test_cache_remove_task() {
        let cache = IntermediateCache::new(100);
        
        cache.put("task_1", "shard_0", vec![1, 2, 3]).await;
        cache.put("task_1", "shard_1", vec![4, 5, 6]).await;
        
        cache.remove_task("task_1").await;
        
        assert!(!cache.contains("task_1", "shard_0").await);
        assert!(!cache.contains("task_1", "shard_1").await);
    }
}
