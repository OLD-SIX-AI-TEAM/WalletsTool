use sqlx::SqlitePool;
use anyhow::Result;
use chrono::Utc;
use rand::seq::SliceRandom;
use serde_json;
use std::sync::Arc;

use crate::wallets_tool::airdrop::models::*;
use crate::wallets_tool::security;

/// 任务执行器
#[allow(dead_code)]
pub struct TaskExecutor {
    pool: SqlitePool,
}

#[allow(dead_code)]
impl TaskExecutor {
    /// 创建新的任务执行器
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 执行单个任务
    pub async fn execute_task(&self, task: &AutomationTask) -> Result<()> {
        // 解析钱包ID列表
        let wallet_ids: Vec<i64> = serde_json::from_str(&task.wallet_ids)?;

        if wallet_ids.is_empty() {
            return Err(anyhow::anyhow!("任务没有关联钱包"));
        }

        // 获取脚本
        let script: AutomationScript = sqlx::query_as(
            "SELECT * FROM automation_scripts WHERE id = ?"
        )
        .bind(task.script_id)
        .fetch_one(&self.pool)
        .await?;

        // 获取所有环境配置
        let profiles: Vec<BrowserProfile> = sqlx::query_as(
            "SELECT * FROM browser_profiles"
        )
        .fetch_all(&self.pool)
        .await?;

        if profiles.is_empty() {
            return Err(anyhow::anyhow!("没有可用的环境配置"));
        }

        // 获取钱包详情
        let wallets = self.get_wallets_by_ids(&wallet_ids).await?;

        // 根据策略分配环境
        let profile_strategy = ProfileStrategy::from(task.profile_strategy.as_str());

        // 并发执行
        let concurrency = task.concurrency.max(1) as usize;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

        let mut handles = vec![];

        for (idx, wallet) in wallets.iter().enumerate() {
            let profile = self.assign_profile(&profile_strategy, &profiles, idx, task.specific_profile_id);

            let permit = semaphore.clone().acquire_owned().await?;
            let pool = self.pool.clone();
            let task = task.clone();
            let wallet = wallet.clone();
            let profile = profile.clone();
            let script_content = script.content.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                // 创建执行记录
                let execution_id = Self::create_execution(&pool, task.id, wallet.id, profile.id).await?;

                // 执行脚本
                let result = Self::execute_script(
                    &pool,
                    execution_id,
                    &script_content,
                    &wallet,
                    &profile,
                    task.timeout_seconds,
                ).await;

                match result {
                    Ok(tx_hash) => {
                        Self::update_execution_success(&pool, execution_id, tx_hash.as_deref()).await?;
                        Ok(())
                    }
                    Err(e) => {
                        Self::update_execution_failed(&pool, execution_id, &e.to_string()).await?;
                        Err(e)
                    }
                }
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let _ = handle.await;
        }

        // 更新任务统计
        self.update_task_stats(task.id).await?;

        Ok(())
    }

    /// 批量执行任务（用于执行面板）
    pub async fn execute_batch(
        &self,
        script_id: i64,
        wallet_ids: Vec<i64>,
        profile_ids: Option<Vec<i64>>,
        profile_strategy: ProfileStrategy,
        _config: &ExecutionConfig,
    ) -> Result<Vec<i64>> {
        // 获取脚本
        let _script: AutomationScript = sqlx::query_as(
            "SELECT * FROM automation_scripts WHERE id = ?"
        )
        .bind(script_id)
        .fetch_one(&self.pool)
        .await?;

        // 获取环境配置
        let profiles = if let Some(ids) = profile_ids {
            self.get_profiles_by_ids(&ids).await?
        } else {
            sqlx::query_as::<_, BrowserProfile>("SELECT * FROM browser_profiles")
                .fetch_all(&self.pool)
                .await?
        };

        if profiles.is_empty() {
            return Err(anyhow::anyhow!("没有可用的环境配置"));
        }

        // 获取钱包
        let wallets = self.get_wallets_by_ids(&wallet_ids).await?;

        // 创建执行记录
        let mut execution_ids = vec![];

        for (idx, wallet) in wallets.iter().enumerate() {
            let profile = self.assign_profile(&profile_strategy, &profiles, idx, None);

            let execution_id = Self::create_execution(&self.pool, 0, wallet.id, profile.id).await?;
            execution_ids.push(execution_id);

            // 这里可以启动异步执行
            // 实际执行逻辑与execute_task类似
        }

        Ok(execution_ids)
    }

    /// 分配环境配置
    fn assign_profile(
        &self,
        strategy: &ProfileStrategy,
        profiles: &[BrowserProfile],
        index: usize,
        specific_id: Option<i64>,
    ) -> BrowserProfile {
        match strategy {
            ProfileStrategy::Random => {
                profiles.choose(&mut rand::thread_rng()).cloned().unwrap_or_else(|| profiles[0].clone())
            }
            ProfileStrategy::Sequential => {
                profiles[index % profiles.len()].clone()
            }
            ProfileStrategy::Specific => {
                if let Some(id) = specific_id {
                    profiles.iter()
                        .find(|p| p.id == id)
                        .cloned()
                        .unwrap_or_else(|| profiles[0].clone())
                } else {
                    profiles[0].clone()
                }
            }
        }
    }

