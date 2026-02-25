use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use std::collections::HashMap;

const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    pub id: String,
    pub url: String,
    pub start_time: i64,
    pub actions: Vec<RecordedAction>,
    pub status: String,
    pub generated_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedAction {
    pub action_type: String,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub timestamp: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingOptions {
    pub url: String,
    pub browser_type: Option<String>,
    pub headless: Option<bool>,
    pub viewport_width: Option<i32>,
    pub viewport_height: Option<i32>,
    pub proxy_type: Option<String>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
}

pub struct PlaywrightRecorder {
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<RecordingSession>>>>>,
    process_id: Arc<RwLock<Option<u32>>>,
}

impl PlaywrightRecorder {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            process_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start_recording(&self, options: RecordingOptions) -> Result<String, String> {
        let start_time = std::time::Instant::now();
        let session_id = Uuid::new_v4().to_string();
        
        println!("[Recorder] ========== 开始录制会话 ==========");
        println!("[Recorder] 会话ID: {}", session_id);
        println!("[Recorder] 目标URL: {}", options.url);
        
        let session = RecordingSession {
            id: session_id.clone(),
            url: options.url.clone(),
            start_time: chrono::Utc::now().timestamp_millis(),
            actions: vec![RecordedAction {
                action_type: "navigate".to_string(),
                selector: None,
                value: Some(options.url.clone()),
                timestamp: chrono::Utc::now().timestamp_millis(),
                description: format!("导航到 {}", options.url),
            }],
            status: "recording".to_string(),
            generated_code: None,
        };

        // 检查 npx 是否可用 (Windows 上使用 npx.cmd)
        let npx_cmd = if cfg!(windows) { "npx.cmd" } else { "npx" };
        println!("[Recorder] 检查 npx 可用性...");
        let check_start = std::time::Instant::now();
        let npx_check = Command::new(npx_cmd)
            .args(&["--version"])
            .output();
        println!("[Recorder] npx 检查耗时: {:?}", check_start.elapsed());
        
        if npx_check.is_err() {
            return Err(format!("npx 命令不可用。请确保已安装 Node.js (https://nodejs.org/)\n\n尝试运行: {} --version", npx_cmd));
        }

        // 保存脚本到临时文件
        let temp_dir = std::env::temp_dir().join(format!("playwright_recorder_{}", session_id));
        println!("[Recorder] 创建临时目录: {:?}", temp_dir);
        let mkdir_start = std::time::Instant::now();
        std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;
        println!("[Recorder] 创建目录耗时: {:?}", mkdir_start.elapsed());
        
        let script_path = temp_dir.join("record.js");

        println!("[Recorder] 生成录制脚本...");
        let script_gen_start = std::time::Instant::now();
        let script_with_npx = self.build_recording_script_with_npx(&session_id, &options);
        println!("[Recorder] 脚本生成耗时: {:?}", script_gen_start.elapsed());
        
        let write_start = std::time::Instant::now();
        std::fs::write(&script_path, script_with_npx).map_err(|e| format!("写入脚本失败: {}", e))?;
        println!("[Recorder] 写入脚本耗时: {:?}", write_start.elapsed());
        println!("[Recorder] 脚本路径: {:?}", script_path);

        // 使用 node 运行脚本
        println!("[Recorder] 启动 Node.js 进程...");
        let node_start = std::time::Instant::now();
        let node_cmd = if cfg!(windows) { "node.cmd" } else { "node" };
        let mut child = Command::new(node_cmd)
            .arg(script_path.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动 Node.js 失败: {}\n\n请确保已安装 Node.js (https://nodejs.org/)", e))?;
        println!("[Recorder] Node.js 进程启动耗时: {:?}", node_start.elapsed());

        // 保存进程 ID
        let pid = child.id();
        let mut process_id = self.process_id.write().await;
        *process_id = Some(pid);
        println!("[Recorder] 进程ID: {}", pid);

        // 存储会话
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), Arc::new(Mutex::new(session)));

        // 等待浏览器启动确认（最多等待60秒，因为可能需要安装 Playwright）
        println!("[Recorder] 等待浏览器启动确认...");
        let wait_start = std::time::Instant::now();
        let mut browser_started = false;
        let mut last_line = String::new();
        if let Some(stdout) = child.stdout.as_mut() {
            use std::io::BufRead;
            let reader = BufReader::new(stdout);
            let start_time = std::time::Instant::now();
            
            for line in reader.lines() {
                if let Ok(line) = line {
                    last_line = line.clone();
                    println!("[Recorder {}] {}", session_id, line);
                    
                    // 检查浏览器是否成功启动
                    if line.contains("浏览器已启动") {
                        browser_started = true;
                        break;
                    }
                    
                    // 检查是否超时 (增加到60秒，给 Playwright 安装留足时间)
                    if start_time.elapsed().as_secs() > 60 {
                        println!("[Recorder] 等待浏览器启动超时 (60秒)");
                        break;
                    }
                }
            }
        }
        println!("[Recorder] 等待浏览器启动耗时: {:?}", wait_start.elapsed());

        if !browser_started {
            // 尝试读取错误信息
            let mut error_msg = String::new();
            if let Some(stderr) = child.stderr.as_mut() {
                use std::io::BufRead;
                let reader = BufReader::new(stderr);
                for line in reader.lines().take(20) {
                    if let Ok(line) = line {
                        error_msg.push_str(&line);
                        error_msg.push('\n');
                    }
                }
            }
            
            // 终止进程
            let _ = child.kill();
            
            println!("[Recorder] 浏览器启动失败，最后输出: {}", last_line);
            println!("[Recorder] 错误信息: {}", error_msg);
            println!("[Recorder] ========== 录制会话启动失败 ==========");
            
            return Err(format!("浏览器启动失败。错误信息:\n{}", 
                if error_msg.is_empty() { "请确保已安装 Playwright: npm install -g playwright && npx playwright install chromium".to_string() } else { error_msg }));
        }
        
        println!("[Recorder] 浏览器启动成功！总耗时: {:?}", start_time.elapsed());
        println!("[Recorder] ========== 录制会话启动成功 ==========");

        // 在后台运行并捕获输出
        let sessions_clone = Arc::clone(&self.sessions);
        let session_id_clone = session_id.clone();
        
        tokio::spawn(async move {
            // 继续读取 stdout
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                let lines = reader.lines();
                
                for line in lines {
                    if let Ok(line) = line {
                        println!("[Recorder {}] {}", session_id_clone, line);
                        
                        // 解析操作记录
                        if line.starts_with("ACTION:") {
                            let action_data = &line[7..];
                            if let Ok(action) = serde_json::from_str::<RecordedAction>(action_data) {
                                let sessions = sessions_clone.read().await;
                                if let Some(session) = sessions.get(&session_id_clone) {
                                    let mut session = session.lock().await;
                                    session.actions.push(action);
                                    println!("[Recorder {}] 操作已记录，当前共 {} 个操作", session_id_clone, session.actions.len());
                                }
                            } else {
                                eprintln!("[Recorder {}] 解析操作失败: {}", session_id_clone, action_data);
                            }
                        }
                        
                        // 解析浏览器关闭事件
                        if line.starts_with("BROWSER_CLOSED:") {
                            println!("[Recorder {}] 检测到浏览器已关闭", session_id_clone);
                            let sessions = sessions_clone.read().await;
                            if let Some(session) = sessions.get(&session_id_clone) {
                                let mut session = session.lock().await;
                                session.status = "stopped".to_string();
                                println!("[Recorder {}] 会话状态已更新为 stopped", session_id_clone);
                            }
                        }
                    }
                }
            }
            
            // 读取 stderr
            if let Some(stderr) = child.stderr.take() {
                let reader = BufReader::new(stderr);
                let lines = reader.lines();
                
                for line in lines {
                    if let Ok(line) = line {
                        eprintln!("[Recorder {}] ERROR: {}", session_id_clone, line);
                    }
                }
            }
            
            // 等待进程结束
            let _ = child.wait();
            
            // 更新会话状态
            let sessions = sessions_clone.read().await;
            if let Some(session) = sessions.get(&session_id_clone) {
                let mut session = session.lock().await;
                session.status = "stopped".to_string();
                println!("[Recorder {}] 会话已停止，共记录 {} 个操作", session_id_clone, session.actions.len());
            }
        });

        Ok(session_id)
    }

    pub async fn stop_recording(&self, session_id: &str) -> Result<Option<String>, String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| "会话不存在".to_string())?;
        
        let session = session.lock().await;
        
        // 生成代码
        let code = self.generate_code(&session);
        
        // 关闭浏览器进程
        drop(session); // 释放 session 锁
        let _ = self.kill_browser().await;
        
        Ok(Some(code))
    }
    
    pub async fn kill_browser(&self) -> Result<(), String> {
        let mut process_id = self.process_id.write().await;
        if let Some(pid) = *process_id {
            println!("[Recorder] 正在关闭浏览器进程 (PID: {})...", pid);
            
            #[cfg(windows)]
            {
                let output = Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &pid.to_string()])
                    .output();
                if let Ok(output) = output {
                    println!("[Recorder] taskkill 输出: {}", String::from_utf8_lossy(&output.stdout));
                }
            }
            
            #[cfg(not(windows))]
            {
                let _ = Command::new("kill")
                    .args(&["-9", &pid.to_string()])
                    .output();
            }
            
            println!("[Recorder] 浏览器进程已关闭");
        }
        *process_id = None;
        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<RecordingSession>, String> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            let session = session.lock().await;
            Ok(Some(session.clone()))
        } else {
            Ok(None)
        }
    }

    fn build_recording_script_with_npx(&self, session_id: &str, options: &RecordingOptions) -> String {
        let browser_type = options.browser_type.as_deref().unwrap_or("chromium");
        let headless = options.headless.unwrap_or(false);
        let width = options.viewport_width.unwrap_or(1280);
        let height = options.viewport_height.unwrap_or(720);
        let url = &options.url;

        let proxy_config = if let (Some(proxy_type), Some(proxy_host), Some(proxy_port)) = 
            (options.proxy_type.as_deref(), options.proxy_host.as_deref(), options.proxy_port) {
            if proxy_type != "direct" {
                format!(r#"
  proxy: {{
    server: '{}://{}:{}',
    username: '{}',
    password: '{}'
  }},"#,
                    proxy_type,
                    proxy_host,
                    proxy_port,
                    options.proxy_username.as_deref().unwrap_or(""),
                    options.proxy_password.as_deref().unwrap_or("")
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let js_code = format!(r#"const {{ spawn, execSync }} = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const isWindows = process.platform === 'win32';
const npmCmd = isWindows ? 'npm.cmd' : 'npm';
const npxCmd = isWindows ? 'npx.cmd' : 'npx';

const globalCacheDir = path.join(os.homedir(), '.wallets_tool', 'playwright_cache');
const tempDir = path.join(os.tmpdir(), 'playwright_recorder_{session_id}');

if (!fs.existsSync(globalCacheDir)) {{
  fs.mkdirSync(globalCacheDir, {{ recursive: true }});
}}
if (!fs.existsSync(tempDir)) {{
  fs.mkdirSync(tempDir, {{ recursive: true }});
}}

process.chdir(tempDir);

const playwrightInstalled = fs.existsSync(path.join(globalCacheDir, 'node_modules', 'playwright'));
const browserInstalled = fs.existsSync(path.join(globalCacheDir, 'node_modules', 'playwright', '.local-browsers'));

if (!playwrightInstalled) {{
  console.log('[Recorder] 首次使用，初始化 Playwright...');
  const initStart = Date.now();
  try {{
    process.chdir(globalCacheDir);
    if (!fs.existsSync('package.json')) {{
      console.log('[Recorder] 创建 package.json...');
      execSync('npm init -y', {{ stdio: 'inherit' }});
    }}
    console.log('[Recorder] 安装 Playwright...');
    execSync('npm install playwright@latest', {{ stdio: 'inherit' }});
  }} catch (err) {{
    console.error('Playwright 安装失败:', err);
    process.exit(1);
  }}
  console.log(`[Recorder] Playwright 安装完成，耗时 ${{Date.now() - initStart}}ms`);
}} else {{
  console.log('[Recorder] 使用已缓存的 Playwright');
}}

if (!browserInstalled) {{
  console.log('[Recorder] 首次使用，安装浏览器...');
  const browserStart = Date.now();
  try {{
    process.chdir(globalCacheDir);
    execSync('npx playwright install chromium', {{ stdio: 'inherit' }});
  }} catch (err) {{
    console.error('浏览器安装失败:', err);
    process.exit(1);
  }}
  console.log(`[Recorder] 浏览器安装完成，耗时 ${{Date.now() - browserStart}}ms`);
}} else {{
  console.log('[Recorder] 使用已缓存的浏览器');
}}

process.chdir(tempDir);

console.log('[Recorder] 启动浏览器...');
const launchStart = Date.now();

const playwrightPath = path.join(globalCacheDir, 'node_modules', 'playwright');
const {{ {browser_type} }} = require(playwrightPath);

(async () => {{
  let browser;
  try {{
    browser = await {browser_type}.launch({{
      headless: {headless},
      {proxy_config}
    }});
  }} catch (launchError) {{
    console.error('[Recorder] 浏览器启动失败:', launchError.message);
    process.exit(1);
  }}
  
  console.log(`[Recorder] 浏览器启动耗时 ${{Date.now() - launchStart}}ms`);
  
  // 监听浏览器关闭事件
  browser.on('disconnected', () => {{
    console.log('[Recorder] 浏览器已关闭');
    console.log('BROWSER_CLOSED:' + JSON.stringify({{ timestamp: Date.now() }}));
    process.exit(0);
  }});
  
  const context = await browser.newContext({{
    viewport: {{ width: {width}, height: {height} }}
  }});
  
  const page = await context.newPage();
  
  // 录制脚本 - 使用 evaluate 在页面加载后执行
  const recordingScript = `
    (function() {{
      console.log('[Recorder] 初始化录制脚本，session: {session_id}');
      
      if (window.__RECORDER_INJECTED__) {{
        console.log('[Recorder] 脚本已注入，跳过重复注入');
        return;
      }}
      window.__RECORDER_INJECTED__ = true;
      
      function logAction(actionType, selector, value, description) {{
        const action = {{
          action_type: actionType,
          selector: selector,
          value: value,
          timestamp: Date.now(),
          description: description
        }};
        console.log('ACTION:' + JSON.stringify(action));
      }}
      
      // 获取元素选择器
      function getSelector(element) {{
        if (!element) return 'unknown';
        
        if (element.id) return '#' + element.id;
        
        if (element.getAttribute('data-testid')) return '[data-testid="' + element.getAttribute('data-testid') + '"]';
        if (element.getAttribute('data-test-id')) return '[data-test-id="' + element.getAttribute('data-test-id') + '"]';
        if (element.name) return '[name="' + element.name + '"]';
        if (element.placeholder) return '[placeholder="' + element.placeholder + '"]';
        if (element.getAttribute('aria-label')) return '[aria-label="' + element.getAttribute('aria-label') + '"]';
        
        if (element.className && typeof element.className === 'string') {{
          const classes = element.className.split(' ').filter(c => c && !c.match(/^\\d+$/));
          if (classes.length > 0) return '.' + classes.slice(0, 2).join('.');
        }}
        
        const path = [];
        let current = element;
        let depth = 0;
        while (current && current.tagName !== 'BODY' && depth < 5) {{
          let selector = current.tagName.toLowerCase();
          const parent = current.parentElement;
          if (parent) {{
            const siblings = Array.from(parent.children).filter(s => s.tagName === current.tagName);
            if (siblings.length > 1) {{
              const index = siblings.indexOf(current) + 1;
              selector += ':nth-child(' + index + ')';
            }}
          }}
          path.unshift(selector);
          current = current.parentElement;
          depth++;
        }}
        
        return path.join(' > ');
      }}
      
      // 点击事件监听 - 只记录点击，不记录导航
      document.addEventListener('click', function(e) {{
        const selector = getSelector(e.target);
        console.log('[Recorder Debug] 点击事件:', selector);
        logAction('click', selector, null, '点击 ' + selector);
      }}, false);
      
      // 输入事件监听
      document.addEventListener('input', function(e) {{
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {{
          const selector = getSelector(e.target);
          console.log('[Recorder Debug] 输入事件:', selector, '值:', e.target.value);
          logAction('fill', selector, e.target.value, '在 ' + selector + ' 输入内容');
        }}
      }}, false);
      
      // 选择事件监听
      document.addEventListener('change', function(e) {{
        if (e.target.tagName === 'SELECT') {{
          const selector = getSelector(e.target);
          console.log('[Recorder Debug] 选择事件:', selector, '值:', e.target.value);
          logAction('select', selector, e.target.value, '选择 ' + e.target.value);
        }}
      }}, false);
      
      // 键盘事件监听
      document.addEventListener('keydown', function(e) {{
        if (e.key === 'Enter' && e.target.tagName !== 'BUTTON') {{
          const selector = getSelector(e.target);
          console.log('[Recorder Debug] 回车键:', selector);
          logAction('press', selector, 'Enter', '在 ' + selector + ' 按下回车');
        }}
      }}, false);
      
      console.log('[Recorder] 事件监听器已注册');
    }})();
  `;
  
  // 监听地址栏变化 - 仅用于调试日志
  page.on('framenavigated', async (frame) => {{
    if (frame === page.mainFrame()) {{
      console.log('[Recorder Debug] 页面已跳转至:', frame.url());
    }}
  }});
  
  // 在页面加载后执行录制脚本
  page.on('console', msg => {{
    const text = msg.text();
    console.log('[Recorder Console]', text);
    if (text.startsWith('ACTION:')) {{
      console.log(text);
    }}
  }});
  
  page.on('pageerror', err => {{
    console.error('[Recorder Page Error]', err.message);
  }});
  
  console.log('[Recorder] 正在导航到:', '{url}');
  console.log('浏览器已启动，开始录制...');
  console.log('会话ID: {session_id}');
  
  try {{
    await page.goto('{url}', {{
      timeout: 60000,
      waitUntil: 'domcontentloaded'
    }});
    console.log('[Recorder] 页面 DOM 加载完成，注入录制脚本...');
    
    // 在页面加载后注入录制脚本
    await page.evaluate(recordingScript);
    console.log('[Recorder] 录制脚本注入成功');
    
    // 等待一下确保脚本执行
    await page.waitForTimeout(500);
    
  }} catch (navError) {{
    console.log('[Recorder] 页面加载警告:', navError.message);
    console.log('[Recorder] 继续录制...');
    const action = {{
      action_type: 'navigate',
      selector: null,
      value: '{url}',
      timestamp: Date.now(),
      description: '导航到 {url} (加载超时)'
    }};
    console.log('ACTION:' + JSON.stringify(action));
  }}
  
  // 保持浏览器打开
  await new Promise(resolve => setTimeout(resolve, {timeout}));
  
  try {{
    await browser.close();
  }} catch (closeError) {{
    console.log('[Recorder] 浏览器关闭时出错:', closeError.message);
  }}
}})().catch(error => {{
  console.error('[Recorder] 未捕获的错误:', error.message);
  console.error('[Recorder] 错误堆栈:', error.stack);
  process.exit(0);
}});
"#, 
            session_id = session_id,
            browser_type = browser_type,
            headless = headless,
            proxy_config = proxy_config,
            width = width,
            height = height,
            url = url,
            timeout = DEFAULT_TIMEOUT_SECS * 1000
        );
        
        js_code
    }

    fn generate_code(&self, session: &RecordingSession) -> String {
        let mut code_lines = vec![
            "// 录制生成的脚本".to_string(),
            format!("// 录制时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
            format!("// 目标URL: {}", session.url),
            format!("// 操作数量: {}", session.actions.len()),
            "".to_string(),
            "async function visitPage(page, wallet, api) {".to_string(),
            "    api.log('info', '开始执行录制脚本');".to_string(),
            "".to_string(),
        ];

        for action in &session.actions {
            let line = match action.action_type.as_str() {
                "navigate" => {
                    if let Some(url) = &action.value {
                        format!("    await page.goto('{}');", url)
                    } else {
                        continue;
                    }
                }
                "click" => {
                    if let Some(selector) = &action.selector {
                        format!("    await page.click('{}');", selector.replace("'", "\\'"))
                    } else {
                        continue;
                    }
                }
                "fill" => {
                    if let (Some(selector), Some(value)) = (&action.selector, &action.value) {
                        format!("    await page.fill('{}', '{}');", 
                            selector.replace("'", "\\'"),
                            value.replace("'", "\\'"))
                    } else {
                        continue;
                    }
                }
                "select" => {
                    if let (Some(selector), Some(value)) = (&action.selector, &action.value) {
                        format!("    await page.selectOption('{}', '{}');",
                            selector.replace("'", "\\'"),
                            value.replace("'", "\\'"))
                    } else {
                        continue;
                    }
                }
                "hover" => {
                    if let Some(selector) = &action.selector {
                        format!("    await page.hover('{}');", selector.replace("'", "\\'"))
                    } else {
                        continue;
                    }
                }
                "screenshot" => {
                    "    await page.screenshot({ path: 'screenshot.png' });".to_string()
                }
                _ => continue,
            };
            code_lines.push(line);
        }

        code_lines.push("".to_string());
        code_lines.push("    api.log('success', '脚本执行完成');".to_string());
        code_lines.push("    return { success: true };".to_string());
        code_lines.push("}".to_string());

        code_lines.join("\n")
    }
}

// 全局录制器实例
lazy_static::lazy_static! {
    static ref RECORDER: PlaywrightRecorder = PlaywrightRecorder::new();
}

pub fn get_recorder() -> &'static PlaywrightRecorder {
    &RECORDER
}

// Tauri 命令
#[tauri::command]
pub async fn playwright_start_recording(options: RecordingOptions) -> Result<String, String> {
    get_recorder().start_recording(options).await
}

#[tauri::command]
pub async fn playwright_stop_recording(session_id: String) -> Result<Option<String>, String> {
    get_recorder().stop_recording(&session_id).await
}

#[tauri::command]
pub async fn playwright_get_recording_session(session_id: String) -> Result<Option<RecordingSession>, String> {
    get_recorder().get_session(&session_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToolStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliCheckResult {
    pub all_installed: bool,
    pub tools: Vec<CliToolStatus>,
}

#[tauri::command]
pub async fn check_cli_tools() -> Result<CliCheckResult, String> {
    let mut tools = Vec::new();
    let mut all_installed = true;

    // 检查 Node.js
    let node_cmd = if cfg!(windows) { "node.exe" } else { "node" };
    let node_status = match Command::new(node_cmd).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CliToolStatus {
                name: "Node.js".to_string(),
                installed: true,
                version: Some(version),
                error: None,
            }
        }
        _ => {
            all_installed = false;
            CliToolStatus {
                name: "Node.js".to_string(),
                installed: false,
                version: None,
                error: Some("未安装 Node.js。请从 https://nodejs.org/ 下载安装".to_string()),
            }
        }
    };
    tools.push(node_status);

    // 检查 npm
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let npm_status = match Command::new(npm_cmd).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CliToolStatus {
                name: "npm".to_string(),
                installed: true,
                version: Some(version),
                error: None,
            }
        }
        _ => {
            all_installed = false;
            CliToolStatus {
                name: "npm".to_string(),
                installed: false,
                version: None,
                error: Some("未安装 npm。通常随 Node.js 一起安装".to_string()),
            }
        }
    };
    tools.push(npm_status);

    // 检查 npx
    let npx_cmd = if cfg!(windows) { "npx.cmd" } else { "npx" };
    let npx_status = match Command::new(npx_cmd).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CliToolStatus {
                name: "npx".to_string(),
                installed: true,
                version: Some(version),
                error: None,
            }
        }
        _ => {
            all_installed = false;
            CliToolStatus {
                name: "npx".to_string(),
                installed: false,
                version: None,
                error: Some("未安装 npx。通常随 Node.js 一起安装".to_string()),
            }
        }
    };
    tools.push(npx_status);

    // 检查 Playwright（可选，因为会自动安装）
    let playwright_status = match Command::new(npx_cmd)
        .args(&["-y", "playwright", "--version"])
        .output() 
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CliToolStatus {
                name: "Playwright".to_string(),
                installed: true,
                version: Some(version),
                error: None,
            }
        }
        _ => {
            // Playwright 不是必须的，会自动安装
            CliToolStatus {
                name: "Playwright".to_string(),
                installed: false,
                version: None,
                error: Some("未安装 Playwright。首次录制时会自动安装".to_string()),
            }
        }
    };
    tools.push(playwright_status);

    Ok(CliCheckResult {
        all_installed,
        tools,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub logs: Vec<String>,
}

/// 自动安装 Node.js 环境
/// 使用 winget (Windows) 或系统包管理器安装 Node.js
#[tauri::command]
pub async fn install_node_environment() -> Result<InstallResult, String> {
    let mut logs = Vec::new();
    
    log_info(&mut logs, "开始安装 Node.js 环境...");
    
    // 首先检查是否已经安装
    let check_result = check_cli_tools().await?;
    let node_installed = check_result.tools.iter().any(|t| t.name == "Node.js" && t.installed);
    let npm_installed = check_result.tools.iter().any(|t| t.name == "npm" && t.installed);
    
    if node_installed && npm_installed {
        log_info(&mut logs, "Node.js 和 npm 已安装，跳过安装");
        return Ok(InstallResult {
            success: true,
            message: "Node.js 和 npm 已安装".to_string(),
            logs,
        });
    }
    
    #[cfg(windows)]
    {
        // Windows 平台使用 winget 安装
        log_info(&mut logs, "检测到 Windows 系统，使用 winget 安装 Node.js...");
        
        // 首先检查 winget 是否可用
        let winget_check = Command::new("winget")
            .arg("--version")
            .output();
        
        if winget_check.is_err() {
            log_error(&mut logs, "winget 不可用，请手动安装 Node.js");
            return Ok(InstallResult {
                success: false,
                message: "winget 不可用，请手动安装 Node.js".to_string(),
                logs,
            });
        }
        
        log_info(&mut logs, "正在使用 winget 安装 Node.js...");
        
        // 使用 winget 安装 Node.js
        let install_output = Command::new("winget")
            .args(&[
                "install",
                "--id", "OpenJS.NodeJS",
                "--source", "winget",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--silent"
            ])
            .output();
        
        match install_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                log_info(&mut logs, &format!("winget 输出: {}", stdout));
                if !stderr.is_empty() {
                    log_warn(&mut logs, &format!("winget 警告: {}", stderr));
                }
                
                if output.status.success() {
                    log_info(&mut logs, "Node.js 安装命令执行成功");
                    
                    // 等待安装完成并刷新环境变量
                    log_info(&mut logs, "等待安装完成...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    // 重新检查安装状态
                    let check_result = check_cli_tools().await?;
                    let node_installed = check_result.tools.iter().any(|t| t.name == "Node.js" && t.installed);
                    let npm_installed = check_result.tools.iter().any(|t| t.name == "npm" && t.installed);
                    
                    if node_installed && npm_installed {
                        log_info(&mut logs, "Node.js 和 npm 安装成功！");
                        Ok(InstallResult {
                            success: true,
                            message: "Node.js 和 npm 安装成功".to_string(),
                            logs,
                        })
                    } else {
                        log_error(&mut logs, "安装后检查失败，可能需要重启应用程序");
                        Ok(InstallResult {
                            success: false,
                            message: "安装后检查失败，请重启应用程序".to_string(),
                            logs,
                        })
                    }
                } else {
                    let error_msg = format!("winget 安装失败，退出码: {:?}", output.status.code());
                    log_error(&mut logs, &error_msg);
                    Ok(InstallResult {
                        success: false,
                        message: error_msg,
                        logs,
                    })
                }
            }
            Err(e) => {
                let error_msg = format!("执行 winget 安装失败: {}", e);
                log_error(&mut logs, &error_msg);
                Ok(InstallResult {
                    success: false,
                    message: error_msg,
                    logs,
                })
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台使用 brew 安装
        log_info(&mut logs, "检测到 macOS 系统，使用 Homebrew 安装 Node.js...");
        
        let install_output = Command::new("brew")
            .args(&["install", "node"])
            .output();
        
        match install_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                log_info(&mut logs, &format!("brew 输出: {}", stdout));
                
                if output.status.success() {
                    log_info(&mut logs, "Node.js 安装成功！");
                    Ok(InstallResult {
                        success: true,
                        message: "Node.js 安装成功".to_string(),
                        logs,
                    })
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_msg = format!("brew 安装失败: {}", stderr);
                    log_error(&mut logs, &error_msg);
                    Ok(InstallResult {
                        success: false,
                        message: error_msg,
                        logs,
                    })
                }
            }
            Err(e) => {
                let error_msg = format!("执行 brew 安装失败: {}", e);
                log_error(&mut logs, &error_msg);
                Ok(InstallResult {
                    success: false,
                    message: error_msg,
                    logs,
                })
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux 平台尝试使用 apt
        log_info(&mut logs, "检测到 Linux 系统，使用 apt 安装 Node.js...");
        
        let install_output = Command::new("sh")
            .args(&["-c", "sudo apt update && sudo apt install -y nodejs npm"])
            .output();
        
        match install_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                log_info(&mut logs, &format!("apt 输出: {}", stdout));
                
                if output.status.success() {
                    log_info(&mut logs, "Node.js 安装成功！");
                    Ok(InstallResult {
                        success: true,
                        message: "Node.js 安装成功".to_string(),
                        logs,
                    })
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_msg = format!("apt 安装失败: {}", stderr);
                    log_error(&mut logs, &error_msg);
                    Ok(InstallResult {
                        success: false,
                        message: error_msg,
                        logs,
                    })
                }
            }
            Err(e) => {
                let error_msg = format!("执行 apt 安装失败: {}", e);
                log_error(&mut logs, &error_msg);
                Ok(InstallResult {
                    success: false,
                    message: error_msg,
                    logs,
                })
            }
        }
    }
}

fn log_info(logs: &mut Vec<String>, message: &str) {
    let log = format!("[INFO] {}", message);
    println!("{}", log);
    logs.push(log);
}

fn log_warn(logs: &mut Vec<String>, message: &str) {
    let log = format!("[WARN] {}", message);
    println!("{}", log);
    logs.push(log);
}

fn log_error(logs: &mut Vec<String>, message: &str) {
    let log = format!("[ERROR] {}", message);
    eprintln!("{}", log);
    logs.push(log);
}
