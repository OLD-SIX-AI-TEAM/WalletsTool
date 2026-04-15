use anyhow::Result;
use sqlx::{SqlitePool, Row};
use crate::database::models::*;
use chrono::Utc;
use rand::prelude::*;

/// RPC服务
pub struct RpcService {
    pool: SqlitePool,
}

impl RpcService {
    pub fn new(pool: &SqlitePool) -> Self {
        Self { pool: pool.clone() }
    }

    /// 为指定链获取最佳RPC URL（按优先级和成功率）
    #[allow(dead_code)]
    pub async fn get_best_rpc_url(&self, chain_key: &str) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT rp.rpc_url 
            FROM rpc_providers rp
            JOIN chains c ON rp.chain_id = c.id
            WHERE c.chain_key = ? AND rp.is_active = TRUE
            ORDER BY rp.priority ASC, rp.failure_count ASC, rp.avg_response_time_ms ASC NULLS LAST
            LIMIT 1
            "#
        )
        .bind(chain_key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("没有可用的RPC提供商: {chain_key}"))?;

        Ok(row.get::<String, _>("rpc_url"))
    }

    /// 获取指定链的随机RPC URL（用于负载均衡）
    pub async fn get_random_rpc_url(&self, chain_key: &str) -> Result<String> {
        let rows = sqlx::query(
            r#"
            SELECT rp.rpc_url 
            FROM rpc_providers rp
            JOIN chains c ON rp.chain_id = c.id
            WHERE c.chain_key = ? AND rp.is_active = TRUE
            ORDER BY rp.priority ASC
            "#
        )
        .bind(chain_key)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("没有可用的RPC提供商: {chain_key}"));
        }

        let rpc_urls: Vec<String> = rows.into_iter().map(|row| {
            row.get::<String, _>("rpc_url")
        }).collect();

        let mut rng = thread_rng();
        let index = rng.gen_range(0..rpc_urls.len());
        Ok(rpc_urls[index].clone())
    }

    /// 获取链的所有RPC URLs
    #[allow(dead_code)]
    pub async fn get_all_rpc_urls(&self, chain_key: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT rp.rpc_url 
            FROM rpc_providers rp
            JOIN chains c ON rp.chain_id = c.id
            WHERE c.chain_key = ? AND rp.is_active = TRUE
            ORDER BY rp.priority ASC
            "#
        )
        .bind(chain_key)
        .fetch_all(&self.pool)
        .await?;

        let rpc_urls: Vec<String> = rows.into_iter().map(|row| {
            row.get::<String, _>("rpc_url")
        }).collect();

        Ok(rpc_urls)
    }

    /// 添加RPC提供商
    #[allow(dead_code)]
    pub async fn add_rpc_provider(&self, request: CreateRpcProviderRequest) -> Result<i64> {
        // 获取链ID
        let row = sqlx::query(
            "SELECT id FROM chains WHERE chain_key = ? AND is_active = TRUE"
        )
        .bind(&request.chain_key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("链不存在: {}", request.chain_key))?;

        let chain_id: i64 = row.get("id");

        let now = Utc::now();
        let rpc_id = sqlx::query_scalar::<_, i64>(
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
        .fetch_one(&self.pool)
        .await?;

        Ok(rpc_id)
    }

    /// 记录RPC请求成功
    #[allow(dead_code)]
    pub async fn record_rpc_success(&self, rpc_url: &str, response_time_ms: i32) -> Result<()> {
        let now = Utc::now();
        
        // 更新成功时间和响应时间，重置失败计数
        sqlx::query(
            r#"
            UPDATE rpc_providers 
            SET last_success_at = ?, 
                failure_count = 0,
                avg_response_time_ms = CASE 
                    WHEN avg_response_time_ms IS NULL THEN ?
                    ELSE (avg_response_time_ms + ?) / 2
                END,
                updated_at = ?
            WHERE rpc_url = ?
            "#
        )
        .bind(now)
        .bind(response_time_ms)
        .bind(response_time_ms)
        .bind(now)
        .bind(rpc_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 记录RPC请求失败
    #[allow(dead_code)]
    pub async fn record_rpc_failure(&self, rpc_url: &str) -> Result<()> {
        let now = Utc::now();
        
        // 增加失败计数
        sqlx::query(
            r#"
            UPDATE rpc_providers 
            SET failure_count = failure_count + 1,
                updated_at = ?
            WHERE rpc_url = ?
            "#
        )
        .bind(now)
        .bind(rpc_url)
        .execute(&self.pool)
        .await?;

        // 如果失败次数过多（比如超过10次），暂时禁用该RPC
        sqlx::query(
            r#"
            UPDATE rpc_providers 
            SET is_active = FALSE,
                updated_at = ?
            WHERE rpc_url = ? AND failure_count >= 10
            "#
        )
        .bind(now)
        .bind(rpc_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 重新激活失败的RPC提供商（可以定期调用）
    #[allow(dead_code)]
    pub async fn reactivate_failed_rpcs(&self) -> Result<()> {
        let now = Utc::now();
        
        // 重新激活24小时前失败的RPC（给它们重新尝试的机会）
        sqlx::query(
            r#"
            UPDATE rpc_providers 
            SET is_active = TRUE,
                failure_count = 0,
                updated_at = ?
            WHERE is_active = FALSE AND updated_at < datetime('now', '-1 day')
            "#
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }



    /// 获取RPC提供商统计信息
    #[allow(dead_code)]
    pub async fn get_rpc_stats(&self, chain_key: &str) -> Result<Vec<RpcProvider>> {
        let stats = sqlx::query_as::<_, RpcProvider>(
            r#"
            SELECT rp.* FROM rpc_providers rp
            JOIN chains c ON rp.chain_id = c.id
            WHERE c.chain_key = ?
            ORDER BY rp.priority ASC, rp.failure_count ASC
            "#
        )
        .bind(chain_key)
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }
}
