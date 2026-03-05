use anyhow::{Result, anyhow};
use sqlx::{SqlitePool, Row};
use chrono::Utc;
use std::sync::Mutex;
use rand::Rng;
use sha2::Sha256;
use hmac::Hmac;
use hmac::Mac;
use pbkdf2::pbkdf2;
use aes::Aes256;
use cbc::{Encryptor, Decryptor};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use alloy_signer_local::{PrivateKeySigner, MnemonicBuilder, coins_bip39::{English, Mnemonic}};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;
use sha2::Sha512;
use super::models::*;
use crate::wallets_tool::security::SecureMemory;
use crate::database::get_database_pool;
use openssl::pkey::Private;
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::symm::{Cipher, Crypter, Mode};
use std::collections::HashMap;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;
type HmacSha512 = Hmac<Sha512>;

pub struct WalletManagerService {
    // Master Data Key stored in SecureMemory for protection
    mdk: Mutex<Option<SecureMemory>>,
    transport_rsa: Mutex<Rsa<Private>>,
    transport_keys: Mutex<HashMap<String, [u8; 32]>>,
}

impl WalletManagerService {
    pub fn new(_pool: SqlitePool) -> Self {
        // pool 参数保留是为了兼容旧 API，但不再存储
        // 所有数据库操作都从全局 get_database_pool() 获取最新的 pool
        let rsa = Rsa::generate(2048).expect("generate rsa key");
        Self {
            mdk: Mutex::new(None),
            transport_rsa: Mutex::new(rsa),
            transport_keys: Mutex::new(HashMap::new()),
        }
    }

    /// 获取当前数据库连接池（动态获取，支持运行时更新）
    fn pool(&self) -> SqlitePool {
        get_database_pool()
    }

    pub fn get_transport_public_key_pem(&self) -> Result<String> {
        let rsa = self.transport_rsa.lock().unwrap();
        let pem = rsa.public_key_to_pem()?;
        Ok(String::from_utf8(pem)?)
    }

    pub fn register_transport_key(&self, encrypted_key_b64: &str) -> Result<String> {
        let encrypted_key = BASE64.decode(encrypted_key_b64.trim())?;
        let rsa = self.transport_rsa.lock().unwrap();
        let mut out = vec![0u8; rsa.size() as usize];
        let len = rsa.private_decrypt(&encrypted_key, &mut out, Padding::PKCS1_OAEP)?;
        out.truncate(len);
        if out.len() != 32 {
            return Err(anyhow!("Invalid transport key length"));
        }
        let key: [u8; 32] = out.as_slice().try_into().map_err(|_| anyhow!("Invalid transport key length"))?;

        let mut token_bytes = [0u8; 16];
        rand_bytes(&mut token_bytes)?;
        let token = hex::encode(token_bytes);
        self.transport_keys.lock().unwrap().insert(token.clone(), key);
        Ok(token)
    }

    pub fn open_rsa_oaep_b64(&self, encrypted_b64: &str) -> Result<String> {
        let encrypted = BASE64.decode(encrypted_b64.trim())?;
        let rsa = self.transport_rsa.lock().unwrap();
        let mut out = vec![0u8; rsa.size() as usize];
        let len = rsa.private_decrypt(&encrypted, &mut out, Padding::PKCS1_OAEP)?;
        out.truncate(len);
        Ok(String::from_utf8(out)?)
    }

    fn seal_secret_for_transport_token(&self, plaintext: &str, token: &str) -> Result<String> {
        let key = *self
            .transport_keys
            .lock()
            .unwrap()
            .get(token)
            .ok_or_else(|| anyhow!("Invalid transport token"))?;

        let mut iv = [0u8; 12];
        rand_bytes(&mut iv)?;

        let cipher = Cipher::aes_256_gcm();
        let mut crypter = Crypter::new(cipher, Mode::Encrypt, &key, Some(&iv))?;
        crypter.pad(false);

        let mut out = vec![0u8; plaintext.as_bytes().len() + cipher.block_size()];
        let mut count = crypter.update(plaintext.as_bytes(), &mut out)?;
        count += crypter.finalize(&mut out[count..])?;
        out.truncate(count);

        let mut tag = [0u8; 16];
        crypter.get_tag(&mut tag)?;
        out.extend_from_slice(&tag);

        Ok(format!("t1:{}:{}:{}", token, hex::encode(iv), BASE64.encode(out)))
    }

    fn open_transport_secret(&self, sealed: &str) -> Result<String> {
        let s = sealed.trim();
        if !s.starts_with("t1:") {
            return Err(anyhow!("Invalid transport sealed format"));
        }
        let parts: Vec<&str> = s[3..].split(':').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid transport sealed format"));
        }
        let token = parts[0];
        let iv = hex::decode(parts[1])?;
        if iv.len() != 12 {
            return Err(anyhow!("Invalid transport iv"));
        }
        let data = BASE64.decode(parts[2])?;
        if data.len() < 16 {
            return Err(anyhow!("Invalid transport cipher"));
        }
        let (ciphertext, tag) = data.split_at(data.len() - 16);

        let key = *self
            .transport_keys
            .lock()
            .unwrap()
            .get(token)
            .ok_or_else(|| anyhow!("Invalid transport token"))?;

        let cipher = Cipher::aes_256_gcm();
        let mut crypter = Crypter::new(cipher, Mode::Decrypt, &key, Some(&iv))?;
        crypter.pad(false);
        crypter.set_tag(tag)?;

        let mut out = vec![0u8; ciphertext.len() + cipher.block_size()];
        let mut count = crypter.update(ciphertext, &mut out)?;
        count += crypter.finalize(&mut out[count..])?;
        out.truncate(count);

