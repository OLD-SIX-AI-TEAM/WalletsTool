# WalletsTool Development Guide

**Generated:** 2026-01-31
**Version:** 0.4.6
**Branch:** dev

## OVERVIEW

Web3 multi-chain wallet desktop app (Vue 3 + Tauri/Rust). Ethereum/Solana, batch import/transfer/balance, RPC/token config, Excel I/O, SQLite local storage. **Security-first architecture**: 私钥/助记词仅以加密形式落库，绝不明文持久化。

## BUILD COMMANDS

```bash
# Frontend-only
yarn dev                  # Vite dev server (port 1422)
yarn build                # Vite production build

# Full stack
yarn tauri-dev            # Full dev stack (recommended)
yarn tauri-build          # Production build (.msi/.dmg/.deb)

# Backend (Rust)
cargo test                # Run all tests
cargo test -- --nocapture # Run tests with output
cargo test <test_name>    # Run single test by name

# Setup
yarn start                # Install deps + dev server (first-time setup)
yarn setup                # Install dependencies only
yarn version:update       # Update version in package.json
```

## RELEASE & UPDATER SIGNING

- **Workflow**: `.github/workflows/release.yml` triggers on tag `v*` and runs `tauri-action` to build bundles and updater artifacts.
- **Required GitHub Actions secrets**:
  - `TAURI_SIGNING_PRIVATE_KEY`: updater signing private key used by the bundler (`createUpdaterArtifacts=true`). Value can be the minisign private key content (multi-line) or a file path.
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for the private key (keep empty if the key has no password; avoid leading/trailing whitespace).
  - `TAURI_UPDATER_PUBKEY`: minisign public key (must match the private key). Release workflow passes it to `WALLETSTOOL_UPDATER_PUBKEY` for compile-time injection.
- **Updater artifacts**: ensure the latest GitHub Release contains `latest.json` (and signatures) as downloadable assets; the app’s updater endpoints point to `releases/latest/download/latest.json` (includes `https://gh-proxy.org/` acceleration for domestic networks).
- **Client behavior**: the desktop app checks updates on startup; if a newer version exists, it shows a modal with “立即更新/前往下载”，and falls back to GitHub Release check when the updater endpoint fails (GitHub API will retry via `https://gh-proxy.org/`).
- **Common failure**: `failed to decode secret key: incorrect updater private key password` usually means the password doesn’t match the private key, or the password secret contains extra whitespace/newline.

## CODE STYLE GUIDELINES

### General Principles

- **Self-documenting code**: Avoid comments unless explaining complex algorithms
- **Consistency**: Match existing patterns in the file/module
- **Security-first**: Never log or persist sensitive data (keys, mnemonics)

### Imports and Dependencies

**Frontend (JavaScript):**
```javascript
// Absolute imports for @ alias (src directory)
import { useTransfer } from '@/features/ethereum/transfer/composables/useTransfer'

// Relative imports for local modules
import { formatAddress } from '@/utils/format'

// Vue imports
import { ref, computed, onMounted } from 'vue'

// Component imports (named exports)
import { Button, Modal, Table } from '@arco-design/web-vue'
```

**Backend (Rust):**
```rust
// Standard module imports
use tauri::{Manager, Runtime};

// Internal imports
use crate::wallets_tool::security::SecureMemory;
```

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Vue composables | `camelCase` prefixed with `use` | `useTransfer`, `useWallet` |
| Vue components | `PascalCase` | `WalletList.vue`, `TransferModal.vue` |
| Variables | `camelCase` | `walletAddress`, `gasPrice` |
| Constants | `UPPER_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_RPC` |
| Database tables | `snake_case` | `wallet_groups`, `chain_configs` |
| Rust functions | `snake_case` | `get_chain_list`, `transfer_tokens` |
| Rust structs/enums | `PascalCase` | `ChainConfig`, `TransferStatus` |
| Chain types | lowercase string | `'evm'`, `'solana'` |

### Component Structure (Vue SFC)

```vue
<template>
  <!-- Template here -->
</template>

<script setup>
// Imports - alphabetical order by package then name
import { ref, computed, onMounted } from 'vue'
import { useWalletStore } from '@/stores/wallet'

// Props with defaults
const props = defineProps({
  modelValue: { type: String, default: '' },
  disabled: { type: Boolean, default: false }
})

// Emits
const emit = defineEmits(['update:modelValue', 'confirm'])

// Composables (use stores first)
const walletStore = useWalletStore()

// Refs and computed
const loading = ref(false)
const items = computed(() => walletStore.items)

// Methods (grouped logically, alphabetical within groups)

// Lifecycle
onMounted(() => {
  loadData()
})
</script>

<style scoped>
/* Local styles only */
</style>
```

