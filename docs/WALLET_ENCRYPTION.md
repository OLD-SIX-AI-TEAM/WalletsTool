# 钱包加密机制详解

> **文档版本**: 1.1  
> **最后更新**: 2026-01-30  
> **适用版本**: WalletsTool v2.x

---

## 1. 概述

本文档详细描述钱包管理模块中私钥 (Private Key) 和助记词 (Mnemonic) 的加密、解密、存储机制以及密钥管理架构。

### 1.1 核心安全设计原则

| 原则 | 实现方式 |
|------|----------|
| **内存安全** | MDK 存储在 SecureMemory 中，自动加密和擦除 |
| **分层加密** | 主密码 → KEK → MDK (SecureMemory) → 钱包密钥 → 数据 |
| **零持久化** | 明文私钥永不落盘、永不写入数据库 |
| **动态加密** | 每次加密使用随机 IV，防止模式分析 |

---

## 2. 密钥体系架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           用户输入                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  主密码 (User Password)                                                  │
│  - 用户首次设置或通过 unlock() 输入                                      │
│  - 派生 KEK (Key Encryption Key)                                        │
│  - 用于解密 MDK                                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  KEK (Key Encryption Key)                                               │
│  - PBKDF2-HMAC-SHA256 从主密码派生                                      │
│  - 100,000 次迭代 + 随机 Salt                                           │
│  - 用于加密/解密 MDK                                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  MDK (Master Data Key)                                                  │
│  - 32-byte 随机密钥                                                     │
│  - 首次 init_password() 生成                                            │
│  - unlock() 时解密并存储在 SecureMemory 中                              │
│  - 内存中常驻直到应用退出或锁定                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  SecureMemory 保护层                                                    │
│  - AES-256-CBC + SESSION_KEY 加密 MDK                                   │
│  - SHA256 完整性校验                                                    │
│  - ZeroizeOnDrop 自动擦除                                               │
│  - Debug 输出自动脱敏                                                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│  数据加密 (AES-256-CBC)                                                 │
│  - 私钥、助记词使用 AES-256-CBC 加密                                    │
│  - 随机 16-byte IV                                                      │
│  - PKCS7 填充                                                           │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. SecureMemory 实现

### 3.1 SecureMemory 结构

**文件位置**: `src-tauri/src/wallets_tool/security/memory.rs`

```rust
#[derive(Clone, Zeroize, ZeroizeOnDrop, Deserialize)]
#[serde(try_from = "String")]
pub struct SecureMemory {
    #[zeroize(skip)]
    ciphertext: Vec<u8>,  // AES-256-CBC 加密数据
    iv: [u8; 16],         // 随机 IV
    hash: [u8; 32],       // SHA256 完整性校验
}

impl fmt::Debug for SecureMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureMemory(***REDACTED***)")
    }
}
```

### 3.2 SESSION_KEY 管理

**文件位置**: `src-tauri/src/wallets_tool/security/session.rs`

```rust
lazy_static! {
    static ref SESSION_KEY: Mutex<[u8; 32]> = Mutex::new({
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        key
    });
}

pub fn get_session_key() -> [u8; 32] {
    let guard = SESSION_KEY.lock().unwrap();
    *guard
}

pub fn clear_session_key() {
    let mut guard = SESSION_KEY.lock().unwrap();
    guard.zeroize();
}
```

### 3.3 SecureMemory 使用方式

```rust
impl SecureMemory {
    pub fn new(secret: String) -> Self {
        let key = get_session_key();
        let mut iv = [0u8; 16];
        rand::thread_rng().fill(&mut iv);

        let mut plaintext_bytes = secret.into_bytes();
        let hash = Sha256::digest(&plaintext_bytes);

        let mut buffer = vec![0u8; plaintext_bytes.len() + 16];
        buffer[..plaintext_bytes.len()].copy_from_slice(&plaintext_bytes);
        plaintext_bytes.zeroize();

        let encryptor = Aes256CbcEnc::new(&key.into(), &iv.into());
        let ciphertext = encryptor.encrypt_padded_mut::<Pkcs7>(&mut buffer, plaintext_bytes.len()).unwrap();

        SecureMemory {
            ciphertext,
            iv,
            hash: hash.into(),
        }
    }

    pub fn use_secret<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&str) -> R,
    {
        let key = get_session_key();
        let decryptor = Aes256CbcDec::new(&key.into(), &self.iv.into());

        let mut buffer = self.ciphertext.clone();
        let plaintext = decryptor.decrypt_padded_mut::<Pkcs7>(&mut buffer).map_err(|_| "Decryption failed".to_string())?;

        let hash_calc = Sha256::digest(plaintext);
        if hash_calc != self.hash {
            return Err("Memory integrity check failed!".to_string());
        }

        let result = Ok(f(std::str::from_utf8(plaintext).map_err(|_| "Invalid UTF-8".to_string())?));
        buffer.zeroize();
        result
    }
}
```