        Ok(String::from_utf8(out)?)
    }

    /// Initialize tables if they don't exist
    pub async fn init_tables(&self) -> Result<()> {
        // App Config table (for master_verifier, master_key, etc.)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&self.pool()).await?;

        // Wallet Groups
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wallet_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id INTEGER,
                name TEXT NOT NULL,
                chain_type TEXT, -- Added for ecosystem isolation
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_id) REFERENCES wallet_groups(id) ON DELETE CASCADE
            )
            "#
        ).execute(&self.pool()).await?;

        // Migration: Check if chain_type column exists, if not add it
        // This is a simple migration check. For production, better use sqlx migrations.
        // But for this project scale, this is fine.
        let check_col = sqlx::query("SELECT chain_type FROM wallet_groups LIMIT 1")
            .fetch_optional(&self.pool())
            .await;
        
        if check_col.is_err() {
            // Column likely doesn't exist
            let _ = sqlx::query("ALTER TABLE wallet_groups ADD COLUMN chain_type TEXT").execute(&self.pool()).await;
        }

        // Wallets
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wallets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id INTEGER,
                name TEXT,
                address TEXT NOT NULL,
                chain_type TEXT NOT NULL, -- 'evm' or 'solana'
                encrypted_private_key TEXT,
                encrypted_mnemonic TEXT,
                mnemonic_index INTEGER,
                remark TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE SET NULL
            )
            "#
        ).execute(&self.pool()).await?;

        // 创建唯一索引防止重复地址（数据完整性保护）
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_wallets_chain_address ON wallets(chain_type, address)"
        ).execute(&self.pool()).await?;

        // Add remark column if not exists (migration)
        let check_col = sqlx::query("SELECT remark FROM wallets LIMIT 1")
            .fetch_optional(&self.pool())
            .await;
        if check_col.is_err() {
            let _ = sqlx::query("ALTER TABLE wallets ADD COLUMN remark TEXT")
                .execute(&self.pool())
                .await;
        }

        let check_col = sqlx::query("SELECT mnemonic_index FROM wallets LIMIT 1")
            .fetch_optional(&self.pool())
            .await;
        if check_col.is_err() {
            let _ = sqlx::query("ALTER TABLE wallets ADD COLUMN mnemonic_index INTEGER")
                .execute(&self.pool())
                .await;
        }

        // Migration: Add wallet_type column if not exists
        let check_col = sqlx::query("SELECT wallet_type FROM wallets LIMIT 1")
            .fetch_optional(&self.pool())
            .await;
        if check_col.is_err() {
            let _ = sqlx::query("ALTER TABLE wallets ADD COLUMN wallet_type TEXT DEFAULT 'full_wallet' NOT NULL")
                .execute(&self.pool())
                .await;
            // Update existing wallets to have wallet_type = 'full_wallet'
            let _ = sqlx::query("UPDATE wallets SET wallet_type = 'full_wallet' WHERE wallet_type IS NULL OR wallet_type = ''")
                .execute(&self.pool())
                .await;
        }

        // App Config (for password hash/salt and encrypted MDK)
        // First check if table exists
        let table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_config'"
        ).fetch_one(&self.pool()).await?;

        if table_exists == 0 {
            sqlx::query(
                "CREATE TABLE app_config (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL)"
            ).execute(&self.pool()).await?;
        } else {
            // Check if key column has PRIMARY KEY using pragma_table_info
            let pk_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pragma_table_info('app_config') WHERE name='key' AND pk=1"
            ).fetch_one(&self.pool()).await?;

            if pk_count == 0 {
                // Migrate: recreate table with PRIMARY KEY
                sqlx::query("ALTER TABLE app_config RENAME TO app_config_old")
                    .execute(&self.pool()).await?;

                sqlx::query("CREATE TABLE app_config (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL)")
                    .execute(&self.pool()).await?;

                sqlx::query("INSERT OR IGNORE INTO app_config (key, value) SELECT key, value FROM app_config_old")
                    .execute(&self.pool()).await?;

                sqlx::query("DROP TABLE IF EXISTS app_config_old")
                    .execute(&self.pool()).await?;
            }
        }

        Ok(())
    }

    /// Check if master password is set
    pub async fn is_password_set(&self) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM app_config WHERE key = 'master_verifier'"
        )
        .fetch_one(&self.pool())
        .await?;
        Ok(count > 0)
    }

    /// Initialize master password
    pub async fn init_password(&self, password: &str) -> Result<()> {
        self.init_tables().await?;
        
        if self.is_password_set().await? {
            return Err(anyhow::anyhow!("密码已设置"));
        }

        // 1. Generate Master Data Key (MDK)
        let mut mdk = [0u8; 32];
        rand::thread_rng().fill(&mut mdk);

        // 2. Generate Verifier (for password check)
        let mut verifier_salt = [0u8; 16];
        rand::thread_rng().fill(&mut verifier_salt);
        let verifier_hash = self.derive_key(password, &verifier_salt); // 32 bytes

        // Store verifier: salt(hex) + hash(hex)
        let verifier_str = format!("{}:{}", hex::encode(verifier_salt), hex::encode(verifier_hash));
        self.set_config("master_verifier", &verifier_str).await?;

        // 3. Encrypt MDK with Password-Derived Key (KEK)
        let mut kek_salt = [0u8; 16];
        rand::thread_rng().fill(&mut kek_salt);
        let kek = self.derive_key(password, &kek_salt);

        let (encrypted_mdk, iv) = self.encrypt_data(&mdk, &kek)?;
        
        // Store MDK: salt(hex) + iv(hex) + ciphertext(base64)
        let mdk_str = format!("{}:{}:{}", hex::encode(kek_salt), hex::encode(iv), encrypted_mdk);
        self.set_config("master_key", &mdk_str).await?;

        // Cache MDK in SecureMemory for protection
        let mdk_hex = hex::encode(mdk);
        *self.mdk.lock().unwrap() = Some(SecureMemory::new(mdk_hex));

        Ok(())
    }

    /// Verify password and unlock MDK
    pub async fn unlock(&self, password: &str) -> Result<bool> {
        // 1. Check Verifier
        let verifier_str = self.get_config("master_verifier").await?
            .ok_or_else(|| anyhow::anyhow!("尚未初始化密码"))?;
        
        let parts: Vec<&str> = verifier_str.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Verifier format error"));
        }
        let salt = hex::decode(parts[0])?;
        let stored_hash = hex::decode(parts[1])?;

        let derived_hash = self.derive_key(password, &salt);
        if derived_hash.as_slice() != stored_hash.as_slice() {
            return Ok(false);
        }

        // 2. Decrypt MDK
        let mdk_str = self.get_config("master_key").await?
            .ok_or_else(|| anyhow::anyhow!("Master Key missing"))?;
        
        let parts: Vec<&str> = mdk_str.split(':').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("Master Key format error"));
        }
        let kek_salt = hex::decode(parts[0])?;
        let kek = self.derive_key(password, &kek_salt);
        
        let iv = hex::decode(parts[1])?;
        let encrypted_mdk = parts[2]; // base64

        let mdk_bytes_vec = self.decrypt_data(encrypted_mdk, &kek, &iv)?;
        if mdk_bytes_vec.len() != 32 {
            return Err(anyhow::anyhow!("Invalid MDK length"));
        }

        let mdk_bytes: [u8; 32] = mdk_bytes_vec
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid MDK length"))?;
        let mdk_hex = hex::encode(mdk_bytes);
        *self.mdk.lock().unwrap() = Some(SecureMemory::new(mdk_hex));
        self.migrate_plaintext_wallet_secrets(&mdk_bytes).await?;

        Ok(true)
    }

    /// Change master password (re-encrypt MDK and all wallet data)
    pub async fn change_password(&self, old_password: &str, new_password: &str) -> Result<()> {
        if !self.unlock(old_password).await? {
            return Err(anyhow::anyhow!("旧密码错误"));
        }

        // Extract MDK bytes from SecureMemory for encryption
        let mdk_bytes: [u8; 32] = {
            let mdk_guard = self.mdk.lock().unwrap();
            let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow::anyhow!("MDK not unlocked"))?;
            let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
            let mdk_vec = hex::decode(mdk_hex)?;
            let bytes: [u8; 32] = mdk_vec
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid MDK length"))?;
            bytes
        };

        // 1. Re-encrypt all wallet secrets in database
        self.reencrypt_wallet_secrets(old_password, new_password).await?;

        // 2. New Verifier
        let mut verifier_salt = [0u8; 16];
        rand::thread_rng().fill(&mut verifier_salt);
        let verifier_hash = self.derive_key(new_password, &verifier_salt);
        let verifier_str = format!("{}:{}", hex::encode(verifier_salt), hex::encode(verifier_hash));
        self.set_config("master_verifier", &verifier_str).await?;

        // 3. Re-encrypt MDK and update SecureMemory
        let mut kek_salt = [0u8; 16];
        rand::thread_rng().fill(&mut kek_salt);
        let kek = self.derive_key(new_password, &kek_salt);
        let (encrypted_mdk, iv) = self.encrypt_data(&mdk_bytes, &kek)?;
        let mdk_str = format!("{}:{}:{}", hex::encode(kek_salt), hex::encode(iv), encrypted_mdk);
        self.set_config("master_key", &mdk_str).await?;

        *self.mdk.lock().unwrap() = Some(SecureMemory::new(hex::encode(mdk_bytes)));

        Ok(())
    }

    /// Re-encrypt all wallet secrets with new password
    async fn reencrypt_wallet_secrets(&self, old_password: &str, new_password: &str) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        let rows = sqlx::query("SELECT id, encrypted_private_key, encrypted_mnemonic FROM wallets")
            .fetch_all(&mut *tx)
            .await?;

        for row in rows {
            let id: i64 = row.get("id");
            let pk: Option<String> = row.get("encrypted_private_key");
            let mn: Option<String> = row.get("encrypted_mnemonic");

            // Decrypt with old password and re-encrypt with new password
            if let Some(sealed_pk) = pk.as_ref().filter(|s| !s.trim().is_empty()) {
                if sealed_pk.starts_with("p1:") {
                    // Use old password to decrypt
                    let decrypted_pk = self.open_sealed_secret(sealed_pk, old_password)?;
                    // Use new password to re-encrypt
                    let new_sealed_pk = self.seal_secret_for_transport(&decrypted_pk, new_password)?;
                    sqlx::query("UPDATE wallets SET encrypted_private_key = ? WHERE id = ?")
                        .bind(new_sealed_pk)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
            }

            if let Some(sealed_mn) = mn.as_ref().filter(|s| !s.trim().is_empty()) {
                if sealed_mn.starts_with("p1:") {
                    // Use old password to decrypt
                    let decrypted_mn = self.open_sealed_secret(sealed_mn, old_password)?;
                    // Use new password to re-encrypt
                    let new_sealed_mn = self.seal_secret_for_transport(&decrypted_mn, new_password)?;
                    sqlx::query("UPDATE wallets SET encrypted_mnemonic = ? WHERE id = ?")
                        .bind(new_sealed_mn)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Create Group
    pub async fn create_group(&self, request: CreateGroupRequest) -> Result<i64> {
        // Check for duplicate name in the SAME chain_type (ecosystem)
        // If parent_id is present, it's a sub-group.
        // The requirement says: "Limit duplicate names in the same ecosystem".
        // It implies globally unique names per ecosystem OR unique siblings?
        // Usually file systems allow same name in different folders.
        // But "different ecosystems can have same name" suggests ecosystem-wide uniqueness or at least root level?
        // Let's assume the user means "sibling uniqueness" or "global uniqueness per chain"?
        // Re-reading: "limit same name groups under same ecosystem".
        // Let's enforce uniqueness within the ecosystem for simplicity and safety as requested,
        // or check if it just means "sibling name conflict".
        // Given "different ecosystems can have same name", it strongly implies the name scope is the Ecosystem.
        // So we will check if ANY group in this chain_type has the same name.
        
        // Wait, standard tree logic usually allows same name in different subfolders.
        // But if the user explicitly mentioned this rule, they likely want to avoid confusion.
        // However, strict global uniqueness is harsh. 
        // Let's interpret "limit same name group under same ecosystem" as:
        // You cannot have two groups named "A" in "EVM" ecosystem, regardless of nesting?
        // OR just "sibling" uniqueness?
        // User said: "Limit same name groups under the same ecosystem, different ecosystem can have same name".
        // This sounds like (Name, ChainType) must be unique.
        
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM wallet_groups WHERE name = ? AND chain_type = ?"
        )
        .bind(&request.name)
        .bind(&request.chain_type)
        .fetch_one(&self.pool())
        .await?;

        if count > 0 {
            return Err(anyhow::anyhow!("该生态下已存在同名分组"));
        }

        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO wallet_groups (parent_id, name, chain_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id"
        )
        .bind(request.parent_id)
        .bind(request.name)
        .bind(request.chain_type)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool())
        .await?;
        Ok(id)
    }

    /// Get Groups
    pub async fn get_groups(&self) -> Result<Vec<WalletGroup>> {
        let groups = sqlx::query_as::<_, WalletGroup>(
            "SELECT * FROM wallet_groups ORDER BY created_at"
        )
        .fetch_all(&self.pool())
        .await?;
        Ok(groups)
    }

    /// Create Wallet
    pub async fn create_wallet(&self, request: CreateWalletRequest) -> Result<i64> {
        let mode = match (
            request
                .sealed_mnemonic
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
            request
                .sealed_private_key
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
        ) {
            (true, false) => CreateWalletsMode::MnemonicImport,
            (false, true) => CreateWalletsMode::PrivateKeyImport,
            (true, true) => return Err(anyhow::anyhow!("助记词与私钥只能二选一")),
            (false, false) => return Err(anyhow::anyhow!("缺少助记词或私钥")),
        };

        let created = self.create_wallets(CreateWalletsRequest {
            group_id: request.group_id,
            name: request.name,
            chain_type: request.chain_type,
            mode,
            sealed_mnemonic: request.sealed_mnemonic,
            sealed_private_key: request.sealed_private_key,
            count: 1,
            start_index: Some(0),
            word_count: None,
            remark: request.remark,
            password: request.password,
            preview_limit: Some(1),
            include_secrets: Some(false),
            transport_token: request.transport_token,
        }, request.address).await?;

        created.preview.first().map(|w| w.id).ok_or_else(|| anyhow::anyhow!("创建失败"))
    }

    pub async fn create_wallets(&self, request: CreateWalletsRequest, address_override: Option<String>) -> Result<CreateWalletsResult> {
        let password_opt = request.password.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        if let Some(pwd) = password_opt {
            if !self.unlock(pwd).await? {
                return Err(anyhow::anyhow!("密码错误"));
            }
        } else if self.mdk.lock().unwrap().is_none() {
            return Err(anyhow::anyhow!("未解锁"));
        }

        if request.count == 0 {
            return Err(anyhow::anyhow!("钱包数量必须大于0"));
        }

        let include_secrets = request.include_secrets.unwrap_or(true);
        let requested_preview_limit = request.preview_limit.unwrap_or(request.count) as usize;
        let preview_count = requested_preview_limit.min(request.count as usize);
        let transport_token = request.transport_token.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

        let mdk_bytes: [u8; 32] = {
            let mdk_guard = self.mdk.lock().unwrap();
            let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow::anyhow!("未解锁"))?;
            let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
            let mdk_vec = hex::decode(mdk_hex)?;
            mdk_vec
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid MDK length"))?
        };

        let now = Utc::now();

        match request.mode {
            CreateWalletsMode::PrivateKeyImport => {
                let sealed_private_key = request.sealed_private_key.clone().unwrap_or_default().trim().to_string();
                if request.sealed_mnemonic.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
                    return Err(anyhow::anyhow!("助记词与私钥只能二选一"));
                }

                if sealed_private_key.is_empty() {
                    return Err(anyhow::anyhow!("私钥不能为空"));
                }
                let private_key = if sealed_private_key.starts_with("t1:") {
                    self.open_transport_secret(&sealed_private_key)?
                } else {
                    let pwd = password_opt.ok_or_else(|| anyhow::anyhow!("缺少密码"))?;
                    self.open_sealed_secret(&sealed_private_key, pwd)?
                };
                let (address, normalized_private_key) = self.derive_address_from_private_key(&request.chain_type, &private_key)?;
                if let Some(addr) = address_override.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                    if addr != address {
                        return Err(anyhow::anyhow!("地址与私钥计算结果不一致"));
                    }
                }

                let name = request.name.as_ref().filter(|s| !s.trim().is_empty()).map(|s| s.clone())
                    .or(Some(Self::auto_generate_name(&address)));
                let remark = request.remark.clone();
                let sealed_private_key_out = if include_secrets && preview_count > 0 {
                    if let Some(token) = transport_token {
                        Some(self.seal_secret_for_transport_token(&normalized_private_key, token)?)
                    } else if let Some(pwd) = password_opt {
                        Some(self.seal_secret_for_transport(&normalized_private_key, pwd)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let (cipher, iv) = self.encrypt_data(normalized_private_key.as_bytes(), &mdk_bytes)?;
                let encrypted_pk = format!("{}:{}", hex::encode(iv), cipher);

                let id = sqlx::query_scalar::<_, i64>(
                    "INSERT INTO wallets (group_id, name, address, chain_type, encrypted_private_key, mnemonic_index, remark, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id"
                )
                .bind(request.group_id)
                .bind(name.clone())
                .bind(&address)
                .bind(&request.chain_type)
                .bind(encrypted_pk)
                .bind(None::<i64>)
                .bind(remark.clone())
                .bind(now)
                .bind(now)
                .fetch_one(&self.pool())
                .await?;

                let preview = if preview_count > 0 {
                    vec![WalletInfo {
                        id,
                        group_id: request.group_id,
                        name,
                        address,
                        chain_type: request.chain_type,
                        wallet_type: "private_key".to_string(),
                        has_private_key: true,
                        has_mnemonic: false,
                        sealed_private_key: sealed_private_key_out,
                        sealed_mnemonic: None,
                        mnemonic_index: None,
                        remark,
                    }]
                } else {
                    Vec::new()
                };

                Ok(CreateWalletsResult {
                    total: 1,
                    preview,
                    sealed_mnemonic: None,
                })
            }
            CreateWalletsMode::MnemonicImport | CreateWalletsMode::GenerateSameMnemonic => {
                // 批量生成模式优化
                let (mnemonic, _) = match request.mode {
                    CreateWalletsMode::MnemonicImport => {
                        let sealed_mnemonic = request.sealed_mnemonic.clone().unwrap_or_default().trim().to_string();
                        if request.sealed_private_key.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
                            return Err(anyhow::anyhow!("助记词与私钥只能二选一"));
                        }
                        if sealed_mnemonic.is_empty() {
                            return Err(anyhow::anyhow!("助记词不能为空"));
                        }
                        let mnemonic = if sealed_mnemonic.starts_with("t1:") {
                            self.open_transport_secret(&sealed_mnemonic)?
                        } else {
                            let pwd = password_opt.ok_or_else(|| anyhow::anyhow!("缺少密码"))?;
                            self.open_sealed_secret(&sealed_mnemonic, pwd)?
                        };
                        if mnemonic.is_empty() {
                            return Err(anyhow::anyhow!("助记词不能为空"));
                        }
                        (mnemonic, None)
                    }
                    CreateWalletsMode::GenerateSameMnemonic => {
                        let (mnemonic, _) = self.generate_mnemonic(request.word_count.unwrap_or(12))?;
                        (mnemonic, Some(()))
                    }
                    _ => unreachable!(),
                };

                let sealed_mnemonic_out = if include_secrets && preview_count > 0 {
                    if let Some(token) = transport_token {
                        Some(self.seal_secret_for_transport_token(&mnemonic, token)?)
                    } else if let Some(pwd) = password_opt {
                        Some(self.seal_secret_for_transport(&mnemonic, pwd)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let (mn_cipher_b64, mn_iv) = self.encrypt_data(mnemonic.as_bytes(), &mdk_bytes)?;
                let encrypted_mnemonic = format!("{}:{}", hex::encode(mn_iv), mn_cipher_b64);
                let start_index = request.start_index.unwrap_or(0);
                let count = request.count as usize;

                struct PreviewRow {
                    index: usize,
                    name: Option<String>,
                    address: String,
                    chain_type: String,
                    has_private_key: bool,
                    has_mnemonic: bool,
                    sealed_private_key: Option<String>,
                    mnemonic_index: Option<i64>,
                    remark: Option<String>,
                }

                let mut preview_rows: Vec<PreviewRow> = Vec::with_capacity(preview_count);

                let mut tx = self.pool().begin().await?;

                let mut first_id: Option<i64> = None;

                const BATCH_SIZE: usize = 80;
                let mut batch_rows: Vec<(
                    Option<i64>,
                    Option<String>,
                    String,
                    String,
                    String,
                    Option<i64>,
                    Option<String>,
                )> = Vec::with_capacity(BATCH_SIZE);

                for i in 0..count {
                    let index = start_index + i as u32;
                    let (address, private_key) = self.derive_from_mnemonic(&request.chain_type, &mnemonic, index)?;

                    if let Some(addr) = address_override.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                        if count == 1 && addr != address {
                            return Err(anyhow::anyhow!("地址与助记词计算结果不一致"));
                        }
                    }

                    let name = self.resolve_wallet_name(request.name.as_ref(), request.count, index, &address);

                    let (cipher_b64, iv) = self.encrypt_data(private_key.as_bytes(), &mdk_bytes)?;
                    let encrypted_pk = format!("{}:{}", hex::encode(iv), cipher_b64);
                    let mnemonic_index = Some((start_index + i as u32) as i64);

                    if i < preview_count {
                        let sealed_private_key = if include_secrets {
                            if let Some(token) = transport_token {
                                Some(self.seal_secret_for_transport_token(&private_key, token)?)
                            } else if let Some(pwd) = password_opt {
                                Some(self.seal_secret_for_transport(&private_key, pwd)?)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        preview_rows.push(PreviewRow {
                            index: i,
                            name: name.clone(),
                            address: address.clone(),
                            chain_type: request.chain_type.clone(),
                            has_private_key: true,
                            has_mnemonic: true,
                            sealed_private_key,
                            mnemonic_index,
                            remark: request.remark.clone(),
                        });
                    }

                    batch_rows.push((
                        request.group_id,
                        name,
                        address,
                        request.chain_type.clone(),
                        encrypted_pk,
                        mnemonic_index,
                        request.remark.clone(),
                    ));

                    if batch_rows.len() >= BATCH_SIZE || i + 1 == count {
                        let mut qb = sqlx::QueryBuilder::new(
                            "INSERT INTO wallets (group_id, name, address, chain_type, encrypted_private_key, encrypted_mnemonic, mnemonic_index, remark, created_at, updated_at) ",
                        );
                        qb.push_values(batch_rows.iter(), |mut b, row| {
                            b.push_bind(row.0)
                                .push_bind(row.1.clone())
                                .push_bind(row.2.as_str())
                                .push_bind(row.3.as_str())
                                .push_bind(row.4.as_str())
                                .push_bind(encrypted_mnemonic.as_str())
                                .push_bind(row.5)
                                .push_bind(row.6.clone())
                                .push_bind(now)
                                .push_bind(now);
                        });

                        qb.build().execute(&mut *tx).await?;
                        let last_id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
                            .fetch_one(&mut *tx)
                            .await?;
                        if first_id.is_none() {
                            let batch_len = batch_rows.len() as i64;
                            first_id = Some(last_id - batch_len + 1);
                        }
                        batch_rows.clear();
                    }
                }

                let start_id = first_id.unwrap_or(0);
                tx.commit().await?;

                let preview = preview_rows
                    .into_iter()
                    .map(|r| WalletInfo {
                        id: start_id + r.index as i64,
                        group_id: request.group_id,
                        name: r.name,
                        address: r.address,
                        chain_type: r.chain_type,
                        wallet_type: if r.has_mnemonic { "mnemonic".to_string() } else { "private_key".to_string() },
                        has_private_key: r.has_private_key,
                        has_mnemonic: r.has_mnemonic,
                        sealed_private_key: r.sealed_private_key,
                        sealed_mnemonic: None,
                        mnemonic_index: r.mnemonic_index,
                        remark: r.remark,
                    })
                    .collect::<Vec<_>>();

                Ok(CreateWalletsResult {
                    total: request.count,
                    preview,
                    sealed_mnemonic: sealed_mnemonic_out,
                })
            }
            CreateWalletsMode::GenerateDifferentMnemonic => {
                // 每个钱包独立助记词
                let count = request.count as usize;
                let now = Utc::now();

                // 批量生成数据
                let mut addresses = Vec::with_capacity(count);
                let mut private_keys = Vec::with_capacity(count);
                let mut mnemonics = Vec::with_capacity(count);
                let mut names = Vec::with_capacity(count);
                let mut sealed_mnemonics = Vec::with_capacity(count);

                for i in 0..count {
                    let (mnemonic, _) = self.generate_mnemonic(request.word_count.unwrap_or(12))?;
                    let index = 0u32;
                    let (address, private_key) = self.derive_from_mnemonic(&request.chain_type, &mnemonic, index)?;
                    let name = self.resolve_wallet_name(request.name.as_ref(), request.count, i as u32, &address);

                    let sealed_mnemonic_out = if include_secrets && i < preview_count {
                        if let Some(token) = transport_token {
                            Some(self.seal_secret_for_transport_token(&mnemonic, token)?)
                        } else if let Some(pwd) = password_opt {
                            Some(self.seal_secret_for_transport(&mnemonic, pwd)?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    addresses.push(address);
                    private_keys.push(private_key);
                    mnemonics.push(mnemonic);
                    names.push(name);
                    sealed_mnemonics.push(sealed_mnemonic_out);
                }

                // 批量加密私钥
                let encrypted_data: Vec<(String, String)> = private_keys.iter()
                    .map(|pk| {
                        let (cipher, iv) = self.encrypt_data(pk.as_bytes(), &mdk_bytes)?;
                        Ok((format!("{}:{}", hex::encode(iv), cipher), pk.clone()))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let encrypted_mnemonics: Vec<String> = mnemonics
                    .iter()
                    .map(|m| {
                        let (cipher, iv) = self.encrypt_data(m.as_bytes(), &mdk_bytes)?;
                        Ok(format!("{}:{}", hex::encode(iv), cipher))
                    })
                    .collect::<Result<Vec<_>>>()?;

                // 批量插入
                let mut tx = self.pool().begin().await?;
                let mut id_start: Option<i64> = None;

                for i in 0..count {
                    let (cipher, _) = &encrypted_data[i];

                    sqlx::query(
                        "INSERT INTO wallets (group_id, name, address, chain_type, encrypted_private_key, encrypted_mnemonic, mnemonic_index, remark, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(request.group_id)
                    .bind(names[i].clone())
                    .bind(&addresses[i])
                    .bind(&request.chain_type)
                    .bind(cipher)
                    .bind(&encrypted_mnemonics[i])
                    .bind(Some(0i64))
                    .bind(request.remark.clone())
                    .bind(now)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;

                    if id_start.is_none() {
                        id_start = Some(sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()").fetch_one(&mut *tx).await?);
                    }
                }

                let start_id = id_start.unwrap_or(0);
                let ids: Vec<i64> = (0..count).map(|i| start_id + i as i64).collect();
                tx.commit().await?;

                // 组装返回结果
                let mut preview = Vec::with_capacity(preview_count);
                for i in 0..preview_count {
                    let sealed_pk = if include_secrets {
                        if let Some(token) = transport_token {
                            Some(self.seal_secret_for_transport_token(&encrypted_data[i].1, token)?)
                        } else if let Some(pwd) = password_opt {
                            Some(self.seal_secret_for_transport(&encrypted_data[i].1, pwd)?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    preview.push(WalletInfo {
                        id: ids[i],
                        group_id: request.group_id,
                        name: names[i].clone(),
                        address: addresses[i].clone(),
                        chain_type: request.chain_type.clone(),
                        wallet_type: "mnemonic".to_string(),
                        has_private_key: true,
                        has_mnemonic: true,
                        sealed_private_key: sealed_pk,
                        sealed_mnemonic: sealed_mnemonics[i].clone(),
                        mnemonic_index: Some(0),
                        remark: request.remark.clone(),
                    });
                }
                Ok(CreateWalletsResult {
                    total: request.count,
                    preview,
                    sealed_mnemonic: None,
                })
            }
        }
    }

    pub async fn get_wallets(&self, group_id: Option<i64>, chain_type: Option<String>, _password: Option<String>) -> Result<Vec<WalletInfo>> {
        let req_chain_type = chain_type
            .clone()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        // 只查询完整钱包，排除仅地址
        let wallets = if let Some(gid) = group_id {
            let group = sqlx::query_as::<_, WalletGroup>("SELECT * FROM wallet_groups WHERE id = ?")
                .bind(gid)
                .fetch_optional(&self.pool())
                .await?;

            let group = match group {
                Some(g) => g,
                None => return Ok(Vec::new()),
            };

            let group_chain_type = group
                .chain_type
                .clone()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty());

            // If request has chain_type, validate it matches group's chain_type
            if let Some(req_ct) = req_chain_type {
                if let Some(group_ct) = group_chain_type {
                    if group_ct != req_ct {
                        return Ok(Vec::new());
                    }
                }
                // Query by group_id and chain_type, only full wallets
                sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE group_id = ? AND lower(trim(chain_type)) = ? AND wallet_type = 'full_wallet'")
                    .bind(gid)
                    .bind(&req_ct)
                    .fetch_all(&self.pool())
                    .await?
            } else if group_chain_type.is_some() {
                // Group has chain_type but request doesn't, use group's chain_type
                sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE group_id = ? AND lower(trim(chain_type)) = ? AND wallet_type = 'full_wallet'")
                    .bind(gid)
                    .bind(group_chain_type.unwrap())
                    .fetch_all(&self.pool())
                    .await?
            } else {
                // Group doesn't have chain_type, query all full wallets in this group
                sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE group_id = ? AND wallet_type = 'full_wallet'")
                    .bind(gid)
                    .fetch_all(&self.pool())
                    .await?
            }
        } else if let Some(ct) = req_chain_type {
            // Query by chain_type only (system groups), only full wallets
            sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE lower(trim(chain_type)) = ? AND wallet_type = 'full_wallet'")
                .bind(ct)
                .fetch_all(&self.pool())
                .await?
        } else {
            // No filter, return all full wallets
            sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE wallet_type = 'full_wallet'")
                .fetch_all(&self.pool())
                .await?
        };

        Ok(wallets
            .into_iter()
            .map(|w| WalletInfo {
                id: w.id,
                group_id: w.group_id,
                name: w.name,
                address: w.address,
                chain_type: w.chain_type,
                wallet_type: w.wallet_type,
                has_private_key: w.encrypted_private_key.is_some(),
                has_mnemonic: w.encrypted_mnemonic.is_some(),
                sealed_private_key: None,
                sealed_mnemonic: None,
                mnemonic_index: w.mnemonic_index,
                remark: w.remark,
            })
            .collect())
    }

    // ==================== Watch Address Functions (Read-only Addresses) ====================

    pub async fn get_watch_addresses(&self, group_id: Option<i64>, chain_type: Option<String>) -> Result<Vec<WatchAddressInfo>> {
        let req_chain_type = chain_type
            .clone()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let wallets = if let Some(gid) = group_id {
            let group = sqlx::query_as::<_, WalletGroup>("SELECT * FROM wallet_groups WHERE id = ?")
                .bind(gid)
                .fetch_optional(&self.pool())
                .await?;

            let group = match group {
                Some(g) => g,
                None => return Ok(Vec::new()),
            };

            let group_chain_type = group
                .chain_type
                .clone()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty());

            // If request has chain_type, validate it matches group's chain_type
            if let Some(req_ct) = req_chain_type {
                if let Some(group_ct) = group_chain_type {
                    if group_ct != req_ct {
                        return Ok(Vec::new());
                    }
                }
                // Query by group_id, chain_type and wallet_type
                sqlx::query_as::<_, Wallet>(
                    "SELECT * FROM wallets WHERE group_id = ? AND lower(trim(chain_type)) = ? AND wallet_type = 'address_only'"
                )
                    .bind(gid)
                    .bind(&req_ct)
                    .fetch_all(&self.pool())
                    .await?
            } else if group_chain_type.is_some() {
                sqlx::query_as::<_, Wallet>(
                    "SELECT * FROM wallets WHERE group_id = ? AND lower(trim(chain_type)) = ? AND wallet_type = 'address_only'"
                )
                    .bind(gid)
                    .bind(group_chain_type.unwrap())
                    .fetch_all(&self.pool())
                    .await?
            } else {
                sqlx::query_as::<_, Wallet>(
                    "SELECT * FROM wallets WHERE group_id = ? AND wallet_type = 'address_only'"
                )
                    .bind(gid)
                    .fetch_all(&self.pool())
                    .await?
            }
        } else if let Some(ct) = req_chain_type {
            sqlx::query_as::<_, Wallet>(
                "SELECT * FROM wallets WHERE lower(trim(chain_type)) = ? AND wallet_type = 'address_only'"
            )
                .bind(ct)
                .fetch_all(&self.pool())
                .await?
        } else {
            sqlx::query_as::<_, Wallet>(
                "SELECT * FROM wallets WHERE wallet_type = 'address_only'"
            )
                .fetch_all(&self.pool())
                .await?
        };

        // 收集所有 group_id
        let group_ids: Vec<i64> = wallets.iter().filter_map(|w| w.group_id).collect();
        
        // 批量查询分组名称
        let mut group_names = std::collections::HashMap::new();
        if !group_ids.is_empty() {
            let placeholders = group_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!("SELECT id, name FROM wallet_groups WHERE id IN ({})", placeholders);
            let mut query_builder = sqlx::query_as::<_, (i64, String)>(&query);
            for gid in &group_ids {
                query_builder = query_builder.bind(gid);
            }
            let groups = query_builder.fetch_all(&self.pool()).await?;
            for (id, name) in groups {
                group_names.insert(id, name);
            }
        }

        Ok(wallets
            .into_iter()
            .map(|w| {
                let group_name = w.group_id
                    .and_then(|gid| group_names.get(&gid).cloned());

                WatchAddressInfo {
                    id: w.id,
                    group_id: w.group_id,
                    group_name,
                    name: w.name,
                    address: w.address,
                    chain_type: w.chain_type,
                    remark: w.remark,
                    created_at: w.created_at,
                }
            })
            .collect())
    }

    pub async fn create_watch_address(&self, request: CreateWatchAddressRequest) -> Result<i64> {
        let chain_type = request.chain_type.trim().to_lowercase();

        // Validate chain_type
        if chain_type != "evm" && chain_type != "solana" {
            return Err(anyhow!("无效的链类型，应为 evm 或 solana"));
        }

        // Validate address format
        self.validate_address(&request.address, &chain_type)?;

        // Check for duplicate address
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM wallets WHERE chain_type = ? AND address = ? AND wallet_type = 'address_only'"
        )
            .bind(&chain_type)
            .bind(&request.address)
            .fetch_one(&self.pool())
            .await?;

        if exists > 0 {
            return Err(anyhow!("地址已存在"));
        }

        let result = sqlx::query(
            "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, remark, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'address_only', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
        )
            .bind(request.group_id)
            .bind(request.name)
            .bind(&request.address)
            .bind(&chain_type)
            .bind(request.remark)
            .execute(&self.pool())
            .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn create_watch_addresses(&self, request: CreateWatchAddressesRequest) -> Result<u32> {
        let chain_type = request.chain_type.trim().to_lowercase();

        // Validate chain_type
        if chain_type != "evm" && chain_type != "solana" {
            return Err(anyhow!("无效的链类型，应为 evm 或 solana"));
        }

        let mut count = 0;
        let _now = Utc::now().naive_utc();
        let remark = request.remark.clone();

        for (index, addr) in request.addresses.iter().enumerate() {
            let address = addr.trim();
            if address.is_empty() {
                continue;
            }

            // Validate address format
            if let Err(e) = self.validate_address(address, &chain_type) {
                log::warn!("地址格式无效，跳过: {} - {}", address, e);
                continue;
            }

            // Check for duplicate address
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM wallets WHERE chain_type = ? AND address = ? AND wallet_type = 'address_only'"
            )
                .bind(&chain_type)
                .bind(address)
                .fetch_one(&self.pool())
                .await?;

            if exists > 0 {
                log::warn!("地址已存在，跳过: {}", address);
                continue;
            }

            let name = request.name_prefix.as_ref().map(|prefix| {
                format!("{} #{}", prefix, index + 1)
            });

            let _ = sqlx::query(
                "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, remark, created_at, updated_at)
                 VALUES (?, ?, ?, ?, 'address_only', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
            )
                .bind(request.group_id)
                .bind(name)
                .bind(address)
                .bind(&chain_type)
                .bind(&remark)
                .execute(&self.pool())
                .await?;

            count += 1;
        }

        Ok(count)
    }

    pub async fn update_watch_address(&self, request: UpdateWatchAddressRequest) -> Result<()> {
        let _wallet = sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE id = ? AND wallet_type = 'address_only'"
        )
            .bind(request.id)
            .fetch_optional(&self.pool())
            .await?
            .ok_or_else(|| anyhow!("地址不存在"))?;

        sqlx::query(
            "UPDATE wallets SET group_id = ?, name = ?, remark = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
            .bind(request.group_id)
            .bind(request.name)
            .bind(request.remark)
            .bind(request.id)
            .execute(&self.pool())
            .await?;

        Ok(())
    }

    pub async fn delete_watch_address(&self, id: i64) -> Result<()> {
        log::debug!("[DEBUG] 删除仅地址，ID: {}", id);
        let result = sqlx::query("DELETE FROM wallets WHERE id = ? AND wallet_type = 'address_only'")
            .bind(id)
            .execute(&self.pool())
            .await?;

        log::debug!("[DEBUG] 删除结果，影响行数: {}", result.rows_affected());
        if result.rows_affected() == 0 {
            return Err(anyhow!("地址不存在"));
        }

        Ok(())
    }

    pub async fn export_watch_addresses(&self, ids: &[i64]) -> Result<Vec<WatchAddressExportData>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT * FROM wallets WHERE id IN ({}) AND wallet_type = 'address_only'",
            placeholders
        );

        let mut q = sqlx::query_as::<_, Wallet>(&query);
        for id in ids {
            q = q.bind(id);
        }
        let wallets = q.fetch_all(&self.pool()).await?;

        let results: Vec<WatchAddressExportData> = wallets
            .into_iter()
            .map(|w| WatchAddressExportData {
                id: w.id,
                name: w.name,
                address: w.address,
                chain_type: w.chain_type,
                remark: w.remark,
                group_id: w.group_id,
            })
            .collect();

        Ok(results)
    }

    fn validate_address(&self, address: &str, chain_type: &str) -> Result<()> {
        let addr = address.trim();

        if chain_type == "evm" {
            // EVM address: starts with 0x, 42 characters (20 bytes hex)
            if !addr.starts_with("0x") {
                return Err(anyhow!("EVM 地址必须以 0x 开头"));
            }
            if addr.len() != 42 {
                return Err(anyhow!("EVM 地址长度应为 42 个字符，实际 {}", addr.len()));
            }
            // Check if remaining characters are valid hex
            let hex_part = &addr[2..];
            if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(anyhow!("EVM 地址包含无效的十六进制字符"));
            }
        } else if chain_type == "solana" {
            // Solana address: base58 encoded, typically 32-44 characters
            if addr.len() < 32 || addr.len() > 44 {
                return Err(anyhow!("Solana 地址长度应在 32-44 个字符之间，实际 {}", addr.len()));
            }
            // Basic validation - base58 chars are alphanumeric except 0, O, I, l
            let valid_chars: std::collections::HashSet<char> = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".chars().collect();
            if !addr.chars().all(|c| valid_chars.contains(&c)) {
                return Err(anyhow!("Solana 地址包含无效的 base58 字符"));
            }
        }

        Ok(())
    }

    pub async fn delete_wallet(&self, id: i64) -> Result<()> {
        log::debug!("[DEBUG] 删除完整钱包，ID: {}", id);
        let result = sqlx::query("DELETE FROM wallets WHERE id = ? AND wallet_type = 'full_wallet'")
            .bind(id)
            .execute(&self.pool())
            .await?;
        log::debug!("[DEBUG] 删除结果，影响行数: {}", result.rows_affected());
        Ok(())
    }

    pub async fn get_wallet_secrets(&self, id: i64, password: Option<&str>, transport_token: Option<&str>) -> Result<WalletSecrets> {
        let wallet = sqlx::query_as::<_, Wallet>(
            "SELECT * FROM wallets WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool())
        .await?
        .ok_or_else(|| anyhow!("钱包不存在"))?;

        let mdk_bytes: [u8; 32] = {
            let mdk_guard = self.mdk.lock().unwrap();
            let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow!("钱包管理器未解锁，请先输入主密码"))?;
            let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
            let mdk_vec = hex::decode(mdk_hex)?;
            mdk_vec
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("Invalid MDK length"))?
        };

        // Decrypt then re-seal for transport
        let sealed_private_key = if let Some(encrypted) = &wallet.encrypted_private_key {
            let parts: Vec<&str> = encrypted.split(':').collect();
            if parts.len() != 2 {
                Some(encrypted.clone())
            } else {
                let iv = hex::decode(parts[0])?;
                let cipher_b64 = parts[1];
                let plaintext = self.decrypt_data(cipher_b64, &mdk_bytes, &iv)?;
                let private_key_str = String::from_utf8(plaintext)?;
                if let Some(token) = transport_token.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    Some(self.seal_secret_for_transport_token(&private_key_str, token)?)
                } else if let Some(pwd) = password.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    Some(self.seal_secret_for_transport(&private_key_str, pwd)?)
                } else {
                    None
                }
            }
        } else {
            None
        };

        let sealed_mnemonic = if let Some(encrypted) = &wallet.encrypted_mnemonic {
            let parts: Vec<&str> = encrypted.split(':').collect();
            if parts.len() != 2 {
                Some(encrypted.clone())
            } else {
                let iv = hex::decode(parts[0])?;
                let cipher_b64 = parts[1];
                let plaintext = self.decrypt_data(cipher_b64, &mdk_bytes, &iv)?;
                let mnemonic_str = String::from_utf8(plaintext)?;
                if let Some(token) = transport_token.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    Some(self.seal_secret_for_transport_token(&mnemonic_str, token)?)
                } else if let Some(pwd) = password.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    Some(self.seal_secret_for_transport(&mnemonic_str, pwd)?)
                } else {
                    None
                }
            }
        } else {
            None
        };

        Ok(WalletSecrets {
            id: wallet.id,
            name: wallet.name,
            address: wallet.address,
            sealed_private_key,
            sealed_mnemonic,
        })
    }

    pub async fn export_wallets(&self, ids: &[i64], _password: &str) -> Result<Vec<WalletExportData>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mdk_bytes: [u8; 32] = {
            let mdk_guard = self.mdk.lock().unwrap();
            let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow!("钱包管理器未解锁，请先输入主密码"))?;
            let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
            let mdk_vec = hex::decode(mdk_hex)?;
            mdk_vec
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("Invalid MDK length"))?
        };

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT * FROM wallets WHERE id IN ({})", placeholders);
        let mut q = sqlx::query_as::<_, Wallet>(&query);
        for id in ids {
            q = q.bind(id);
        }
        let wallets = q.fetch_all(&self.pool()).await?;

        let mut results = Vec::with_capacity(wallets.len());
        for wallet in wallets {
            let private_key = if let Some(encrypted) = &wallet.encrypted_private_key {
                let parts: Vec<&str> = encrypted.split(':').collect();
                if parts.len() != 2 {
                    Some(encrypted.clone())
                } else {
                    let iv = hex::decode(parts[0])?;
                    let cipher_b64 = parts[1];
                    match self.decrypt_data(cipher_b64, &mdk_bytes, &iv) {
                        Ok(plaintext) => Some(String::from_utf8(plaintext)?),
                        Err(_) => Some(encrypted.clone()),
                    }
                }
            } else {
                None
            };

            let mnemonic = if let Some(encrypted) = &wallet.encrypted_mnemonic {
                let parts: Vec<&str> = encrypted.split(':').collect();
                if parts.len() != 2 {
                    Some(encrypted.clone())
                } else {
                    let iv = hex::decode(parts[0])?;
                    let cipher_b64 = parts[1];
                    match self.decrypt_data(cipher_b64, &mdk_bytes, &iv) {
                        Ok(plaintext) => Some(String::from_utf8(plaintext)?),
                        Err(_) => Some(encrypted.clone()),
                    }
                }
            } else {
                None
            };

            results.push(WalletExportData {
                id: wallet.id,
                name: wallet.name,
                address: wallet.address,
                chain_type: wallet.chain_type,
                private_key,
                mnemonic,
                mnemonic_index: wallet.mnemonic_index,
                remark: wallet.remark,
                group_id: wallet.group_id,
            });
        }

        Ok(results)
    }

    pub async fn update_wallet(&self, request: UpdateWalletRequest) -> Result<()> {
        sqlx::query(
            "UPDATE wallets SET group_id = ?, name = ?, remark = ?, updated_at = ? WHERE id = ?"
        )
        .bind(request.group_id)
        .bind(request.name)
        .bind(request.remark)
        .bind(Utc::now())
        .bind(request.id)
        .execute(&self.pool())
        .await?;
        Ok(())
    }

    pub async fn update_group(&self, request: UpdateGroupRequest) -> Result<()> {
        sqlx::query("UPDATE wallet_groups SET name = ?, updated_at = ? WHERE id = ?")
            .bind(request.name)
            .bind(Utc::now())
            .bind(request.id)
            .execute(&self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_group(&self, id: i64) -> Result<()> {
        // Get all descendant group IDs (including current group) using recursive CTE
        let descendant_ids: Vec<i64> = sqlx::query_scalar::<_, i64>(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT id FROM wallet_groups WHERE id = ?
                UNION ALL
                SELECT wg.id FROM wallet_groups wg
                INNER JOIN descendants d ON wg.parent_id = d.id
            )
            SELECT id FROM descendants
            "#
        ).bind(id).fetch_all(&self.pool()).await?;

        // Delete all wallets in these groups first
        if !descendant_ids.is_empty() {
            let placeholders = descendant_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!("DELETE FROM wallets WHERE group_id IN ({})", placeholders);
            let mut q = sqlx::query(&query);
            for id in &descendant_ids {
                q = q.bind(id);
            }
            q.execute(&self.pool()).await?;
        }

        // Delete the groups (child groups will be deleted by CASCADE)
        sqlx::query("DELETE FROM wallet_groups WHERE id = ?")
            .bind(id)
            .execute(&self.pool())
            .await?;

        Ok(())
    }

    // --- Helper Methods ---

    fn derive_key(&self, password: &str, salt: &[u8]) -> [u8; 32] {
        let mut key = [0u8; 32];
        let _ = pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 100_000, &mut key);
        key
    }

    fn seal_secret_for_transport(&self, plaintext: &str, password: &str) -> Result<String> {
        let mut salt = [0u8; 16];
        rand::thread_rng().fill(&mut salt);
        let key = self.derive_key(password, &salt);
        let (cipher_b64, iv) = self.encrypt_data(plaintext.as_bytes(), &key)?;
        Ok(format!(
            "p1:{}:{}:{}",
            hex::encode(salt),
            hex::encode(iv),
            cipher_b64
        ))
    }

    fn open_sealed_secret(&self, sealed: &str, password: &str) -> Result<String> {
        let s = sealed.trim();
        if !s.starts_with("p1:") {
            return Err(anyhow::anyhow!("敏感数据必须使用加密格式传输"));
        }
        let parts: Vec<&str> = s[3..].split(':').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("密文格式错误"));
        }
        let salt = hex::decode(parts[0])?;
        let iv = hex::decode(parts[1])?;
        let cipher_b64 = parts[2];
        let key = self.derive_key(password, &salt);
        let bytes = self.decrypt_data(cipher_b64, &key, &iv)?;
        Ok(String::from_utf8(bytes)?)
    }

    fn looks_like_db_encrypted_field(&self, field_value: &str) -> bool {
        let parts: Vec<&str> = field_value.split(':').collect();
        if parts.len() != 2 {
            return false;
        }
        let iv_ok = hex::decode(parts[0]).ok().map(|v| v.len() == 16).unwrap_or(false);
        if !iv_ok {
            return false;
        }
        BASE64.decode(parts[1]).is_ok()
    }

    async fn migrate_plaintext_wallet_secrets(&self, mdk: &[u8; 32]) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        let rows = sqlx::query("SELECT id, encrypted_private_key, encrypted_mnemonic FROM wallets")
            .fetch_all(&mut *tx)
            .await?;

        for row in rows {
            let id: i64 = row.get("id");
            let pk: Option<String> = row.get("encrypted_private_key");
            let mn: Option<String> = row.get("encrypted_mnemonic");

            let mut new_pk: Option<String> = None;
            if let Some(v) = pk.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !self.looks_like_db_encrypted_field(v) && !v.starts_with("p1:") {
                    let (cipher, iv) = self.encrypt_data(v.as_bytes(), mdk)?;
                    new_pk = Some(format!("{}:{}", hex::encode(iv), cipher));
                }
            }

            let mut new_mn: Option<String> = None;
            if let Some(v) = mn.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !self.looks_like_db_encrypted_field(v) && !v.starts_with("p1:") {
                    let (cipher, iv) = self.encrypt_data(v.as_bytes(), mdk)?;
                    new_mn = Some(format!("{}:{}", hex::encode(iv), cipher));
                }
            }

            if new_pk.is_some() || new_mn.is_some() {
                sqlx::query(
                    "UPDATE wallets SET encrypted_private_key = COALESCE(?, encrypted_private_key), encrypted_mnemonic = COALESCE(?, encrypted_mnemonic), updated_at = ? WHERE id = ?"
                )
                .bind(new_pk)
                .bind(new_mn)
                .bind(Utc::now())
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    fn resolve_wallet_name(&self, name: Option<&String>, count: u32, index: u32, address: &str) -> Option<String> {
        let name_input = name.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        match (name_input, count) {
            (Some(prefix), 1) => Some(prefix.to_string()),
            (Some(prefix), _) => Some(format!("{}-{}", prefix, index)),
            (None, 1) => Some(Self::auto_generate_name(address)),
            (None, _) => Some(format!("{}-{}", Self::auto_generate_name(address), index)),
        }
    }

    fn auto_generate_name(address: &str) -> String {
        let address_end = address.chars().rev().take(6).collect::<String>().chars().rev().collect::<String>();
        address_end.to_uppercase()
    }

    fn derive_address_from_private_key(&self, chain_type: &str, private_key: &str) -> Result<(String, String)> {
        match chain_type {
            "evm" => {
                let normalized = if private_key.starts_with("0x") || private_key.starts_with("0X") {
                    private_key[2..].to_string()
                } else {
                    private_key.to_string()
                };
                let normalized = normalized.trim().to_string();
                let signer: PrivateKeySigner = normalized.parse().map_err(|e| anyhow::anyhow!("私钥格式错误: {e}"))?;
                Ok((signer.address().to_string(), normalized))
            }
            "solana" => {
                let normalized = private_key.trim().to_string();
                let bytes = bs58::decode(&normalized).into_vec().map_err(|e| anyhow::anyhow!("Solana私钥格式错误: {e}"))?;
                let keypair = Keypair::try_from(bytes.as_slice()).map_err(|e| anyhow::anyhow!("Solana私钥格式错误: {e}"))?;
                Ok((keypair.pubkey().to_string(), normalized))
            }
            _ => Err(anyhow::anyhow!("不支持的链类型")),
        }
    }

    fn derive_from_mnemonic(&self, chain_type: &str, mnemonic: &str, index: u32) -> Result<(String, String)> {
        match chain_type {
            "evm" => {
                let signer = MnemonicBuilder::<English>::default()
                    .phrase(mnemonic)
                    .index(index)?
                    .build()?;
                let pk_bytes = signer.to_bytes();
                let private_key = hex::encode(pk_bytes.as_slice());
                Ok((signer.address().to_string(), private_key))
            }
            "solana" => {
                let seed = self.mnemonic_seed(mnemonic)?;
                let derived = self.derive_slip10_ed25519(&seed, &[44, 501, index, 0])?;
                let keypair = Keypair::new_from_array(derived);
                let private_key = bs58::encode(keypair.to_bytes()).into_string();
                Ok((keypair.pubkey().to_string(), private_key))
            }
            _ => Err(anyhow::anyhow!("不支持的链类型")),
        }
    }

    fn generate_mnemonic(&self, word_count: u32) -> Result<(String, u32)> {
        let count = match word_count {
            12 => 12usize,
            24 => 24usize,
            _ => return Err(anyhow::anyhow!("仅支持12或24词助记词")),
        };
        let mut rng = rand::thread_rng();
        let mnemonic = Mnemonic::<English>::new_with_count(&mut rng, count)?;
        Ok((mnemonic.to_phrase(), word_count))
    }

    fn mnemonic_seed(&self, phrase: &str) -> Result<[u8; 64]> {
        let mnemonic = Mnemonic::<English>::new_from_phrase(phrase)?;
        Ok(mnemonic.to_seed(Some(""))?)
    }

    fn derive_slip10_ed25519(&self, seed: &[u8; 64], path: &[u32]) -> Result<[u8; 32]> {
        let mut mac = HmacSha512::new_from_slice(b"ed25519 seed")?;
        mac.update(seed);
        let mut i = mac.finalize().into_bytes();
        let mut key = [0u8; 32];
        let mut chain_code = [0u8; 32];
        key.copy_from_slice(&i[..32]);
        chain_code.copy_from_slice(&i[32..]);

        for idx in path {
            let hardened = idx | 0x8000_0000;
            let mut mac = HmacSha512::new_from_slice(&chain_code)?;
            mac.update(&[0u8]);
            mac.update(&key);
            mac.update(&hardened.to_be_bytes());
            i = mac.finalize().into_bytes();
            key.copy_from_slice(&i[..32]);
            chain_code.copy_from_slice(&i[32..]);
        }

        Ok(key)
    }

    fn encrypt_data(&self, data: &[u8], key: &[u8; 32]) -> Result<(String, Vec<u8>)> {
        let mut iv = [0u8; 16];
        rand::thread_rng().fill(&mut iv);

        let len = data.len();
        let mut buffer = vec![0u8; len + 16];
        buffer[..len].copy_from_slice(data);

        let encryptor = Aes256CbcEnc::new(key.into(), &iv.into());
        let ciphertext_len = encryptor.encrypt_padded_mut::<Pkcs7>(&mut buffer, len)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?
            .len();
        
        buffer.truncate(ciphertext_len);
        Ok((BASE64.encode(&buffer), iv.to_vec()))
    }

    fn decrypt_data(&self, ciphertext_base64: &str, key: &[u8; 32], iv: &[u8]) -> Result<Vec<u8>> {
        let mut ciphertext = BASE64.decode(ciphertext_base64)?;
        let decryptor = Aes256CbcDec::new(key.into(), iv.into());
        
        let plaintext = decryptor.decrypt_padded_mut::<Pkcs7>(&mut ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {:?}", e))?;
        
        Ok(plaintext.to_vec())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO app_config ([key], value) VALUES (?, ?) ON CONFLICT([key]) DO UPDATE SET value = ?"
        )
        .bind(key)
        .bind(value)
        .bind(value)
        .execute(&self.pool())
        .await?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM app_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool())
            .await?;
        
        if let Some(r) = row {
            Ok(Some(r.get("value")))
        } else {
            Ok(None)
        }
    }

    // ==================== Encrypted Cloud Backup Methods ====================

    /// 创建加密备份
    pub async fn create_encrypted_backup(&self, request: CreateBackupRequest) -> Result<EncryptedBackup> {
        // 1. 收集备份数据
        let backup_data = self.collect_backup_data(&request.backup_type).await?;
        
        // 2. 序列化数据
        let json_data = serde_json::to_string(&backup_data)?;
        
        // 3. 生成加密参数
        let mut salt = [0u8; 16];
        let mut iv = [0u8; 12];
        rand::thread_rng().fill(&mut salt);
        rand::thread_rng().fill(&mut iv);
        
        // 4. 使用备份密码派生密钥
        let key = self.derive_backup_key(&request.backup_password, &salt);
        
        // 5. 使用 AES-256-GCM 加密
        let (ciphertext, auth_tag) = self.encrypt_backup_data(&json_data, &key, &iv)?;
        
        // 6. 构建元数据
        let metadata = BackupMetadata {
            version: "1.0".to_string(),
            created_at: Utc::now(),
            wallet_count: backup_data.wallets.len() as u32,
            group_count: backup_data.groups.len() as u32,
            watch_address_count: backup_data.watch_addresses.len() as u32,
            backup_type: request.backup_type,
            description: request.description,
        };
        
        Ok(EncryptedBackup {
            metadata,
            encrypted_data: BASE64.encode(&ciphertext),
            encryption_version: "aes-256-gcm-v1".to_string(),
            salt: hex::encode(salt),
            iv: hex::encode(iv),
            auth_tag: hex::encode(auth_tag),
        })
    }

    /// 恢复加密备份
    pub async fn restore_encrypted_backup(&self, request: RestoreBackupRequest) -> Result<RestoreResult> {
        let backup = request.backup_data;
        
        // 1. 验证加密版本
        if backup.encryption_version != "aes-256-gcm-v1" {
            return Err(anyhow!("不支持的加密版本: {}", backup.encryption_version));
        }
        
        // 2. 解析加密参数
        let salt = hex::decode(&backup.salt)?;
        let iv = hex::decode(&backup.iv)?;
        let auth_tag = hex::decode(&backup.auth_tag)?;
        let ciphertext = BASE64.decode(&backup.encrypted_data)?;
        
        // 3. 派生密钥
        let key = self.derive_backup_key(&request.backup_password, &salt[..16].try_into()?);
        
        // 4. 解密数据
        let plaintext = self.decrypt_backup_data(&ciphertext, &key, &iv[..12].try_into()?, &auth_tag)?;
        let backup_data: BackupData = serde_json::from_str(&plaintext)?;
        
        // 5. 根据合并策略恢复数据
        match request.merge_strategy {
            MergeStrategy::ReplaceAll => {
                self.restore_with_replace_all(backup_data).await
            }
            MergeStrategy::SkipExisting => {
                self.restore_with_skip_existing(backup_data).await
            }
            MergeStrategy::OverwriteExisting => {
                self.restore_with_overwrite(backup_data).await
            }
        }
    }

    /// 收集备份数据
    async fn collect_backup_data(&self, backup_type: &BackupType) -> Result<BackupData> {
        let mut groups = Vec::new();
        let mut wallets = Vec::new();
        let mut watch_addresses = Vec::new();
        let app_config;
        
        // 获取 MDK 用于解密钱包数据
        let mdk_bytes: [u8; 32] = {
            let mdk_guard = self.mdk.lock().unwrap();
            let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow!("钱包管理器未解锁"))?;
            let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
            let mdk_vec = hex::decode(mdk_hex)?;
            mdk_vec.as_slice().try_into().map_err(|_| anyhow!("Invalid MDK length"))?
        };
        
        match backup_type {
            BackupType::Full | BackupType::GroupsOnly => {
                groups = sqlx::query_as::<_, WalletGroup>("SELECT * FROM wallet_groups")
                    .fetch_all(&self.pool()).await?;
            }
            _ => {}
        }
        
        match backup_type {
            BackupType::Full | BackupType::WalletsOnly => {
                let wallet_rows = sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE wallet_type = 'full_wallet'")
                    .fetch_all(&self.pool()).await?;
                
                for wallet in wallet_rows {
                    let private_key = self.decrypt_wallet_field(&wallet.encrypted_private_key, &mdk_bytes)?;
                    let mnemonic = self.decrypt_wallet_field(&wallet.encrypted_mnemonic, &mdk_bytes)?;
                    
                    wallets.push(WalletBackupData {
                        id: wallet.id,
                        group_id: wallet.group_id,
                        name: wallet.name,
                        address: wallet.address,
                        chain_type: wallet.chain_type,
                        wallet_type: wallet.wallet_type,
                        private_key,
                        mnemonic,
                        mnemonic_index: wallet.mnemonic_index,
                        remark: wallet.remark,
                        created_at: wallet.created_at,
                        updated_at: wallet.updated_at,
                    });
                }
                
                // 收集仅地址数据
                let watch_rows = sqlx::query_as::<_, Wallet>("SELECT * FROM wallets WHERE wallet_type = 'address_only'")
                    .fetch_all(&self.pool()).await?;
                
                for wallet in watch_rows {
                    watch_addresses.push(WatchAddressBackupData {
                        id: wallet.id,
                        group_id: wallet.group_id,
                        name: wallet.name,
                        address: wallet.address,
                        chain_type: wallet.chain_type,
                        remark: wallet.remark,
                        created_at: wallet.created_at,
                    });
                }
            }
            _ => {}
        }
        
        // 收集必要的配置（排除敏感信息）
        let config_rows = sqlx::query_as::<_, AppConfig>("SELECT * FROM app_config WHERE key NOT LIKE '%master%' AND key NOT LIKE '%verifier%'")
            .fetch_all(&self.pool()).await?;
        app_config = config_rows;
        
        Ok(BackupData {
            groups,
            wallets,
            watch_addresses,
            app_config,
        })
    }

    /// 解密钱包字段
    fn decrypt_wallet_field(&self, encrypted: &Option<String>, mdk: &[u8; 32]) -> Result<Option<String>> {
        if let Some(encrypted_str) = encrypted.as_ref().filter(|s| !s.is_empty()) {
            let parts: Vec<&str> = encrypted_str.split(':').collect();
            if parts.len() != 2 {
                return Ok(Some(encrypted_str.clone()));
            }
            let iv = hex::decode(parts[0])?;
            let cipher_b64 = parts[1];
            match self.decrypt_data(cipher_b64, mdk, &iv) {
                Ok(plaintext) => Ok(Some(String::from_utf8(plaintext)?)),
                Err(_) => Ok(Some(encrypted_str.clone())),
            }
        } else {
            Ok(None)
        }
    }

    /// 派生备份密钥
    fn derive_backup_key(&self, password: &str, salt: &[u8; 16]) -> [u8; 32] {
        let mut key = [0u8; 32];
        let _ = pbkdf2::<Hmac<Sha512>>(password.as_bytes(), salt, 600_000, &mut key);
        key
    }

    /// 加密备份数据（AES-256-GCM）
    fn encrypt_backup_data(&self, data: &str, key: &[u8; 32], iv: &[u8; 12]) -> Result<(Vec<u8>, Vec<u8>)> {
        use openssl::symm::{Cipher, Crypter, Mode};
        
        let cipher = Cipher::aes_256_gcm();
        let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))?;
        crypter.pad(false);
        
        let mut ciphertext = vec![0u8; data.len() + cipher.block_size()];
        let mut count = crypter.update(data.as_bytes(), &mut ciphertext)?;
        count += crypter.finalize(&mut ciphertext[count..])?;
        ciphertext.truncate(count);
        
        let mut tag = [0u8; 16];
        crypter.get_tag(&mut tag)?;
        
        Ok((ciphertext, tag.to_vec()))
    }

    /// 解密备份数据（AES-256-GCM）
    fn decrypt_backup_data(&self, ciphertext: &[u8], key: &[u8; 32], iv: &[u8; 12], auth_tag: &[u8]) -> Result<String> {
        use openssl::symm::{Cipher, Crypter, Mode};
        
        let cipher = Cipher::aes_256_gcm();
        let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))?;
        crypter.pad(false);
        crypter.set_tag(auth_tag)?;
        
        let mut plaintext = vec![0u8; ciphertext.len() + cipher.block_size()];
        let mut count = crypter.update(ciphertext, &mut plaintext)?;
        count += crypter.finalize(&mut plaintext[count..])?;
        plaintext.truncate(count);
        
        Ok(String::from_utf8(plaintext)?)
    }

    /// 完全替换恢复
    async fn restore_with_replace_all(&self, data: BackupData) -> Result<RestoreResult> {
        let mut tx = self.pool().begin().await?;
        let mut errors = Vec::new();
        
        // 清空现有数据
        sqlx::query("DELETE FROM wallets").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM wallet_groups").execute(&mut *tx).await?;
        
        // 恢复分组
        let mut group_id_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
        for group in &data.groups {
            let new_id: i64 = sqlx::query_scalar(
                "INSERT INTO wallet_groups (parent_id, name, chain_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id"
            )
            .bind(group.parent_id.and_then(|pid| group_id_map.get(&pid).copied()))
            .bind(&group.name)
            .bind(&group.chain_type)
            .bind(group.created_at)
            .bind(group.updated_at)
            .fetch_one(&mut *tx).await?;
            group_id_map.insert(group.id, new_id);
        }
        
        // 恢复钱包
        let mut wallets_restored = 0u32;
        for wallet in &data.wallets {
            let new_group_id = wallet.group_id.and_then(|gid| group_id_map.get(&gid).copied());
            
            // 加密私钥和助记词
            let encrypted_pk = if let Some(pk) = &wallet.private_key {
                let mdk = self.get_mdk_bytes()?;
                let (cipher, iv) = self.encrypt_data(pk.as_bytes(), &mdk)?;
                Some(format!("{}:{}", hex::encode(iv), cipher))
            } else { None };
            
            let encrypted_mn = if let Some(mn) = &wallet.mnemonic {
                let mdk = self.get_mdk_bytes()?;
                let (cipher, iv) = self.encrypt_data(mn.as_bytes(), &mdk)?;
                Some(format!("{}:{}", hex::encode(iv), cipher))
            } else { None };
            
            let result = sqlx::query(
                "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, encrypted_private_key, encrypted_mnemonic, mnemonic_index, remark, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(new_group_id)
            .bind(&wallet.name)
            .bind(&wallet.address)
            .bind(&wallet.chain_type)
            .bind(&wallet.wallet_type)
            .bind(&encrypted_pk)
            .bind(&encrypted_mn)
            .bind(wallet.mnemonic_index)
            .bind(&wallet.remark)
            .bind(wallet.created_at)
            .bind(wallet.updated_at)
            .execute(&mut *tx).await;
            
            match result {
                Ok(_) => wallets_restored += 1,
                Err(e) => errors.push(format!("恢复钱包 {} 失败: {}", wallet.address, e)),
            }
        }
        
        // 恢复仅地址
        let mut watch_addresses_restored = 0u32;
        for watch in &data.watch_addresses {
            let new_group_id = watch.group_id.and_then(|gid| group_id_map.get(&gid).copied());
            
            let result = sqlx::query(
                "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, remark, created_at, updated_at) VALUES (?, ?, ?, ?, 'address_only', ?, ?, ?)"
            )
            .bind(new_group_id)
            .bind(&watch.name)
            .bind(&watch.address)
            .bind(&watch.chain_type)
            .bind(&watch.remark)
            .bind(watch.created_at)
            .bind(Utc::now())
            .execute(&mut *tx).await;
            
            match result {
                Ok(_) => watch_addresses_restored += 1,
                Err(e) => errors.push(format!("恢复仅地址 {} 失败: {}", watch.address, e)),
            }
        }
        
        tx.commit().await?;
        
        Ok(RestoreResult {
            success: errors.is_empty(),
            groups_restored: data.groups.len() as u32,
            wallets_restored,
            watch_addresses_restored,
            errors,
        })
    }

    /// 跳过已存在恢复
    async fn restore_with_skip_existing(&self, data: BackupData) -> Result<RestoreResult> {
        let mut tx = self.pool().begin().await?;
        let mut errors = Vec::new();
        let mut groups_restored = 0u32;
        let mut wallets_restored = 0u32;
        let mut watch_addresses_restored = 0u32;

        // 获取 MDK 用于加密钱包数据
        let mdk_bytes = self.get_mdk_bytes()?;

        // 检查现有地址
        let existing_addresses: Vec<String> = sqlx::query_scalar("SELECT address FROM wallets")
            .fetch_all(&mut *tx).await?;
        let existing_set: std::collections::HashSet<String> = existing_addresses.into_iter().collect();

        // 恢复分组（跳过同名）
        let mut group_id_map: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
        for group in &data.groups {
            let exists: bool = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM wallet_groups WHERE name = ? AND chain_type = ?"
            )
            .bind(&group.name)
            .bind(&group.chain_type)
            .fetch_one(&mut *tx).await? > 0;

            if !exists {
                let new_id: i64 = sqlx::query_scalar(
                    "INSERT INTO wallet_groups (parent_id, name, chain_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?) RETURNING id"
                )
                .bind(group.parent_id.and_then(|pid| group_id_map.get(&pid).copied()))
                .bind(&group.name)
                .bind(&group.chain_type)
                .bind(group.created_at)
                .bind(group.updated_at)
                .fetch_one(&mut *tx).await?;
                group_id_map.insert(group.id, new_id);
                groups_restored += 1;
            }
        }

        // 恢复钱包（跳过已存在地址）
        for wallet in &data.wallets {
            if existing_set.contains(&wallet.address) {
                continue;
            }

            let new_group_id = wallet.group_id.and_then(|gid| group_id_map.get(&gid).copied());

            // 加密私钥和助记词
            let encrypted_pk = if let Some(pk) = &wallet.private_key {
                let (cipher, iv) = self.encrypt_data(pk.as_bytes(), &mdk_bytes)?;
                Some(format!("{}:{}", hex::encode(iv), cipher))
            } else { None };

            let encrypted_mn = if let Some(mn) = &wallet.mnemonic {
                let (cipher, iv) = self.encrypt_data(mn.as_bytes(), &mdk_bytes)?;
                Some(format!("{}:{}", hex::encode(iv), cipher))
            } else { None };

            let result = sqlx::query(
                "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, encrypted_private_key, encrypted_mnemonic, mnemonic_index, remark, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(new_group_id)
            .bind(&wallet.name)
            .bind(&wallet.address)
            .bind(&wallet.chain_type)
            .bind(&wallet.wallet_type)
            .bind(&encrypted_pk)
            .bind(&encrypted_mn)
            .bind(wallet.mnemonic_index)
            .bind(&wallet.remark)
            .bind(wallet.created_at)
            .bind(wallet.updated_at)
            .execute(&mut *tx).await;

            match result {
                Ok(_) => wallets_restored += 1,
                Err(e) => errors.push(format!("恢复钱包 {} 失败: {}", wallet.address, e)),
            }
        }

        // 恢复仅地址（跳过已存在地址）
        for watch in &data.watch_addresses {
            if existing_set.contains(&watch.address) {
                continue;
            }

            let new_group_id = watch.group_id.and_then(|gid| group_id_map.get(&gid).copied());

            let result = sqlx::query(
                "INSERT INTO wallets (group_id, name, address, chain_type, wallet_type, remark, created_at, updated_at) VALUES (?, ?, ?, ?, 'address_only', ?, ?, ?)"
            )
            .bind(new_group_id)
            .bind(&watch.name)
            .bind(&watch.address)
            .bind(&watch.chain_type)
            .bind(&watch.remark)
            .bind(watch.created_at)
            .bind(Utc::now())
            .execute(&mut *tx).await;

            match result {
                Ok(_) => watch_addresses_restored += 1,
                Err(e) => errors.push(format!("恢复仅地址 {} 失败: {}", watch.address, e)),
            }
        }

        tx.commit().await?;

        Ok(RestoreResult {
            success: errors.is_empty(),
            groups_restored,
            wallets_restored,
            watch_addresses_restored,
            errors,
        })
    }

    /// 覆盖已存在恢复
    async fn restore_with_overwrite(&self, data: BackupData) -> Result<RestoreResult> {
        // 类似 skip_existing，但会更新已存在的记录
        // 简化实现，实际使用 replace_all 逻辑
        self.restore_with_replace_all(data).await
    }

    /// 获取 MDK 字节
    fn get_mdk_bytes(&self) -> Result<[u8; 32]> {
        let mdk_guard = self.mdk.lock().unwrap();
        let secure_mdk = mdk_guard.as_ref().ok_or_else(|| anyhow!("钱包管理器未解锁"))?;
        let mdk_hex = secure_mdk.use_secret(|s| s.to_string()).map_err(|e| anyhow!(e))?;
        let mdk_vec = hex::decode(mdk_hex)?;
        let bytes: [u8; 32] = mdk_vec.as_slice().try_into().map_err(|_| anyhow!("Invalid MDK length"))?;
        Ok(bytes)
    }
}

