use serde::{Deserialize, Serialize};
use crate::wallets_tool::security::SecureMemory;
use crate::database::chain_service::ChainService;
use crate::database::get_database_pool;
#[allow(deprecated)]
use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    system_instruction,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
    program_pack::Pack,
};
use spl_token::state::Mint;
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::create_associated_token_account,
};
use std::str::FromStr;
use crate::wallets_tool::ecosystems::solana::provider::get_rpc_client;
use tauri::Emitter;
use serde_json::json;
use base64::Engine;

#[derive(Deserialize)]
pub struct TransferItem {
    pub private_key: SecureMemory,
    pub to_addr: String,
    pub amount: Option<String>,
}

#[derive(Deserialize)]
pub struct TransferConfig {
    pub transfer_amount: f64,
    pub gas_price: Option<u64>,
    pub contract_address: Option<String>,
    #[allow(dead_code)]
    pub amount_precision: Option<u8>,
    pub chain: Option<String>,
}

#[derive(Serialize)]
pub struct TransferResult {
    success: bool,
    tx_hash: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub async fn sol_transfer(
    _index: usize,
    item: TransferItem,
    config: TransferConfig,
    _chain_service: tauri::State<'_, ChainService>,
) -> Result<TransferResult, String> {
    let chain = config.chain.as_deref().unwrap_or("sol");
    let client = match get_rpc_client(chain, Some(&get_database_pool())).await {
        Ok(c) => c,
        Err(e) => return Ok(TransferResult { success: false, tx_hash: None, error: Some(format!("RPC连接失败: {e}")) }),
    };
    
    let keypair = item.private_key.use_secret(|secret_str| {
        let bytes = bs58::decode(secret_str).into_vec().map_err(|e| e.to_string())?;
        Keypair::try_from(bytes.as_slice()).map_err(|e| e.to_string())
    }).map_err(|e| e.to_string())??;

    let to_pubkey = Pubkey::from_str(&item.to_addr).map_err(|e| e.to_string())?;
    
    let input_amount = if let Some(amt) = &item.amount {
         if !amt.is_empty() { amt.parse::<f64>().unwrap_or(config.transfer_amount) } else { config.transfer_amount }
    } else {
        config.transfer_amount
    };

    let mut instructions = vec![];
    let lamports;

    if input_amount < 0.0 {
        // Send All Logic
        let balance = client.get_balance(&keypair.pubkey()).await.map_err(|e| e.to_string())?;
        
        let base_fee = 5000;
        let mut priority_fee = 0;
        // Simple transfer usually takes < 500 CU. Set 1000 to be safe and deterministic.
        let compute_unit_limit = 1000; 

        if let Some(price) = config.gas_price {
            if price > 0 {
                instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
                instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(compute_unit_limit));
                priority_fee = (compute_unit_limit as u64 * price + 999_999) / 1_000_000;
            }
        }

        let total_fee = base_fee + priority_fee;
        
        if balance <= total_fee {
            return Ok(TransferResult { success: false, tx_hash: None, error: Some(format!("余额不足支付手续费 (余额: {}, 手续费: {})", balance, total_fee)) });
        }

        lamports = balance - total_fee;
    } else {
        // Normal Transfer
        lamports = (input_amount * 1_000_000_000.0) as u64;
        
        if let Some(fee) = config.gas_price {
            if fee > 0 {
                 instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
            }
        }
    }

    instructions.push(system_instruction::transfer(&keypair.pubkey(), &to_pubkey, lamports));

    let recent_blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash,
    );

    match client.send_and_confirm_transaction(&transaction).await {
        Ok(sig) => Ok(TransferResult { success: true, tx_hash: Some(sig.to_string()), error: None }),
        Err(e) => Ok(TransferResult { success: false, tx_hash: None, error: Some(e.to_string()) }),
    }
}

