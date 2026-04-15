use anyhow::Result;
use sqlx::Row;
use crate::database::{models::*, get_database_pool};
use chrono::Utc;

/// 链服务（动态获取数据库连接池，避免 pool 关闭后引用失效）
#[derive(Clone)]
pub struct ChainService;

impl ChainService {
    /// 创建链服务实例
    pub fn new() -> Self {
        Self
    }

    /// 获取所有活跃链的配置信息
    pub async fn get_all_chains(&self) -> Result<Vec<ChainInfo>> {
        let pool = get_database_pool();
        let chains = sqlx::query_as::<_, Chain>(
            "SELECT * FROM chains WHERE is_active = TRUE ORDER BY chain_name"
        ).fetch_all(&pool).await?;

        let mut chain_infos = Vec::new();
        for chain in chains {
            let rpc_urls = self.get_chain_rpc_urls(chain.id).await?;
            chain_infos.push(ChainInfo {
                key: chain.chain_key,
                chain: chain.chain_name,
                ecosystem: chain.ecosystem,
                chain_id: chain.chain_id,
                symbol: chain.native_currency_symbol,
                currency_name: chain.native_currency_name,
                decimals: chain.native_currency_decimals,
                pic_data: chain.pic_data,
                scan_url: chain.scan_url.unwrap_or_default(),
                scan_api: chain.scan_api.unwrap_or_default(),
                verify_api: chain.verify_api.unwrap_or_default(),
                check_verify_api: chain.check_verify_api.unwrap_or_default(),
                rpc_urls,
            });
        }

        Ok(chain_infos)
    }

    /// 根据chain_key获取链信息
    pub async fn get_chain_by_key(&self, chain_key: &str) -> Result<Option<Chain>> {
        let pool = get_database_pool();
        let chain = sqlx::query_as::<_, Chain>(
            "SELECT * FROM chains WHERE chain_key = ? AND is_active = TRUE"
        )
        .bind(chain_key)
        .fetch_optional(&pool)
        .await?;

        Ok(chain)
    }

    /// 获取链的RPC URLs
    pub async fn get_chain_rpc_urls(&self, chain_id: i64) -> Result<Vec<String>> {
        let pool = get_database_pool();
        let rows = sqlx::query(
            "SELECT rpc_url FROM rpc_providers WHERE chain_id = ? AND is_active = TRUE ORDER BY priority ASC"
        )
        .bind(chain_id)
        .fetch_all(&pool)
        .await?;

        let rpc_urls: Vec<String> = rows.into_iter().map(|row| {
            row.get::<String, _>("rpc_url")
        }).collect();

        Ok(rpc_urls)
    }

    /// 添加新链
    pub async fn add_chain(&self, request: CreateChainRequest) -> Result<i64> {
        println!("正在添加新链: key={}, name={}, ecosystem={}", request.chain_key, request.chain_name, request.ecosystem);
        let pool = get_database_pool();

        // 检查链标识符是否已存在
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM chains WHERE chain_key = ?"
        )
        .bind(&request.chain_key)
        .fetch_one(&pool)
        .await?;

        if exists > 0 {
            return Err(anyhow::anyhow!("链标识符 '{}' 已存在，请使用不同的标识符", request.chain_key));
        }

        let now = Utc::now();
        let chain_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO chains (
                chain_key, chain_name, ecosystem, chain_id, native_currency_symbol,
                native_currency_name, native_currency_decimals, pic_data,
                scan_url, scan_api, verify_api, check_verify_api, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&request.chain_key)
        .bind(&request.chain_name)
        .bind(&request.ecosystem)
        .bind(request.chain_id)
        .bind(&request.native_currency_symbol)
        .bind(&request.native_currency_name)
        .bind(request.native_currency_decimals)
        .bind(&request.pic_data)
        .bind(&request.scan_url)
        .bind(&request.scan_api)
        .bind(&request.verify_api)
        .bind(&request.check_verify_api)
        .bind(now)
        .bind(now)
        .fetch_one(&pool)
        .await?;