### Error Handling

**Frontend:**
```javascript
try {
  await transferFunds(wallet)
} catch (error) {
  console.error('Transfer failed:', error.message)
  notify.error(`Transfer failed: ${error.message}`)
}
```

**Backend (Rust):**
```rust
// All Tauri commands return Result<T, String>
#[tauri::command]
async fn transfer<R: Runtime>(app: AppHandle<R>, request: TransferRequest) -> Result<TransferResult, String> {
    // Use ? operator or map_err for conversion
    let provider = get_provider(&request.chain_id).await.map_err(|e| e.to_string())?;
    
    // Explicit error handling for complex cases
    match execute_transfer(&provider, &request).await {
        Ok(tx) => Ok(tx),
        Err(e) => Err(format!("Transfer failed: {}", e)),
    }
}
```

### Type Handling

- **Frontend**: No TypeScript (`.js` files only). Use JSDoc for complex types if needed.
- **Backend**: Full Rust type safety required. No `unwrap()` on `Result` or `Option` in production code.
- Never suppress type errors: no `as any`, `@ts-ignore`, `.unwrap()` without context.

## STRUCTURE

```
./
├── src/                      # Vue 3 frontend (Feature-based)
│   ├── features/             # Business domains (Deep nesting)
│   │   ├── ethereum/         # EVM logic (transfer/balance/monitor)
│   │   ├── solana/           # SVM logic (transfer/balance)
│   │   ├── airdrop/          # Browser Automation & Scripting
│   │   ├── wallet_manager/   # Wallet management UI
│   │   └── home/             # Home page
│   ├── pages/                # Route targets (deprecated, use features/)
│   ├── components/           # Global UI components
│   │   ├── ChainIcon.vue
│   │   ├── ChainManagement.vue
│   │   ├── RpcManagement.vue
│   │   ├── TokenManagement.vue
│   │   ├── ProxyConfigModal.vue
│   │   ├── WalletImportModal.vue
│   │   ├── WalletSystemImportModal.vue
│   │   ├── SecretRevealModal.vue
│   │   ├── VirtualScrollerTable.vue
│   │   ├── TableSkeleton.vue
│   │   ├── TitleBar.vue
│   │   └── transfer/         # Transfer-related components
│   ├── composables/          # Global composables
│   ├── stores/               # Pinia stores
│   │   ├── index.js          # Global state (confetti, theme)
│   │   └── ecosystem.js      # Ecosystem switching
│   ├── router/               # Vue Router config
│   ├── utils/                # Utility functions
│   ├── main.js               # Entry (Plugins + Config)
│   └── App.vue               # Root + Global Events
├── src-tauri/                # Tauri backend (Library-style)
│   ├── src/
│   │   ├── main.rs           # Application entry
│   │   ├── utils.rs          # Common utilities
│   │   ├── database/         # SQLite Service Layer
│   │   │   ├── mod.rs        # DB manager, connection pool
│   │   │   ├── models.rs     # Data models
│   │   │   ├── chain_service.rs
│   │   │   └── rpc_service.rs
│   │   ├── plugins/          # Tauri plugin extensions
│   │   └── wallets_tool/     # Core Business Logic
│   │       ├── airdrop/      # Airdrop automation
│   │       ├── ecosystems/   # Chain implementations
│   │       │   ├── ethereum/ # EVM chain support
│   │       │   │   ├── chain_config.rs
│   │       │   │   ├── provider.rs
│   │       │   │   ├── rpc_management.rs
│   │       │   │   ├── proxy_manager.rs
│   │       │   │   ├── proxy_commands.rs
│   │       │   │   ├── transfer.rs
│   │       │   │   ├── token_transfer.rs
│   │       │   │   ├── simple_balance_query.rs
│   │       │   │   └── alloy_utils.rs
│   │       │   └── solana/   # Solana chain support
│   │       │       ├── provider.rs
│   │       │       └── transfer.rs
│   │       ├── playwright/   # Browser automation
│   │       ├── security/     # AES-256 Memory Guard
│   │       │   ├── mod.rs
│   │       │   ├── memory.rs
│   │       │   ├── protection.rs
│   │       │   └── session.rs
│   │       ├── update.rs     # GitHub Release update check
│   │       └── wallet_manager/  # Wallet CRUD + encryption
│   │           ├── mod.rs
│   │           ├── models.rs
│   │           ├── service.rs
│   │           └── commands.rs
│   └── data/                 # DB file + public_init.sql + secure_init.sql
├── docs/                     # Documentation
├── scripts/                  # Build/Version utilities
└── vcpkg/                    # C++ Dependencies
```

