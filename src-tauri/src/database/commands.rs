use tauri::State;
use crate::database::{
    DualDatabaseManager,
    DatabaseStatus,
    init_public_database,
    init_secure_database,
    unlock_secure_database,
    lock_secure_database,
    is_secure_database_initialized,
};
use crate::wallets_tool::wallet_manager::service::WalletManagerService;

#[tauri::command]
pub async fn get_dual_database_status() -> Result<DatabaseStatus, String> {
    Ok(DualDatabaseManager::get_status())
}

#[tauri::command]
pub async fn init_public_db() -> Result<(), String> {
    init_public_database()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn init_secure_db(
    _state: State<'_, WalletManagerService>,
    encrypted_password: String,
) -> Result<(), String> {
    init_secure_database(&encrypted_password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unlock_secure_db(
    encrypted_password: String,
) -> Result<(), String> {
    unlock_secure_database(&encrypted_password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lock_secure_db() -> Result<(), String> {
    lock_secure_database()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_secure_db_initialized() -> bool {
    is_secure_database_initialized()
}

#[tauri::command]
pub fn is_public_db_ready() -> bool {
    DualDatabaseManager::public_pool_ready()
}

#[tauri::command]
pub fn is_secure_db_unlocked() -> bool {
    DualDatabaseManager::secure_pool_ready()
}

/// 检查钱包管理模块是否就绪（安全数据库已初始化并解锁）
#[tauri::command]
pub async fn is_wallet_manager_ready() -> Result<bool, String> {
    // 首先检查安全数据库文件是否存在
    if !is_secure_database_initialized() {
        return Ok(false);
    }

    // 然后检查是否已解锁
    if !DualDatabaseManager::secure_pool_ready() {
        return Ok(false);
    }

    // 最后验证钱包管理表结构是否就绪
    let pool = match DualDatabaseManager::secure_pool() {
        Ok(pool) => pool,
        Err(_) => return Ok(false),
    };

    let wallets_table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='wallets'"
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("检查数据库状态失败: {e}"))?;

    Ok(wallets_table_exists > 0)
}
