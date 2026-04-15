# WalletsTool Development Guide

## OVERVIEW

Web3 multi-chain wallet desktop app (Vue 3 + Tauri/Rust). Ethereum/Solana, batch import/transfer/balance, RPC/token config, Excel I/O, local SQLite storage. **Security-first**: 私钥/助记词仅以加密形式落库，绝不明文持久化。

## BUILD COMMANDS

```bash
yarn dev                  # Vite dev server (port 1422, frontend only)
yarn tauri-dev            # Full Tauri dev stack (recommended)
yarn tauri-build          # Production build (.exe/.dmg/.deb)
yarn start                # Install deps + tauri-dev (first-time setup)

# Backend (Rust)
cargo test                # Run all tests
cargo test <test_name>    # Run single test by name

# E2E tests (Playwright)
npx playwright install chromium  # First time only
yarn test:e2e             # Headless
yarn test:e2e:headed      # With browser visible
yarn test:e2e:ui          # Interactive UI mode

# Version management
yarn version:update <v>   # Update version in package.json + Cargo.toml
```

## ARCHITECTURE

### Window System

The main window is a **dock-style floating bar** (small, bottom-center, ~96px tall). Function windows (transfer, balance, monitor) open as separate child windows via `open_function_window`. The system tray allows opening functions directly.

- Custom title bar (`decorations: false`) with `TitleBar.vue`
- Main window close is intercepted → emits event to frontend for confirmation
- `force_close_main_window` bypasses the intercept (calls `std::process::exit(0)`)
- Never hardcode window labels; they're dynamically generated (`transfer1`, `balance2`, etc.)

### Database Architecture (Dual DB)

Two separate SQLite databases:

| Database | File | Purpose |
|----------|------|---------|
| Public DB | `data/public.db` | Chain configs, RPC nodes, tokens (plaintext) |
| Secure DB | `data/secure.db` | Wallets, private keys (SQLCipher encrypted) |

- Public DB initialized from `src-tauri/data/public_init.sql` on first run
- Secure DB requires user password via `init_secure_db` / `unlock_secure_db`
- Access pools via `DualDatabaseManager::public_pool()` and `DualDatabaseManager::secure_pool()`
- Legacy single-DB mode (`data/wallets_tool.db`) still supported for backward compat
- Password transmitted encrypted (RSA-OAEP); batch import uses AES-256-GCM (`t1:` prefix)

### Frontend Structure

Feature-based under `src/features/{ecosystem}/{domain}/`:
```
features/
├── ethereum/          # transfer, balance, monitor, common
├── solana/            # transfer, balance
├── airdrop/           # Browser automation & scripting
├── wallet_manager/    # Wallet CRUD UI
├── common/            # Shared ecosystem pages
└── home/              # Dashboard
```

Each feature: `pages/`, `composables/` (may use `.ts`), `components/`, `services/`, `styles/`.

### Backend Structure

```
src-tauri/src/
├── main.rs               # Entry, tray, window management, command registration
├── utils.rs              # Shared utilities
├── database/             # Dual DB manager, services, encryption
├── plugins/              # fs_extra, etc.
└── wallets_tool/
    ├── ecosystems/ethereum/  # chain_config, transfer, provider, proxy, balance
    ├── ecosystems/solana/    # transfer, provider
    ├── wallet_manager/       # Wallet CRUD + field-level encryption
    ├── airdrop/              # Scheduler, executor, models
    ├── playwright/           # Browser automation bridge
    ├── security/             # AES-256 memory guard, anti-debug, zeroize
    └── transfer.rs           # Unified transfer entry (re-exports ethereum)
```

### Routing (Hash-based)

Uses `createWebHashHistory`. Key routes: `/`, `/entry`, `/settings`, `/eth/transfer`, `/eth/balance`, `/eth/monitor`, `/sol/transfer`, `/sol/balance`, `/airdrop`, `/airdrop/browser`, `/wallet-manager`.

### UI Frameworks

- **PrimeVue 4.5**: Data tables, virtual scroller, toasts
- **Arco Design Vue 2.57**: Forms, modals, inputs, interactions