## WHERE TO LOOK

| Task | Location |
|------|----------|
| **Frontend Entry** | `src/main.js` |
| **Backend Entry** | `src-tauri/src/main.rs` |
| **Transfer Logic (Frontend)** | `src/features/{chain}/transfer/composables/useTransfer.ts` |
| **Transfer Logic (Backend)** | `src-tauri/src/wallets_tool/ecosystems/{chain}/transfer.rs` |
| **Balance Logic (Frontend)** | `src/features/{chain}/balance/` |
| **Balance Logic (Backend)** | `src-tauri/src/wallets_tool/ecosystems/ethereum/simple_balance_query.rs` |
| **Chain Backend** | `src-tauri/src/wallets_tool/ecosystems/` |
| **Security Core** | `src-tauri/src/wallets_tool/security/` |
| **Database Ops** | `src-tauri/src/database/` |
| **Wallet Manager** | `src-tauri/src/wallets_tool/wallet_manager/` |
| **Airdrop/Automation** | `src/features/airdrop/`, `src-tauri/src/wallets_tool/airdrop/` |
| **Browser Automation** | `src-tauri/src/wallets_tool/playwright/` |
| **Update Check** | `src/App.vue`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json` |
| **Chain Config** | `src/components/ChainManagement.vue` |
| **RPC Management** | `src/components/RpcManagement.vue` |
| **Token Management** | `src/components/TokenManagement.vue` |
| **Proxy Config** | `src/components/ProxyConfigModal.vue` |

## CONVENTIONS

- **Feature-First**: Frontend code lives in `src/features/{chain}/{domain}/`
- **Dual UI**: PrimeVue (Data/Lists) + Arco Design (Interactions/Modals)
- **JS over TS**: Frontend is strictly `.js` (composables may use `.ts`)
- **Async Backend**: All Tauri commands return `Result<T, String>`
- **Chain Filtering**: Filter `get_chain_list` results by `ecosystem` ('evm' | 'solana')
- **Encrypted Secrets**: 私钥/助记词传输使用加密封装（`p1:` 或 `t1:`），不走明文
- **Fury Mode**: 90+ concurrent threads for batch transfers
- **Virtual Scrolling**: Use `VirtualScrollerTable` for large datasets
- **Auto Updates**: 使用 Tauri updater 插件；Release workflow 需配置签名私钥 secrets，并通过 WALLETSTOOL_UPDATER_PUBKEY 注入公钥

## ANTI-PATTERNS

- **NEVER** persist private keys to disk or DB in plain text
- **NEVER** log sensitive data (keys, mnemonics)
- **NEVER** mix Pinia state with local feature state unnecessarily
- **NEVER** block the main thread; use `tokio` for heavy lifting
- **NEVER** remove `custom-protocol` from `tauri.conf.json`
- **NEVER** use `as any`, `@ts-ignore`, or `.unwrap()` to suppress errors
- **NEVER** hardcode RPC URLs; use chain config from database
- **NEVER** skip error handling in async functions

## SECURITY RULES

1. **Memory-Only Keys**: Private keys never persisted, only in RAM
2. **Dynamic Encryption**: AES-256-CBC encryption in memory
3. **Zeroize**: Immediate memory wipe after signing
4. **Encrypted Persistence**: If needed, use Master Data Key (PBKDF2 derived from user password)
5. **IPC 传输加密**: 密码可用 RSA-OAEP 加密传输；批量导入/预览 secrets 使用 AES-256-GCM(`t1:`) 传输
6. **SQLCipher**: Database encrypted with PBKDF2 (600,000 iterations)
7. **Anti-Debug**: Runtime protection against memory scanning (Windows)

## DATABASE MIGRATIONS

- `src-tauri/data/public_init.sql` 包含非敏感表结构和默认数据（链配置、RPC、代币等）
- `src-tauri/data/secure_init.sql` 包含敏感数据初始化（数据库加密密钥配置）
- 升级旧用户数据库必须走迁移机制
- 迁移脚本目录：`src-tauri/data/migrations/`，文件命名：`V0001__short_name.sql` 递增
- 迁移注册清单：`src-tauri/src/database/migrations.rs`（`version/name/check_sql/sql`）
- `check_sql` 用于判断迁移是否需要执行；即使已满足也会写入 `schema_migrations`，避免重复检查
- 自动执行时机：解锁/初始化真实加密数据库后自动执行版本化迁移，并在需要执行迁移时先生成 `*.bak.<timestamp>` 备份
- 变更流程：更新 `public_init.sql` 和 `secure_init.sql` → 新增迁移脚本（升级路径）→ 更新注册清单 → `cargo test`

## TOOLS AVAILABLE

| Tool | Purpose |
|------|---------|
| ESLint (via LSP) | JavaScript linting |
| cargo | Rust compilation and testing |
| vite | Frontend build |
| tauri | Desktop app packaging |
| playwright | E2E testing for frontend + backend integration |

## E2E TESTING FRAMEWORK

### Overview

The project uses **Playwright** for end-to-end testing of frontend + Tauri backend integration. Tests verify that frontend UI correctly interacts with Rust backend commands.

### Test Architecture

```
e2e/
├── playwright.config.ts      # Playwright configuration
├── global-setup.ts           # Pre-test setup
├── global-teardown.ts        # Post-test cleanup
├── tauri-helpers.ts          # Tauri-specific test utilities
├── wallet-manager.spec.ts    # Wallet manager integration tests
├── balance-query.spec.ts     # Balance query integration tests
└── api-integration.spec.ts   # API contract tests
```

### Running Tests

```bash
# Install Playwright browsers (first time)
npx playwright install chromium

