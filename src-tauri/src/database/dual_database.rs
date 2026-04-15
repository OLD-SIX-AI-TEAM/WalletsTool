//! 双数据库管理器
//! 
//! 实现公开数据库与安全数据库的分离存储机制：
//! - 公开数据库 (public.db): 未加密，存储链配置、RPC、代币等公开信息
//! - 安全数据库 (secure.db): SQLCipher加密，存储钱包、私钥等敏感信息

use anyhow::{Result, anyhow};
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::{OnceLock, RwLock};
use serde::{Deserialize, Serialize};

/// 公开数据库连接池 - 始终可用
static PUBLIC_DB_POOL: OnceLock<RwLock<SqlitePool>> = OnceLock::new();

/// 安全数据库连接池 - 仅初始化后可用
static SECURE_DB_POOL: OnceLock<RwLock<Option<SqlitePool>>> = OnceLock::new();

/// 数据库文件路径
pub const PUBLIC_DB_PATH: &str = "data/public.db";
pub const SECURE_DB_PATH: &str = "data/secure.db";

/// 旧数据库文件路径（用于迁移）
#[allow(dead_code)]
pub const LEGACY_DB_PATH: &str = "data/wallets_tool.db";

/// 安全数据库状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecureDbState {
    /// 未初始化 - 用户尚未设置密码
    NotInitialized,
    /// 已初始化但未解锁 - 等待用户输入密码
    Locked,
    /// 已解锁 - 可正常使用
    Unlocked,
}

impl std::fmt::Display for SecureDbState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureDbState::NotInitialized => write!(f, "not_initialized"),
            SecureDbState::Locked => write!(f, "locked"),
            SecureDbState::Unlocked => write!(f, "unlocked"),
        }
    }
}

/// 数据库状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatus {
    /// 公开数据库是否就绪
    pub public_ready: bool,
    /// 安全数据库状态
    pub secure_state: SecureDbState,
    /// 是否存在旧版数据库需要迁移
    pub needs_migration: bool,
}

/// 双数据库管理器
/// 
/// 提供对公开数据库和安全数据库的统一访问接口
pub struct DualDatabaseManager;

impl DualDatabaseManager {
    // ==================== 公开数据库操作 ====================

    /// 初始化公开数据库连接池
    pub fn init_public_pool(pool: SqlitePool) {
        if PUBLIC_DB_POOL.get().is_none() {
            let _ = PUBLIC_DB_POOL.set(RwLock::new(pool));
        } else {
            // 如果已存在，更新连接池
            if let Some(lock) = PUBLIC_DB_POOL.get() {
                let mut guard = lock.write().unwrap();
                *guard = pool;
            }
        }
    }

    /// 获取公开数据库连接池（始终可用）
    pub fn public_pool() -> SqlitePool {
        PUBLIC_DB_POOL
            .get()
            .expect("Public database not initialized. Call init_public_database() first.")
            .read()
            .unwrap()
            .clone()
    }

    /// 检查公开数据库是否就绪
    pub fn public_pool_ready() -> bool {
        PUBLIC_DB_POOL.get().is_some()
    }

    // ==================== 安全数据库操作 ====================

    /// 初始化安全数据库连接池占位符
    pub fn init_secure_pool_placeholder() {
        if SECURE_DB_POOL.get().is_none() {
            let _ = SECURE_DB_POOL.set(RwLock::new(None));
        }
    }

    /// 更新安全数据库连接池
    pub fn update_secure_pool(pool: Option<SqlitePool>) {
        if let Some(lock) = SECURE_DB_POOL.get() {
            let mut guard = lock.write().unwrap();
            *guard = pool;
        } else {
            let _ = SECURE_DB_POOL.set(RwLock::new(pool));
        }
    }