#[tauri::command]
pub async fn sol_token_transfer(
    _index: usize,
    item: TransferItem,
    config: TransferConfig,
    _chain_service: tauri::State<'_, ChainService>,
) -> Result<TransferResult, String> {
    let chain = config.chain.as_deref().unwrap_or("sol");
    let client = match get_rpc_client(chain, Some(&get_database_pool())).await {
        Ok(c) => c,
        Err(e) => return Ok(TransferResult { success: false, tx_hash: None, error: Some(format!("RPC连接失败: {e}")) }),
    };
    let mint_str = config.contract_address.ok_or("Missing Mint Address")?;
    let mint = Pubkey::from_str(&mint_str).map_err(|_| "Invalid Mint Address")?;
    
    let keypair = item.private_key.use_secret(|secret_str| {
        let bytes = bs58::decode(secret_str).into_vec().map_err(|e| e.to_string())?;
        Keypair::try_from(bytes.as_slice()).map_err(|e| e.to_string())
    }).map_err(|e| e.to_string())??;

    let to_pubkey = Pubkey::from_str(&item.to_addr).map_err(|_| "Invalid To Address")?;
    
    let from_ata = get_associated_token_address(&keypair.pubkey(), &mint);
    let to_ata = get_associated_token_address(&to_pubkey, &mint);
    
    let amount_val = if let Some(amt) = &item.amount {
         if !amt.is_empty() { amt.parse::<f64>().unwrap_or(config.transfer_amount) } else { config.transfer_amount }
    } else {
        config.transfer_amount
    };
    
    let decimals = {
        // 自动获取代币精度 (忽略 config.amount_precision，因为它通常是前端UI的随机数保留位数，而非Token精度)
        let account_info = client.get_account(&mint).await.map_err(|e| format!("无法获取代币信息: {e}"))?;
        if account_info["value"].is_null() {
            return Ok(TransferResult { success: false, tx_hash: None, error: Some("代币Mint账户不存在".to_string()) });
        }
        let data_str = account_info["value"]["data"][0].as_str().ok_or("无效的代币数据格式")?;
        let data_bytes = base64::engine::general_purpose::STANDARD.decode(data_str)
            .map_err(|_| "Base64解码失败")?;
            
        let mint_state = Mint::unpack(&data_bytes).map_err(|_| "无法解析Mint数据")?;
        mint_state.decimals
    };

    let amount_u64 = if amount_val < 0.0 {
        // Send All Logic for Token
        // 查询ATA余额
        let from_ata = get_associated_token_address(&keypair.pubkey(), &mint);
        match client.get_token_account_balance(&from_ata).await {
            Ok(balance_res) => {
                 // 优先使用 amount 字段（原始u64字符串），如果不存在则尝试从 uiAmount 计算（不推荐，有精度风险）
                 if !balance_res.amount.is_empty() {
                     balance_res.amount.parse::<u64>().unwrap_or(0)
                 } else if let Some(ui_amt) = balance_res.ui_amount {
                     (ui_amt * 10f64.powi(decimals as i32)) as u64
                 } else {
                     0
                 }
            },
            Err(_) => 0,
        }
    } else {
        (amount_val * 10f64.powi(decimals as i32)) as u64
    };

    let mut instructions = vec![];
    if let Some(fee) = config.gas_price {
        if fee > 0 {
             instructions.push(ComputeBudgetInstruction::set_compute_unit_price(fee));
        }
    }

    // Check if destination account exists
    let should_create_ata = match client.get_account(&to_ata).await {
        Ok(res) => res["value"].is_null(),
        Err(_) => true,
    };

    if should_create_ata {
        instructions.push(create_associated_token_account(
            &keypair.pubkey(),
            &to_pubkey,
            &mint,
            &spl_token::id(),
        ));
    }
    
    instructions.push(spl_token::instruction::transfer(
        &spl_token::id(),
        &from_ata,
        &to_ata,
        &keypair.pubkey(),
        &[],
        amount_u64,
    ).map_err(|e| e.to_string())?);

    let recent_blockhash = client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash,
    );

    match client.send_and_confirm_transaction(&transaction).await {
        Ok(sig) => Ok(TransferResult { success: true, tx_hash: Some(sig.to_string()), error: None }),
        Err(e) => Ok(TransferResult { success: false, tx_hash: None, error: Some(e.to_string()) }),
    }
}