# Run all tests (headless - CI mode)
npm run test:e2e

# Run with browser visible (development)
npm run test:e2e:headed

# Interactive UI mode (debugging)
npm run test:e2e:ui

# Debug mode with step-through
npm run test:e2e:debug

# View HTML report
npm run test:e2e:report

# Run specific test file
npx playwright test wallet-manager.spec.ts
```

### Core Test Utilities (`tauri-helpers.ts`)

#### invokeTauriCommand
Call Tauri backend commands from tests:

```typescript
import { invokeTauriCommand } from './tauri-helpers';

// Example: Get wallet list from backend
const wallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
  group_id: null,
  chain_type: null,
  password: null,
});

// Example: Create a new group
const newGroupId = await invokeTauriCommand<number>(page, 'create_group', {
  name: 'Test Group',
  parent_id: null,
});
```

#### waitForTauriApp
Wait for Tauri app to fully load:

```typescript
import { waitForTauriApp } from './tauri-helpers';

await page.goto('/#/wallet-manager');
await waitForTauriApp(page);  // Waits for __TAURI__ to be available
```

#### waitForLoadingComplete
Wait for all loading indicators to disappear:

```typescript
import { waitForLoadingComplete } from './tauri-helpers';

await waitForLoadingComplete(page);
```

### Writing New Tests

#### Basic Test Template

```typescript
import { test, expect } from '@playwright/test';
import {
  waitForTauriApp,
  invokeTauriCommand,
  waitForLoadingComplete,
} from './tauri-helpers';

