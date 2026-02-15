//! distributed inference result aggregator
//!
//! Responsible for aggregating inference results from multiple nodes

use super::protocol::{PartialResult, AggregationMethod, InferenceMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

/// Result aggregator
pub struct ResultAggregator {
    /// Pending aggregation tasks (task_id -> partial results)
    pending: Arc<RwLock<HashMap<String, AggregationTask>>>,
    /// Aggregation method
    default_method: AggregationMethod,
}

/// Aggregation task
#[derive(Debug, Clone)]
pub struct AggregationTask {
    /// Task ID
    pub task_id: String,
    /// Model ID
    pub model_id: String,
    /// Expected number of results
    pub expected_count: usize,
    /// Received partial results
    pub partial_results: Vec<PartialResult>,
    /// Creation time
    pub created_at: i64,
    /// Timeout (milliseconds)
    pub timeout_ms: u64,
}

/// Aggregated result
#[derive(Debug, Clone)]
pub struct AggregatedResult {
    /// Task ID
    pub task_id: String,
    /// Final output
    pub final_output: String,
    /// Partial results from each node
    pub partial_results: Vec<PartialResult>,
    /// Aggregation method used
    pub method: AggregationMethod,
    /// Total execution time (ms)
    pub total_time_ms: u64,
    /// Whether aggregation was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl ResultAggregator {
    /// Create new aggregator
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            default_method: AggregationMethod::Concatenate,
        }
    }
    
    /// Set default aggregation method
    pub fn with_method(mut self, method: AggregationMethod) -> Self {
        self.default_method = method;
        self
    }
    
    /// Create a new aggregation task
    pub async fn create_task(
        &self,
        task_id: String,
        model_id: String,
        expected_count: usize,
        timeout_ms: u64,
    ) {
        let task = AggregationTask {
            task_id: task_id.clone(),
            model_id,
            expected_count,
            partial_results: Vec::new(),
            created_at: Utc::now().timestamp(),
            timeout_ms,
        };
        
        let mut pending = self.pending.write().await;
        pending.insert(task_id.clone(), task);
        
        log::info!("[Aggregator] Created aggregation task {} expecting {} results", 
            task_id, expected_count);
    }
    
    /// Add a partial result
    pub async fn add_partial_result(&self, result: PartialResult) -> Option<AggregatedResult> {
        let task_id_to_update = {
            let pending = self.pending.read().await;
            // Find a task that doesn't already have this node's result
            pending.iter()
                .find(|(_, task)| {
                    task.partial_results.iter().all(|p| p.node_id != result.node_id)
                })
                .map(|(k, _)| k.clone())
        };
        
        if let Some(task_id) = task_id_to_update {
            let mut pending = self.pending.write().await;
            if let Some(task) = pending.get_mut(&task_id) {
                task.partial_results.push(result.clone());
                
                log::info!("[Aggregator] Added partial result from node {} to task {}, {}/{}", 
                    result.node_id, task_id, task.partial_results.len(), task.expected_count);
                
                // Check if we have all results
                if task.partial_results.len() >= task.expected_count {
                    let task = pending.remove(&task_id).unwrap();
                    return Some(self.aggregate(task));
                }
            }
        }
        
        None
    }
    
    /// Add partial result by task_id
    pub async fn add_result(&self, task_id: &str, result: PartialResult) -> Option<AggregatedResult> {
        let mut pending = self.pending.write().await;
        
        if let Some(task) = pending.get_mut(task_id) {
            task.partial_results.push(result.clone());
            
            log::info!("[Aggregator] Added partial result from node {} to task {}, {}/{}", 
                result.node_id, task_id, task.partial_results.len(), task.expected_count);
            
            // Check if we have all results
            if task.partial_results.len() >= task.expected_count {
                let task = pending.remove(task_id).unwrap();
                return Some(self.aggregate(task));
            }
        }
        
        None
    }
    
    /// Aggregate results
    fn aggregate(&self, task: AggregationTask) -> AggregatedResult {
        let start = std::time::Instant::now();
        
        // Sort by shard_id to ensure correct order (layer order)
        let mut sorted_results = task.partial_results.clone();
        sorted_results.sort_by(|a, b| {
            // Extract layer number from shard_id (e.g., "shard_0" -> 0)
            let a_num: usize = a.shard_id.split('_')
                .last()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let b_num: usize = b.shard_id.split('_')
                .last()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            a_num.cmp(&b_num)
        });
        
        // Concatenate outputs (for layer-wise model sharding)
        let final_output = match &self.default_method {
            AggregationMethod::Concatenate => {
                // Simply concatenate all outputs
                sorted_results.iter()
                    .map(|r| r.output_text.as_str())
                    .collect::<Vec<_>>()
                    .join("")
            }
            AggregationMethod::Voting => {
                // Find the most common output
                let mut counts: HashMap<String, usize> = HashMap::new();
                for result in &sorted_results {
                    *counts.entry(result.output_text.clone()).or_insert(0) += 1;
                }
                counts.into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(text, _)| text)
                    .unwrap_or_default()
            }
            AggregationMethod::BestConfidence => {
                // Select the output with highest confidence
                sorted_results.iter()
                    .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
                    .map(|r| r.output_text.clone())
                    .unwrap_or_default()
            }
            AggregationMethod::Average => {
                // For text, this is similar to concatenate
                sorted_results.iter()
                    .map(|r| r.output_text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
            AggregationMethod::WeightedAverage => {
                // Weight by confidence
                let total_confidence: f32 = sorted_results.iter()
                    .map(|r| r.confidence)
                    .sum();
                
                if total_confidence > 0.0 {
                    // For text, we just pick the highest weighted one
                    sorted_results.iter()
                        .max_by(|a, b| {
                            let a_weight = a.confidence / total_confidence;
                            let b_weight = b.confidence / total_confidence;
                            a_weight.partial_cmp(&b_weight).unwrap()
                        })
                        .map(|r| r.output_text.clone())
                        .unwrap_or_default()
                } else {
                    sorted_results.first()
                        .map(|r| r.output_text.clone())
                        .unwrap_or_default()
                }
            }
        };
        
        let total_time_ms = start.elapsed().as_millis() as u64;
        
        log::info!("[Aggregator] Aggregated {} partial results into final output ({} chars)", 
            sorted_results.len(), final_output.len());
        
        AggregatedResult {
            task_id: task.task_id,
            final_output,
            partial_results: sorted_results,
            method: self.default_method.clone(),
            total_time_ms,
            success: true,
            error: None,
        }
    }
    
    /// Check for timed out tasks
    pub async fn check_timeouts(&self) -> Vec<AggregatedResult> {
        let mut pending = self.pending.write().await;
        let now = Utc::now().timestamp();
        
        let mut timed_out = Vec::new();
        let mut to_remove = Vec::new();
        
        for (task_id, task) in pending.iter() {
            let elapsed_ms = (now - task.created_at) as u64 * 1000;
            if elapsed_ms > task.timeout_ms {
                to_remove.push(task_id.clone());
            }
        }
        
        for task_id in to_remove {
            if let Some(task) = pending.remove(&task_id) {
                log::warn!("[Aggregator] Task {} timed out with {}/{} results", 
                    task_id, task.partial_results.len(), task.expected_count);
                
                // Return partial results if we have any
                if !task.partial_results.is_empty() {
                    timed_out.push(AggregatedResult {
                        task_id: task.task_id,
                        final_output: task.partial_results.iter()
                            .map(|r| r.output_text.as_str())
                            .collect::<Vec<_>>()
                            .join(""),
                        partial_results: task.partial_results,
                        method: self.default_method.clone(),
                        total_time_ms: task.timeout_ms,
                        success: false,
                        error: Some("Timeout - incomplete results".to_string()),
                    });
                }
            }
        }
        
        timed_out
    }
    
    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Option<(usize, usize)> {
        let pending = self.pending.read().await;
        pending.get(task_id)
            .map(|task| (task.partial_results.len(), task.expected_count))
    }
}

