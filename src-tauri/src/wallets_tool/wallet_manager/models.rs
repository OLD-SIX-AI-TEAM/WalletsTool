use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletGroup {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub chain_type: Option<String>, // 'evm' or 'solana' or NULL for old data (migration) or if flexible
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String, // 'evm' or 'solana'
    pub wallet_type: String, // 'full_wallet' or 'address_only'
    #[serde(skip_serializing)]
    pub encrypted_private_key: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_mnemonic: Option<String>,
    pub mnemonic_index: Option<i64>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    #[serde(default)]
    pub wallet_type: String,
    #[serde(default)]
    pub has_private_key: bool,
    #[serde(default)]
    pub has_mnemonic: bool,
    pub sealed_private_key: Option<String>,
    pub sealed_mnemonic: Option<String>,
    pub mnemonic_index: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletsResult {
    pub total: u32,
    #[serde(default)]
    pub preview: Vec<WalletInfo>,
    pub sealed_mnemonic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSecrets {
    pub id: i64,
    pub name: Option<String>,
    pub address: String,
    pub sealed_private_key: Option<String>,
    pub sealed_mnemonic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppConfig {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub parent_id: Option<i64>,
    pub name: String,
    pub chain_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub group_id: Option<i64>,
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    pub chain_type: String,
    pub sealed_private_key: Option<String>,
    pub sealed_mnemonic: Option<String>,
    pub remark: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub transport_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateWalletsMode {
    MnemonicImport,
    PrivateKeyImport,
    GenerateSameMnemonic,
    GenerateDifferentMnemonic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWalletsRequest {
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub chain_type: String,
    pub mode: CreateWalletsMode,
    pub sealed_mnemonic: Option<String>,
    pub sealed_private_key: Option<String>,
    pub count: u32,
    #[serde(default)]
    pub start_index: Option<u32>,
    #[serde(default)]
    pub word_count: Option<u32>,
    pub remark: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub preview_limit: Option<u32>,
    #[serde(default)]
    pub include_secrets: Option<bool>,
    #[serde(default)]
    pub transport_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWalletRequest {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InitPasswordRequest {
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub encrypted_password_b64: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPasswordRequest {
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub encrypted_password_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletExportData {
    pub id: i64,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub private_key: Option<String>,
    pub mnemonic: Option<String>,
    pub mnemonic_index: Option<i64>,
    pub remark: Option<String>,
    pub group_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExportWalletsRequest {
    pub ids: Vec<i64>,
    pub password: String,
}

// ==================== Watch Address (Read-only Address) Types ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchAddressInfo {
    pub id: i64,
    pub group_id: Option<i64>,
    pub group_name: Option<String>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWatchAddressRequest {
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWatchAddressesRequest {
    pub group_id: Option<i64>,
    pub name_prefix: Option<String>,
    pub chain_type: String,
    pub addresses: Vec<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWatchAddressRequest {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WatchAddressExportData {
    pub id: i64,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub remark: Option<String>,
    pub group_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExportWatchAddressesRequest {
    pub ids: Vec<i64>,
}

// ==================== Encrypted Cloud Backup Types ====================

/// 加密备份元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub wallet_count: u32,
    pub group_count: u32,
    pub watch_address_count: u32,
    pub backup_type: BackupType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "wallets_only")]
    WalletsOnly,
    #[serde(rename = "groups_only")]
    GroupsOnly,
}

/// 加密备份数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBackup {
    /// 备份元数据（明文，用于预览）
    pub metadata: BackupMetadata,
    /// 加密的数据（包含所有钱包、分组、仅地址数据）
    pub encrypted_data: String,
    /// 加密算法版本
    pub encryption_version: String,
    /// 盐值（用于密钥派生）
    pub salt: String,
    /// IV（初始化向量）
    pub iv: String,
    /// 认证标签（AES-GCM）
    pub auth_tag: String,
}

/// 备份数据内容（加密前）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupData {
    pub groups: Vec<WalletGroup>,
    pub wallets: Vec<WalletBackupData>,
    pub watch_addresses: Vec<WatchAddressBackupData>,
    pub app_config: Vec<AppConfig>,
}

/// 钱包备份数据（包含解密的敏感信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackupData {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub wallet_type: String,
    pub private_key: Option<String>,
    pub mnemonic: Option<String>,
    pub mnemonic_index: Option<i64>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 仅地址备份数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchAddressBackupData {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 创建备份请求
#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub backup_password: String,
    pub description: Option<String>,
    pub backup_type: BackupType,
}

impl CreateBackupRequest {
    pub fn validate_password(&self) -> Result<(), String> {
        let password = &self.backup_password;
        
        if password.len() < 12 {
            return Err("密码长度至少需要12位".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err("密码需要包含大写字母".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_lowercase()) {
            return Err("密码需要包含小写字母".to_string());
        }
        
        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err("密码需要包含数字".to_string());
        }
        
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        if !password.chars().any(|c| special_chars.contains(c)) {
            return Err("密码需要包含特殊字符".to_string());
        }
        
        Ok(())
    }
}

/// 恢复备份请求
#[derive(Debug, Deserialize)]
pub struct RestoreBackupRequest {
    pub backup_data: EncryptedBackup,
    pub backup_password: String,
    pub merge_strategy: MergeStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// 完全替换现有数据
    #[serde(rename = "replace_all")]
    ReplaceAll,
    /// 合并，跳过冲突
    #[serde(rename = "skip_existing")]
    SkipExisting,
    /// 合并，覆盖冲突
    #[serde(rename = "overwrite_existing")]
    OverwriteExisting,
}

/// 备份恢复结果
#[derive(Debug, Clone, Serialize)]
pub struct RestoreResult {
    pub success: bool,
    pub groups_restored: u32,
    pub wallets_restored: u32,
    pub watch_addresses_restored: u32,
    pub errors: Vec<String>,
}