#[cfg(test)]
static TEST_DB_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

#[cfg(test)]
mod wallet_manager_get_wallets_tests {
    use super::*;
    use sqlx::SqlitePool;
    use crate::database::init_test_pool;

    #[tokio::test]
    async fn get_wallets_filters_by_chain_type() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool.clone());
        service.init_tables().await.unwrap();

        sqlx::query("INSERT INTO wallet_groups (id, parent_id, name, chain_type, created_at, updated_at) VALUES (7, NULL, 'g7', 'evm', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO wallet_groups (id, parent_id, name, chain_type, created_at, updated_at) VALUES (8, NULL, 'g8', 'solana', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO wallets (group_id, name, address, chain_type, created_at, updated_at) VALUES (7, 'a', '0x1', 'evm', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO wallets (group_id, name, address, chain_type, created_at, updated_at) VALUES (8, 'b', 'S1', 'solana', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO wallets (group_id, name, address, chain_type, created_at, updated_at) VALUES (8, 'c', 'S2', ' SOLANA ', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();

        let all_solana = service.get_wallets(None, Some("solana".into()), None).await.unwrap();
        assert!(all_solana.iter().all(|w| w.chain_type.trim().eq_ignore_ascii_case("solana")));

        let g8_solana = service.get_wallets(Some(8), Some("solana".into()), None).await.unwrap();
        assert!(g8_solana.iter().all(|w| w.group_id == Some(8)));
        assert!(g8_solana.iter().all(|w| w.chain_type.trim().eq_ignore_ascii_case("solana")));

        let g8_evm = service.get_wallets(Some(8), Some("evm".into()), None).await.unwrap();
        assert!(g8_evm.is_empty());
    }
}