    /// 获取安全数据库连接池（需要先解锁）
    pub fn secure_pool() -> Result<SqlitePool> {
        let guard = SECURE_DB_POOL
            .get()
            .ok_or_else(|| anyhow!("Secure database manager not initialized"))?
            .read()
            .unwrap();
        
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("安全数据库已锁定，请先解锁"))
    }

    /// 尝试获取安全数据库连接池（不抛出错误）
    pub fn try_secure_pool() -> Option<SqlitePool> {
        SECURE_DB_POOL
            .get()
            .and_then(|lock| lock.read().ok())
            .and_then(|guard| guard.clone())
    }

    /// 检查安全数据库是否已解锁
    pub fn secure_pool_ready() -> bool {
        Self::try_secure_pool().is_some()
    }

    // ==================== 状态检查 ====================

    /// 获取安全数据库状态
    pub fn secure_db_state() -> SecureDbState {
        // 检查安全数据库文件是否存在
        if !Path::new(SECURE_DB_PATH).exists() {
            return SecureDbState::NotInitialized;
        }
        
        // 检查是否已解锁
        if Self::secure_pool_ready() {
            return SecureDbState::Unlocked;
        }
        
        SecureDbState::Locked
    }

    /// 获取完整数据库状态
    pub fn get_status() -> DatabaseStatus {
        DatabaseStatus {
            public_ready: Self::public_pool_ready(),
            secure_state: Self::secure_db_state(),
            needs_migration: false,
        }
    }

    /// 关闭安全数据库连接池（锁定）
    pub async fn close_secure_pool() {
        if let Some(lock) = SECURE_DB_POOL.get() {
            let pool_opt = lock.write().unwrap().take();
            if let Some(pool) = pool_opt {
                // 尝试执行 WAL checkpoint 以确保数据写入主文件
                let _ = sqlx::query("PRAGMA wal_checkpoint(FULL)").execute(&pool).await;
                // 关闭连接池
                pool.close().await;
                // 短暂等待确保所有连接关闭
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }

    /// 关闭公开数据库连接池
    #[allow(dead_code)]
    pub async fn close_public_pool() {
        if let Some(lock) = PUBLIC_DB_POOL.get() {
            // 获取池的克隆
            let pool = {
                let guard = lock.read().unwrap();
                guard.clone()
            };

            // 尝试执行 WAL checkpoint 以确保数据写入主文件
            let _ = sqlx::query("PRAGMA wal_checkpoint(FULL)").execute(&pool).await;

            // 关闭连接池
            pool.close().await;
            // 等待确保所有连接关闭和文件锁释放
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// 关闭所有数据库连接池
    #[allow(dead_code)]
    pub async fn close_all() {
        // 先关闭安全数据库
        Self::close_secure_pool().await;
        // 再关闭公开数据库
        Self::close_public_pool().await;
        // 等待确保文件锁释放
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    /// 强制断开所有数据库连接（用于恢复出厂设置）
    #[allow(dead_code)]
    pub async fn force_disconnect_all() {
        // 关闭两个池
        Self::close_all().await;
        // 额外等待 Windows 文件锁释放
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

// ==================== 兼容性接口 ====================

/// 获取当前活动的数据库连接池（内部使用）
#[allow(dead_code)]
pub fn get_active_database_pool() -> SqlitePool {
    DualDatabaseManager::public_pool()
}

/// 获取钱包数据库连接池（安全数据库）
#[allow(dead_code)]
pub fn get_wallet_database_pool() -> Result<SqlitePool> {
    DualDatabaseManager::secure_pool()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_db_state_display() {
        assert_eq!(SecureDbState::NotInitialized.to_string(), "not_initialized");
        assert_eq!(SecureDbState::Locked.to_string(), "locked");
        assert_eq!(SecureDbState::Unlocked.to_string(), "unlocked");
    }

    #[test]
    fn test_database_status_serialization() {
        let status = DatabaseStatus {
            public_ready: true,
            secure_state: SecureDbState::Locked,
            needs_migration: false,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"public_ready\":true"));
        assert!(json.contains("\"secure_state\":\"locked\""));
    }
}
