# WalletsTool 初始化与恢复出厂设置文档

## 概述

WalletsTool 采用**双数据库架构**，将公开配置数据与敏感钱包数据分离存储，实现安全与性能的平衡。

### 核心设计原则

- **公开数据库 (public.db)**: 未加密，存储链配置、RPC节点、代币信息等公开数据
- **安全数据库 (secure.db)**: SQLCipher加密，存储钱包、私钥、助记词等敏感信息
- **分离存储**: 敏感数据与公开数据物理隔离，降低攻击面

---

## 一、数据库架构

### 1.1 文件结构

```
data/
├── public.db           # 公开数据库（未加密）
├── secure.db           # 安全数据库（SQLCipher加密）
└── wallets_tool.db     # 旧版兼容数据库（逐步迁移）
```

### 1.2 公开数据库 (public.db)

**存储内容**:
- 区块链配置 (chains) - 链ID、名称、图标、扫描URL等
- RPC节点配置 (rpc_providers) - RPC地址、优先级、健康状态
- 代币配置 (tokens) - 代币合约地址、ABI、精度
- 应用配置 (app_config) - 非敏感配置项
- 通知配置 (notification_configs) - 通知方式配置
- 监控配置 (monitor_configs/monitor_history) - 地址监控规则
- 代理配置 (proxy_configs) - HTTP/SOCKS5代理设置
- 迁移记录 (public_schema_migrations) - 版本化迁移记录

### 1.3 安全数据库 (secure.db)

**存储内容**:
- 主密码验证器 (master_config) - PBKDF2哈希验证
- 钱包分组 (wallet_groups) - 钱包分组信息
- 钱包数据 (wallets) - 钱包地址、私钥(加密)、助记词(加密)
- 空投配置 (airdrop_configs) - 空投任务配置
- 空投历史 (airdrop_history) - 执行记录

---

## 二、初始化流程

### 2.1 应用启动初始化

```
main.rs
  └── database::init_public_database()
        │
        ├── 检查 data 目录是否存在
        ├── 创建 unencrypted pool (public.db)
        ├── 执行 public_init.sql (包含所有表结构和初始数据)
        └── insert_default_chain_data() (空函数，向后兼容)
```

**代码入口**:
```rust
// src-tauri/src/database/public_init.rs
pub async fn init_public_database() -> Result<()> {
    std::fs::create_dir_all("data")?;

    let pool = create_unencrypted_pool(PUBLIC_DB_PATH).await?;

    // 执行 public_init.sql 中的所有语句（表结构 + 初始数据）
    execute_init_sql(&pool, PUBLIC_INIT_SQL).await?;

    // 保留调用以保持向后兼容（现在 public_init.sql 已包含所有数据）
    insert_default_chain_data(&pool).await?;

    DualDatabaseManager::init_public_pool(pool);
    DualDatabaseManager::init_secure_pool_placeholder();

    Ok(())
}

// src-tauri/src/database/public_init.rs
/// 插入默认链和 RPC 数据
/// 注意：现在完全由 public_init.sql 提供，此函数保留为空以保持向后兼容
async fn insert_default_chain_data(_pool: &SqlitePool) -> Result<()> {
    // public_init.sql 已包含所有默认链和 RPC 数据
    // 此函数保留为空，避免重复插入导致的数据不一致
    Ok(())
}
```

**关键说明**: 所有链配置、RPC节点、代币信息等初始数据现在完全由 `public_init.sql` 统一提供，避免了从多个数据源插入可能导致的数据不一致问题。

### 2.2 public_init.sql 执行内容

**位置**: `src-tauri/data/public_init.sql`

**执行顺序**:
1. **创建表结构**: chains, rpc_providers, tokens, app_config, notification_configs, monitor_configs, monitor_history, proxy_configs, public_schema_migrations
2. **插入索引**: 优化查询性能
3. **插入默认数据** (完全由本文件统一提供):
   - EVM链: Ethereum, BSC, Polygon, Arbitrum, Optimism, Base, Linea, Mantle, opBNB, Sei, zkSync Era 等
   - Solana链: Solana主网, Solana测试网
   - 各链默认RPC节点 (带优先级和健康状态)
   - 各链默认代币配置
   - Nexus, OKT Chain 等扩展链配置

