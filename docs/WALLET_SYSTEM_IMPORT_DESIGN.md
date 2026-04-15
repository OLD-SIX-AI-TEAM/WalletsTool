# 钱包系统导入功能设计文档

## 1. 概述

### 1.1 背景
当前钱包导入方式包括：
- **手动录入**：通过 WalletImportModal 逐行输入私钥/地址
- **文件导入**：通过 Excel/CSV 文件批量导入

需要新增**系统导入**方式，允许从钱包管理系统中快速选择并导入已管理的钱包到 Transfer、Balance 和浏览器自动化页面。

### 1.2 目标
- 将钱包管理数据无缝融入到 Transfer、Balance 和浏览器自动化页面
- 提供统一的系统导入体验
- 支持按链类型、分组、钱包类型筛选
- 保持与现有导入方式一致的用户体验

### 1.3 适用范围
- Transfer 页面（EVM + Solana）
- Balance 页面（EVM + Solana）
- 浏览器自动化页面的 WalletManager 组件

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    钱包管理系统 (WalletManager)                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   分组管理    │  │   钱包列表    │  │  导入/导出   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              新增：WalletSystemImportModal 组件              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  分组树选择   │  │  钱包列表    │  │  筛选/搜索   │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│ Transfer页面  │      │ Balance页面  │      │ 浏览器自动化  │
│  - EVM       │      │  - EVM       │      │  - WalletMgr │
│  - Solana    │      │  - Solana    │      │              │
└──────────────┘      └──────────────┘      └──────────────┘
```

### 2.2 组件关系图

```
WalletSystemImportModal
├── GroupTree (分组树形选择)
├── FilterBar (筛选栏)
│   ├── ChainTypeSelect (链类型选择)
│   ├── WalletTypeSelect (钱包类型选择)
│   └── SearchInput (搜索框)
├── WalletList (钱包列表)
│   └── VirtualScrollerTable (虚拟滚动表格)
└── ActionBar (操作栏)
    ├── SelectAll (全选)
    ├── ClearSelection (清空)
    └── ImportButton (导入按钮)
```

---

## 3. 组件设计

### 3.1 WalletSystemImportModal 组件

#### 3.1.1 文件位置
```
src/components/WalletSystemImportModal.vue
```

#### 3.1.2 Props 定义

> 说明：本项目前端以 `.js` 为主（非 TypeScript）。此处用「Vue props + JSDoc」表达约束，落地实现不引入 `.ts`。

```js
const props = defineProps({
  visible: { type: Boolean, default: false },

  ecosystem: { type: String, default: 'evm' },

  importMode: { type: String, default: 'address_only' },

  title: { type: String, default: '从系统钱包导入' },

  multiple: { type: Boolean, default: true },

  maxSelection: { type: Number, default: undefined },
})
```

#### 3.1.3 Events 定义

```js
const emit = defineEmits(['confirm', 'cancel', 'update:visible'])

/**
 * @typedef {Object} ImportedWallet
 * @property {number} id
 * @property {string=} name
 * @property {string} address
 * @property {string=} private_key       importMode='full' 时才会回填（仅内存）
 * @property {string=} mnemonic
 * @property {'evm'|'solana'} chain_type
 * @property {'full_wallet'|'address_only'} wallet_type
 * @property {number=} group_id
 * @property {string=} group_name
 * @property {string=} remark
 * @property {'wallet'|'watch_address'} source_type  数据来源：完整钱包 / 观察地址
 */
```

#### 3.1.4 内部状态

```js
// 加载状态
const loading = ref(false);
const groupsLoading = ref(false);
const walletsLoading = ref(false);

// 分组数据
const groups = ref([]);
const selectedGroupId = ref('all');

// 钱包数据（系统钱包 + 观察地址）
const wallets = ref([]);
const watchAddresses = ref([]);

// 列表统一后的可选项
const items = ref([]);
const selectedItemIds = ref([]);

// 筛选条件
const filterForm = reactive({
  chainType: 'all',
  walletType: 'all',
  sourceType: 'all',
  searchKeyword: ''
});

