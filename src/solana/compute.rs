//! 节点算力贡献管理模块
//!
//! 本模块提供节点算力贡献的记录和计算功能。

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::device::types::GpuUsageInfo;
use super::types::*;

/// 算力贡献跟踪器
pub struct ComputeTracker {
    /// 节点 ID
    node_id: String,
    /// 当前任务 ID
    current_task_id: Option<String>,
    /// 任务开始时间
    task_start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 任务开始时的资源使用快照
    start_snapshot: Option<ResourceSnapshot>,
    /// 累计统计
    accumulated_stats: ComputeStats,
}

/// 资源使用快照
#[derive(Debug, Clone)]
struct ResourceSnapshot {
    timestamp: i64,
    cpu_usage_percent: f32,
    gpu_usage: Vec<GpuUsageInfo>,
    memory_used_mb: u64,
    network_upload_mb: u64,
    network_download_mb: u64,
}

impl ComputeTracker {
    /// 创建新的算力跟踪器
    pub fn new(node_id: String) -> Self {
        Self {
            node_id: node_id.clone(),  // Clone to avoid move
            current_task_id: None,
            task_start_time: None,
            start_snapshot: None,
            accumulated_stats: ComputeStats {
                node_id: node_id.clone(),
                total_compute_seconds: 0,
                total_samples_processed: 0,
                total_compute_score: 0.0,
                avg_gpu_usage_percent: 0.0,
                avg_cpu_usage_percent: 0.0,
                total_network_mb: 0,
                contribution_count: 0,
            },
        }
    }

    /// 开始一个新的计算任务
    pub fn start_task(&mut self, task_id: String) -> Result<()> {
        if self.current_task_id.is_some() {
            return Err(anyhow!("已有任务正在进行中"));
        }

        self.current_task_id = Some(task_id.clone());
        self.task_start_time = Some(Utc::now());
        self.start_snapshot = Some(self.capture_snapshot());

        log::info!("节点 {} 开始任务: {}", self.node_id, task_id);
        Ok(())
    }

    /// 完成当前计算任务并生成贡献记录
    pub fn complete_task(
        &mut self,
        samples_processed: u64,
        batches_processed: u64,
        network_upload_mb: u64,
        network_download_mb: u64,
    ) -> Result<ComputeContribution> {
        let task_id = self.current_task_id
            .as_ref()
            .ok_or_else(|| anyhow!("没有正在进行的任务"))?
            .clone();

        let start_time = self.task_start_time
            .ok_or_else(|| anyhow!("任务开始时间未记录"))?;

        let end_time = Utc::now();
        let duration_seconds = (end_time - start_time).num_seconds() as u64;

        if duration_seconds == 0 {
            return Err(anyhow!("任务持续时间太短"));
        }

        let end_snapshot = self.capture_snapshot();
        let start_snapshot = self.start_snapshot
            .as_ref()
            .ok_or_else(|| anyhow!("开始快照未记录"))?;

        // 计算平均资源使用率
        let avg_cpu_usage_percent = (start_snapshot.cpu_usage_percent + end_snapshot.cpu_usage_percent) / 2.0;

        let avg_gpu_usage = if end_snapshot.gpu_usage.is_empty() {
            0.0
        } else {
            let total: f32 = end_snapshot.gpu_usage.iter().map(|g| g.usage_percent).sum();
            total / end_snapshot.gpu_usage.len() as f32
        };

        let gpu_memory_used = end_snapshot.gpu_usage
            .first()
            .and_then(|g| g.memory_used_mb)
            .unwrap_or(0);

        let memory_used_mb = end_snapshot.memory_used_mb;
        let network_upload_mb = end_snapshot.network_upload_mb - start_snapshot.network_upload_mb + network_upload_mb;
        let network_download_mb = end_snapshot.network_download_mb - start_snapshot.network_download_mb + network_download_mb;

        // 计算算力评分
        let compute_score = self.calculate_compute_score(
            duration_seconds,
            samples_processed,
            batches_processed,
            avg_gpu_usage,
            avg_cpu_usage_percent,
            network_upload_mb + network_download_mb,
        );

        let contribution = ComputeContribution {
            id: Uuid::new_v4().to_string(),
            node_id: self.node_id.clone(),
            task_id,
            start_timestamp: start_time.timestamp(),
            end_timestamp: end_time.timestamp(),
            duration_seconds,
            avg_gpu_usage_percent: avg_gpu_usage,
            gpu_memory_used_mb: gpu_memory_used,
            avg_cpu_usage_percent,
            memory_used_mb,
            network_upload_mb,
            network_download_mb,
            samples_processed,
            batches_processed,
            compute_score,
        };

        // 更新累计统计
        self.update_accumulated_stats(&contribution);

        // 重置任务状态
        self.current_task_id = None;
        self.task_start_time = None;
        self.start_snapshot = None;

        log::info!("节点 {} 完成任务: {}, 算力评分: {:.2}", self.node_id, contribution.task_id, compute_score);

        Ok(contribution)
    }

