use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use super::ExecutionConfig;

const DEFAULT_MAX_CONCURRENCY: usize = 5;
const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessage {
    pub session_id: String,
    pub wallet_id: Option<String>,
    pub level: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletExecutionStatus {
    pub wallet_id: String,
    pub status: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub total_wallets: usize,
    pub completed_wallets: usize,
    pub failed_wallets: usize,
    pub running_wallets: usize,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub wallets: Vec<WalletExecutionStatus>,
}

pub struct ExecutionSession {
    pub id: String,
    pub config: ExecutionConfig,
    pub status: Arc<RwLock<SessionStatus>>,
    pub cancel_token: tokio_util::sync::CancellationToken,
    pub process_handles: Arc<RwLock<Vec<Child>>>,
}

pub struct ExecutionManager {
    semaphore: Arc<Semaphore>,
    sessions: Arc<RwLock<HashMap<String, Arc<ExecutionSession>>>>,
}

async fn execute_wallet(
    session: &ExecutionSession,
    wallet: &super::WalletInfo,
    log_prefix: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
) -> Result<(), String> {
    let script_content = super::build_script_from_config(&session.config, vec![wallet.clone()])
        .map_err(|e| format!("构建脚本失败: {}", e))?;

    // 创建临时文件
    let temp_dir = std::env::temp_dir().join(format!("wallets_tool_{}", session.id));
    tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;
    
    let script_path = temp_dir.join(format!("wallet_{}.js", wallet.id));
    tokio::fs::write(&script_path, script_content).await.map_err(|e| e.to_string())?;

    // 启动 Node.js 进程
    let mut child = Command::new("node")
        .arg(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 Node.js 失败: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // 存储进程句柄
    {
        let mut handles = session.process_handles.write().await;
        handles.push(child);
    }

    // 读取 stdout
    let stdout_handle = {
        let log_prefix = log_prefix.to_string();
        let cancel_token = cancel_token.clone();
        
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                println!("{} {}", log_prefix, line);
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    _ = cancel_token.cancelled() => break,
                }
            }
        })
    };

    // 读取 stderr
    let stderr_handle = {
        let log_prefix = log_prefix.to_string();
        let cancel_token = cancel_token.clone();
        
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                eprintln!("{} {}", log_prefix, line);
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    _ = cancel_token.cancelled() => break,
                }
            }
        })
    };

    // 等待进程完成或超时
    let timeout_duration = Duration::from_secs(session.config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    
    let result = tokio::select! {
        _ = cancel_token.cancelled() => {
            // 取消执行
            let mut handles = session.process_handles.write().await;
            if let Some(mut child) = handles.pop() {
                let _ = child.kill().await;
            }
            Err("执行已取消".to_string())
        }
        result = timeout(timeout_duration, async {
            stdout_handle.await.ok();
            stderr_handle.await.ok();
            
            let mut handles = session.process_handles.write().await;
            if let Some(mut child) = handles.pop() {
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            Ok(())
                        } else {
                            Err(format!("进程退出码: {:?}", status.code()))
                        }
                    }
                    Err(e) => Err(format!("等待进程失败: {}", e)),
                }
            } else {
                Err("进程句柄丢失".to_string())
            }
        }) => {
            match result {
                Ok(r) => r,
                Err(_) => Err("执行超时".to_string()),
            }
        }
    };

    // 清理临时文件
    let _ = tokio::fs::remove_file(&script_path).await;

    result
}