// 分页/虚拟滚动
const pagination = reactive({
  current: 1,
  pageSize: 50,
  total: 0
});
```

#### 3.1.5 方法定义

```js
// 加载分组列表
async function loadGroups();

// 加载系统钱包列表（完整钱包）
async function loadWallets();

// 加载观察地址列表（仅地址）
async function loadWatchAddresses();

// 处理分组选择
function handleGroupSelect(groupId);

// 处理钱包选择
function handleItemSelect(itemId, selected);

// 处理全选
function handleSelectAll(selected);

// 处理搜索
function handleSearch(keyword);

// 处理筛选条件变化
function handleFilterChange();

// 确认导入
async function handleConfirm();

// 取消/关闭
function handleCancel();

// 获取选中项的最终输出（Transfer 模式会按需取密钥）
async function buildConfirmPayload();
```

#### 3.1.6 UI 布局

```
┌─────────────────────────────────────────────────────────────┐
│  从系统钱包导入                                    [X]      │
├──────────────────┬──────────────────────────────────────────┤
│                  │  筛选: [全部链 ▼] [完整钱包 ▼]           │
│  📁 分组          │  搜索: [____________] [🔍]              │
│  ├── 全部        │                                          │
│  ├── EVM 钱包    │  ┌─────────────────────────────────────┐ │
│  │   ├── 主钱包  │  │ ☑ │ 名称    │ 地址           │ 类型 │ │
│  │   └── 测试钱包│  │───┼─────────┼────────────────┼──────┤ │
│  └── Solana 钱包 │  │ ☑ │ 钱包1   │ 0x1234...abcd  │ 完整 │ │
│                  │  │ ☐ │ 钱包2   │ 0x5678...efgh  │ 仅地址│ │
│                  │  │ ☑ │ 钱包3   │ 0xabcd...1234  │ 完整 │ │
│                  │  └─────────────────────────────────────┘ │
│                  │                                          │
├──────────────────┴──────────────────────────────────────────┤
│  已选择: 3 个钱包                               [取消] [确认] │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 页面集成方案

### 4.1 Transfer 页面集成

#### 4.1.1 修改位置
- `src/features/ethereum/transfer/pages/Transfer.vue`
- `src/features/solana/transfer/pages/Transfer.vue`

#### 4.1.2 集成点

**1. 添加导入按钮组**

在现有的"录入"按钮旁添加下拉菜单或按钮组：

```vue
<a-button-group>
  <a-button @click="openManualImport">
    <icon-plus /> 手动录入
  </a-button>
  <a-button @click="openFileImport">
    <icon-file /> 文件导入
  </a-button>
  <a-button type="primary" @click="openSystemImport">
    <icon-safe /> 从系统导入
  </a-button>
</a-button-group>
```

**2. 注册模态框组件**

```typescript
import WalletSystemImportModal from '@/components/WalletSystemImportModal.vue';

const systemImportVisible = ref(false);
const systemImportRef = ref<InstanceType<typeof WalletSystemImportModal>>();
```

**3. 处理导入数据**

```typescript
async function handleSystemImport(wallets: ImportedWallet[]) {
  // 验证导入数据
  if (!wallets || wallets.length === 0) {
    Notification.warning({ title: '未选择任何钱包' });
    return;
  }
  
  // 检查私钥可用性（Transfer 需要私钥）
  const invalidWallets = wallets.filter(w => !w.private_key);
  if (invalidWallets.length > 0) {
    Modal.confirm({
      title: '部分钱包无私钥',
      content: `有 ${invalidWallets.length} 个钱包仅包含地址，无法用于转账。是否继续导入其他钱包？`,
      onOk: () => doImport(wallets.filter(w => w.private_key)),
    });
    return;
  }
  
  doImport(wallets);
}

function doImport(wallets: ImportedWallet[]) {
  wallets.forEach((wallet, index) => {
    data.value.push({
      key: Date.now() + index,
      private_key: wallet.private_key,
      to_addr: '',           // 用户后续填写
      amount: '',
      plat_balance: '',
      coin_balance: '',
      exec_status: '待执行',
      error_msg: '',
    });
  });
  
  Notification.success({ 
    title: '导入成功', 
    content: `成功导入 ${wallets.length} 个钱包` 
  });
}
```