    /// 取消当前任务
    pub fn cancel_task(&mut self) -> Result<()> {
        if self.current_task_id.is_none() {
            return Err(anyhow!("没有正在进行的任务"));
        }

        let task_id = self.current_task_id.take().unwrap();
        self.task_start_time = None;
        self.start_snapshot = None;

        log::warn!("节点 {} 取消任务: {}", self.node_id, task_id);
        Ok(())
    }

    /// 获取累计统计
    pub fn get_accumulated_stats(&self) -> &ComputeStats {
        &self.accumulated_stats
    }

    /// 获取当前任务持续时间（秒）
    pub fn get_current_task_duration(&self) -> Option<u64> {
        self.task_start_time.map(|start| {
            (Utc::now() - start).num_seconds().max(0) as u64
        })
    }

    /// 检查是否有任务正在进行
    pub fn is_task_active(&self) -> bool {
        self.current_task_id.is_some()
    }

    /// 获取当前任务 ID
    pub fn get_current_task_id(&self) -> Option<&String> {
        self.current_task_id.as_ref()
    }

    // ============ 私有方法 ============

    /// 捕获当前资源使用快照
    fn capture_snapshot(&self) -> ResourceSnapshot {
        // 这里应该调用实际的系统监控函数
        // 为了示例，我们返回模拟数据
        ResourceSnapshot {
            timestamp: Utc::now().timestamp(),
            cpu_usage_percent: 30.0, // 应该从系统获取
            gpu_usage: vec![], // 应该从 GPU 检测获取
            memory_used_mb: 2048, // 应该从系统获取
            network_upload_mb: 0, // 应该从网络监控获取
            network_download_mb: 0,
        }
    }

    /// 计算算力评分
    fn calculate_compute_score(
        &self,
        duration_seconds: u64,
        samples_processed: u64,
        batches_processed: u64,
        avg_gpu_usage: f32,
        avg_cpu_usage: f32,
        network_mb: u64,
    ) -> f64 {
        // 权重系数
        const TIME_WEIGHT: f64 = 0.3;
        const SAMPLE_WEIGHT: f64 = 0.25;
        const GPU_WEIGHT: f64 = 0.25;
        const CPU_WEIGHT: f64 = 0.1;
        const NETWORK_WEIGHT: f64 = 0.1;

        // 归一化各指标
        let time_score = (duration_seconds as f64).ln_1p() / 10.0; // 对数缩放
        let sample_score = (samples_processed as f64).ln_1p() / 10.0;
        let gpu_score = avg_gpu_usage as f64 / 100.0;
        let cpu_score = avg_cpu_usage as f64 / 100.0;
        let network_score = (network_mb as f64).ln_1p() / 10.0;

        // 加权总分
        TIME_WEIGHT * time_score +
        SAMPLE_WEIGHT * sample_score +
        GPU_WEIGHT * gpu_score +
        CPU_WEIGHT * cpu_score +
        NETWORK_WEIGHT * network_score
    }

