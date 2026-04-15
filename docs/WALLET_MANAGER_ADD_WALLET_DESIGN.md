## 钱包管理「添加钱包 / 批量生成」改造设计

### 目标

1. 「地址」不再由用户填写，统一由后端根据导入/生成的密钥自动计算并落库。
2. 「助记词 / 私钥」二选一（互斥），并支持助记词一次派生多个钱包。
3. 在「添加钱包」窗口内增加「批量生成钱包」能力：
   - 同助记词模式：生成 1 条助记词，派生 N 个钱包；
   - 不同助记词模式：生成 N 条助记词，各派生 1 个钱包（序号固定为 0）。

### 前端交互设计（WalletManager.vue）

#### 弹窗结构

- 弹窗标题：添加钱包
- 顶部 Tabs：
  - 导入/添加
  - 批量生成

#### 导入/添加 Tab

- 链类型：EVM / Solana
- 组名称：级联选择
- 新增子分组：可选
- 钱包名称：可选；当导入助记词并选择多钱包数量时，该字段作为「名称前缀」使用
- 导入方式：Radio（助记词 / 私钥）
  - 助记词：
    - 助记词输入（必填）
    - 钱包数量（默认 1）
    - 起始序号（默认 0）
  - 私钥：
    - 私钥输入（必填）
- 地址：展示但禁用，提示「自动生成并保存」
- 备注：可选

#### 批量生成 Tab

- 生成模式：Radio（同助记词 / 不同助记词）
- 词数：Select（12 / 24，默认 12）
- 钱包数量：
  - 同助记词：表示派生钱包数量
  - 不同助记词：表示助记词数量（每条助记词派生 1 个钱包）
- 起始序号：仅同助记词模式需要（默认 0）
- 名称前缀：可选（批量时自动拼接序号）
- 地址：不展示输入
- 生成结果：表格展示（地址 + 序号 + 助记词/私钥脱敏显示，可一键复制）

### 后端接口设计（Tauri Commands）

新增一个统一的批量创建命令（同时覆盖单个添加、助记词派生与批量生成）：

- `create_wallets(request: CreateWalletsRequest) -> Result<Vec<WalletInfo>, String>`

保留兼容命令：

- `create_wallet(request: CreateWalletRequest) -> Result<i64, String>`
  - 兼容旧调用；内部转发到 `create_wallets`（count=1），并执行地址自动计算与互斥校验。

#### CreateWalletsRequest（核心字段）

- group_id: Option<i64>
- chain_type: String ('evm' | 'solana')
- mode: String
  - mnemonic_import：导入助记词，派生 count 个钱包（序号从 start_index 开始）
  - private_key_import：导入私钥（count 固定为 1）
  - generate_same_mnemonic：生成 1 条助记词并派生 count 个钱包
  - generate_different_mnemonic：生成 count 条助记词，每条派生 1 个钱包
- sealed_mnemonic: Option<String>
- sealed_private_key: Option<String>
- count: u32
- start_index: Option<u32>
- word_count: Option<u32>（12/24，生成模式用）
- name: Option<String>（单钱包名称 / 批量名称前缀）
- remark: Option<String>
- password: String（用于解锁并获取 MDK，加密入库）

sealed_* 传输格式：

- `p1:{salt_hex}:{iv_hex}:{ciphertext_base64}`
- 其中 key = PBKDF2-HMAC-SHA256(password, salt, 100_000) 得到 32B，再用 AES-256-CBC 加密明文。

#### 地址计算规则

- EVM：
  - private_key_import：解析十六进制私钥（支持 0x 前缀）→ 计算地址
  - mnemonic_*：使用标准路径 `m/44'/60'/0'/0/{index}` 派生私钥 → 计算地址
- Solana：
  - private_key_import：解析 bs58 keypair bytes → 计算 pubkey 地址
  - mnemonic_*：使用 SLIP-0010 Ed25519 派生（仅 hardened）：
    - 路径：`m/44'/501'/{index}'/0'`
    - 派生出的 32-byte seed → Keypair → 地址

### 数据库结构设计（SQLite）

现有 wallets 表新增字段：

- mnemonic_index INTEGER NULL

迁移策略：

- 仍由 `init_wallet_manager_tables` 运行时创建/迁移。
- 使用 `ALTER TABLE wallets ADD COLUMN mnemonic_index INTEGER` 做列存在性兜底（与现有 remark/chain_type 风格一致）。

写入规则：

- 私钥导入：encrypted_private_key 非空；encrypted_mnemonic 为空；mnemonic_index 为空
- 助记词派生：encrypted_private_key 与 encrypted_mnemonic 均非空；mnemonic_index 为派生序号

### 返回结构（WalletInfo）

- 列表查询（get_wallets）：不返回任何明文私钥/助记词，仅返回 `has_private_key/has_mnemonic` 作为状态展示
- 批量生成（create_wallets）：返回 `sealed_private_key/sealed_mnemonic` 供前端在内存中解密后做脱敏展示与复制

### 校验与约束

- 助记词与私钥互斥：同时提供或同时为空均报错。
- 地址一致性：若请求仍带 address 且不为空，可校验其与计算地址一致（不一致报错）。
- 批量数量限制：前端限制默认最大 100（可后续调整）；后端也做上限保护以避免误操作。