impl Default for ResultAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_aggregation() {
        let aggregator = ResultAggregator::new()
            .with_method(AggregationMethod::Concatenate);
        
        // Create task expecting 3 results
        aggregator.create_task(
            "task_1".to_string(),
            "model_1".to_string(),
            3,
            60000,
        ).await;
        
        // Add partial results
        let r1 = PartialResult {
            node_id: "node_1".to_string(),
            shard_id: "shard_0".to_string(),
            output_text: "Hello".to_string(),
            confidence: 0.9,
            execution_time_ms: 100,
        };
        
        let r2 = PartialResult {
            node_id: "node_2".to_string(),
            shard_id: "shard_1".to_string(),
            output_text: " World".to_string(),
            confidence: 0.85,
            execution_time_ms: 120,
        };
        
        let r3 = PartialResult {
            node_id: "node_3".to_string(),
            shard_id: "shard_2".to_string(),
            output_text: "!".to_string(),
            confidence: 0.95,
            execution_time_ms: 90,
        };
        
        // Add results
        aggregator.add_result("task_1", r1).await;
        aggregator.add_result("task_1", r2).await;
        let result = aggregator.add_result("task_1", r3).await;
        
        // Should get aggregated result after all 3 are added
        assert!(result.is_some());
        let aggregated = result.unwrap();
        assert_eq!(aggregated.final_output, "Hello World!");
        assert!(aggregated.success);
    }
}