impl ExecutionManager {
    pub fn new(max_concurrency: Option<usize>) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrency.unwrap_or(DEFAULT_MAX_CONCURRENCY))),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_session_status(&self, session_id: &str) -> Result<SessionStatus, String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        
        let status = session.status.read().await;
        Ok(status.clone())
    }

    pub async fn cancel_execution(&self, session_id: &str) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        
        session.cancel_token.cancel();
        
        // 终止所有进程
        let mut handles = session.process_handles.write().await;
        for child in handles.iter_mut() {
            let _ = child.kill().await;
        }
        handles.clear();

        let mut status = session.status.write().await;
        status.status = "cancelled".to_string();
        status.end_time = Some(chrono::Utc::now().timestamp_millis());

        Ok(())
    }

    pub async fn cleanup_session(&self, session_id: &str) -> Result<(), String> {
        // 清理临时目录
        let temp_dir = std::env::temp_dir().join(format!("wallets_tool_{}", session_id));
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);

        Ok(())
    }

    pub async fn create_session(&self, config: ExecutionConfig) -> Result<String, String> {
        let session_id = Uuid::new_v4().to_string();
        let total_wallets = config.wallets.len();
        
        let wallet_statuses: Vec<WalletExecutionStatus> = config
            .wallets
            .iter()
            .map(|w| WalletExecutionStatus {
                wallet_id: w.id.clone(),
                status: "pending".to_string(),
                start_time: None,
                end_time: None,
                error: None,
            })
            .collect();

        let session_status = SessionStatus {
            session_id: session_id.clone(),
            total_wallets,
            completed_wallets: 0,
            failed_wallets: 0,
            running_wallets: 0,
            status: "created".to_string(),
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
            wallets: wallet_statuses,
        };

        let session = Arc::new(ExecutionSession {
            id: session_id.clone(),
            config,
            status: Arc::new(RwLock::new(session_status)),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            process_handles: Arc::new(RwLock::new(Vec::new())),
        });

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        Ok(session_id)
    }

    pub async fn start_execution(&self, session_id: &str) -> Result<(), String> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| "Session not found".to_string())?
        };

        {
            let mut status = session.status.write().await;
            status.status = "running".to_string();
        }

        let session_clone = Arc::clone(&session);
        let semaphore = Arc::clone(&self.semaphore);
        let sessions = Arc::clone(&self.sessions);

        tokio::spawn(async move {
            let config = &session_clone.config;
            let concurrency = config.concurrency.unwrap_or(DEFAULT_MAX_CONCURRENCY).min(session_clone.config.wallets.len());
            let wallets = session_clone.config.wallets.clone();

            // 使用 futures::stream 进行并发控制
            let stream = futures::stream::iter(wallets.into_iter().enumerate())
                .map(|(index, wallet)| {
                    let session = Arc::clone(&session_clone);
                    let semaphore = Arc::clone(&semaphore);
                    let cancel_token = session.cancel_token.clone();

                    async move {
                        // 获取信号量许可
                        let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;

                        if cancel_token.is_cancelled() {
                            return Ok::<(), String>(());
                        }

                        // 更新钱包状态为运行中
                        {
                            let mut status = session.status.write().await;
                            if let Some(wallet_status) = status.wallets.iter_mut().find(|w| w.wallet_id == wallet.id) {
                                wallet_status.status = "running".to_string();
                                wallet_status.start_time = Some(chrono::Utc::now().timestamp_millis());
                            }
                            status.running_wallets += 1;
                        }

                        let log_prefix = format!("[Wallet {}/{} - {}]", index + 1, session.config.wallets.len(), &wallet.address[..8.min(wallet.address.len())]);
                        
                        println!("{} 开始执行", log_prefix);

                        // 执行单个钱包
                        let result = execute_wallet(
                            &session,
                            &wallet,
                            &log_prefix,
                            &cancel_token,
                        ).await;

                        // 更新钱包状态
                        {
                            let mut status = session.status.write().await;
                            if let Some(wallet_status) = status.wallets.iter_mut().find(|w| w.wallet_id == wallet.id) {
                                wallet_status.end_time = Some(chrono::Utc::now().timestamp_millis());
                                match result {
                                    Ok(_) => {
                                        wallet_status.status = "completed".to_string();
                                        status.completed_wallets += 1;
                                        println!("{} 执行完成", log_prefix);
                                    }
                                    Err(e) => {
                                        wallet_status.status = "failed".to_string();
                                        wallet_status.error = Some(e.clone());
                                        status.failed_wallets += 1;
                                        eprintln!("{} 执行失败: {}", log_prefix, e);
                                    }
                                }
                            }
                            status.running_wallets -= 1;
                        }

                        Ok::<(), String>(())
                    }
                })
                .buffer_unordered(concurrency);

            // 等待所有任务完成
            let results: Vec<Result<(), String>> = stream.collect().await;

            // 更新会话状态为完成
            {
                let mut status = session_clone.status.write().await;
                status.status = "completed".to_string();
                status.end_time = Some(chrono::Utc::now().timestamp_millis());
            }

            // 清理会话
            let mut sessions = sessions.write().await;
            sessions.remove(&session_clone.id);

            println!("会话执行完成，成功: {}, 失败: {}", 
                results.iter().filter(|r| r.is_ok()).count(),
                results.iter().filter(|r| r.is_err()).count()
            );
        });

        Ok(())
    }
}

// 全局执行管理器
lazy_static::lazy_static! {
    static ref EXECUTION_MANAGER: ExecutionManager = ExecutionManager::new(None);
}

pub fn get_execution_manager() -> &'static ExecutionManager {
    &EXECUTION_MANAGER
}

// Tauri 命令 - 使用新的命名避免冲突
#[tauri::command]
pub async fn playwright_create_session(config: ExecutionConfig) -> Result<String, String> {
    get_execution_manager().create_session(config).await
}

#[tauri::command]
pub async fn playwright_start_execution(session_id: String) -> Result<(), String> {
    get_execution_manager().start_execution(&session_id).await
}

#[tauri::command]
pub async fn playwright_get_execution_status(session_id: String) -> Result<SessionStatus, String> {
    get_execution_manager().get_session_status(&session_id).await
}

#[tauri::command]
pub async fn playwright_cancel_execution(session_id: String) -> Result<(), String> {
    get_execution_manager().cancel_execution(&session_id).await
}

#[tauri::command]
pub async fn playwright_cleanup_session(session_id: String) -> Result<(), String> {
    get_execution_manager().cleanup_session(&session_id).await
}
