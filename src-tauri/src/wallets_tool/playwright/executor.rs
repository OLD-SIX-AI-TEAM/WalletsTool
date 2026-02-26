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
use std::path::Path;
use std::env;

use super::ExecutionConfig;

const DEFAULT_MAX_CONCURRENCY: usize = 5;
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// 查找 npm 可执行文件的路径
async fn find_npm_path() -> Result<String, String> {
    // 首先尝试直接使用 npm（如果在 PATH 中）
    let test_cmd = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg("npm")
            .output()
            .await
    } else {
        Command::new("which")
            .arg("npm")
            .output()
            .await
    };

    if let Ok(output) = test_cmd {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            // Windows: 优先选择 .cmd 文件，避免选择没有扩展名的 shell 脚本
            let npm_path = if cfg!(target_os = "windows") {
                path.lines()
                    .find(|line| line.trim().ends_with(".cmd"))
                    .or_else(|| path.lines().next())
                    .unwrap_or("npm")
                    .trim()
                    .to_string()
            } else {
                path.lines().next().unwrap_or("npm").trim().to_string()
            };
            if !npm_path.is_empty() {
                println!("[Playwright] 找到 npm 路径: {}", npm_path);
                return Ok(npm_path);
            }
        }
    }

    // 尝试常见的安装路径（Windows 优先尝试 .cmd）
    let common_paths = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\nodejs\npm.cmd",
            r"C:\Program Files (x86)\nodejs\npm.cmd",
            r"C:\Users\%USERNAME%\AppData\Roaming\npm\npm.cmd",
            r"C:\nvm4w\nodejs\npm.cmd",
        ]
    } else {
        vec![
            "/usr/local/bin/npm",
            "/usr/bin/npm",
            "/opt/homebrew/bin/npm",
            "/opt/local/bin/npm",
        ]
    };

    for path in common_paths {
        let expanded_path = if cfg!(target_os = "windows") && path.contains("%USERNAME%") {
            if let Ok(username) = env::var("USERNAME") {
                path.replace("%USERNAME%", &username)
            } else {
                continue;
            }
        } else {
            path.to_string()
        };

        if std::path::Path::new(&expanded_path).exists() {
            println!("[Playwright] 找到 npm 路径: {}", expanded_path);
            return Ok(expanded_path);
        }
    }

    // 最后尝试直接使用 npm，让系统自己找
    println!("[Playwright] 使用默认 npm 命令");
    Ok("npm".to_string())
}

/// 获取全局 Playwright 安装目录
fn get_global_playwright_dir() -> std::path::PathBuf {
    // 使用应用程序数据目录，而不是临时目录
    let app_data_dir = if cfg!(target_os = "windows") {
        env::var("LOCALAPPDATA")
            .map(|p| std::path::PathBuf::from(p).join("WalletsTool").join("playwright"))
            .unwrap_or_else(|_| std::env::temp_dir().join("wallets_tool_playwright"))
    } else {
        env::var("HOME")
            .map(|p| std::path::PathBuf::from(p).join(".local").join("share").join("wallets_tool").join("playwright"))
            .unwrap_or_else(|_| std::env::temp_dir().join("wallets_tool_playwright"))
    };
    app_data_dir
}