test.describe('Feature Name', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/your-route');
    await waitForTauriApp(page);
    await waitForLoadingComplete(page);
  });

  test('should call backend command and verify response', async ({ page }) => {
    // Call backend command
    const result = await invokeTauriCommand(page, 'your_command', { arg: 'value' });
    
    // Verify response
    expect(result).toBeDefined();
    expect(Array.isArray(result)).toBe(true);
    
    // Verify UI updated correctly
    const element = page.locator('.your-selector');
    await expect(element).toBeVisible();
  });
});
```

#### Testing Frontend-Backend Data Consistency

```typescript
test('frontend and backend data should match', async ({ page }) => {
  // Get data from backend
  const backendData = await invokeTauriCommand<any[]>(page, 'get_wallets', {});
  
  // Get displayed data from frontend
  const rows = page.locator('table tbody tr');
  const frontendCount = await rows.count();
  
  // Verify consistency
  expect(frontendCount).toBe(backendData.length);
});
```

#### Testing Error Handling

```typescript
test('should handle backend errors gracefully', async ({ page }) => {
  try {
    await invokeTauriCommand(page, 'invalid_command', {});
    expect(false).toBe(true); // Should not reach here
  } catch (error) {
    expect(error).toBeDefined();
    // Verify error message format
  }
});
```

### Available Tauri Commands for Testing

#### Wallet Manager Commands
- `get_groups` - Get all wallet groups
- `get_wallets` - Get wallets (with optional filters)
- `create_group` - Create a new wallet group
- `create_wallet` - Create a new wallet
- `delete_wallet` - Delete a wallet
- `get_watch_addresses` - Get watch-only addresses
- `is_wallet_manager_initialized` - Check if DB is initialized
- `is_password_set` - Check if encryption password is set
- `verify_password` - Verify encryption password

#### Chain Configuration Commands
- `get_chain_list` - Get all supported chains
- `get_coin_list` - Get tokens for a chain
- `add_chain` / `update_chain` / `delete_chain` - Chain CRUD
- `get_rpc_providers` - Get RPC providers
- `test_rpc_connection` - Test RPC connectivity
- `get_chain_gas_price` - Get gas price for chain
- `get_multiple_gas_prices` - Batch gas price query

#### Transfer Commands
- `base_coin_transfer` - Transfer native coin (ETH/SOL)
- `token_transfer` - Transfer ERC-20/SPL tokens
- `query_balance` - Query wallet balance
- `check_transaction_status` - Check tx status

#### Database Commands
- `reload_database` - Hot reload database
- `check_database_schema` - Verify schema integrity
- `export_database_to_public_init_sql` - Export schema

### Test Best Practices

1. **Always use `waitForTauriApp`** before interacting with Tauri commands
2. **Clean up test data** after tests (delete created groups/wallets)
3. **Use descriptive test names** that explain what is being tested
4. **Test both success and error cases**
5. **Verify response format** matches expected structure
6. **Take screenshots** on failure for debugging
7. **Run tests in headed mode** during development for visibility

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Run E2E Tests
  run: |
    npm install
    npx playwright install chromium
    npm run test:e2e
  env:
    CI: true
```

## VERIFICATION

Before submitting:
1. Run `lsp_diagnostics` on changed files
2. Run `cargo test` for Rust changes
3. Build succeeds with `yarn tauri-build`
4. Test on both EVM and Solana chains if applicable

## ROUTES

| Path | Component | Description |
|------|-----------|-------------|
| `/` | Home.vue | Dashboard with feature cards |
| `/entry` | EcosystemEntry.vue | Chain selection entry |
| `/eth/transfer` | Transfer.vue | ETH batch transfer |
| `/eth/balance` | Balance.vue | ETH balance query |
| `/eth/monitor` | Monitor.vue | ETH chain monitoring |
| `/sol/transfer` | Transfer.vue | SOL batch transfer |
| `/sol/balance` | Balance.vue | SOL balance query |
| `/airdrop` | Airdrop.vue | Airdrop management |
| `/airdrop/browser` | BrowserAutomation.vue | Browser automation |
| `/wallet-manager` | WalletManager.vue | Wallet management |

## DEPENDENCIES

### Frontend
- **Core**: Vue 3.5, Vue Router 4.6, Pinia 3.0, Vite 7
- **UI**: Arco Design Vue 2.57, PrimeVue 4.5, PrimeIcons 7.0
- **Blockchain**: Ethers 6.13, @solana/web3.js 1.91, @solana/spl-token 0.4
- **Tauri**: @tauri-apps/api 2.9, @tauri-apps/plugin-dialog 2.0, @tauri-apps/plugin-shell 2.3
- **Utils**: XLSX 0.18, QRCode 1.5, @tanstack/vue-virtual 3.13

### Backend
- **Core**: Tauri 2.9, Tokio 1.47
- **Database**: SQLx 0.7, libsqlite3-sys (SQLCipher)
- **Ethereum**: Alloy 1.4 (alloy-provider, alloy-primitives, alloy-signer-local)
- **Solana**: solana-sdk 2.2, spl-token 7.0, spl-associated-token-account 7.0
- **Security**: AES 0.8, CBC 0.1, Zeroize 1.8, PBKDF2, SHA2, HMAC
- **Network**: Reqwest 0.12, Futures 0.3