    /// 获取钱包列表
    async fn get_wallets_by_ids(&self, ids: &[i64]) -> Result<Vec<AirdropWallet>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT * FROM airdrop_wallets WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut query = sqlx::query_as::<_, AirdropWallet>(&sql);
        for id in ids {
            query = query.bind(id);
        }

        let wallets = query.fetch_all(&self.pool).await?;
        Ok(wallets)
    }

    /// 获取环境配置列表
    async fn get_profiles_by_ids(&self, ids: &[i64]) -> Result<Vec<BrowserProfile>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT * FROM browser_profiles WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut query = sqlx::query_as::<_, BrowserProfile>(&sql);
        for id in ids {
            query = query.bind(id);
        }

        let profiles = query.fetch_all(&self.pool).await?;
        Ok(profiles)
    }

    /// 创建执行记录
    async fn create_execution(
        pool: &SqlitePool,
        task_id: i64,
        wallet_id: i64,
        profile_id: i64,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO task_executions (task_id, wallet_id, profile_id, status, start_time)
            VALUES (?, ?, ?, 'running', ?)
            "#
        )
        .bind(task_id)
        .bind(wallet_id)
        .bind(profile_id)
        .bind(Utc::now())
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 更新执行记录为成功
    async fn update_execution_success(
        pool: &SqlitePool,
        execution_id: i64,
        tx_hash: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE task_executions
            SET status = 'success',
                end_time = ?,
                duration_ms = CAST((julianday(?) - julianday(start_time)) * 24 * 60 * 60 * 1000 AS INTEGER),
                result_data = ?
            WHERE id = ?
            "#
        )
        .bind(&now)
        .bind(&now)
        .bind(tx_hash.map(|h| format!("{{\"txHash\":\"{}\"}}", h)))
        .bind(execution_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 更新执行记录为失败
    async fn update_execution_failed(
        pool: &SqlitePool,
        execution_id: i64,
        error: &str,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE task_executions
            SET status = 'failed',
                end_time = ?,
                duration_ms = CAST((julianday(?) - julianday(start_time)) * 24 * 60 * 60 * 1000 AS INTEGER),
                error_message = ?
            WHERE id = ?
            "#
        )
        .bind(&now)
        .bind(&now)
        .bind(error)
        .bind(execution_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 更新任务统计
    async fn update_task_stats(&self, task_id: i64) -> Result<()> {
        let stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'success' THEN 1 END) as success,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed
            FROM task_executions
            WHERE task_id = ?
            "#
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE automation_tasks SET success_runs = ?, failed_runs = ? WHERE id = ?"
        )
        .bind(stats.0 as i32)
        .bind(stats.1 as i32)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 执行脚本（模拟实现，实际应调用Playwright）
    async fn execute_script(
        pool: &SqlitePool,
        execution_id: i64,
        _script_content: &str,
        wallet: &AirdropWallet,
        profile: &BrowserProfile,
        _timeout_seconds: i32,
    ) -> Result<Option<String>> {
        // 记录日志
        Self::add_execution_log(pool, execution_id, "info", &format!(
            "开始执行脚本，钱包: {}, 环境: {}",
            wallet.name, profile.name
        )).await?;

        // 获取解密的私钥
        let encrypted_key = &wallet.encrypted_private_key;
        let _private_key = security::memory::decrypt_string(encrypted_key)
            .map_err(|e| anyhow::anyhow!(e))?;

        // TODO: 这里应该调用Playwright执行实际脚本
        // 目前使用模拟执行

        Self::add_execution_log(pool, execution_id, "info", "初始化浏览器环境...").await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Self::add_execution_log(pool, execution_id, "info", &format!(
            "设置User-Agent: {}",
            profile.user_agent.as_deref().unwrap_or("default")
        )).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        Self::add_execution_log(pool, execution_id, "info", "启动浏览器...").await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        Self::add_execution_log(pool, execution_id, "info", "导航到目标页面...").await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        Self::add_execution_log(pool, execution_id, "info", "执行脚本逻辑...").await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        // 模拟成功率
        let success = rand::random::<f64>() > 0.1;

        if success {
            Self::add_execution_log(pool, execution_id, "success", "脚本执行成功").await?;
            Ok(None) // 返回交易哈希（如果有）
        } else {
            Err(anyhow::anyhow!("模拟执行失败"))
        }
    }

    /// 添加执行日志
    async fn add_execution_log(
        pool: &SqlitePool,
        execution_id: i64,
        level: &str,
        message: &str,
    ) -> Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        let log_entry = format!("[{}] [{}] {}\n", timestamp, level.to_uppercase(), message);

        // 获取现有日志
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT logs FROM task_executions WHERE id = ?"
        )
        .bind(execution_id)
        .fetch_optional(pool)
        .await?;

        let new_logs = match existing {
            Some(logs) => format!("{}{}", logs, log_entry),
            None => log_entry,
        };

        sqlx::query(
            "UPDATE task_executions SET logs = ? WHERE id = ?"
        )
        .bind(new_logs)
        .bind(execution_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