        // 添加基础代币
        sqlx::query(
            r#"
            INSERT INTO tokens (
                chain_id, token_key, token_name, symbol, decimals, token_type, contract_type, abi, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, 'base', NULL, NULL, ?, ?)
            "#
        )
        .bind(chain_id)
        .bind(format!("{}_base", request.chain_key))
        .bind(&request.native_currency_name)
        .bind(&request.native_currency_symbol)
        .bind(request.native_currency_decimals)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await?;

        // 添加RPC URLs
        if let Some(rpc_urls) = request.rpc_urls {
            for rpc_url in rpc_urls.iter() {
                if !rpc_url.trim().is_empty() {
                    sqlx::query(
                        r#"
                        INSERT INTO rpc_providers (
                            chain_id, rpc_url, priority, created_at, updated_at
                        ) VALUES (?, ?, ?, ?, ?)
                        "#
                    )
                    .bind(chain_id)
                    .bind(rpc_url.trim())
                    .bind(100)
                    .bind(now)
                    .bind(now)
                    .execute(&pool)
                    .await?;
                }
            }
        }

        Ok(chain_id)
    }

    /// 更新链信息
    pub async fn update_chain(&self, chain_key: &str, request: UpdateChainRequest) -> Result<()> {
        let pool = get_database_pool();
        let now = Utc::now();

        // 检查链是否存在
        let chain = self.get_chain_by_key(chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {chain_key}"))?;

        // 更新链基本信息
        sqlx::query(
            r#"
            UPDATE chains SET
                chain_name = ?, ecosystem = ?, chain_id = ?, native_currency_symbol = ?,
                native_currency_name = ?, native_currency_decimals = ?, pic_data = ?,
                scan_url = ?, scan_api = ?, verify_api = ?, check_verify_api = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&request.chain_name)
        .bind(&request.ecosystem)
        .bind(request.chain_id)
        .bind(&request.native_currency_symbol)
        .bind(&request.native_currency_name)
        .bind(request.native_currency_decimals)
        .bind(&request.pic_data)
        .bind(&request.scan_url)
        .bind(&request.scan_api)
        .bind(&request.verify_api)
        .bind(&request.check_verify_api)
        .bind(now)
        .bind(chain.id)
        .execute(&pool)
        .await?;

        // 更新基础代币信息
        sqlx::query(
            r#"
            UPDATE tokens SET
                token_name = ?, symbol = ?, decimals = ?, updated_at = ?
            WHERE chain_id = ? AND token_type = 'base'
            "#
        )
        .bind(&request.native_currency_name)
        .bind(&request.native_currency_symbol)
        .bind(request.native_currency_decimals)
        .bind(now)
        .bind(chain.id)
        .execute(&pool)
        .await?;

        // 如果提供了新的RPC URLs，先删除旧的，再添加新的
        if let Some(rpc_urls) = request.rpc_urls {
            // 删除旧的RPC提供商
            sqlx::query(
                "UPDATE rpc_providers SET is_active = FALSE, updated_at = ? WHERE chain_id = ?"
            )
            .bind(now)
            .bind(chain.id)
            .execute(&pool)
            .await?;

            // 添加新的RPC提供商
            for rpc_url in rpc_urls.iter() {
                sqlx::query(
                    r#"
                    INSERT INTO rpc_providers (
                        chain_id, rpc_url, priority, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?)
                    "#
                )
                .bind(chain.id)
                .bind(rpc_url)
                .bind(100)
                .bind(now)
                .bind(now)
                .execute(&pool)
                .await?;
            }
        }

        Ok(())
    }

    /// 删除链（真实删除）
    pub async fn remove_chain(&self, chain_key: &str) -> Result<()> {
        let pool = get_database_pool();

        // 检查链是否存在
        let chain = self.get_chain_by_key(chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {chain_key}"))?;

        // 开始事务
        let mut tx = pool.begin().await?;

        // 删除链下的所有代币
        sqlx::query("DELETE FROM tokens WHERE chain_id = ?")
            .bind(chain.id)
            .execute(&mut *tx)
            .await?;

        // 删除链下的所有RPC提供商
        sqlx::query("DELETE FROM rpc_providers WHERE chain_id = ?")
            .bind(chain.id)
            .execute(&mut *tx)
            .await?;

        // 删除链本身
        sqlx::query("DELETE FROM chains WHERE id = ?")
            .bind(chain.id)
            .execute(&mut *tx)
            .await?;

        // 提交事务
        tx.commit().await?;

        Ok(())
    }

    /// 添加代币
    pub async fn add_token(&self, request: CreateTokenRequest) -> Result<i64> {
        let pool = get_database_pool();

        // 获取链ID
        let chain = self.get_chain_by_key(&request.chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {}", request.chain_key))?;

        let now = Utc::now();
        let token_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO tokens (
                chain_id, token_key, token_name, symbol, contract_address,
                decimals, token_type, contract_type, abi, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(chain.id)
        .bind(&request.token_key)
        .bind(&request.token_name)
        .bind(&request.symbol)
        .bind(&request.contract_address)
        .bind(request.decimals)
        .bind(&request.token_type)
        .bind(&request.contract_type)
        .bind(&request.abi)
        .bind(now)
        .bind(now)
        .fetch_one(&pool)
        .await?;

        Ok(token_id)
    }

    /// 删除代币
    pub async fn remove_token(&self, chain_key: &str, token_key: &str) -> Result<()> {
        let pool = get_database_pool();

        // 直接删除代币记录，不使用软删除
        sqlx::query(
            r#"
            DELETE FROM tokens
            WHERE tokens.chain_id IN (
                SELECT id FROM chains WHERE chain_key = ?
            ) AND tokens.token_key = ?
            "#
        )
        .bind(chain_key)
        .bind(token_key)
        .execute(&pool)
        .await?;

        Ok(())
    }

    /// 获取代币信息（用于查询余额）
    #[allow(dead_code)]
    pub async fn get_token_info(&self, chain_key: &str, token_key: &str) -> Result<Option<Token>> {
        let pool = get_database_pool();
        let token = sqlx::query_as::<_, Token>(
            r#"
            SELECT t.* FROM tokens t
            JOIN chains c ON t.chain_id = c.id
            WHERE c.chain_key = ? AND t.token_key = ?
            "#
        )
        .bind(chain_key)
        .bind(token_key)
        .fetch_optional(&pool)
        .await?;

        Ok(token)
    }

    /// 根据链key和合约地址获取代币的decimals配置
    pub async fn get_token_decimals_by_contract(&self, chain_key: &str, contract_address: &str) -> Result<Option<i32>> {
        let pool = get_database_pool();
        let decimals = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT t.decimals FROM tokens t
            JOIN chains c ON t.chain_id = c.id
            WHERE c.chain_key = ? AND t.contract_address = ?
            "#
        )
        .bind(chain_key)
        .bind(contract_address)
        .fetch_optional(&pool)
        .await?;

        Ok(decimals)
    }

    /// 根据链key和token_key获取代币的decimals配置
    #[allow(dead_code)]
    pub async fn get_token_decimals_by_key(&self, chain_key: &str, token_key: &str) -> Result<Option<i32>> {
        let pool = get_database_pool();
        let decimals = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT t.decimals FROM tokens t
            JOIN chains c ON t.chain_id = c.id
            WHERE c.chain_key = ? AND t.token_key = ?
            "#
        )
        .bind(chain_key)
        .bind(token_key)
        .fetch_optional(&pool)
        .await?;

        Ok(decimals)
    }

    /// 更新代币的ABI
    pub async fn update_token_abi(&self, chain_key: &str, token_key: &str, abi: Option<String>) -> Result<()> {
        let pool = get_database_pool();

        // 获取链信息验证链是否存在
        let chain = self.get_chain_by_key(chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {chain_key}"))?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE tokens SET abi = ?, updated_at = ?
            WHERE chain_id = ? AND token_key = ?
            "#
        )
        .bind(&abi)
        .bind(Utc::now())
        .bind(chain.id)
        .bind(token_key)
        .execute(&pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("代币不存在: {chain_key}/{token_key}"));
        }

        Ok(())
    }

    /// 更新代币信息
    pub async fn update_token(&self, chain_key: &str, token_key: &str, request: UpdateTokenRequest) -> Result<()> {
        let pool = get_database_pool();

        // 获取链信息验证链是否存在
        let chain = self.get_chain_by_key(chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {chain_key}"))?;

        let now = Utc::now();
        let rows_affected = sqlx::query(
            r#"
            UPDATE tokens SET
                token_name = ?, symbol = ?, contract_address = ?,
                decimals = ?, token_type = ?, contract_type = ?, abi = ?, updated_at = ?
            WHERE chain_id = ? AND token_key = ?
            "#
        )
        .bind(&request.token_name)
        .bind(&request.symbol)
        .bind(&request.contract_address)
        .bind(request.decimals)
        .bind(&request.token_type)
        .bind(&request.contract_type)
        .bind(&request.abi)
        .bind(now)
        .bind(chain.id)
        .bind(token_key)
        .execute(&pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("代币不存在: {chain_key}/{token_key}"));
        }

        Ok(())
    }

    /// 获取所有 RPC 提供商
    #[allow(dead_code)]
    pub async fn get_all_rpc_providers(&self) -> Result<Vec<RpcProvider>> {
        let pool = get_database_pool();
        let providers = sqlx::query_as::<_, RpcProvider>(
            "SELECT * FROM rpc_providers ORDER BY chain_id, priority ASC"
        ).fetch_all(&pool).await?;

        Ok(providers)
    }

    /// 根据链标识符获取 RPC 提供商
    pub async fn get_rpc_providers_by_chain(&self, chain_key: &str) -> Result<Vec<RpcProvider>> {
        let pool = get_database_pool();
        let providers = sqlx::query_as::<_, RpcProvider>(
            r#"
            SELECT rp.* FROM rpc_providers rp
            JOIN chains c ON rp.chain_id = c.id
            WHERE c.chain_key = ?
            ORDER BY rp.priority ASC
            "#
        )
        .bind(chain_key)
        .fetch_all(&pool)
        .await?;

        Ok(providers)
    }

    /// 添加 RPC 提供商
    pub async fn add_rpc_provider(&self, chain_id: i64, request: &CreateRpcProviderRequest) -> Result<RpcProvider> {
        let pool = get_database_pool();
        let now = Utc::now();
        let provider_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO rpc_providers (
                chain_id, rpc_url, priority, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(chain_id)
        .bind(&request.rpc_url)
        .bind(request.priority)
        .bind(now)
        .bind(now)
        .fetch_one(&pool)
        .await?;

        let provider = sqlx::query_as::<_, RpcProvider>(
            "SELECT * FROM rpc_providers WHERE id = ?"
        )
        .bind(provider_id)
        .fetch_one(&pool)
        .await?;

        Ok(provider)
    }

    /// 通过链标识符添加 RPC 提供商
    pub async fn add_rpc_provider_by_chain_key(&self, chain_key: &str, request: &CreateRpcProviderRequest) -> Result<RpcProvider> {
        // 首先根据 chain_key 获取 chain_id
        let chain = self.get_chain_by_key(chain_key).await?
            .ok_or_else(|| anyhow::anyhow!("链不存在: {chain_key}"))?;

        // 调用原有的添加方法
        self.add_rpc_provider(chain.id, request).await
    }

    /// 更新 RPC 提供商
    pub async fn update_rpc_provider(&self, id: i64, rpc_url: &str, is_active: bool, priority: i32) -> Result<RpcProvider> {
        let pool = get_database_pool();
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE rpc_providers SET
                rpc_url = ?, is_active = ?, priority = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(rpc_url)
        .bind(is_active)
        .bind(priority)
        .bind(now)
        .bind(id)
        .execute(&pool)
        .await?;

        let provider = sqlx::query_as::<_, RpcProvider>(
            "SELECT * FROM rpc_providers WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;

        Ok(provider)
    }

    /// 删除 RPC 提供商
    pub async fn delete_rpc_provider(&self, id: i64) -> Result<()> {
        let pool = get_database_pool();
        sqlx::query("DELETE FROM rpc_providers WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await?;

        Ok(())
    }
}