#### 4.1.3 调用示例

```vue
<template>
  <!-- ... 其他代码 ... -->
  
  <WalletSystemImportModal
    v-model:visible="systemImportVisible"
    :ecosystem="currentEcosystem"
    import-mode="full"
    :title="'从系统导入转账钱包'"
    @confirm="handleSystemImport"
    @cancel="systemImportVisible = false"
  />
</template>
```

### 4.2 Balance 页面集成

#### 4.2.1 修改位置
- `src/features/ethereum/balance/pages/Balance.vue`
- `src/features/solana/balance/pages/Balance.vue`

#### 4.2.2 集成点

**1. 添加导入按钮**

```vue
<a-space>
  <a-button @click="visible = true">
    <icon-edit /> 手动录入
  </a-button>
  <a-button @click="handleFileImport">
    <icon-upload /> 文件导入
  </a-button>
  <a-button type="primary" @click="systemImportVisible = true">
    <icon-safe /> 从系统导入
  </a-button>
</a-space>
```

**2. 处理导入数据**

```typescript
async function handleSystemImport(wallets: ImportedWallet[]) {
  if (!wallets || wallets.length === 0) return;
  
  // Balance 只需要地址，不需要私钥
  wallets.forEach((wallet, index) => {
    data.value.push({
      key: wallet.address,
      address: wallet.address,
      nonce: '',
      plat_balance: '',
      coin_balance: '',
      exec_status: '待查询',
      error_msg: '',
    });
  });
  
  Notification.success({ 
    title: '导入成功', 
    content: `成功导入 ${wallets.length} 个地址` 
  });
}
```

#### 4.2.3 调用示例

```vue
<WalletSystemImportModal
  v-model:visible="systemImportVisible"
  :ecosystem="currentEcosystem"
  import-mode="address_only"
  :title="'从系统导入查询地址'"
  @confirm="handleSystemImport"
/>
```

### 4.3 浏览器自动化页面集成

#### 4.3.1 修改位置
- `src/features/airdrop/components/WalletManager.vue`

#### 4.3.2 集成点

**1. 添加同步按钮**

```vue
<a-space>
  <a-button @click="openAddWalletModal">
    <icon-plus /> 添加钱包
  </a-button>
  <a-button type="primary" @click="openSystemSync">
    <icon-sync /> 从系统同步
  </a-button>
</a-space>
```

**2. 处理同步数据**

```typescript
async function handleSystemSync(wallets: ImportedWallet[]) {
  // 与现有钱包去重
  const existingAddresses = new Set(localWallets.value.map(w => w.address));
  const newWallets = wallets.filter(w => !existingAddresses.has(w.address));
  
  if (newWallets.length === 0) {
    Notification.info({ title: '没有新钱包需要同步' });
    return;
  }
  
  // 添加到本地钱包列表
  newWallets.forEach(wallet => {
    localWallets.value.push({
      id: wallet.id,
      address: wallet.address,
      name: wallet.name,
      chainType: wallet.chain_type,
      // ... 其他字段
    });
  });
  
  Notification.success({ 
    title: '同步成功', 
    content: `新增 ${newWallets.length} 个钱包` 
  });
}
```

---

## 5. 后端 API 设计

### 5.1 现有 API 评估

根据 `src-tauri/src/wallets_tool/wallet_manager/commands.rs` 分析：

| 命令 | 功能 | 状态 |
|------|------|------|
| `get_wallets` | 获取钱包列表 | ✅ 可用 |
| `get_groups` | 获取分组列表 | ✅ 可用 |
| `get_watch_addresses` | 获取观察地址列表（仅地址） | ✅ 可用 |
| `get_wallet_secrets` | 获取单个钱包密钥 | ✅ 可用 |
| `export_wallets` | 导出钱包（含明文私钥/助记词） | ✅ 可用（但不建议作为系统导入取密钥通道） |
| `get_wallet_transport_public_key` / `register_wallet_transport_key` | 传输密钥注册 | ✅ 可用 |

### 5.2 API 使用方案

#### 5.2.1 获取钱包列表