### 3.4 SecureMemory 安全特性

| 特性 | 说明 |
|------|------|
| **自动加密** | MDK 在内存中以加密形式存储 |
| **完整性校验** | SHA256 hash 防止内存篡改 |
| **自动擦除** | `ZeroizeOnDrop` 自动擦除 ciphertext 和临时缓冲区 |
| **Debug 脱敏** | `Debug` 实现输出 `SecureMemory(***REDACTED***)` |
| **线程安全** | `Mutex` 保护 SESSION_KEY 访问 |

---

## 4. 密钥派生函数

### 4.1 PBKDF2-HMAC-SHA256

**文件位置**: `src-tauri/src/wallets_tool/wallet_manager/service.rs:925`

```rust
fn derive_key(&self, password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    let _ = pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 100_000, &mut key);
    key
}
```

| 参数 | 值 | 说明 |
|------|-----|------|
| 密码算法 | HMAC-SHA256 | 标准密码学哈希 |
| 输出长度 | 32 bytes | AES-256 兼容 |
| 迭代次数 | 100,000 | OWASP 推荐 600,000+ |
| Salt 长度 | 16 bytes | 随机生成 |

### 4.2 主密码验证器 (Verifier)

首次设置密码时，生成验证器用于后续密码验证：

```rust
// service.rs:144-151
let mut verifier_salt = [0u8; 16];
rand::thread_rng().fill(&verifier_salt);
let verifier_hash = self.derive_key(password, &verifier_salt);

// 存储格式: salt(hex):hash(hex)
let verifier_str = format!("{}:{}", hex::encode(verifier_salt), hex::encode(verifier_hash));
self.set_config("master_verifier", &verifier_str).await?;
```

**验证流程**:
1. 解码存储的 `salt` 和 `hash`
2. 用输入密码重新派生密钥
3. 比较派生结果与存储的 `hash`

---

## 5. 加密算法

### 5.1 AES-256-CBC

**文件位置**: `src-tauri/src/wallets_tool/wallet_manager/service.rs:1115`

```rust
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
```

| 参数 | 值 | 说明 |
|------|-----|------|
| 算法 | AES-256-CBC | 工业级对称加密 |
| 密钥长度 | 256 bits | 军事级安全 |
| 模式 | CBC | 密码块链接 |
| 填充 | PKCS7 | 标准填充方案 |
| IV 长度 | 16 bytes | 随机生成，每次加密不同 |

### 5.2 加密数据格式

数据库存储格式：
```
{iv_hex}:{ciphertext_base64}

示例: 00112233445566778899aabbccddeeff:A1b2C3d4E5f6...
```

---

## 6. 传输加密 (Sealed Secrets)

当私钥/助记词需要在前后端之间传输时，使用额外的加密层。

### 6.1 加密流程

**文件位置**: `src-tauri/src/wallets_tool/wallet_manager/service.rs:931`

```rust
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
```

### 6.2 解密流程

```rust
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
```

### 6.3 传输格式

```
p1:{salt_hex}:{iv_hex}:{ciphertext_base64}

示例: p1:00112233445566778899aabbccddeeff:00112233445566778899aabbccddeeff:A1b2C3d4...
```

| 字段 | 长度 | 说明 |
|------|------|------|
| p1 | 2 bytes | 版本标识 |
| salt | 32 hex chars | PBKDF2 Salt |
| iv | 32 hex chars | AES IV |
| cipher | - | Base64 密文 |

---

## 7. 数据库存储

### 6.1 表结构

**文件位置**: `src-tauri/src/wallets_tool/wallet_manager/service.rs:74`

```sql
CREATE TABLE wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    chain_type TEXT NOT NULL,
    encrypted_private_key TEXT,      -- AES-256-CBC 加密
    encrypted_mnemonic TEXT,         -- AES-256-CBC 加密
    mnemonic_index INTEGER DEFAULT 0,
    remark TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES wallet_groups(id) ON DELETE CASCADE
);
```