**重要**: 所有初始数据统一由 `public_init.sql` 提供，`insert_default_chain_data()` 函数已保留为空（仅用于向后兼容），避免数据重复或不一致。

### 2.3 安全数据库初始化

**触发条件**: 用户首次设置主密码时

```rust
// src-tauri/src/database/secure_init.rs
pub async fn init_secure_database(password: &str) -> Result<()> {
    if Path::new(SECURE_DB_PATH).exists() {
        return Err(anyhow!("安全数据库已存在，请使用 unlock_secure_database"));
    }
    
    std::fs::create_dir_all("data")?;
    
    // 使用PBKDF2派生数据库密钥
    let db_key = derive_db_key(password)?;
    
    let pool = create_encrypted_pool(SECURE_DB_PATH, &db_key).await?;
    
    // 执行 secure_init.sql
    execute_init_sql(&pool, SECURE_INIT_SQL).await?;
    
    // 保存主密码验证器
    save_master_verifier(&pool, password).await?;
    
    DualDatabaseManager::update_secure_pool(Some(pool));
    
    Ok(())
}
```

### 2.4 secure_init.sql 执行内容

**位置**: `src-tauri/data/secure_init.sql`

**创建表结构**:
- master_config - 存储主密码验证器和加密配置

---

## 三、密码与加密机制

### 3.1 PBKDF2 密钥派生

```rust
// src-tauri/src/database/encryption.rs
const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_PREFIX: &[u8] = b"WalletsTool_SecureDB_v2_";

pub fn derive_db_key(password: &str) -> Result<String> {
    let salt = derive_salt(password);
    
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(
        password.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
        &mut key,
    )?;
    
    Ok(hex::encode(key))
}

fn derive_salt(password: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(SALT_PREFIX);
    hasher.update(password.as_bytes());
    hasher.finalize()[..16].to_vec()
}
```

### 3.2 SQLCipher 配置

```rust
// 连接后执行
PRAGMA key = '<derived_key>';           // 数据库密钥
PRAGMA cipher_page_size = 4096;          // 页面大小
PRAGMA kdf_iter = 600000;                # KDF迭代次数
PRAGMA cipher_memory_security = ON;      # 内存安全
PRAGMA foreign_keys = ON;               # 外键约束
```

### 3.3 主密码验证器

```rust
async fn save_master_verifier(pool: &SqlitePool, password: &str) -> Result<()> {
    // 生成随机盐
    let mut salt = [0u8; 16];
    openssl::rand::rand_bytes(&mut salt)?;
    let salt_b64 = BASE64.encode(&salt);
    
    // 计算SHA256哈希
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(password.as_bytes());
    let hash = hasher.finalize();
    let hash_b64 = BASE64.encode(&hash);
    
    // 格式: salt_base64:hash_base64
    let verifier = format!("{}:{}", salt_b64, hash_b64);
    
    sqlx::query("INSERT OR REPLACE INTO master_config (key, value) VALUES ('master_verifier', ?)")
        .bind(&verifier)
        .execute(pool)
        .await?;
    
    Ok(())
}
```

---

## 四、钱包管理初始化

### 4.1 钱包服务结构

```rust
// src-tauri/src/wallet_manager/service.rs
pub struct WalletManagerService<'a> {
    pool: &'a SqlitePool,
}

impl WalletManagerService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self;
    
    /// 初始化钱包管理表结构
    pub async fn init_tables(&self) -> Result<()>;
    
    /// 创建钱包分组
    pub async fn create_group(&self, name: &str, parent_id: Option<i64>) -> Result<i64>;
    
    /// 创建钱包
    pub async fn create_wallet(&self, request: CreateWalletRequest) -> Result<i64>;
}
```

### 4.2 钱包存储格式

**敏感数据加密**:
- 私钥: AES-256-CBC加密后存储
- 助记词: AES-256-CBC加密后存储
- 加密密钥: 动态派生，每次会话不同