#[derive(Serialize)]
pub struct CheckResult {
    has_recent_transfer: bool,
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn sol_check_recent_transfers(
    chain: String,
    private_key: SecureMemory,
    _target_address: String,
    start_timestamp: Option<i64>,
    _coin_type: String,
    _contract_address: Option<String>,
    _amount: Option<String>,
    _chain_service: tauri::State<'_, ChainService>,
) -> Result<CheckResult, String> {
    let client = match get_rpc_client(&chain, Some(&get_database_pool())).await {
        Ok(c) => c,
        Err(e) => return Err(format!("RPC连接失败: {e}")),
    };

    let keypair = private_key.use_secret(|secret_str| {
        let bytes = bs58::decode(secret_str).into_vec().map_err(|e| e.to_string())?;
        Keypair::try_from(bytes.as_slice()).map_err(|e| e.to_string())
    }).map_err(|e| e.to_string())??;

    // 获取最近的20条交易记录
    let signatures = client.get_signatures_for_address(&keypair.pubkey(), 20).await
        .map_err(|e| e.to_string())?;

    if let Some(start_ts) = start_timestamp {
        // 自动判断时间戳单位 (毫秒转秒)
        let start_seconds = if start_ts > 10_000_000_000 {
            start_ts / 1000
        } else {
            start_ts
        };

        for sig_info in signatures {
            if let Some(block_time) = sig_info.get("blockTime").and_then(|t| t.as_i64()) {
                // 如果交易时间晚于或等于开始时间，且没有错误（成功交易）
                if block_time >= start_seconds
                    && (sig_info.get("err").is_none() || sig_info["err"].is_null()) {
                         return Ok(CheckResult { has_recent_transfer: true });
                    }
            }
        }
    }

    Ok(CheckResult { has_recent_transfer: false })
}

#[tauri::command]
pub async fn sol_check_transactions_status_batch(
    chain: String,
    tx_hashes: Vec<String>,
    _chain_service: tauri::State<'_, ChainService>,
) -> Result<Vec<serde_json::Value>, String> {
    if tx_hashes.is_empty() {
        return Ok(vec![]);
    }

    let client = match get_rpc_client(&chain, Some(&get_database_pool())).await {
        Ok(c) => c,
        Err(e) => return Err(format!("RPC连接失败: {e}")),
    };

    let statuses = client.get_signature_statuses_batch(&tx_hashes).await
        .map_err(|e| e.to_string())?;
        
    let mut results = vec![];
    for (i, status_val) in statuses.iter().enumerate() {
        if i >= tx_hashes.len() { break; }
        let hash = &tx_hashes[i];
        
        let mut confirmed = false;
        let mut success = false;
        let mut error = serde_json::Value::Null;

        if !status_val.is_null() {
            if let Some(confirmation_status) = status_val.get("confirmationStatus").and_then(|s| s.as_str()) {
                if confirmation_status == "confirmed" || confirmation_status == "finalized" {
                    confirmed = true;
                    let err_val = status_val.get("err");
                    if err_val.is_none() || err_val.unwrap().is_null() {
                        success = true;
                    } else {
                        success = false;
                        error = err_val.unwrap().clone();
                    }
                }
            }
        }

        results.push(json!({
            "hash": hash,
            "status": {
                "confirmed": confirmed,
                "success": success,
                "error": error
            }
        }));
    }

    Ok(results)
}

#[tauri::command]
pub async fn sol_transfer_fast(
    index: usize,
    item: TransferItem,
    config: TransferConfig,
    chain_service: tauri::State<'_, ChainService>,
) -> Result<TransferResult, String> {
    sol_transfer(index, item, config, chain_service).await
}

#[tauri::command]
pub async fn sol_token_transfer_fast(
    index: usize,
    item: TransferItem,
    config: TransferConfig,
    chain_service: tauri::State<'_, ChainService>,
) -> Result<TransferResult, String> {
    sol_token_transfer(index, item, config, chain_service).await
}

#[derive(Deserialize, Debug, Clone)]
pub struct CoinConfig {
    pub coin_type: String, // "base" or "token"
    pub contract_address: Option<String>,
    #[allow(dead_code)]
    pub abi: Option<String>,
}

#[derive(Deserialize)]
pub struct BalanceQueryParams {
    pub items: Vec<BalanceItem>,
    pub window_id: String,
    pub query_id: String,
    pub chain: Option<String>,
    pub coin_config: Option<CoinConfig>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BalanceItem {
    pub key: String,
    pub address: String,
    pub private_key: Option<String>, // We don't need private key for balance, but struct has it
    pub plat_balance: Option<String>,
    pub coin_balance: Option<String>,
    pub exec_status: String,
    pub error_msg: Option<String>,
}

#[tauri::command]
pub async fn sol_query_balances_with_updates(
    params: BalanceQueryParams,
    window: tauri::Window,
    _chain_service: tauri::State<'_, ChainService>,
) -> Result<serde_json::Value, String> {
    let chain = params.chain.as_deref().unwrap_or("sol");
    let client = match get_rpc_client(chain, Some(&get_database_pool())).await {
        Ok(c) => c,
        Err(e) => return Err(format!("无法连接到 Solana RPC: {e}")),
    };

    // Determine if we are querying a token
    let mut mint_pubkey = None;
    if let Some(config) = &params.coin_config {
        if config.coin_type == "token" {
            if let Some(addr) = &config.contract_address {
                if !addr.is_empty() {
                    match Pubkey::from_str(addr) {
                        Ok(pk) => mint_pubkey = Some(pk),
                        Err(_) => return Err("无效的代币合约地址".to_string()),
                    }
                }
            }
        }
    }
    
    let mut results = vec![];
    
    for mut item in params.items {
        if let Ok(pubkey) = Pubkey::from_str(&item.address) {
            match client.get_balance(&pubkey).await {
                Ok(bal) => {
                    item.plat_balance = Some((bal as f64 / 1_000_000_000.0).to_string());
                    item.exec_status = "2".to_string();
                },
                Err(e) => {
                    item.exec_status = "3".to_string();
                    item.error_msg = Some(e.to_string());
                }
            }

            // Query SPL Token Balance (If configured)
            if let Some(mint) = mint_pubkey {
                if item.exec_status != "3" { 
                    let ata = get_associated_token_address(&pubkey, &mint);
                    match client.get_token_account_balance(&ata).await {
                        Ok(balance_res) => {
                            if let Some(amount_str) = balance_res.ui_amount_string {
                                item.coin_balance = Some(amount_str);
                            } else {
                                item.coin_balance = Some("0".to_string());
                            }
                        },
                        Err(_) => {
                            // If ATA doesn't exist or other error, usually means 0 balance for tokens
                            item.coin_balance = Some("0".to_string());
                        }
                    }
                }
            }
        } else {
            item.exec_status = "3".to_string();
            item.error_msg = Some("Invalid Address".to_string());
        }
        
        let _ = window.emit("balance_item_update", json!({
            "item": item,
            "window_id": params.window_id,
            "query_id": params.query_id
        }));
        
        results.push(item);
    }
    
    Ok(json!({ "success": true, "items": results }))
}
