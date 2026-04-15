use tauri::State;
use crate::database::chain_service::ChainService;
use crate::database::models::CreateRpcProviderRequest;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use reqwest;

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcProviderInfo {
    pub id: i64,
    pub chain_id: i64,
    pub rpc_url: String,
    pub is_active: bool,
    pub priority: i32,
    pub last_success_at: Option<String>,
    pub failure_count: i32,
    pub avg_response_time_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRpcProviderRequest {
    pub rpc_url: String,
    pub is_active: bool,
    pub priority: i32,
}

#[derive(Debug, Serialize)]
pub struct RpcTestResult {
    pub success: bool,
    pub response_time_ms: u64,
}

/// 获取指定链的 RPC 提供商
#[tauri::command]
pub async fn get_rpc_providers(
    chain_key: String,
    chain_service: State<'_, ChainService>,
) -> Result<Vec<RpcProviderInfo>, String> {
    let providers = chain_service.get_rpc_providers_by_chain(&chain_key).await
        .map_err(|e| format!("获取 RPC 提供商失败: {e}"))?;
    
    let provider_infos = providers.into_iter().map(|p| RpcProviderInfo {
        id: p.id,
        chain_id: p.chain_id,
        rpc_url: p.rpc_url,
        is_active: p.is_active,
        priority: p.priority,
        last_success_at: p.last_success_at.map(|dt| dt.to_rfc3339()),
        failure_count: p.failure_count,
        avg_response_time_ms: p.avg_response_time_ms,
    }).collect();
    
    Ok(provider_infos)
}

/// 添加 RPC 提供商
#[tauri::command]
pub async fn add_rpc_provider(
    chain_key: String,
    rpc_url: String,
    priority: i32,
    chain_service: State<'_, ChainService>,
) -> Result<RpcProviderInfo, String> {
    let request = CreateRpcProviderRequest {
        chain_key: chain_key.clone(),
        rpc_url: rpc_url.clone(),
        priority,
    };
    
    let provider = chain_service.add_rpc_provider_by_chain_key(&chain_key, &request).await
        .map_err(|e| format!("添加 RPC 提供商失败: {e}"))?;
    
    Ok(RpcProviderInfo {
        id: provider.id,
        chain_id: provider.chain_id,
        rpc_url: provider.rpc_url,
        is_active: provider.is_active,
        priority: provider.priority,
        last_success_at: provider.last_success_at.map(|dt| dt.to_rfc3339()),
        failure_count: provider.failure_count,
        avg_response_time_ms: provider.avg_response_time_ms,
    })
}

/// 更新 RPC 提供商
#[tauri::command]
pub async fn update_rpc_provider(
    id: i64,
    request: UpdateRpcProviderRequest,
    chain_service: State<'_, ChainService>,
) -> Result<RpcProviderInfo, String> {
    let provider = chain_service.update_rpc_provider(id, &request.rpc_url, request.is_active, request.priority).await
        .map_err(|e| format!("更新 RPC 提供商失败: {e}"))?;
    
    Ok(RpcProviderInfo {
        id: provider.id,
        chain_id: provider.chain_id,
        rpc_url: provider.rpc_url,
        is_active: provider.is_active,
        priority: provider.priority,
        last_success_at: provider.last_success_at.map(|dt| dt.to_rfc3339()),
        failure_count: provider.failure_count,
        avg_response_time_ms: provider.avg_response_time_ms,
    })
}

/// 删除 RPC 提供商
#[tauri::command]
pub async fn delete_rpc_provider(
    id: i64,
    chain_service: State<'_, ChainService>,
) -> Result<(), String> {
    chain_service.delete_rpc_provider(id).await
        .map_err(|e| format!("删除 RPC 提供商失败: {e}"))?;
    
    Ok(())
}

/// 测试 RPC 连接
#[tauri::command]
pub async fn test_rpc_connection(
    rpc_url: String,
) -> Result<RpcTestResult, String> {
    use crate::wallets_tool::ecosystems::ethereum::proxy_manager::PROXY_MANAGER;
    
    println!("[RPC测试] 开始测试RPC连接: {rpc_url}");
    
    // 检查代理状态
    let proxy_config = PROXY_MANAGER.get_config();
    let using_proxy = proxy_config.enabled && !proxy_config.proxies.is_empty();
    
    if using_proxy {
        println!("[RPC测试] 代理已启用，使用代理进行测试 (代理数量: {})", proxy_config.proxies.len());
    } else if proxy_config.enabled {
        println!("[RPC测试] 代理已启用但无可用代理，使用直连模式");
    } else {
        println!("[RPC测试] 代理未启用，使用直连模式");
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
            println!("[RPC测试] 使用代理客户端发送请求");
            (proxy_client, format!("代理: {selected_proxy}"))
        } else {
            println!("[RPC测试] 代理客户端创建失败，使用直连模式");
            let default_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create default HTTP client");
            (default_client, "直连模式".to_string())
        }
    } else {
        println!("[RPC测试] 使用默认客户端发送请求（直连模式）");
        let default_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create default HTTP client");
        (default_client, "直连模式".to_string())
    };
    
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });
    
    println!("[RPC测试] 发送请求到: {rpc_url} [{proxy_info}]");
    
    let success = match client.post(&rpc_url)
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            println!("[RPC测试] 收到响应，状态码: {}", response.status());
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        println!("[RPC测试] 响应JSON: {json}");
                        let has_result = json.get("result").is_some();
                        println!("[RPC测试] 是否包含result字段: {has_result}");
                        has_result
                    }
                    Err(e) => {
                        println!("[RPC测试] 解析JSON失败: {e}");
                        false
                    }
                }
            } else {
                println!("[RPC测试] HTTP状态码不成功: {}", response.status());
                false
            }
        }
        Err(e) => {
            println!("[RPC测试] 请求失败: {e}");
            false
        }
    };
    
    let response_time_ms = start_time.elapsed().as_millis() as u64;
    
    println!("[RPC测试] 测试完成 - 成功: {success}, 响应时间: {response_time_ms}ms");
    
    Ok(RpcTestResult {
        success,
        response_time_ms,
    })
}