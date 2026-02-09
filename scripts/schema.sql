-- 去中心化训练系统 D1 数据库架构
-- 用于 Cloudflare Workers D1 数据库

-- 节点表
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL,
    public_key TEXT NOT NULL,
    registered_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    capabilities TEXT NOT NULL,
    total_compute_score REAL NOT NULL DEFAULT 0.0,
    total_rewards_lamports INTEGER NOT NULL DEFAULT 0,
    contribution_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status);
CREATE INDEX IF NOT EXISTS idx_nodes_last_active ON nodes(last_active_at);
CREATE INDEX IF NOT EXISTS idx_nodes_registered ON nodes(registered_at);

-- 贡献表
CREATE TABLE IF NOT EXISTS contributions (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    compute_score REAL NOT NULL,
    duration_seconds INTEGER NOT NULL,
    samples_processed INTEGER NOT NULL,
    batches_processed INTEGER NOT NULL,
    gpu_usage_percent REAL,
    memory_used_mb INTEGER,
    network_upload_mb INTEGER,
    network_download_mb INTEGER,
    gpu_memory_used_mb INTEGER,
    cpu_usage_percent REAL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_contributions_node ON contributions(node_id);
CREATE INDEX IF NOT EXISTS idx_contributions_task ON contributions(task_id);
CREATE INDEX IF NOT EXISTS idx_contributions_timestamp ON contributions(timestamp);
CREATE INDEX IF NOT EXISTS idx_contributions_score ON contributions(compute_score);

-- 收益表
CREATE TABLE IF NOT EXISTS rewards (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    amount_lamports INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    distributed_at INTEGER NOT NULL,
    transaction_signature TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_rewards_node ON rewards(node_id);
CREATE INDEX IF NOT EXISTS idx_rewards_status ON rewards(status);
CREATE INDEX IF NOT EXISTS idx_rewards_distributed ON rewards(distributed_at);

-- 任务表
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    params TEXT NOT NULL,
    target_node TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    scheduled_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    error_message TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    FOREIGN KEY (target_node) REFERENCES nodes(id) ON DELETE SET NULL
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_scheduled ON tasks(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_tasks_node ON tasks(target_node);

-- 钱包余额表
CREATE TABLE IF NOT EXISTS wallets (
    node_id TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    sol_balance_lamports INTEGER NOT NULL DEFAULT 0,
    pending_rewards_lamports INTEGER NOT NULL DEFAULT 0,
    total_rewards_distributed_lamports INTEGER NOT NULL DEFAULT 0,
    last_updated_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_wallets_address ON wallets(wallet_address);

-- 统计表
CREATE TABLE IF NOT EXISTS stats (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- 初始化统计数据
INSERT OR IGNORE INTO stats (key, value, updated_at) VALUES
    ('total_nodes', '0', strftime('%s', 'now') * 1000),
    ('total_contributions', '0', strftime('%s', 'now') * 1000),
    ('total_rewards_distributed', '0', strftime('%s', 'now') * 1000),
    ('active_nodes', '0', strftime('%s', 'now') * 1000),
    ('system_start_time', strftime('%s', 'now') * 1000, strftime('%s', 'now') * 1000);

-- 触发器：更新 updated_at
CREATE TRIGGER IF NOT EXISTS update_nodes_timestamp
AFTER UPDATE ON nodes
BEGIN
    UPDATE nodes SET updated_at = strftime('%s', 'now') * 1000 WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_rewards_timestamp
AFTER UPDATE ON rewards
BEGIN
    UPDATE rewards SET updated_at = strftime('%s', 'now') * 1000 WHERE id = NEW.id;
END;