Arco components are registered globally in `src/main.js`. PrimeVue uses plugin system.

## CODE STYLE

- **Frontend**: `.js` only (no TypeScript). Composables may use `.ts`.
- **Backend**: Rust 2024 edition. `Result<T, String>` for all Tauri commands. No `unwrap()` in production code.
- **Imports**: `@/` alias maps to `src/`. Arco uses named imports; PrimeVue uses plugin registration.
- **Naming**: composables prefixed `use` (`useTransfer`), components PascalCase, constants UPPER_SNAKE_CASE.
- **No comments** unless explaining complex algorithms.

## SECURITY RULES

1. Private keys in RAM only, never persisted to disk/DB
2. AES-256-CBC encryption in memory; zeroize after use
3. SQLCipher for secure.db (PBKDF2, 600k iterations)
4. IPC encryption: passwords via RSA-OAEP, batch secrets via AES-256-GCM (`t1:`)
5. Anti-debug protection on Windows (memory scanning defense)
6. Never log or persist sensitive data (keys, mnemonics)

## ANTI-PATTERNS

- **NEVER** persist private keys or log sensitive data
- **NEVER** use `as any`, `@ts-ignore`, `.unwrap()` without context
- **NEVER** remove `custom-protocol` from `tauri.conf.json` features
- **NEVER** hardcode RPC URLs; use chain config from database
- **NEVER** block the main thread; use `tokio` for heavy operations
- **NEVER** mix Pinia state with local feature state unnecessarily

## DATABASE INIT & EXPORT

- `public_init.sql` defines non-sensitive schema + seed data (chains, RPCs, tokens)
- `secure_init.sql` defines secure schema (wallets, app_config)
- Schema hot-reload: `reload_database` command deletes DB files and re-inits from init SQL
- Export current public.db state: `export_database_to_init_sql` command
- Config in `package.json` → `config.database` (forceInit, enableDebugLog, initSqlPath)
- No migration framework; schema changes go directly into `public_init.sql` / `secure_init.sql`

## E2E TESTING (Playwright)

- Config: `playwright.config.ts`, tests in `e2e/`, single worker, serial execution
- Must call `waitForTauriApp(page)` before invoking any Tauri command
- Call backend via `invokeTauriCommand(page, 'command_name', { args })` (see `e2e/tauri-helpers.ts`)
- Tests run against Vite dev server (`http://localhost:1420`)

## RELEASE & UPDATER

- Workflow: `.github/workflows/release.yml` triggers on `v*` tags
- Requires secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `TAURI_UPDATER_PUBKEY`
- Signing key prepared via `scripts/ci/prepare-tauri-signing-key.cjs`
- Updater endpoints include `gh-proxy.org` mirrors for domestic networks
- Client checks on startup; falls back to GitHub API if updater endpoints fail

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Frontend entry | `src/main.js` |
| Backend entry | `src-tauri/src/main.rs` |
| Transfer logic | `src/features/{chain}/transfer/`, `src-tauri/src/wallets_tool/ecosystems/{chain}/transfer.rs` |
| Balance logic | `src/features/{chain}/balance/`, `src-tauri/src/wallets_tool/ecosystems/ethereum/simple_balance_query.rs` |
| Wallet manager | `src/features/wallet_manager/`, `src-tauri/src/wallets_tool/wallet_manager/` |
| DB operations | `src-tauri/src/database/` (dual_database.rs, mod.rs, services) |
| Security | `src-tauri/src/wallets_tool/security/` |
| Browser automation | `src/features/airdrop/`, `src-tauri/src/wallets_tool/playwright/` |
| Chain/RPC config | `src/components/ChainManagement.vue`, `src/components/RpcManagement.vue` |
| Update check | `src/App.vue`, `src-tauri/tauri.conf.json` |

## VERIFICATION BEFORE SUBMIT

1. Run `cargo test` for Rust changes
2. Build succeeds with `yarn tauri-build`
3. Test on both EVM and Solana chains if applicable
