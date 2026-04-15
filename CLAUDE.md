# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WalletsTool is a Web3 multi-chain wallet management desktop application built with Vue 3 + Tauri (Rust). It supports Ethereum and Solana ecosystems, providing features like batch wallet import, balance queries, batch transfers, RPC/token configuration, and Excel import/export with local SQLite storage.

## Development Commands

### Frontend (Vue 3 + Vite)
- `yarn dev` - Start Vite development server only (port 1422)
- `yarn build` - Build frontend for production
- `yarn preview` - Preview production build

### Tauri Desktop App
- `yarn tauri-dev` - Start Tauri development server (combines frontend + backend)
- `yarn tauri-build` - Build production desktop application
- `yarn icon` - Generate app icons from source

### Quick Start & Setup
- `yarn start` or `yarn setup` - Auto-install dependencies (Node.js, Yarn, Rust, Tauri CLI) and start dev environment
- `node scripts/install-deps.js` - Manually run dependency installation

### Utilities
- `yarn version:update <version>` - Update version across package.json and Cargo.toml, create git tag
- `node scripts/update-version.js <version>` - Direct version update script

## Architecture Overview

### Frontend Structure (Vue 3)
- **Feature-based organization** under `src/features/`
  - `ethereum/` - Ethereum-specific functionality (balance, transfer)
  - `solana/` - Solana-specific functionality (balance, transfer)
  - `home/` - Main dashboard
- **Shared components** in `src/components/`
- **Vue Router** with hash-based routing (`createWebHashHistory`)
- **Pinia** for state management in `src/stores/`

### Backend Structure (Tauri/Rust)
- **Modular architecture** in `src-tauri/src/`
  - `wallets_tool/` - Core business logic
    - `ecosystems/` - Blockchain-specific implementations
      - `ethereum/` - Complete Ethereum implementation (RPC management, transfers, balance queries)
      - `solana/` - Solana implementation (in progress)
    - `database/` - Database management and models (hot reload capable)
    - `plugins/` - Tauri plugins and custom filesystem operations
- **Multi-window support** with tray integration
- **SQLite database** with schema management and hot reload capabilities

**Key Backend Patterns:**
- Async/await throughout with tokio runtime
- Result<T, String> error handling pattern
- Modular command registration in main.rs
- Database connection pooling with SQLx

### Key Integrations
- **ethers.js** (v5.7.0) for Ethereum blockchain interactions in frontend
- **ethers** (v1.0 Rust) for Ethereum operations in backend
- **SQLite** with SQLx for local data persistence (chains, tokens, RPC providers)
- **Tauri commands** bridge frontend-backend communication
- **System tray** with context menu for quick function access
- **Multi-RPC load balancing** for Ethereum with automatic failover

## Database Architecture

The application uses SQLite with these main tables:
- `chains` - Blockchain network configurations
- `rpc_providers` - RPC endpoint management with priority and health tracking
- `tokens` - Token contract configurations with ABI support
- `monitor_configs` - Address monitoring settings (future feature)
- `monitor_history` - Monitoring history (future feature)

**Key Features:**
- **Hot reload**: Use `reload_database()` command to reload schema without restarting
- **Automatic migrations**: Schema changes detected and managed via SQLx
- **Base64-encoded icons**: Chain/token icons stored in database
- **Foreign key relationships**: Proper indexing with foreign key constraints

**Security Note**: Private keys are never stored in the database - they exist only in memory during operations.

## Development Patterns

### Adding New Blockchain Support
1. Create new ecosystem directory under `src-tauri/src/wallets_tool/ecosystems/`
2. Implement chain-specific provider, transfer, and balance query logic
3. Add frontend feature directory under `src/features/`
4. Update routing in `src/router/index.js`
5. Register Tauri commands in `main.rs`

### Tauri Command Pattern
Backend functions exposed to frontend via `#[tauri::command]` attribute:
```rust
#[tauri::command]
async fn function_name(param: Type) -> Result<ReturnType, String> {
    // Implementation
}
```

Register in `main.rs` invoke_handler via `#[tauri::generate_handler!]` macro and call from frontend via Tauri's `invoke()` API.

**Key Command Categories:**
- Chain management (get_chain_list, add_chain, update_chain)
- RPC provider management (add_rpc_provider, test_rpc_connection)
- Balance queries (query_balances_simple, stop_balance_query)
- Transfer operations (base_coin_transfer, token_transfer)
- Window management (open_function_window, close_all_child_windows)
- Database utilities (reload_database, check_database_schema)