```typescript
// 调用现有命令
const wallets = await invoke('get_wallets', {
  group_id: selectedGroupId === 'all' ? null : selectedGroupId,
  chain_type: filterForm.chainType === 'all' ? null : filterForm.chainType,
});
```

#### 5.2.2 获取观察地址列表（仅地址）

```typescript
const watchAddresses = await invoke('get_watch_addresses', {
  group_id: selectedGroupId === 'all' ? null : selectedGroupId,
  chain_type: filterForm.chainType === 'all' ? null : filterForm.chainType,
});
```

#### 5.2.3 获取钱包私钥（Transfer 页面需要）

```typescript
// 推荐：使用 get_wallet_secrets（支持 transport_token 或 password 重新封装）
const secrets = await Promise.all(
  selectedWalletIds.map(id =>
    invoke('get_wallet_secrets', { id, password: null, transport_token })
  )
);

// 不推荐：export_wallets 返回明文私钥/助记词，且当前不支持 transport_token
// 仅用于“导出到文件”的明确交互，不用于系统导入弹窗的密钥获取。
```

### 5.3 可能需要新增的 API

#### 5.3.1 batch_get_wallet_secrets

**用途**：批量获取多个钱包的密钥信息，提升 Transfer 页面导入性能（避免 N 次 invoke）

**参数**：
```rust
#[derive(Debug, Deserialize)]
pub struct BatchGetWalletSecretsRequest {
    pub ids: Vec<i64>,
    pub password: Option<String>,
    pub transport_token: Option<String>,
}
```

**返回值**：
```rust
#[derive(Debug, Clone, Serialize)]
pub struct BatchWalletSecrets {
    pub id: i64,
    pub address: String,
    pub sealed_private_key: Option<String>,
    pub sealed_mnemonic: Option<String>,
}
```

---

## 6. 数据模型

### 6.1 前端数据模型

```js
/**
 * @typedef {Object} WalletGroup
 * @property {number} id
 * @property {number=} parent_id
 * @property {string} name
 * @property {('evm'|'solana')=} chain_type
 * @property {WalletGroup[]=} children
 */

/**
 * @typedef {Object} WalletInfo
 * @property {number} id
 * @property {number=} group_id
 * @property {string=} name
 * @property {string} address
 * @property {'evm'|'solana'} chain_type
 * @property {'full_wallet'|'address_only'} wallet_type
 * @property {boolean} has_private_key
 * @property {boolean} has_mnemonic
 * @property {number=} mnemonic_index
 * @property {string=} remark
 */

/**
 * @typedef {Object} WatchAddressInfo
 * @property {number} id
 * @property {number=} group_id
 * @property {string=} group_name
 * @property {string=} name
 * @property {string} address
 * @property {'evm'|'solana'} chain_type
 * @property {string=} remark
 */

/**
 * 系统导入弹窗统一展示的列表项（来自 wallet / watch_address）
 * @typedef {Object} SystemImportItem
 * @property {number} id
 * @property {'wallet'|'watch_address'} source_type
 * @property {string=} name
 * @property {string} address
 * @property {'evm'|'solana'} chain_type
 * @property {'full_wallet'|'address_only'} wallet_type
 * @property {number=} group_id
 * @property {string=} group_name
 * @property {boolean=} has_private_key
 * @property {boolean=} has_mnemonic
 * @property {string=} remark
 */

/**
 * 导入输出（给 Transfer / Balance / 浏览器自动化页面）
 * @typedef {Object} ImportedWallet
 * @property {number} id
 * @property {'wallet'|'watch_address'} source_type
 * @property {string=} name
 * @property {string} address
 * @property {string=} private_key
 * @property {string=} mnemonic
 * @property {'evm'|'solana'} chain_type
 * @property {'full_wallet'|'address_only'} wallet_type
 * @property {number=} group_id
 * @property {string=} group_name
 * @property {string=} remark
 */
```

### 6.2 后端数据模型

参考 `src-tauri/src/wallets_tool/wallet_manager/models.rs`：