    /// 更新累计统计
    fn update_accumulated_stats(&mut self, contribution: &ComputeContribution) {
        let stats = &mut self.accumulated_stats;

        stats.total_compute_seconds += contribution.duration_seconds;
        stats.total_samples_processed += contribution.samples_processed;
        stats.total_compute_score += contribution.compute_score;
        stats.total_network_mb += contribution.network_upload_mb + contribution.network_download_mb;
        stats.contribution_count += 1;

        // 更新平均使用率（使用移动平均）
        let count = stats.contribution_count as f32;
        stats.avg_gpu_usage_percent = (stats.avg_gpu_usage_percent * (count - 1.0) + contribution.avg_gpu_usage_percent) / count;
        stats.avg_cpu_usage_percent = (stats.avg_cpu_usage_percent * (count - 1.0) + contribution.avg_cpu_usage_percent) / count;
    }
}

/// 算力贡献计算器
pub struct ComputeCalculator;

impl ComputeCalculator {
    /// 计算贡献对应的预估收益
    pub fn calculate_reward(
        contribution: &ComputeContribution,
        base_reward_per_compute_lamports: u64,
    ) -> u64 {
        // 基础奖励 + 算力评分加成
        let base_reward = base_reward_per_compute_lamports as f64;
        let score_multiplier = 1.0 + contribution.compute_score;

        // 持续时间奖励（每额外1小时增加5%）
        let hours = contribution.duration_seconds as f64 / 3600.0;
        let duration_multiplier = 1.0 + (hours * 0.05);

        let total_reward = base_reward * score_multiplier * duration_multiplier;
        total_reward as u64
    }

    /// 批量计算多个贡献的总收益
    pub fn calculate_total_reward(
        contributions: &[ComputeContribution],
        base_reward_per_compute_lamports: u64,
    ) -> u64 {
        contributions
            .iter()
            .map(|c| Self::calculate_reward(c, base_reward_per_compute_lamports))
            .sum()
    }

    /// 计算节点的贡献等级
    pub fn calculate_contribution_level(
        total_compute_score: f64,
        contribution_count: u32,
    ) -> ContributionLevel {
        let avg_score = if contribution_count > 0 {
            total_compute_score / contribution_count as f64
        } else {
            0.0
        };

        match (avg_score, contribution_count) {
            (s, c) if s >= 5.0 && c >= 100 => ContributionLevel::Elite,
            (s, c) if s >= 3.0 && c >= 50 => ContributionLevel::High,
            (s, c) if s >= 1.5 && c >= 20 => ContributionLevel::Medium,
            (s, c) if s >= 0.5 && c >= 10 => ContributionLevel::Regular,
            _ => ContributionLevel::Beginner,
        }
    }
}

/// 贡献等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContributionLevel {
    /// 初级
    Beginner,
    /// 常规
    Regular,
    /// 中级
    Medium,
    /// 高级
    High,
    /// 精英
    Elite,
}

impl ContributionLevel {
    /// 获取奖励倍率
    pub fn reward_multiplier(&self) -> f64 {
        match self {
            ContributionLevel::Beginner => 1.0,
            ContributionLevel::Regular => 1.1,
            ContributionLevel::Medium => 1.25,
            ContributionLevel::High => 1.5,
            ContributionLevel::Elite => 2.0,
        }
    }

    /// 获取等级名称
    pub fn name(&self) -> &'static str {
        match self {
            ContributionLevel::Beginner => "初级贡献者",
            ContributionLevel::Regular => "常规贡献者",
            ContributionLevel::Medium => "中级贡献者",
            ContributionLevel::High => "高级贡献者",
            ContributionLevel::Elite => "精英贡献者",
        }
    }
}