### Window Management
- Main window: "main" 
- Dynamic child windows for different functions (transfer, balance, etc.)
- Tray integration allows opening function windows independently
- Custom close handling prevents accidental app termination

## Technology Stack

### Frontend Dependencies
- Vue 3.5.26 with Composition API
- PrimeVue 4.5.4 + Arco Design 2.57.0 for UI components
- Vue Router 4.6.5 with hash-based routing
- Pinia 3.0.4 for state management
- ethers.js 6.13.4 for Ethereum interactions
- xlsx 0.18.5 for Excel file handling
- party-js 2.2.0 for animations
- qrcode for QR code generation
- @solana/web3.js 1.91.0 + @solana/spl-token 0.4.0 for Solana support

### Backend Dependencies
- Tauri 2.9 with custom protocol support
- SQLx 0.7 for database operations with SQLite
- tokio 1.0 for async runtime
- reqwest 0.11 for HTTP client functionality
- ethers 2.0 (Rust) for blockchain operations
- chrono for datetime handling
- serde for serialization/deserialization

## Build Configuration

### Vite Configuration
- **Base path**: Dynamic path setting for Tauri compatibility
- **Development server**: Port 1422 with fallback options
- **Node.js polyfills**: Comprehensive polyfill configuration for ethers.js (crypto, stream, http, etc.)
- **Code splitting**: Strategic chunking for vendor libraries, pages, and components
  - Vue-related libraries in separate chunk
  - Arco Design vendor isolation
  - ethers and third-party libraries consolidated
  - Feature-based code splitting for better caching
- **Terser optimization**: Production builds remove console logs and optimize bundle size
- **Build optimizations**: Maximum compression with dead code elimination

### Cargo Release Profile
- Maximum optimization level (opt-level = 3)
- Fat LTO enabled for best performance
- Debug symbols stripped for smaller binaries
- Panic strategy set to abort

## Security Considerations

- Private keys handled only in memory, never persisted
- Local SQLite storage for configuration data only
- Transaction validation and confirmation mechanisms
- No sensitive data in version control or logs

## Important Architectural Patterns

### Error Handling
- **Frontend**: Vue error boundaries for graceful degradation
- **Backend**: Result<T, String> pattern throughout Rust codebase
- **User-facing**: Structured error messages with recovery options
- **Network**: Graceful handling of RPC failures with automatic retries

### Performance Optimizations
- **Virtual scrolling**: For large datasets (e.g., wallet lists, transaction history)
- **Lazy loading**: Non-critical components loaded on-demand
- **Debounced input**: User input throttling for search/filter operations
- **Database connection pooling**: Efficient connection management via SQLx
- **Background tasks**: Async operations with proper cancellation support

### Multi-Window Architecture
- Main window ("main") serves as primary interface
- Child windows open independently for specific functions (transfer, balance, etc.)
- Event-driven coordination between windows via Tauri events
- System tray integration allows opening function windows without main window
- Custom close handling prevents accidental app termination when child windows close

### Ethereum-Specific Patterns
- **Multi-RPC management**: Automatic load balancing and failover across RPC providers
- **Gas optimization**: Automatic gas price estimation and adjustment
- **Batch transfers**: Configurable delays between transactions to mimic natural user behavior
- **Transaction tracking**: Status monitoring with automatic retry on failure
- **ERC-20 support**: Full token transfer capability with ABI management

## E2E TESTING

The project includes a comprehensive Playwright-based E2E testing framework for testing frontend + Tauri backend integration.

### Quick Start

```bash
# Install Playwright browsers
npx playwright install chromium

# Run tests
npm run test:e2e          # Headless mode
npm run test:e2e:headed   # With browser visible
npm run test:e2e:ui       # Interactive UI mode
```

### Key Testing Patterns

1. **Invoke Tauri Commands from Tests:**
```typescript
const wallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
  group_id: null,
  chain_type: null,
  password: null,
});
```

2. **Wait for Tauri App:**
```typescript
await page.goto('/#/wallet-manager');
await waitForTauriApp(page);
```

3. **Test Data Consistency:**
```typescript
const backendData = await invokeTauriCommand(page, 'get_wallets', {});
const frontendCount = await page.locator('table tbody tr').count();
expect(frontendCount).toBe(backendData.length);
```

### Test Files
- `e2e/wallet-manager.spec.ts` - Wallet manager integration tests
- `e2e/balance-query.spec.ts` - Balance query integration tests
- `e2e/api-integration.spec.ts` - API contract tests
- `e2e/tauri-helpers.ts` - Test utilities

See `AGENTS.md` for detailed testing documentation.