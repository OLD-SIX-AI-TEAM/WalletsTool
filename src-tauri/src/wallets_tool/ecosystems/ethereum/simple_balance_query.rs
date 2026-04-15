use anyhow::{anyhow, Result};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tauri::Emitter;
use rand;
use alloy::signers::local::PrivateKeySigner;
use crate::database::{get_database_pool, rpc_service::RpcService, chain_service::ChainService};

// 基于窗口ID的停止标志映射
static STOP_FLAGS: LazyLock<Mutex<HashMap<String, AtomicBool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// 辅助函数：获取窗口的停止状态
fn get_stop_flag(window_id: &str) -> bool {
    let flags = STOP_FLAGS.lock().unwrap();
    if let Some(flag) = flags.get(window_id) {
        flag.load(Ordering::Relaxed)
    } else {
        false
    }
}

// 辅助函数：设置窗口的停止状态
fn set_stop_flag(window_id: &str, value: bool) {
    let mut flags = STOP_FLAGS.lock().unwrap();
    if let Some(flag) = flags.get(window_id) {
        flag.store(value, Ordering::Relaxed);
    } else {
        flags.insert(window_id.to_string(), AtomicBool::new(value));
    }
}

// 辅助函数：重置窗口的停止状态
fn reset_stop_flag(window_id: &str) {
    set_stop_flag(window_id, false);
}

// 查询项目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryItem {
    pub key: String,
    pub address: String,
    pub private_key: Option<String>,
    pub plat_balance: Option<String>,
    pub coin_balance: Option<String>,
    pub nonce: Option<u64>,
    pub retry_flag: bool,
    pub exec_status: String, // "0"=未执行, "1"=执行中, "2"=成功, "3"=失败
    pub error_msg: Option<String>,
}

// 币种配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinConfig {
    pub coin_type: String, // "base" 或 "token"
    pub contract_address: Option<String>,
    pub abi: Option<String>,
}

// 查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    pub chain: String,
    pub coin_config: CoinConfig,
    pub items: Vec<QueryItem>,
    pub only_coin_config: bool,
    pub thread_count: usize,
    #[serde(default)]
    pub window_id: Option<String>,
    #[serde(default)]
    pub query_id: Option<String>,
}

// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub success: bool,
    pub items: Vec<QueryItem>,
    pub error_msg: Option<String>,
}

// RPC 请求结构
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: Option<i32>,
}

// RPC 响应结构
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// RPC 提供商配置
#[allow(dead_code)]
pub struct ProviderConfig {
    pub rpc_urls: Vec<String>,
    pub chain_id: u64,
}

// 余额查询器
pub struct SimpleBalanceQueryService {
    client: Client,
}

impl SimpleBalanceQueryService {
    pub fn new() -> Self {
        // 创建默认客户端（用于非代理模式）
        // 设置30秒超时，与代理客户端保持一致
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }

    // 从数据库获取RPC URL
    async fn get_rpc_url(&self, chain: &str) -> Result<String> {
        let pool = get_database_pool();
        let rpc_service = RpcService::new(&pool);
        
        rpc_service.get_random_rpc_url(chain).await
    }

    // 发送 JSON-RPC 请求（带超时、代理支持和429重试）
    async fn send_rpc_request(&self, rpc_url: &str, method: &str, params: serde_json::Value, window_id: Option<&str>) -> Result<serde_json::Value> {
        use crate::wallets_tool::ecosystems::ethereum::proxy_manager::PROXY_MANAGER;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(1),
        };

        // 设置10秒超时
        let timeout = Duration::from_secs(10);
        
        // 根据 window_id 获取代理配置
        let (proxy_config, proxy_client) = if let Some(wid) = window_id {
            let config = PROXY_MANAGER.get_config_for_window(wid);
            let client = PROXY_MANAGER.get_random_proxy_client_for_window(wid);
            (config, client)
        } else {
            let config = PROXY_MANAGER.get_config();
            let client = PROXY_MANAGER.get_random_proxy_client();
            (config, client)
        };
        
