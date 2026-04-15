# Solana Transfer Feature

**Parent:** [AGENTS.md](../../AGENTS.md)

## OVERVIEW

Batch transfer composables for Solana (native + SPL tokens). Mirror of Ethereum feature but uses Solana-specific logic (ATA creation, rent exemption).

## STRUCTURE

```
./
├── pages/Transfer.vue    # Route target
├── composables/
│   ├── useTransfer.ts    # Core logic (Solana specific)
│   ├── useBalanceQuery.ts # Batch balance fetcher
│   ├── useValidation.ts  # Address/Amount validation
│   ├── useDataOperations.ts # Excel I/O
│   ├── useTip.ts         # Notification helper
│   └── index.ts          # Barrel export
└── styles/index.ts       # Shared styles
```

## WHERE TO LOOK

| Task | File | Notes |
|------|------|-------|
| **Core Logic** | `useTransfer.ts` | `iterTransfer` handles the main loop |
| **Validation** | `useValidation.ts` | Checks SOL address format (Base58) |
| **Data I/O** | `useDataOperations.ts` | Handles Excel import/export |
| **UI** | `../pages/Transfer.vue` | Main view component |

## CONVENTIONS

- **Address Format**: Validate Base58 string length (32-44 chars).
- **Backend Calls**: Prefix commands with `sol_` (e.g., `sol_transfer`).
- **State**: Local state only; no global Pinia store for transient transfer data.

## IMPORTING WALLETS

Transfer supports three import sources:

- **Manual input:** `src/components/WalletImportModal.vue` → emits `{ privateKeys, addresses }` and `Transfer.vue` derives the from-address using `Keypair.fromSecretKey(bs58.decode(privateKey))`.
- **File import:** `composables/useDataOperations.ts` reads CSV/XLSX and appends rows into `data.value`.
- **System import:** `src/components/WalletSystemImportModal.vue` selects wallets from wallet manager (`get_wallets` / `get_watch_addresses`). For transfer, it requests secrets via `get_wallet_secrets` (password required) and only imports wallets that have private keys.

## BACKEND COMMANDS

```javascript
invoke('sol_transfer', { index, item, config })
invoke('sol_token_transfer', { index, item, config })
invoke('sol_transfer_fast', { ... }) // Fury mode
invoke('sol_token_transfer_fast', { ... })
```