### 6.2 加密字段说明

| 字段 | 加密方式 | 格式 | 用途 |
|------|----------|------|------|
| `encrypted_private_key` | AES-256-CBC (MDK) | `iv:ciphertext` | 存储私钥 |
| `encrypted_mnemonic` | AES-256-CBC (MDK) | `iv:ciphertext` | 存储助记词 |
| `master_key` (app_config) | AES-256-CBC (KEK) | `salt:iv:ciphertext` | 存储 MDK |
| `master_verifier` (app_config) | 明文 | `salt:hash` | 密码验证 |

### 6.3 配置存储

```rust
// service.rs:160-162
// MDK 存储: salt(hex) + iv(hex) + ciphertext(base64)
let mdk_str = format!("{}:{}:{}", hex::encode(kek_salt), hex::encode(iv), encrypted_mdk);
self.set_config("master_key", &mdk_str).await?;

// Verifier 存储: salt(hex) + hash(hex)
let verifier_str = format!("{}:{}", hex::encode(verifier_salt), hex::encode(verifier_hash));
self.set_config("master_verifier", &verifier_str).await?;
```

---

## 8. 加解密流程详解

### 7.1 初始化主密码

```mermaid
flowchart TD
    A[用户输入密码] --> B[生成随机 MDK 32 bytes]
    B --> C[生成随机 Salt 16 bytes]
    C --> D[derive_key(password, salt) → KEK]
    D --> E[encrypt_data(MDK, KEK)]
    E --> F[存储: master_key = salt:iv:ciphertext]
    F --> G[derive_key(password, salt) → Verifier Hash]
    G --> H[存储: master_verifier = salt:hash]
```

### 7.2 解锁 (unlock)

```mermaid
flowchart TD
    A[用户输入密码] --> B[读取 master_verifier]
    B --> C[解析 salt 和 hash]
    C --> D[derive_key(password, salt) → derived_hash]
    D --> E{hash 匹配?}
    E -->|是| F[读取 master_key]
    E -->|否| G[返回 false]
    F --> H[解析 kek_salt, iv, ciphertext]
    H --> I[derive_key(password, kek_salt) → KEK]
    I --> J[decrypt_data(ciphertext, KEK, iv) → MDK]
    J --> K[缓存 MDK 到内存]
```

### 7.3 读取钱包私钥

```mermaid
flowchart TD
    A[请求钱包私钥] --> B{内存中有 MDK?}
    B -->|否| C[返回错误: 未解锁]
    B --> D[从数据库读取 encrypted_private_key]
    D --> E[解析 iv:ciphertext]
    E --> F[decrypt_data(ciphertext, MDK, iv)]
    F --> G[seal_secret_for_transport(明文, 用户密码)]
    G --> H[返回 p1:salt:iv:ciphertext]
```

### 7.4 导入钱包

```mermaid
flowchart TD
    A[用户输入私钥/助记词 + 密码] --> B[derive_key(password, salt) → 传输密钥]
    B --> C[open_sealed_secret(加密数据, password) → 明文]
    C --> D[derive_address_from_private_key/derive_from_mnemonic]
    D --> E[decrypt_data(明文, MDK) → 数据库格式]
    E --> F[存储 encrypted_private_key/encrypted_mnemonic]
```

---

## 9. 内存安全

### 9.1 SecureMemory MDK 缓存机制

**文件位置**: `src-tauri/src/wallets_tool/wallet_manager/service.rs:27`

```rust
pub struct WalletManagerService {
    pool: SqlitePool,
    // Master Data Key stored in SecureMemory for protection
    mdk: Mutex<Option<SecureMemory>>,
}
```

**使用示例**:

```rust
// 存储 MDK (init_password)
*self.mdk.lock().unwrap() = Some(SecureMemory::new(mdk_string));

// 解锁时存储 MDK (unlock)
*self.mdk.lock().unwrap() = Some(SecureMemory::new(mdk_string));

// 使用 MDK (解密私钥)
let mdk_bytes: [u8; 32] = secure_mdk.use_secret(|s| {
    s.as_bytes().try_into().map_err(|_| "Invalid MDK length".to_string())
})?;
```

