use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use sqlx::SqlitePool;
use chrono::{Utc, DateTime};
use cron::Schedule;

use crate::wallets_tool::airdrop::models::*;
use crate::wallets_tool::airdrop::executor::TaskExecutor;

/// 任务调度器
#[allow(dead_code)]
pub struct TaskScheduler {
    pool: SqlitePool,
    running_tasks: Arc<RwLock<HashMap<i64, tokio::task::JoinHandle<()>>>>,
    executor: Arc<TaskExecutor>,
    check_interval: Duration,
}

#[allow(dead_code)]
impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(pool: SqlitePool) -> Self {
        let executor = Arc::new(TaskExecutor::new(pool.clone()));
        Self {
            pool,
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            executor,
            check_interval: Duration::from_secs(60), // 每分钟检查一次
        }
    }

    /// 启动调度器
    pub async fn start(&self) {
        let pool = self.pool.clone();
        let running_tasks = self.running_tasks.clone();
        let executor = self.executor.clone();
        let check_interval = self.check_interval;

        tokio::spawn(async move {
            let mut ticker = interval(check_interval);

            loop {
                ticker.tick().await;

                // 检查并执行到期的任务
                if let Err(e) = Self::check_and_execute_tasks(&pool, &running_tasks, &executor).await {
                    eprintln!("[Scheduler] 检查任务失败: {}", e);
                }
            }
        });

        println!("[Scheduler] 任务调度器已启动");
    }

    /// 检查并执行到期的任务
    async fn check_and_execute_tasks(
        pool: &SqlitePool,
        running_tasks: &Arc<RwLock<HashMap<i64, tokio::task::JoinHandle<()>>>>,
        executor: &Arc<TaskExecutor>,
    ) -> anyhow::Result<()> {
        let now = Utc::now();

        // 获取所有启用的任务
        let tasks: Vec<AutomationTask> = sqlx::query_as(
            "SELECT * FROM automation_tasks WHERE status = 'enabled' AND (next_run_time IS NULL OR next_run_time <= ?)"
        )
        .bind(now)
        .fetch_all(pool)
        .await?;

        for task in tasks {
            let task_id = task.id;

            // 检查任务是否已经在运行
            {
                let running = running_tasks.read().await;
                if running.contains_key(&task_id) {
                    continue;
                }
            }

            // 执行任务
            let pool = pool.clone();
            let executor = executor.clone();
            let running_tasks_for_task = running_tasks.clone();

            let handle = tokio::spawn(async move {
                println!("[Scheduler] 开始执行任务: {}", task.name);

                // 更新任务状态为运行中
                let _ = sqlx::query(
                    "UPDATE automation_tasks SET status = 'running', last_run_time = ? WHERE id = ?"
                )
                .bind(Utc::now())
                .bind(task_id)
                .execute(&pool)
                .await;

                // 执行任务
                match executor.execute_task(&task).await {
                    Ok(_) => {
                        println!("[Scheduler] 任务执行成功: {}", task.name);
                    }
                    Err(e) => {
                        eprintln!("[Scheduler] 任务执行失败: {} - {}", task.name, e);
                    }
                }

                // 更新任务状态和下次执行时间
                let next_run = Self::calculate_next_run(&task);
                let _ = sqlx::query(
                    "UPDATE automation_tasks SET status = 'enabled', next_run_time = ?, total_runs = total_runs + 1 WHERE id = ?"
                )
                .bind(next_run)
                .bind(task_id)
                .execute(&pool)
                .await;

                // 从运行列表中移除
                let mut running = running_tasks_for_task.write().await;
                running.remove(&task_id);
            });

            // 添加到运行列表
            let mut running = running_tasks.write().await;
            running.insert(task_id, handle);
        }

        Ok(())
    }

    /// 计算下次执行时间
    fn calculate_next_run(task: &AutomationTask) -> Option<DateTime<Utc>> {
        let schedule_config: serde_json::Value = serde_json::from_str(&task.schedule_config).ok()?;

        match task.schedule_type.as_str() {
            "once" => None,
            "interval" => {
                let interval_seconds = schedule_config.get("interval")?.as_i64()?;
                Some(Utc::now() + chrono::Duration::seconds(interval_seconds))
            }
            "cron" => {
                let cron_expr = schedule_config.get("cron")?.as_str()?;
                let schedule = Schedule::from_str(cron_expr).ok()?;
                schedule.upcoming(Utc).next()
            }
            _ => None,
        }
    }

    /// 立即停止任务
    pub async fn stop_task(&self, task_id: i64) -> anyhow::Result<()> {
        let mut running = self.running_tasks.write().await;

        if let Some(handle) = running.remove(&task_id) {
            handle.abort();

            // 更新任务状态
            sqlx::query(
                "UPDATE automation_tasks SET status = 'paused' WHERE id = ?"
            )
            .bind(task_id)
            .execute(&self.pool)
            .await?;

            println!("[Scheduler] 任务已停止: {}", task_id);
        }

        Ok(())
    }

    /// 立即执行任务
    pub async fn run_task_now(&self, task_id: i64) -> anyhow::Result<()> {
        // 获取任务
        let task: AutomationTask = sqlx::query_as(
            "SELECT * FROM automation_tasks WHERE id = ?"
        )
        .bind(task_id)
        .fetch_one(&self.pool)
        .await?;

        // 检查任务是否已经在运行
        {
            let running = self.running_tasks.read().await;
            if running.contains_key(&task_id) {
                return Err(anyhow::anyhow!("任务已经在运行中"));
            }
        }

        // 执行任务
        let pool = self.pool.clone();
        let executor = self.executor.clone();
        let running_tasks = self.running_tasks.clone();

        let handle = tokio::spawn(async move {
            println!("[Scheduler] 立即执行任务: {}", task.name);

            // 更新任务状态
            let _ = sqlx::query(
                "UPDATE automation_tasks SET status = 'running', last_run_time = ? WHERE id = ?"
            )
            .bind(Utc::now())
            .bind(task_id)
            .execute(&pool)
            .await;

            // 执行任务
            match executor.execute_task(&task).await {
                Ok(_) => {
                    println!("[Scheduler] 任务执行成功: {}", task.name);
                }
                Err(e) => {
                    eprintln!("[Scheduler] 任务执行失败: {} - {}", task.name, e);
                }
            }

            // 更新任务状态
            let _ = sqlx::query(
                "UPDATE automation_tasks SET status = 'enabled', total_runs = total_runs + 1 WHERE id = ?"
            )
            .bind(task_id)
            .execute(&pool)
            .await;

            // 从运行列表中移除
            let mut running = running_tasks.write().await;
            running.remove(&task_id);
        });

        // 添加到运行列表
        let mut running = self.running_tasks.write().await;
        running.insert(task_id, handle);

        Ok(())
    }

    /// 获取运行中的任务列表
    pub async fn get_running_tasks(&self) -> Vec<i64> {
        let running = self.running_tasks.read().await;
        running.keys().cloned().collect()
    }
}

use std::str::FromStr;