#[cfg(test)]
mod wallet_manager_secrets_tests {
    use super::*;
    use sqlx::SqlitePool;
    use crate::database::init_test_pool;

    #[tokio::test]
    async fn seal_and_open_roundtrip() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool);
        let sealed = service.seal_secret_for_transport("hello", "pass").unwrap();
        assert!(sealed.starts_with("p1:"));
        let opened = service.open_sealed_secret(&sealed, "pass").unwrap();
        assert_eq!(opened, "hello");
        assert!(service.open_sealed_secret("hello", "pass").is_err());
    }

    #[tokio::test]
    async fn migrate_plaintext_wallet_secrets_encrypts() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool.clone());
        service.init_tables().await.unwrap();
        service.init_password("pass").await.unwrap();

        sqlx::query("INSERT INTO wallets (group_id, name, address, chain_type, encrypted_private_key, encrypted_mnemonic, created_at, updated_at) VALUES (NULL, 'n', '0x1', 'evm', 'plain_private_key', 'plain mnemonic', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&pool)
            .await
            .unwrap();

        assert!(service.unlock("pass").await.unwrap());

        let row = sqlx::query("SELECT encrypted_private_key, encrypted_mnemonic FROM wallets WHERE address = '0x1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let enc_pk: String = row.get("encrypted_private_key");
        let enc_mn: String = row.get("encrypted_mnemonic");
        assert!(service.looks_like_db_encrypted_field(&enc_pk));
        assert!(service.looks_like_db_encrypted_field(&enc_mn));
        assert_ne!(enc_pk, "plain_private_key");
        assert_ne!(enc_mn, "plain mnemonic");
    }

    #[tokio::test]
    async fn create_wallets_returns_sealed_secrets() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool.clone());
        service.init_tables().await.unwrap();
        service.init_password("pass").await.unwrap();

        let pk = format!("0x{}", "1".repeat(64));
        let sealed_pk = service.seal_secret_for_transport(&pk, "pass").unwrap();

        let created = service
            .create_wallets(
                CreateWalletsRequest {
                    group_id: None,
                    name: Some("w".into()),
                    chain_type: "evm".into(),
                    mode: CreateWalletsMode::PrivateKeyImport,
                    sealed_mnemonic: None,
                    sealed_private_key: Some(sealed_pk),
                    count: 1,
                    start_index: None,
                    word_count: None,
                    remark: None,
                    password: Some("pass".into()),
                    preview_limit: Some(1),
                    include_secrets: Some(true),
                    transport_token: None,
                },
                None,
            )
            .await
            .unwrap();

        assert_eq!(created.preview.len(), 1);
        let w = &created.preview[0];
        assert!(w.has_private_key);
        assert!(!w.has_mnemonic);
        let sealed_out = w.sealed_private_key.clone().unwrap();
        assert_ne!(sealed_out, pk.trim_start_matches("0x"));
        let opened = service.open_sealed_secret(&sealed_out, "pass").unwrap();
        assert_eq!(opened, pk.trim_start_matches("0x"));
    }

    #[tokio::test]
    async fn create_wallets_persists_encrypted_mnemonic_for_same_mnemonic() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool.clone());
        service.init_tables().await.unwrap();
        service.init_password("pass").await.unwrap();

        let count = 10u32;
        service
            .create_wallets(
                CreateWalletsRequest {
                    group_id: None,
                    name: Some("w".into()),
                    chain_type: "evm".into(),
                    mode: CreateWalletsMode::GenerateSameMnemonic,
                    sealed_mnemonic: None,
                    sealed_private_key: None,
                    count,
                    start_index: Some(0),
                    word_count: Some(12),
                    remark: None,
                    password: Some("pass".into()),
                    preview_limit: Some(0),
                    include_secrets: Some(false),
                    transport_token: None,
                },
                None,
            )
            .await
            .unwrap();

        let stored: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM wallets WHERE encrypted_mnemonic IS NOT NULL AND length(trim(encrypted_mnemonic)) > 0",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, count as i64);
    }

    #[tokio::test]
    async fn create_wallets_persists_encrypted_mnemonic_for_different_mnemonic() {
        let _guard = super::TEST_DB_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap();
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        init_test_pool(pool.clone());
        let service = WalletManagerService::new(pool.clone());
        service.init_tables().await.unwrap();
        service.init_password("pass").await.unwrap();

        let count = 7u32;
        service
            .create_wallets(
                CreateWalletsRequest {
                    group_id: None,
                    name: Some("w".into()),
                    chain_type: "evm".into(),
                    mode: CreateWalletsMode::GenerateDifferentMnemonic,
                    sealed_mnemonic: None,
                    sealed_private_key: None,
                    count,
                    start_index: Some(0),
                    word_count: Some(12),
                    remark: None,
                    password: Some("pass".into()),
                    preview_limit: Some(0),
                    include_secrets: Some(false),
                    transport_token: None,
                },
                None,
            )
            .await
            .unwrap();

        let stored: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM wallets WHERE encrypted_mnemonic IS NOT NULL AND length(trim(encrypted_mnemonic)) > 0",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, count as i64);
    }
}
