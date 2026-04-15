use reqwest::Client;
use serde_json::{json, Value};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use sqlx::{SqlitePool, Row};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use crate::wallets_tool::ecosystems::ethereum::proxy_manager::PROXY_MANAGER;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Serialize)]
pub struct RpcTestResult {
    pub success: bool,
    pub response_time_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenBalance {
    pub amount: String,
    #[allow(dead_code)]
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
}

pub struct SolanaProvider {
    client: Client,
    rpc_url: String,
}

impl SolanaProvider {
    pub fn new(rpc_url: String) -> Self {
        // 尝试使用代理客户端，如果代理未启用则使用默认客户端
        let client = if let Some(proxy_client) = PROXY_MANAGER.get_random_proxy_client() {
            println!("[SolanaProvider] 使用代理客户端: {}", rpc_url);
            proxy_client
        } else {
            println!("[SolanaProvider] 代理未启用，使用直连模式: {}", rpc_url);
            Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new())
        };
        Self {
            client,
            rpc_url,
        }
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        let res = self.client.post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "WalletsTool/1.0")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("RPC请求失败 [{}]: {}", self.rpc_url, e))?;

        let json: Value = res.json().await.map_err(|e| format!("RPC响应解析失败 [{}]: {}", self.rpc_url, e))?;
        
        if let Some(err) = json.get("error") {
            let error_msg = err.to_string();
            let detailed_msg = if error_msg.contains("not available") {
                format!("[{}] RPC节点不可用或已达到速率限制", self.rpc_url)
            } else if error_msg.contains("too many requests") {
                format!("[{}] 请求过于频繁，请稍后再试或更换RPC节点", self.rpc_url)
            } else if error_msg.contains("Invalid param") || error_msg.contains("Invalid request") {
                format!("[{}] 无效的请求参数，请检查地址格式是否正确", self.rpc_url)
            } else {
                format!("[{}] RPC错误: {}", self.rpc_url, error_msg)
            };
            return Err(detailed_msg);
        }
        
        json.get("result").cloned().ok_or_else(|| format!("[{}] 无效的响应格式", self.rpc_url))
    }

    pub async fn get_latest_blockhash(&self) -> Result<Hash, String> {
        let res = self.request("getLatestBlockhash", json!([{"commitment": "finalized"}])).await?;
        let hash_str = res["value"]["blockhash"].as_str().ok_or("无效的blockhash格式")?;
        Hash::from_str(hash_str).map_err(|e| e.to_string())
    }

    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, String> {
        let res = self.request("getBalance", json!([pubkey.to_string(), {"commitment": "confirmed"}])).await?;
        res["value"].as_u64().ok_or_else(|| "无效的余额格式".to_string())
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Value, String> {
         self.request("getAccountInfo", json!([pubkey.to_string(), {"encoding": "base64", "commitment": "confirmed"}])).await
    }

    pub async fn get_token_account_balance(&self, pubkey: &Pubkey) -> Result<TokenBalance, String> {
        let res = self.request("getTokenAccountBalance", json!([pubkey.to_string()])).await?;
        serde_json::from_value(res["value"].clone()).map_err(|e| format!("解析Token余额失败: {}", e))
    }

    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, String> {
        let serialized = bincode::serialize(transaction).map_err(|e| e.to_string())?;
        let base64_tx = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, serialized);
        
        let res = self.request("sendTransaction", json!([base64_tx, {"encoding": "base64"}])).await?;
        let sig_str = res.as_str().ok_or("无效的交易签名格式")?;
        Signature::from_str(sig_str).map_err(|e| e.to_string())
    }

    pub async fn confirm_transaction(&self, signature: &Signature) -> Result<bool, String> {
        for _ in 0..30 {
            let res = self.request("getSignatureStatuses", json!([[signature.to_string()]])).await?;
            if let Some(statuses) = res["value"].as_array() {
                if let Some(status) = statuses.get(0) {
                     if !status.is_null() {
                         if let Some(confirmation) = status.get("confirmationStatus") {
                              let s = confirmation.as_str().unwrap_or("");
                              if s == "confirmed" || s == "finalized" {
                                  return Ok(true);
                              }
                         }
                     }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
        Err("交易确认超时".to_string())
    }

    pub async fn send_and_confirm_transaction(&self, transaction: &Transaction) -> Result<Signature, String> {
        let sig = self.send_transaction(transaction).await?;
        self.confirm_transaction(&sig).await?;
        Ok(sig)
    }

    pub async fn get_signature_statuses_batch(&self, signatures: &[String]) -> Result<Vec<Value>, String> {
        // Solana RPC getSignatureStatuses supports up to 256 signatures
        let res = self.request("getSignatureStatuses", json!([signatures, {"searchTransactionHistory": true}])).await?;
        res["value"].as_array().cloned().ok_or_else(|| "无效的签名状态响应".to_string())
    }

    pub async fn get_signatures_for_address(&self, address: &Pubkey, limit: usize) -> Result<Vec<Value>, String> {
        let params = json!([
            address.to_string(),
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]);
        let res = self.request("getSignaturesForAddress", params).await?;
        res.as_array().cloned().ok_or_else(|| "无效的交易列表响应".to_string())
    }
}

#[tauri::command]
pub async fn test_solana_rpc_connection(rpc_url: String) -> Result<RpcTestResult, String> {
    println!("[Solana RPC测试] 开始测试RPC连接: {}", rpc_url);

    // 检查代理状态
    let proxy_config = PROXY_MANAGER.get_config();
    let using_proxy = proxy_config.enabled && !proxy_config.proxies.is_empty();
    
    if using_proxy {
        println!("[Solana RPC测试] 代理已启用，使用代理进行测试 (代理数量: {})", proxy_config.proxies.len());
    } else if proxy_config.enabled {
        println!("[Solana RPC测试] 代理已启用但无可用代理，使用直连模式");
    } else {
        println!("[Solana RPC测试] 代理未启用，使用直连模式");
    }

    let start_time = std::time::Instant::now();

    // 尝试使用代理客户端，如果没有代理则创建默认客户端
    let (client, proxy_info) = if using_proxy {
        // 随机选择一个代理地址用于显示
        use rand::Rng;
        let selected_proxy = if !proxy_config.proxies.is_empty() {
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..proxy_config.proxies.len());
            proxy_config.proxies[index].clone()
        } else {
            "未知".to_string()
        };
        
        if let Some(proxy_client) = PROXY_MANAGER.get_random_proxy_client() {
            println!("[Solana RPC测试] 使用代理客户端发送请求");
            (proxy_client, format!("代理: {}", selected_proxy))
        } else {
            println!("[Solana RPC测试] 代理客户端创建失败，使用直连模式");
            let default_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| e.to_string())?;
            (default_client, "直连模式".to_string())
        }
    } else {
        println!("[Solana RPC测试] 使用默认客户端发送请求（直连模式）");
        let default_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| e.to_string())?;
        (default_client, "直连模式".to_string())
    };

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getVersion",
        "params": [],
        "id": 1
    });

    println!("[Solana RPC测试] 发送请求到: {} [{}]", rpc_url, proxy_info);

    let success = match client.post(&rpc_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "WalletsTool/1.0")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
             println!("[Solana RPC测试] 收到响应，状态码: {}", response.status());
            if response.status().is_success() {
                 match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        println!("[Solana RPC测试] 响应JSON: {}", json);
                        if let Some(error) = json.get("error") {
                            println!("[Solana RPC测试] RPC返回错误: {}", error);
                            false
                        } else {
                            let has_result = json.get("result").is_some();
                            println!("[Solana RPC测试] 是否包含result字段: {}", has_result);
                            has_result
                        }
                    }
                    Err(e) => {
                        println!("[Solana RPC测试] 解析JSON失败: {}", e);
                        false
                    }
                }
            } else {
                println!("[Solana RPC测试] HTTP状态码不成功: {}", response.status());
                false
            }
        }
        Err(e) => {
            println!("[Solana RPC测试] 请求失败: {}", e);
            if e.is_timeout() {
                println!("[Solana RPC测试] 错误类型: 连接超时 (请检查网络或代理设置)");
            } else if e.is_connect() {
                println!("[Solana RPC测试] 错误类型: 连接失败 (可能是DNS解析失败或无法连接到服务器)");
            } else if e.is_status() {
                if let Some(status) = e.status() {
                     println!("[Solana RPC测试] 错误类型: HTTP状态错误 (状态码: {})", status);
                } else {
                    println!("[Solana RPC测试] 错误类型: HTTP状态错误");
                }
            } else if e.is_decode() {
                println!("[Solana RPC测试] 错误类型: 响应解码失败");
            } else if e.is_builder() {
                println!("[Solana RPC测试] 错误类型: 请求构建失败 (URL格式可能不正确)");
            } else if e.is_redirect() {
                println!("[Solana RPC测试] 错误类型: 重定向循环");
            } else {
                 println!("[Solana RPC测试] 错误详情: {:?}", e);
            }
            false
        }
    };

    let response_time_ms = start_time.elapsed().as_millis() as u64;

    println!("[Solana RPC测试] 测试完成 - 成功: {}, 响应时间: {}ms", success, response_time_ms);

    Ok(RpcTestResult {
        success,
        response_time_ms,
    })
}

async fn get_rpc_url_from_db(pool: &SqlitePool, chain_key: &str) -> Result<Option<String>, String> {
    let row = sqlx::query(
        r#"
        SELECT rp.rpc_url 
        FROM rpc_providers rp
        JOIN chains c ON rp.chain_id = c.id
        WHERE c.chain_key = ? AND rp.is_active = TRUE
        ORDER BY rp.priority ASC
        LIMIT 1
        "#
    )
    .bind(chain_key)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|r| r.get::<String, _>("rpc_url")))
}

pub async fn get_rpc_client(chain: &str, pool: Option<&SqlitePool>) -> Result<SolanaProvider, String> {
    if let Some(db_pool) = pool {
        if let Some(rpc_url) = get_rpc_url_from_db(db_pool, chain).await? {
            return Ok(SolanaProvider::new(rpc_url));
        }
    }
    
    let url = if chain == "sol" {
        "https://api.mainnet.solana.com".to_string()
    } else {
        "https://api.devnet.solana.com".to_string()
    };
    
    Ok(SolanaProvider::new(url))
}
