-- 安全数据库初始化脚本 (secure.db)
-- 存储钱包、私钥等敏感数据
-- 此数据库使用 SQLCipher 加密 (AES-256, PBKDF2 600000 iterations)
-- 仅在用户设置密码后创建

-- 主密码验证器表
CREATE TABLE IF NOT EXISTS master_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 钱包分组表
CREATE TABLE IF NOT EXISTS wallet_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER,
    name TEXT NOT NULL,
    chain_type TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES wallet_groups(id) ON DELETE CASCADE
);

-- 钱包表（敏感数据）
CREATE TABLE IF NOT EXISTS wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER,
    name TEXT,
    address TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    ecosystem TEXT,
    wallet_type TEXT DEFAULT 'full_wallet' NOT NULL,
    encrypted_private_key TEXT,
    encrypted_mnemonic TEXT,
    mnemonic_index INTEGER,
    remark TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE SET NULL
);

-- 观察钱包地址表
CREATE TABLE IF NOT EXISTS watch_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER,
    name TEXT,
    address TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    ecosystem TEXT,
    remark TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE SET NULL
);

-- 空投任务表
CREATE TABLE IF NOT EXISTS airdrop_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    script_path TEXT,
    script_content TEXT,
    wallet_ids TEXT NOT NULL,
    config_json TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 空投执行历史表
CREATE TABLE IF NOT EXISTS airdrop_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    wallet_id INTEGER NOT NULL,
    wallet_address TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'running', 'success', 'failed', 'skipped')),
    result TEXT,
    error TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES airdrop_tasks(id) ON DELETE CASCADE
);

-- 安全数据库迁移记录表
CREATE TABLE IF NOT EXISTS secure_schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    app_version TEXT NOT NULL,
    applied INTEGER NOT NULL DEFAULT 1,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_wallets_group_id ON wallets(group_id);
CREATE INDEX IF NOT EXISTS idx_wallets_address ON wallets(address);
CREATE INDEX IF NOT EXISTS idx_wallets_ecosystem ON wallets(ecosystem);
CREATE INDEX IF NOT EXISTS idx_wallet_groups_parent ON wallet_groups(parent_id);
CREATE INDEX IF NOT EXISTS idx_watch_addresses_group_id ON watch_addresses(group_id);
CREATE INDEX IF NOT EXISTS idx_airdrop_history_task_id ON airdrop_history(task_id);
CREATE INDEX IF NOT EXISTS idx_airdrop_history_wallet_id ON airdrop_history(wallet_id);
