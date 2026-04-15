use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;
use alloy_provider::{Provider, RootProvider};
use alloy_transport_http::{Http, Client as AlloyClient};
use alloy::rpc::client::RpcClient;
use alloy_network::Ethereum;
use alloy_primitives::U256;
use reqwest::{Client, Proxy};
use url::Url;
use std::sync::Arc;
use crate::database::chain_service::ChainService;
use super::alloy_utils::format_wei_to_gwei;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainRpcConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_urls: Vec<String>,
    pub currency_symbol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GasPriceInfo {
    pub gas_price_gwei: f64,
    pub chain: String,
}

pub async fn create_http_client_with_proxy(proxy_url: Option<&str>) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(30));
    
    if let Some(proxy_str) = proxy_url {
        let proxy = Proxy::all(proxy_str)
            .map_err(|e| format!("代理地址解析失败: {e}"))?;
        builder = builder.proxy(proxy);
    }
    
    builder.build()
        .map_err(|e| format!("HTTP客户端创建失败: {e}"))
}

pub type AlloyProvider = RootProvider<Ethereum>;

pub async fn create_provider_with_client(
    rpc_url: &str,
    proxy_url: Option<&str>,
) -> Result<AlloyProvider, String> {
    let url: Url = rpc_url.parse()
        .map_err(|e| format!("RPC URL 解析失败: {e}"))?;
    
    let client = create_http_client_with_proxy(proxy_url).await?;
    let client: AlloyClient = client;

    let http = Http::with_client(client, url);
    let rpc_client = RpcClient::new(http, false);
    let provider = RootProvider::new(rpc_client);
    
    Ok(provider)
}

pub async fn get_all_chain_configs() -> Result<HashMap<String, ChainRpcConfig>, Box<dyn std::error::Error>> {
    let chain_service = ChainService::new();
    
    let chain_infos = chain_service.get_all_chains().await?;
    let mut configs = HashMap::new();
    
    for chain_info in chain_infos {
        configs.insert(chain_info.key.clone(), ChainRpcConfig {
            name: chain_info.chain,
            chain_id: chain_info.chain_id as u64,
            rpc_urls: chain_info.rpc_urls,
            currency_symbol: chain_info.symbol,
        });
    }
    
    Ok(configs)
}

pub struct ProviderUtils;

impl ProviderUtils {
    pub async fn get_chain_config(chain: &str) -> Result<ChainRpcConfig, String> {
        let configs = get_all_chain_configs()
            .await
            .map_err(|e| e.to_string())?;
        configs
            .get(chain)
            .cloned()
            .ok_or_else(|| format!("不支持的链: {chain}"))
    }

    pub async fn get_chain_id(chain: &str) -> Result<u64, String> {
        let cfg = Self::get_chain_config(chain).await?;
        Ok(cfg.chain_id)
    }
    
    fn get_random_rpc_url(rpc_urls: &[String]) -> Result<&str, String> {
        if rpc_urls.is_empty() {
            return Err("没有可用的RPC URL".to_string());
        }
        let mut rng = rand::thread_rng();
        Ok(&rpc_urls[rng.gen_range(0..rpc_urls.len())])
    }
    
    pub async fn get_provider(chain: &str) -> Result<AlloyProvider, Box<dyn std::error::Error>> {
        use crate::wallets_tool::ecosystems::ethereum::proxy_manager::PROXY_MANAGER;
        
        println!("[DEBUG] get_provider - 开始获取链 '{chain}' 的Provider");
        
        let chain_config = Self::get_chain_config(chain).await?;
        println!("[DEBUG] get_provider - 获取到链配置，chain_id: {}, rpc_urls数量: {}", 
                 chain_config.chain_id, chain_config.rpc_urls.len());
        
        let rpc_url = Self::get_random_rpc_url(&chain_config.rpc_urls)
            .map_err(|e| format!("获取RPC URL失败: {e}. 请检查链 '{chain}' 是否配置了RPC节点。"))?;
        println!("[DEBUG] get_provider - 选择的RPC URL: {rpc_url}");
        
        let proxy_url = PROXY_MANAGER.get_random_proxy();
        if proxy_url.is_some() {
            println!("[DEBUG] get_provider - 使用代理: {}", proxy_url.as_ref().unwrap());
        } else {
            println!("[DEBUG] get_provider - 未使用代理");
        }
        
        let provider = create_provider_with_client(rpc_url, proxy_url.as_deref())
            .await
            .map_err(|e| {
                println!("[ERROR] get_provider - Provider创建失败: {e}");
                e
            })?;
        
        println!("[DEBUG] get_provider - Provider创建成功");
        Ok(provider)
    }
    
    pub async fn get_base_gas_price(chain: &str) -> Result<f64, Box<dyn std::error::Error>> {
        let provider = Arc::new(Self::get_provider(chain).await?);
        let gas_price = provider.get_gas_price().await?;
        let gas_price_gwei: f64 = format_wei_to_gwei(U256::from(gas_price)).parse::<f64>()?;
        Ok(gas_price_gwei)
    }
    
    pub async fn test_rpc_connection(rpc_url: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let provider = create_provider_with_client(rpc_url, None).await?;
        let _block_number = provider.get_block_number().await?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn get_chain_gas_price(chain: String) -> Result<GasPriceInfo, String> {
    match ProviderUtils::get_base_gas_price(&chain).await {
        Ok(gas_price) => Ok(GasPriceInfo {
            gas_price_gwei: gas_price,
            chain,
        }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn test_rpc_url(rpc_url: String) -> Result<bool, String> {
    match ProviderUtils::test_rpc_connection(&rpc_url).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_multiple_gas_prices(chains: Vec<String>) -> Result<Vec<GasPriceInfo>, String> {
    let mut results = Vec::new();
    
    for chain in chains {
        match ProviderUtils::get_base_gas_price(&chain).await {
            Ok(gas_price) => {
                results.push(GasPriceInfo {
                    gas_price_gwei: gas_price,
                    chain: chain.clone(),
                });
            }
            Err(e) => {
                eprintln!("获取{chain}链Gas Price失败: {e}");
                results.push(GasPriceInfo {
                    gas_price_gwei: 0.0,
                    chain: chain.clone(),
                });
            }
        }
    }
    
    Ok(results)
}