/// 检查并安装 Playwright（使用全局缓存）
async fn ensure_playwright_installed(_temp_dir: &Path) -> Result<(), String> {
    let global_dir = get_global_playwright_dir();
    let node_modules_path = global_dir.join("node_modules").join("playwright");
    let browsers_installed_marker = global_dir.join(".browsers_installed");
    
    // 检查是否已完全安装（包括浏览器）
    let playwright_installed = node_modules_path.exists();
    let browsers_installed = browsers_installed_marker.exists();
    
    if playwright_installed && browsers_installed {
        println!("[Playwright] 使用已安装的全局 Playwright: {:?}", global_dir);
        return Ok(());
    }
    
    // 确保全局目录存在
    tokio::fs::create_dir_all(&global_dir).await
        .map_err(|e| format!("创建全局目录失败: {}", e))?;
    
    // 查找 npm 路径
    let npm_path = find_npm_path().await?;
    
    // 1. 安装 Playwright npm 包（如果还没有安装）
    if !playwright_installed {
        println!("[Playwright] 首次初始化，正在安装 Playwright...");
        println!("[Playwright] 安装目录: {:?}", global_dir);
        println!("[Playwright] 使用 npm: {}", npm_path);
        
        // 初始化 npm 项目
        let npm_init = Command::new(&npm_path)
            .arg("init")
            .arg("-y")
            .current_dir(&global_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("npm init 失败: {} (路径: {})", e, npm_path))?;
        
        if !npm_init.status.success() {
            let stderr = String::from_utf8_lossy(&npm_init.stderr);
            return Err(format!("npm init 失败: {} (路径: {})", stderr, npm_path));
        }
        
        // 安装 playwright
        let npm_install = Command::new(&npm_path)
            .arg("install")
            .arg("playwright")
            .arg("--no-save")
            .current_dir(&global_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("npm install playwright 失败: {} (路径: {})", e, npm_path))?;
        
        if !npm_install.status.success() {
            let stderr = String::from_utf8_lossy(&npm_install.stderr);
            return Err(format!("npm install playwright 失败: {} (路径: {})", stderr, npm_path));
        }
        
        println!("[Playwright] npm 包安装完成");
    }
    
    // 2. 安装浏览器（如果还没有安装）
    if !browsers_installed {
        println!("[Playwright] 正在安装 Chromium 浏览器...");
        
        // 使用 npx playwright install chromium 安装浏览器
        let npx_cmd = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };
        let browser_install = Command::new(&npx_cmd)
            .arg("playwright")
            .arg("install")
            .arg("chromium")
            .current_dir(&global_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("安装浏览器失败: {}。请确保网络连接正常", e))?;
        
        if !browser_install.status.success() {
            let stderr = String::from_utf8_lossy(&browser_install.stderr);
            let stdout = String::from_utf8_lossy(&browser_install.stdout);
            eprintln!("[Playwright] 浏览器安装 stdout: {}", stdout);
            eprintln!("[Playwright] 浏览器安装 stderr: {}", stderr);
            return Err(format!("安装 Chromium 浏览器失败: {}。请检查网络连接或手动运行: npx playwright install chromium", stderr));
        }
        
        // 创建标记文件表示浏览器已安装
        tokio::fs::write(&browsers_installed_marker, "chromium installed")
            .await
            .map_err(|e| format!("创建浏览器安装标记失败: {}", e))?;
        
        println!("[Playwright] Chromium 浏览器安装完成");
    }
    
    println!("[Playwright] 初始化完成");
    Ok(())
}

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
    println!("{} 开始构建脚本", log_prefix);
    
    let script_content = super::build_script_from_config(&session.config, vec![wallet.clone()])
        .map_err(|e| format!("构建脚本失败: {}", e))?;

    // 创建临时文件
    let temp_dir = std::env::temp_dir().join(format!("wallets_tool_{}", session.id));
    tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;
    
    // 确保 Playwright 已安装
    if let Err(e) = ensure_playwright_installed(&temp_dir).await {
        println!("{} Playwright 安装失败: {}", log_prefix, e);
        return Err(format!("Playwright 安装失败: {}", e));
    }
    
    let script_path = temp_dir.join(format!("wallet_{}.js", wallet.id));
    tokio::fs::write(&script_path, script_content).await.map_err(|e| e.to_string())?;
    
    println!("{} 脚本已写入: {:?}", log_prefix, script_path);

    // 获取全局 Playwright 目录，用于设置 NODE_PATH
    let global_playwright_dir = get_global_playwright_dir();
    let node_modules_path = global_playwright_dir.join("node_modules");
    
    // 构建 NODE_PATH 环境变量
    let mut env_vars = env::vars().collect::<std::collections::HashMap<String, String>>();
    let existing_node_path = env::var("NODE_PATH").unwrap_or_default();
    let path_separator = if cfg!(target_os = "windows") { ";" } else { ":" };
    let new_node_path = if existing_node_path.is_empty() {
        node_modules_path.to_string_lossy().to_string()
    } else {
        format!("{}{}{}", node_modules_path.to_string_lossy(), path_separator, existing_node_path)
    };
    env_vars.insert("NODE_PATH".to_string(), new_node_path.clone());
    
    println!("{} 正在启动 Node.js 进程...", log_prefix);
    println!("{} NODE_PATH: {}", log_prefix, new_node_path);
    
    // 启动 Node.js 进程，使用全局 Playwright
    let mut child = Command::new("node")
        .arg(&script_path)
        .env("NODE_PATH", &new_node_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 Node.js 失败: {}", e))?;
    
    println!("{} Node.js 进程已启动, pid: {:?}", log_prefix, child.id());

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // 注意：我们直接使用 child 变量，不存储到 handles 中
    // 因为我们需要在 async 块中使用它

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
    
    println!("{} 等待进程完成, 超时: {}秒", log_prefix, timeout_duration.as_secs());
    
    let result = tokio::select! {
        _ = cancel_token.cancelled() => {
            println!("{} 收到取消信号", log_prefix);
            // 取消执行
            let mut handles = session.process_handles.write().await;
            if let Some(mut child) = handles.pop() {
                let _ = child.kill().await;
            }
            Err("执行已取消".to_string())
        }
        result = timeout(timeout_duration, async {
            println!("{} 开始等待 stdout/stderr...", log_prefix);
            stdout_handle.await.ok();
            stderr_handle.await.ok();
            println!("{} stdout/stderr 已关闭", log_prefix);
            
            // 直接使用 child 变量，而不是从 handles 中 pop
            match child.wait().await {
                Ok(status) => {
                    println!("{} 进程已结束, 退出码: {:?}", log_prefix, status.code());
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("进程退出码: {:?}", status.code()))
                    }
                }
                Err(e) => {
                    println!("{} 等待进程失败: {}", log_prefix, e);
                    Err(format!("等待进程失败: {}", e))
                }
            }
        }) => {
            match result {
                Ok(r) => r,
                Err(_) => {
                    println!("{} 执行超时", log_prefix);
                    Err("执行超时".to_string())
                }
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
                        let task_result = {
                            let mut status = session.status.write().await;
                            if let Some(wallet_status) = status.wallets.iter_mut().find(|w| w.wallet_id == wallet.id) {
                                wallet_status.end_time = Some(chrono::Utc::now().timestamp_millis());
                                match result {
                                    Ok(_) => {
                                        wallet_status.status = "completed".to_string();
                                        status.completed_wallets += 1;
                                        println!("{} 执行完成", log_prefix);
                                        Ok::<(), String>(())
                                    }
                                    Err(e) => {
                                        wallet_status.status = "failed".to_string();
                                        wallet_status.error = Some(e.clone());
                                        status.failed_wallets += 1;
                                        eprintln!("{} 执行失败: {}", log_prefix, e);
                                        Err(e)
                                    }
                                }
                            } else {
                                Ok(())
                            }
                        };
                        
                        {
                            let mut status = session.status.write().await;
                            status.running_wallets -= 1;
                        }

                        task_result
                    }
                })
                .buffer_unordered(concurrency);

            // 等待所有任务完成
            let results: Vec<Result<(), String>> = stream.collect().await;

            // 计算成功和失败数量
            let success_count = results.iter().filter(|r| r.is_ok()).count();
            let fail_count = results.iter().filter(|r| r.is_err()).count();
            
            println!("会话执行完成，成功: {}, 失败: {}", success_count, fail_count);

            // 更新会话状态为完成
            {
                let mut status = session_clone.status.write().await;
                // 如果有失败的任务，标记为 error 状态
                if fail_count > 0 {
                    status.status = "error".to_string();
                } else {
                    status.status = "completed".to_string();
                }
                status.end_time = Some(chrono::Utc::now().timestamp_millis());
            }

            // 延迟清理会话，给前端时间获取最终状态
            let session_id = session_clone.id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let mut sessions = sessions.write().await;
                sessions.remove(&session_id);
                println!("会话 {} 已清理", session_id);
            });
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