```rust
// 钱包分组
pub struct WalletGroup {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub chain_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 钱包信息
pub struct WalletInfo {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: Option<String>,
    pub address: String,
    pub chain_type: String,
    pub wallet_type: String,
    pub has_private_key: bool,
    pub has_mnemonic: bool,
    pub sealed_private_key: Option<String>,
    pub sealed_mnemonic: Option<String>,
    pub remark: Option<String>,
}
```

---

## 7. 安全性设计

### 7.1 私钥安全传输

1. **最小暴露面**：列表加载阶段仅取钱包元信息，绝不返回明文密钥
2. **按需取密钥**：仅在用户点击“确认导入”且 importMode='full' 时获取密钥
3. **传输封装优先**：优先走 transport_token 封装（t1），无 token 时走 password 封装（p1）
4. **内存安全**：私钥/助记词仅在内存中短暂存在，使用后立即清空临时变量

### 7.2 密封密钥处理

```typescript
// 使用现有的 secretCrypto 工具
import { openSealedSecret } from '@/utils/secretCrypto';

async function decryptPrivateKey(sealedKey: string, password: string): Promise<string> {
  return await openSealedSecret(sealedKey, password);
}
```

### 7.3 会话管理

> 说明：目前代码中不存在 `get_session_password` 命令。系统导入弹窗的“解密上下文”建议采用以下最优策略：
>
> - **优先复用已解锁态的 transport_token**：若外部页面（如钱包管理）已初始化 transport token/aesKey，则通过 props 注入或全局 store 复用。
> - **兜底密码输入**：若无 token，则在确认导入时弹出密码输入（仅用于向后端请求 sealed secret 或解封装），不落盘、不缓存为“会话密码”。

建议新增通用工具模块（避免复制粘贴安全逻辑）：
- `src/utils/transportSecret.js`：封装 t1 生成/解封装逻辑（当前实现散落在 WalletManager 与 SecretRevealModal）

---

## 8. 性能优化

### 8.1 虚拟滚动

使用现有的 `VirtualScrollerTable` 组件处理大量钱包数据：

```vue
<VirtualScrollerTable
  :data="filteredWallets"
  :columns="walletColumns"
  :item-height="40"
  :visible-count="15"
  v-model:selected-keys="selectedWalletIds"
/>
```

### 8.2 懒加载

- 分组列表：一次性加载
- 钱包列表：按分组懒加载
- 私钥：仅在确认导入时按需获取

### 8.3 缓存策略

```typescript
// 可选：若后续引入缓存（Pinia 或组件内 Map），建议按 (group_id + chain_type + source_type) 作为 key
// - 列表数据可短 TTL 缓存
// - secrets 永不缓存，仅确认导入时现取现用
```

---

## 9. 错误处理

### 9.1 错误类型

| 错误类型 | 处理方式 |
|---------|---------|
| 密码错误 | 提示重新输入密码 |
| 网络错误 | 重试机制 + 错误提示 |
| 无权限 | 提示用户检查钱包权限 |
| 数据为空 | 友好提示引导用户添加钱包 |

### 9.2 错误处理示例

```typescript
async function loadWallets() {
  try {
    walletsLoading.value = true;
    const result = await invoke('get_wallets', { group_id: selectedGroupId.value });
    wallets.value = result;
  } catch (error) {
    Notification.error({
      title: '加载钱包失败',
      content: error.toString()
    });
  } finally {
    walletsLoading.value = false;
  }
}
```

---

## 10. 测试计划

### 10.1 功能测试

| 测试项 | 测试内容 | 预期结果 |
|--------|---------|---------|
| 分组筛选 | 选择不同分组 | 正确显示对应钱包 |
| 链类型筛选 | 切换 EVM/Solana | 正确筛选对应链钱包 |
| 钱包类型筛选 | 切换完整/仅地址 | 正确筛选对应类型 |
| 搜索功能 | 输入地址/名称 | 正确过滤结果 |
| 多选功能 | 选择多个钱包 | 正确记录选中状态 |
| 全选功能 | 点击全选 | 选中当前页所有钱包 |
| 导入功能 | 确认导入 | 正确填充到目标页面 |
| 密码验证 | 输入错误密码 | 提示密码错误 |