        let using_proxy = proxy_config.enabled && !proxy_config.proxies.is_empty();
        
        // 优先使用代理客户端，如果没有代理则使用默认客户端
        let client = if let Some(pc) = proxy_client {
            println!("[DEBUG] 使用代理发送RPC请求 (余额查询): {rpc_url}, window_id: {window_id:?}");
            if using_proxy {
                println!("[INFO] 代理已启用，当前有 {} 个代理可用", proxy_config.proxies.len());
            }
            pc
        } else {
            if proxy_config.enabled {
                println!("[WARN] 代理已启用但没有可用代理，使用直连模式: {rpc_url}");
            } else {
                println!("[DEBUG] 代理未启用，使用直连模式发送RPC请求 (余额查询): {rpc_url}");
            }
            self.client.clone()
        };
        
        // 实现429错误重试机制（最多重试3次）
        let mut retry_count = 0;
        let max_retries = 3;
        
        loop {
            let response = tokio::time::timeout(timeout, 
                client
                    .post(rpc_url)
                    .json(&request)
                    .send()
            ).await
            .map_err(|_| anyhow!("RPC请求超时（10秒），RPC地址: {rpc_url}"))??;

            // 检查是否为429错误
            if response.status().as_u16() == 429 {
                retry_count += 1;
                if retry_count > max_retries {
                    return Err(anyhow!("RPC请求速率限制（429错误），已达到最大重试次数，RPC地址: {rpc_url}"));
                }
                
                // 指数退避：等待时间随重试次数增加而增加
                let wait_time = Duration::from_secs(2_u64.pow(retry_count as u32));
                println!("[WARN] 遇到429速率限制，等待 {wait_time:?} 后重试（第 {retry_count} 次重试），RPC: {rpc_url}");
                sleep(wait_time).await;
                continue;
            }
            
            let json_response: JsonRpcResponse = tokio::time::timeout(timeout,
                response.json::<JsonRpcResponse>()
            ).await
            .map_err(|_| anyhow!("RPC响应解析超时（10秒），RPC地址: {rpc_url}"))??;

            if let Some(error) = json_response.error {
                return Err(anyhow!("RPC Error: {} - {}", error.code, error.message));
            }

            return json_response.result.ok_or_else(|| anyhow!("No result in RPC response"));
        }
    }

    // 查询基础币种余额（带重试机制）
    async fn query_base_balance_with_retry(&self, item: &mut QueryItem, chain: &str, max_retries: usize, window_id: Option<&str>) -> Result<()> {
        let mut last_error = None;
        
        for retry_count in 0..max_retries {
            // 每次重试都重新获取RPC地址（会随机选择不同的RPC节点）
            match self.query_base_balance(item, chain, window_id).await {
                Ok(_) => {
                    if retry_count > 0 {
                        println!("[SUCCESS] 平台币查询重试成功 - 地址: {}, 重试次数: {}", item.address, retry_count);
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if retry_count < max_retries - 1 {
                        println!("[RETRY] 平台币查询失败，将重试 ({}/{}) - 地址: {}, 错误: {}", 
                                retry_count + 1, max_retries - 1, item.address, last_error.as_ref().unwrap());
                        sleep(Duration::from_millis(500)).await; // 重试间隔500ms
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("平台币查询失败")))
    }

    // 查询基础币种余额
    async fn query_base_balance(&self, item: &mut QueryItem, chain: &str, window_id: Option<&str>) -> Result<()> {
        let rpc_url = self.get_rpc_url(chain).await?;
        
        // 查询余额
        let balance_result = self.send_rpc_request(
            &rpc_url,
            "eth_getBalance",
            serde_json::json!([item.address, "latest"]),
            window_id
        ).await?;

        if let Some(balance_hex) = balance_result.as_str() {
            let hex_without_prefix = &balance_hex[2..];
            match u128::from_str_radix(hex_without_prefix, 16) {
                Ok(balance_wei) => {
                    let balance_eth = balance_wei as f64 / 1e18;
                    item.plat_balance = Some(format!("{balance_eth:.6}"));
                }
                Err(e) => {
                    return Err(anyhow!("余额数值转换失败: {e} (原始值: {balance_hex})"));
                }
            }
        }

        Ok(())
    }

    // 查询代币余额
    async fn query_token_balance(&self, item: &mut QueryItem, chain: &str, contract_address: &str, window_id: Option<&str>) -> Result<()> {
        let rpc_url = self.get_rpc_url(chain).await?;
        
        println!("[DEBUG] 开始查询代币余额 - 链: {}, 地址: {}, 合约: {}", chain, item.address, contract_address);
        
        // 首先尝试从数据库获取代币的 decimals 配置
        let decimals = {
            let chain_service = ChainService::new();
            match chain_service.get_token_decimals_by_contract(chain, contract_address).await {
                Ok(Some(db_decimals)) => {
                    println!("[DEBUG] 从数据库获取到代币decimals - 链: {chain}, 合约: {contract_address}, decimals: {db_decimals}");
                    db_decimals as u8
                }
                Ok(None) | Err(_) => {
                    println!("[DEBUG] 数据库中未找到代币配置，回退到合约查询 - 链: {chain}, 合约: {contract_address}");
                    // 回退到从合约查询 decimals
                    let decimals_method = "313ce567"; // decimals() 函数的方法ID
                    let decimals_data = format!("0x{decimals_method}");
                    
                    match self.send_rpc_request(
                        &rpc_url,
                        "eth_call",
                        serde_json::json!([{
                            "to": contract_address,
                            "data": decimals_data
                        }, "latest"]),
                        window_id
                    ).await {
                        Ok(decimals_result) => {
                            if let Some(decimals_hex) = decimals_result.as_str() {
                                let hex_without_prefix = &decimals_hex[2..];
                                match u8::from_str_radix(hex_without_prefix, 16) {
                                    Ok(d) => {
                                        println!("[DEBUG] 从合约查询到代币decimals - 链: {chain}, 合约: {contract_address}, decimals: {d}");
                                        d
                                    }
                                    Err(e) => {
                                        println!("[WARNING] decimals查询失败，使用默认值18 - 链: {chain}, 合约: {contract_address}, 错误: {e}");
                                        18 // 默认使用18位小数
                                    }
                                }
                            } else {
                                println!("[WARNING] decimals查询返回空值，使用默认值18 - 链: {chain}, 合约: {contract_address}");
                                18 // 默认使用18位小数
                            }
                        }
                        Err(e) => {
                            println!("[WARNING] decimals查询失败，使用默认值18 - 链: {chain}, 合约: {contract_address}, 错误: {e}");
                            18 // 默认使用18位小数
                        }
                    }
                }
            }
        };
        
        // ERC20 balanceOf 函数的方法ID
        let balance_of_method = "70a08231";
        
        // 编码地址参数（去掉0x前缀，左填充到32字节）
        let address_param = format!("{:0>64}", &item.address[2..]);
        let data = format!("0x{balance_of_method}{address_param}");

        let balance_result = self.send_rpc_request(
            &rpc_url,
            "eth_call",
            serde_json::json!([{
                "to": contract_address,
                "data": data
            }, "latest"]),
            window_id
        ).await?;

        if let Some(balance_hex) = balance_result.as_str() {
            
            // 检查十六进制字符串长度
            let hex_without_prefix = &balance_hex[2..];
            
            println!("[DEBUG] 原始余额查询结果 - 链: {}, 地址: {}, 合约: {}, 十六进制: {}", 
                    chain, item.address, contract_address, balance_hex);
            
            // 将十六进制转换为十进制，使用u128避免溢出
            match u128::from_str_radix(hex_without_prefix, 16) {
                Ok(balance_wei) => {
                    println!("[DEBUG] 原始余额转换 - 链: {}, 地址: {}, 合约: {}, 原始余额(十进制): {}, 是否为零: {}", 
                            chain, item.address, contract_address, balance_wei, balance_wei == 0);
                    
                    // 使用实际的 decimals 值计算余额
                    let divisor = 10_u128.pow(decimals as u32);
                    let balance_tokens = balance_wei as f64 / divisor as f64;
                    
                    println!("[DEBUG] 余额计算过程 - 链: {}, 地址: {}, 合约: {}, 原始余额: {}, decimals: {}, 除数: {}, 最终余额: {}", 
                            chain, item.address, contract_address, balance_wei, decimals, divisor, balance_tokens);
                    
                    item.coin_balance = Some(format!("{balance_tokens:.6}")); // 显示6位小数
                    
                    println!("[DEBUG] 代币余额查询完成 - 链: {}, 地址: {}, 合约: {}, 格式化余额: {}", 
                            chain, item.address, contract_address, item.coin_balance.as_ref().unwrap());
                }
                Err(e) => {
                    println!("[ERROR] 代币余额十六进制转换失败 - 链: {}, 地址: {}, 合约: {}, 十六进制: {}, 错误: {}", 
                            chain, item.address, contract_address, balance_hex, e);
                    return Err(anyhow!("代币余额数值转换失败: {e} (原始值: {balance_hex})"));
                }
            }
        } else {
            println!("[WARNING] 代币余额查询返回空值 - 链: {}, 地址: {}, 合约: {}", 
                    chain, item.address, contract_address);
        }

        Ok(())
    }

    // 查询单个项目的余额（带超时控制和失败重试）
    async fn query_single_item(&self, mut item: QueryItem, params: &QueryParams, window_id: &str) -> QueryItem {
        // 检查是否需要停止查询
        if get_stop_flag(window_id) {
            item.exec_status = "3".to_string(); // 失败
            item.error_msg = Some("查询已被用户停止".to_string());
            return item;
        }

        item.exec_status = "1".to_string(); // 执行中
        item.error_msg = None;

        // 如果有私钥，优先从私钥生成地址
        if let Some(private_key_str) = &item.private_key {
            if !private_key_str.trim().is_empty() {
                // 处理私钥格式，兼容带0x和不带0x的格式
                let private_key = if private_key_str.starts_with("0x") || private_key_str.starts_with("0X") {
                    private_key_str[2..].to_string()
                } else {
                    private_key_str.clone()
                };
                
// 从私钥生成地址
                if let Ok(signer) = private_key.parse::<PrivateKeySigner>() {
                    let address = format!("{:?}", signer.address());
                    item.address = address;
                    println!("[INFO] 从私钥生成地址: {}", item.address);
            } else {
                    item.exec_status = "3".to_string();
                    item.error_msg = Some("私钥格式错误，无法生成地址".to_string());
                    return item;
                }
            }
        }

        // 检查地址是否为空
        if item.address.trim().is_empty() {
            item.exec_status = "3".to_string();
            item.error_msg = Some("地址为空，无法查询余额".to_string());
            println!("[ERROR] 地址为空，无法查询余额 - key: {}", item.key);
            return item;
        }

        // 实现失败重试机制（最多3次，每次重试更换RPC节点）
        let max_retries = 3;
        let mut last_error = String::new();
        let mut last_rpc_url = String::new();
        
        for retry_count in 0..max_retries {
            // 检查是否需要停止查询
            if get_stop_flag(window_id) {
                item.exec_status = "3".to_string();
                item.error_msg = Some("查询已被用户停止".to_string());
                return item;
            }
            
            // 如果是重试（不是第一次尝试），添加短暂延迟
            if retry_count > 0 {
                println!("[RETRY] 余额查询重试 {}/{} - 地址: {}，更换RPC节点", retry_count, max_retries - 1, item.address);
                sleep(Duration::from_millis(500)).await; // 重试间隔500ms
            }
            
            // 每次重试都重新获取RPC地址（会随机选择不同的RPC节点）
            let rpc_url = match self.get_rpc_url(&params.chain).await {
                Ok(url) => url,
                Err(e) => {
                    last_error = format!("获取RPC地址失败: {e}");
                    println!("[ERROR] 获取RPC地址失败: {}, 错误: {}", params.chain, e);
                    item.exec_status = "3".to_string();
                    item.error_msg = Some(last_error.clone());
                    return item;
                }
            };
            
            // 记录当前使用的RPC
            if retry_count > 0 {
                println!("[RPC] 第{}次尝试使用RPC: {}", retry_count + 1, rpc_url);
            }
            last_rpc_url = rpc_url.clone();
            
            // 设置单个查询任务的超时时间为15秒
            let query_timeout = Duration::from_secs(15);
            
            let result = tokio::time::timeout(query_timeout, async {
                if params.coin_config.coin_type == "base" {
                    self.query_base_balance(&mut item, &params.chain, Some(window_id)).await?;
                } else if params.coin_config.coin_type == "token" {
                    // 查询代币时，同时查询平台币和代币余额
                    let mut base_query_success = true;
                    let mut token_query_success = true;
                    let mut errors = Vec::new();
                    
                    // 如果不是仅查询目标代币，也要查询平台币（带重试机制）
                    if !params.only_coin_config {
                        // 平台币查询失败时记录错误，但不立即返回
                        if let Err(e) = self.query_base_balance_with_retry(&mut item, &params.chain, 3, Some(window_id)).await {
                            base_query_success = false;
                            errors.push(format!("平台币查询失败: {e}"));
                            println!("[WARN] 平台币查询最终失败: {e}");
                        } else {
                            println!("[SUCCESS] 平台币查询成功");
                        }
                    }
                    
                    // 查询代币余额
                    if let Some(contract_address) = &params.coin_config.contract_address {
                        if let Err(e) = self.query_token_balance(&mut item, &params.chain, contract_address, Some(window_id)).await {
                            token_query_success = false;
                            errors.push(format!("代币查询失败: {e}"));
                            println!("[WARN] 代币查询失败: {e}");
                        } else {
                            println!("[SUCCESS] 代币查询成功");
                        }
                    }
                    
                    // 只有当代币查询失败时才返回错误（平台币查询失败不影响整体结果）
                    if !token_query_success {
                        return Err(anyhow!(errors.join("; ")));
                    }
                    
                    // 如果平台币查询失败但代币查询成功，记录警告但不返回错误
                    if !base_query_success {
                        println!("[WARN] 平台币查询失败但代币查询成功，继续处理");
                    }
                }
                Ok::<(), anyhow::Error>(())
            }).await;

            match result {
                Ok(Ok(_)) => {
                    // 查询成功
                    item.exec_status = "2".to_string();
                    if retry_count > 0 {
                        println!("[SUCCESS] 余额查询重试成功 - 地址: {}, 重试次数: {}, 使用RPC: {}", 
                                item.address, retry_count, last_rpc_url);
                    }
                    return item; // 成功后直接返回
                }
                Ok(Err(e)) => {
                    last_error = format!("查询失败: {e}");
                    println!("[ERROR] 余额查询失败 (尝试 {}/{}) - 地址: {}, RPC: {}, 错误: {}", 
                            retry_count + 1, max_retries, item.address, last_rpc_url, e);
                }
                Err(_) => {
                    last_error = "查询超时（15秒）".to_string();
                    println!("[ERROR] 余额查询超时 (尝试 {}/{}) - 地址: {}, RPC: {}", 
                            retry_count + 1, max_retries, item.address, last_rpc_url);
                }
            }
        }
        
        // 所有重试都失败，设置最终失败状态
        item.exec_status = "3".to_string();
        item.retry_flag = true;
        item.error_msg = Some(format!("{} (已重试{}次，最后RPC: {})", last_error, max_retries - 1, last_rpc_url));
        println!("[FAILED] 余额查询最终失败 - 地址: {}, 已重试{}次，最后RPC: {}", 
                item.address, max_retries - 1, last_rpc_url);

        item
    }

    // 批量查询余额（多线程）
    pub async fn query_balances(&self, params: QueryParams) -> QueryResult {
        let thread_count = params.thread_count.clamp(1, 99); // 限制线程数在1-99之间
        let semaphore = Arc::new(Semaphore::new(thread_count));

        println!("开始批量查询余额，线程数: {}, 总任务数: {}", thread_count, params.items.len());

        let items = params.items.clone();
        let tasks: Vec<_> = items.into_iter().map(|item| {
            let semaphore = semaphore.clone();
            let params = params.clone();
            let service = self;

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                // 添加轻微随机延迟（50-150ms），避免所有请求同时发送导致RPC限流
                // 移除原来的300-800ms延迟，改为真正的并发
                let delay = Duration::from_millis(50 + (rand::random::<u64>() % 100));
                sleep(delay).await;

                service.query_single_item(item, &params, "default").await
            }
        }).collect();

        let results = join_all(tasks).await;

        let success = results.iter().all(|item| item.exec_status == "2");
        let error_msg = if success {
            None
        } else {
            Some("部分查询失败".to_string())
        };

        println!("查询完成，成功: {success}");

        QueryResult {
            success,
            items: results,
            error_msg,
        }
    }

    // 批量查询余额（带实时更新）
    pub async fn query_balances_with_updates<R: tauri::Runtime>(
        &self,
        params: QueryParams,
        app_handle: tauri::AppHandle<R>,
        window_id: String,
    ) -> QueryResult {
        // 重置停止标志
        reset_stop_flag(&window_id);

        let thread_count = params.thread_count.clamp(1, 99); // 限制线程数在1-99之间
        let semaphore = Arc::new(Semaphore::new(thread_count));

        println!("开始批量查询余额（实时更新），线程数: {}, 总任务数: {}", thread_count, params.items.len());

        let total_items = params.items.len();
        let service = Arc::new(SimpleBalanceQueryService::new());

        // 创建结果数组，使用索引映射确保按原始顺序返回
        // results[original_index] = Some(result) 或 None（如果任务失败）
        let results: Arc<Mutex<Vec<Option<QueryItem>>>> = Arc::new(Mutex::new(vec![None; total_items]));

        // 创建任务，每个任务保留其原始索引
        let tasks: Vec<_> = params.items.clone().into_iter().enumerate().map(|(original_index, item)| {
            let semaphore = semaphore.clone();
            let params = params.clone();
            let service = service.clone();
            let app_handle = app_handle.clone();
            let window_id = window_id.clone();
            let results = results.clone();

            // 使用tokio::spawn创建真正的独立任务
            tokio::spawn(async move {
                // 在获取信号量前检查停止标志
                if get_stop_flag(&window_id) {
                    println!("[DEBUG] 任务 {original_index} 因停止标志取消");
                    return original_index;
                }

                let _permit = semaphore.acquire().await.unwrap();

                // 获取信号量后再次检查停止标志
                if get_stop_flag(&window_id) {
                    println!("[DEBUG] 任务 {original_index} 获取信号量后因停止标志取消");
                    return original_index;
                }

                // 添加轻微随机延迟（50-150ms），避免所有请求同时发送导致RPC限流
                let delay = Duration::from_millis(50 + (rand::random::<u64>() % 100));
                sleep(delay).await;

                // 通知前端该项目开始执行
                let mut updating_item = item.clone();
                updating_item.exec_status = "1".to_string();
                let query_id = params.query_id.clone().unwrap_or_default();
                if let Err(e) = app_handle.emit("balance_item_update", serde_json::json!({
                    "index": original_index,
                    "item": updating_item,
                    "window_id": window_id,
                    "query_id": query_id
                })) {
                    println!("发送开始执行事件失败: {e}");
                }

                // 直接调用查询函数（已在 tokio 运行时中）
                let result = service.query_single_item(item, &params, &window_id).await;

                // 将结果存入对应索引位置
                let mut results_guard = results.lock().unwrap();
                results_guard[original_index] = Some(result.clone());
                let result_for_emit = result.clone();
                drop(results_guard);

                // 通知前端该项目查询完成
                if let Err(e) = app_handle.emit("balance_item_update", serde_json::json!({
                    "index": original_index,
                    "item": result_for_emit,
                    "window_id": window_id,
                    "query_id": query_id
                })) {
                    println!("发送查询完成事件失败: {e}");
                }

                original_index
            })
        }).collect();

        // 等待所有tokio任务完成
        let mut join_errors = 0;
        for task in tasks {
            match task.await {
                Ok(_) => {
                    // 任务正常完成，结果已在results中
                }
                Err(e) => {
                    println!("[ERROR] 任务执行失败: {e}");
                    join_errors += 1;
                }
            }
        }

        // 按原始顺序收集结果
        let mut ordered_results = Vec::with_capacity(total_items);
        let results_guard = results.lock().unwrap();
        for i in 0..total_items {
            if let Some(item) = results_guard[i].clone() {
                ordered_results.push(item);
            } else {
                // 创建错误结果
                let error_item = QueryItem {
                    key: String::new(),
                    address: String::new(),
                    private_key: None,
                    plat_balance: None,
                    coin_balance: None,
                    nonce: None,
                    retry_flag: true,
                    exec_status: "3".to_string(),
                    error_msg: Some(format!("任务执行失败{}", if join_errors > 0 { format!("（{join_errors}个任务异常）") } else { String::new() })),
                };
                ordered_results.push(error_item);
            }
        }

        let success = ordered_results.iter().all(|item| item.exec_status == "2");
        let error_msg = if success {
            None
        } else {
            let first_error = ordered_results.iter()
                .find(|item| item.error_msg.is_some() && !item.error_msg.as_ref().unwrap().is_empty())
                .and_then(|item| item.error_msg.clone());
            Some(first_error.unwrap_or_else(|| "查询失败".to_string()))
        };

        println!("查询完成，成功: {success}");

        QueryResult {
            success,
            items: ordered_results,
            error_msg,
        }
    }
}

// Tauri 命令：查询余额
#[tauri::command]
pub async fn query_balances_simple(params: QueryParams) -> Result<QueryResult, String> {
    let service = SimpleBalanceQueryService::new();
    
    let result = service.query_balances(params).await;
    Ok(result)
}

// 带有实时更新的查询余额命令
#[tauri::command]
pub async fn query_balances_with_updates<R: tauri::Runtime>(
    params: QueryParams,
    app_handle: tauri::AppHandle<R>,
) -> Result<QueryResult, String> {
    let window_id = params.window_id.clone().unwrap_or_default();
    let service = SimpleBalanceQueryService::new();
    
    let result = service.query_balances_with_updates(params, app_handle, window_id).await;
    Ok(result)
}

// 停止余额查询命令
#[tauri::command]
pub async fn stop_balance_query(window_id: String) -> Result<(), String> {
    set_stop_flag(&window_id, true);
    println!("收到停止余额查询请求，窗口ID: {window_id}");
    Ok(())
}

// 重置停止标志
#[tauri::command]
pub async fn reset_balance_query_stop(window_id: String) -> Result<(), String> {
    reset_stop_flag(&window_id);
    println!("重置余额查询停止标志，窗口ID: {window_id}");
    Ok(())
}