```rust
// 加密示例
let encrypted = encrypt_data(plaintext, session_key)?;
let encrypted_hex = hex::encode(encrypted);
```

---

## 五、恢复出厂设置

### 5.1 执行流程

```rust
// src-tauri/src/database/mod.rs
#[tauri::command]
pub async fn reload_database() -> Result<String, String> {
    let config = load_database_config();
    let enable_debug = config.enable_debug_log.unwrap_or(false);

    if enable_debug {
        println!("开始恢复出厂设置...");
    }

    let public_db_path = "data/public.db";
    let secure_db_path = "data/secure.db";
    let legacy_db_path = "data/wallets_tool.db";

    // 1. 关闭所有数据库连接
    DualDatabaseManager::force_disconnect_all();
    if let Some(lock) = DATABASE_POOL.get() {
        let pool = lock.read().unwrap().clone();
        pool.close().await;
    }
    
    // 2. 等待文件锁释放
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 3. 删除数据库文件及相关文件(WAL/SHM)
    delete_db_files(public_db_path, enable_debug).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    delete_db_files(secure_db_path, enable_debug).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    delete_db_files(legacy_db_path, enable_debug).await?;

    // 4. 重新初始化公开数据库
    init_public_database().await.map_err(|e| e.to_string())?;

    Ok("恢复出厂设置完成".to_string())
}
```

### 5.2 删除辅助函数

```rust
async fn delete_db_files(db_path: &str, enable_debug: bool) -> Result<(), String> {
    let db_path = Path::new(db_path);

    // 删除 WAL 文件
    let wal_path = format!("{}-wal", db_path.display());
    if Path::new(&wal_path).exists() {
        tokio::fs::remove_file(wal_path).await?;
    }

    // 删除 SHM 文件
    let shm_path = format!("{}-shm", db_path.display());
    if Path::new(&shm_path).exists() {
        tokio::fs::remove_file(shm_path).await?;
    }

    // 删除主数据库文件（多次重试处理Windows文件锁）
    let max_retries = 5;
    for attempt in 1..=max_retries {
        match tokio::fs::remove_file(db_path).await {
            Ok(()) => return Ok(()),
            Err(e) if attempt < max_retries && e.raw_os_error() == Some(32) => {
                // Windows文件锁定错误，等待后重试
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(e) => return Err(format!("删除失败: {}", e)),
        }
    }
    
    Ok(())
}
```

### 5.3 恢复出厂设置效果

| 数据项 | 处理方式 |
|--------|----------|
| 公开数据库 (public.db) | 删除后重新执行 public_init.sql |
| 安全数据库 (secure.db) | 删除后重新初始化 |
| 旧版数据库 (wallets_tool.db) | 删除 |
| 用户钱包 | **全部清除，不可恢复** |
| 链配置 | 恢复默认值 |
| RPC节点 | 恢复默认值 |
| 代币配置 | 恢复默认值 |
| 监控配置 | 全部清除 |
| 代理配置 | 全部清除 |

**⚠️ 警告**: 恢复出厂设置将**永久删除所有钱包数据**，请确保已备份私钥或助记词。

---

## 六、数据库迁移机制

### 6.1 迁移注册

**位置**: `src-tauri/src/database/migrations.rs`

```rust
// 迁移注册示例
pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub check_sql: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    // Migration { version: 1, name: "init", ... },
    // Migration { version: 2, name: "add_column", ... },
];
```

### 6.2 迁移检查机制

```rust
// 检查是否需要执行迁移
async fn needs_migration(pool: &SqlitePool, migration: &Migration) -> Result<bool> {
    // 使用 check_sql 判断迁移条件
    let result: Option<i64> = sqlx::query_scalar(migration.check_sql)
        .fetch_optional(pool)
        .await?;
    
    // 如果 check_sql 返回 None 或 0，表示需要迁移
    Ok(result.unwrap_or(0) == 0)
}
```

### 6.3 迁移执行流程