### 10.2 性能测试

| 测试项 | 测试数据量 | 预期性能 |
|--------|-----------|---------|
| 钱包列表加载 | 1000 个钱包 | < 1s |
| 虚拟滚动 | 10000 个钱包 | 流畅无卡顿 |
| 批量导入 | 500 个钱包 | < 3s |

### 10.3 兼容性测试

- EVM Transfer 页面
- Solana Transfer 页面
- EVM Balance 页面
- Solana Balance 页面
- 浏览器自动化 WalletManager

---

## 11. 实施计划

### 11.1 阶段划分

#### 阶段一：前端核心闭环（Balance 优先）（2-3 天）
- [ ] 新增 WalletSystemImportModal.vue（分组树/筛选/搜索/虚拟列表/多选）
- [ ] 对接 get_groups + get_wallets + get_watch_addresses（统一 items 列表）
- [ ] Balance(EVM/Solana) 集成系统导入（复用既有校验与去重策略）

#### 阶段二：Transfer 安全闭环（1-2 天）
- [ ] Transfer(EVM/Solana) 集成系统导入
- [ ] 确认导入时按需获取密钥：优先 transport_token，兜底 password
- [ ] 明文密钥仅进入 Transfer 内存数据源，不做任何持久化

#### 阶段三：性能与复用增强（1-2 天）
- [ ] 后端新增 batch_get_wallet_secrets，前端改为一次 invoke 批量取 sealed secrets
- [ ] 抽离 transportSecret 工具到 src/utils/transportSecret.js（复用 WalletManager/SecretRevealModal/系统导入）
- [ ] 浏览器自动化 WalletManager 增加“从系统同步”，按 address 去重，保留本地字段（如 proxy）

#### 阶段四：测试与体验打磨（1-2 天）
- [ ] 覆盖大数据量虚拟滚动、筛选、全选/清空、错误提示
- [ ] 覆盖密码错误、token 缺失、无私钥钱包导入 Transfer 的提示策略

### 11.2 总计时间

预计 **5-9 天** 完成全部开发和测试工作（若先仅交付 Balance/Transfer，可更快）。

---

## 12. 附录

### 12.1 相关文件

| 文件路径 | 说明 |
|---------|------|
| `src/components/WalletSystemImportModal.vue` | 新增组件 |
| `src/components/WalletImportModal.vue` | 现有手动导入组件 |
| `src/features/ethereum/transfer/pages/Transfer.vue` | EVM 转账页面 |
| `src/features/solana/transfer/pages/Transfer.vue` | Solana 转账页面 |
| `src/features/ethereum/balance/pages/Balance.vue` | EVM 余额页面 |
| `src/features/solana/balance/pages/Balance.vue` | Solana 余额页面 |
| `src/features/airdrop/components/WalletManager.vue` | 浏览器自动化钱包管理 |
| `src/utils/transportSecret.js` | 建议新增：t1 传输封装工具（供系统导入复用） |
| `src-tauri/src/wallets_tool/wallet_manager/commands.rs` | 后端命令 |
| `src-tauri/src/wallets_tool/wallet_manager/models.rs` | 后端模型 |

### 12.2 依赖组件

| 组件 | 来源 |
|------|------|
| VirtualScrollerTable | `src/components/VirtualScrollerTable.vue` |
| TitleBar | `src/components/TitleBar.vue` |
| Arco Design Vue | 第三方 UI 库 |

### 12.3 参考文档

- [WALLET_MANAGER_ADD_WALLET_DESIGN.md](./WALLET_MANAGER_ADD_WALLET_DESIGN.md) - 钱包添加功能设计
- [WALLET_ENCRYPTION.md](./WALLET_ENCRYPTION.md) - 钱包加密设计
- [未来功能开发计划书.md](./未来功能开发计划书.md) - 功能开发计划

---

## 13. 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|---------|------|
| 1.0 | 2026-01-31 | 初始版本 | Claude |
| 1.1 | 2026-01-31 | 融合最优解：支持观察地址、调整密钥获取策略、优化批量 secrets API、修正文档为 JS 约定 | GPT-5.2 |

---

*文档结束*