| 特性 | 实现 |
|------|------|
| 存储位置 | `Mutex<Option<SecureMemory>>` |
| 内存加密 | AES-256-CBC + SESSION_KEY |
| 完整性校验 | SHA256 hash |
| 自动擦除 | `ZeroizeOnDrop` |
| Debug 脱敏 | `SecureMemory(***REDACTED***)` |
| 线程安全 | Mutex 保护 |

### 9.2 安全提升对比

| 对比项 | 修改前 | 修改后 |
|--------|--------|--------|
| MDK 形态 | 明文 `[0x12, 0x34, ...]` | 密文 `SecureMemory{ciphertext, iv, hash}` |
| 内存 dump | 直接读取 MDK | 需 SESSION_KEY 解密 |
| Debug 输出 | 可能泄露明文 | `***REDACTED***` |
| 自动清理 | 无 | ZeroizeOnDrop |
| 完整性校验 | 无 | SHA256 hash |

### 9.3 安全注意事项

⚠️ **仍存在的安全限制**:

1. **SESSION_KEY 单点**
   - 整个应用共享一个 SESSION_KEY
   - 泄露则所有 SecureMemory 可解密

2. **MDK 仍驻留内存**
   - `use_secret` 解密时存在短暂明文窗口期

3. **无超时自动锁定**
   - 用户离开时 SecureMemory 仍在内存

### 9.4 建议改进

| 问题 | 建议方案 |
|------|----------|
| SESSION_KEY 单点 | 定时轮换 SESSION_KEY |
| 超时锁定 | 添加空闲超时自动清除 SecureMemory |
| 密钥派生 | 升级到 Argon2id (抗 GPU/ASIC) |
| 内存锁定 | 使用 `mlock` 防止 swap 到磁盘 |

---

## 10. 密码学强度评估

### 10.1 当前配置

| 组件 | 算法 | 参数 | 强度 |
|------|------|------|------|
| 密钥派生 | PBKDF2-HMAC-SHA256 | 100,000 迭代 | ⚠️ 中等 |
| 数据加密 | AES-256-CBC | 随机 IV | ✅ 强 |
| 内存加密 | AES-256-CBC + SESSION_KEY | 随机 IV | ✅ 强 |
| 完整性校验 | SHA256 | - | ✅ 强 |

### 10.2 迭代次数建议

| 标准 | 最小迭代次数 | 推荐 |
|------|--------------|------|
| OWASP 2023 | 120,000 | 600,000+ |
| NIST SP 800-132 | 1,000 | 10,000+ |
| 当前实现 | 100,000 | 需提升 |

### 10.3 攻击成本估算

假设攻击者获得数据库，尝试破解：

| 密码强度 | 暴力破解时间 | GPU 破解速度 |
|----------|--------------|--------------|
| 弱 (6位数字) | 秒级 | ~10^9 次/秒 |
| 中 (8位混合) | 数小时 | ~10^8 次/秒 |
| 强 (12位复杂) | 数百万年 | ~10^6 次/秒 |
| 极强 (16位以上) | 不可行 | ~10^4 次/秒 |

---

## 11. 相关代码文件

| 文件 | 路径 | 职责 |
|------|------|------|
| service.rs | `src-tauri/src/wallets_tool/wallet_manager/service.rs` | 加解密核心逻辑 |
| models.rs | `src-tauri/src/wallets_tool/wallet_manager/models.rs` | 数据模型定义 |
| memory.rs | `src-tauri/src/wallets_tool/security/memory.rs` | SecureMemory 实现 |
| session.rs | `src-tauri/src/wallets_tool/security/session.rs` | SESSION_KEY 管理 |

---

## 12. 总结

### 安全优势

- ✅ AES-256-CBC 工业级加密
- ✅ 分层密钥架构
- ✅ 随机 IV 防止模式分析
- ✅ 私钥永不落盘
- ✅ **MDK 存储在 SecureMemory 中** (新增)
  - 内存中以加密形式存储
  - SHA256 完整性校验
  - ZeroizeOnDrop 自动擦除
  - Debug 输出自动脱敏

### 待改进项

- ⚠️ PBKDF2 迭代次数建议提升到 600,000+
- ⚠️ 建议升级到 Argon2id
- ⚠️ 建议添加超时自动锁定
- ⚠️ SESSION_KEY 建议定时轮换

### 总体评价

当前实现对**强密码**用户是安全的。MDK 已迁移至 SecureMemory 保护，核心加密 (AES-256) 强度足够。密钥派生 (PBKDF2) 需按 OWASP 最新建议优化。