1. 检查 `public_schema_migrations` 表中是否已应用该版本
2. 如果未应用，执行迁移SQL
3. 记录迁移历史
4. 执行前自动备份数据库

---

## 七、相关命令

### 7.1 Tauri 命令

| 命令 | 描述 | 参数 |
|------|------|------|
| `init_public_db` | 初始化公开数据库 | 无 |
| `init_secure_db` | 初始化安全数据库 | encrypted_password: String |
| `unlock_secure_db` | 解锁安全数据库 | encrypted_password: String |
| `lock_secure_db` | 锁定安全数据库 | 无 |
| `reload_database` | 恢复出厂设置 | 无 |
| `get_dual_database_status` | 获取数据库状态 | 无 |
| `check_database_schema` | 检查数据库结构 | 无 |
| `export_database_to_public_init_sql` | 导出数据库到SQL文件 | 无 |

### 7.2 前端调用示例

```javascript
import { invoke } from '@tauri-apps/api/tauri';

// 初始化公开数据库
await invoke('init_public_db');

// 初始化安全数据库（首次设置密码）
await invoke('init_secure_db', { encrypted_password: 'your_password' });

// 解锁安全数据库
await invoke('unlock_secure_db', { encrypted_password: 'your_password' });

// 恢复出厂设置
await invoke('reload_database');
```

---

## 八、配置文件

### 8.1 package.json 数据库配置

```json
{
  "config": {
    "database": {
      "forceInit": false,
      "enableDebugLog": false,
      "initSqlPath": "data/public_init.sql"
    }
  }
}
```

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| forceInit | 强制重新初始化 | false |
| enableDebugLog | 启用调试日志 | false |
| initSqlPath | public_init.sql路径 | data/public_init.sql |

---

## 九、最佳实践

### 9.1 开发时重置数据库

```bash
# 方式1: 使用reload_database命令
# 在前端界面执行恢复出厂设置

# 方式2: 手动删除数据库文件
rm -rf data/*.db data/*-wal data/*-shm
```

### 9.2 导出当前数据库配置

```rust
#[tauri::command]
pub async fn export_database_to_init_sql() -> Result<String, String> {
    // 导出 public.db 数据到 public_init.sql
    // 用于同步最新的RPC配置到代码仓库
}
```

### 9.3 更新默认RPC配置流程

1. 在开发环境中配置好RPC节点
2. 执行 `export_database_to_public_init_sql` 命令
3. 提交 public_init.sql 更改到代码仓库
4. 用户更新版本后自动获得新RPC配置

---

## 十、故障排除

### 10.1 常见错误

**"Secure database not initialized"**
- 原因: 安全数据库文件不存在
- 解决: 调用 `init_secure_db` 设置主密码

**"Database password error"**
- 原因: 输入的密码与初始化时不一致
- 解决: 使用正确的密码，或恢复出厂设置后重新初始化

**"File is locked" (Windows)**
- 原因: 数据库文件被其他进程锁定
- 解决: 关闭所有连接后重试，恢复出厂设置会自动处理

### 10.2 日志调试

启用调试日志:
```json
{
  "config": {
    "database": {
      "enableDebugLog": true
    }
  }
}
```

查看初始化过程中的详细日志输出。

---

## 十一、文件清单

### 11.1 核心文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/database/mod.rs` | 数据库模块主文件 |
| `src-tauri/src/database/public_init.rs` | 公开数据库初始化 |
| `src-tauri/src/database/secure_init.rs` | 安全数据库初始化 |
| `src-tauri/src/database/dual_database.rs` | 双数据库管理器 |
| `src-tauri/src/database/encryption.rs` | 加密工具 |
| `src-tauri/src/database/commands.rs` | Tauri命令 |
| `src-tauri/data/public_init.sql` | 公开数据库初始化SQL |
| `src-tauri/data/secure_init.sql` | 安全数据库初始化SQL |

### 11.2 辅助脚本

| 脚本 | 说明 |
|------|------|
| `scripts/export_db_to_sql.py` | 导出数据库到SQL文件 |
| `src-tauri/data/fix_sql.py` | SQL文件修复工具 |
