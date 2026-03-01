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
    pub page_url: Option<String>,
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
    pub record_mouse_move: Option<bool>,
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
                page_url: Some(options.url.clone()),
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
        let _width = options.viewport_width.unwrap_or(1280);
        let _height = options.viewport_height.unwrap_or(720);
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

        // 生成随机指纹参数
        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36 Edg/133.0.0.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
        ];
        let _user_agent = user_agents[session_id.len() % user_agents.len()];

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

console.log('[Recorder] 启动浏览器（启用抗检测模式）...');
const launchStart = Date.now();

const playwrightPath = path.join(globalCacheDir, 'node_modules', 'playwright');
const {{ {browser_type} }} = require(playwrightPath);

// 生成随机指纹参数
function generateFingerprint() {{
  const userAgents = [
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36 Edg/133.0.0.0',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36',
  ];
  
  const screenSizes = [
    {{ width: 1920, height: 1080 }},
    {{ width: 2560, height: 1440 }},
    {{ width: 1366, height: 768 }},
    {{ width: 1440, height: 900 }},
    {{ width: 1536, height: 864 }},
  ];
  
  const timezones = [
    'Asia/Shanghai', 'Asia/Tokyo', 'Asia/Singapore', 'America/New_York', 'America/Los_Angeles',
    'Europe/London', 'Europe/Paris', 'Europe/Berlin'
  ];
  
  const locales = ['zh-CN', 'zh-TW', 'en-US', 'en-GB', 'ja-JP', 'ko-KR', 'de-DE', 'fr-FR'];
  const hardwareConcurrencyOptions = [2, 4, 6, 8, 12, 16];
  const deviceMemoryOptions = [2, 4, 6, 8, 12, 16];
  const devicePixelRatios = [1, 1.25, 1.5, 1.75, 2];
  
  const randomChoice = (arr) => arr[Math.floor(Math.random() * arr.length)];
  
  const userAgent = randomChoice(userAgents);
  const screenSize = randomChoice(screenSizes);
  const platform = userAgent.includes('Macintosh') ? 'MacIntel' : 'Win32';
  const locale = randomChoice(locales);
  const timezone = randomChoice(timezones);
  
  return {{
    userAgent,
    screenSize,
    platform,
    locale,
    timezone,
    hardwareConcurrency: randomChoice(hardwareConcurrencyOptions),
    deviceMemory: randomChoice(deviceMemoryOptions),
    devicePixelRatio: randomChoice(devicePixelRatios),
    languages: [locale, 'en-US', 'en'],
    vendor: 'Google Inc.',
    colorDepth: 24,
    maxTouchPoints: 0,
  }};
}}

const fingerprint = generateFingerprint();
console.log('[Recorder] 生成浏览器指纹:', fingerprint.userAgent);

// 录制选项
const recordMouseMove = {record_mouse_move};

// 生成增强版抗检测脚本
function generateStealthScript(fp) {{
  return `
    (() => {{
      'use strict';
      
      const DEBUG = false;
      const log = (...args) => {{ if (DEBUG) console.log('[Stealth]', ...args); }};
      
      // ========== 1. WebGPU 完整支持 ==========
      try {{
        const createGPUAdapter = () => {{
          const features = new Set([
            'depth-clip-control', 'indirect-first-instance', 'shader-f16',
            'depth24unorm-stencil8', 'depth32float-stencil8', 'texture-compression-bc',
            'texture-compression-etc2', 'texture-compression-astc', 'timestamp-query',
            'float32-filterable', 'readonly_and_readwrite_storage_textures',
            'packed_4x8_integer_dot_product', 'unrestricted_pointer_parameters'
          ]);
          
          const limits = {{
            maxTextureDimension1D: 8192, maxTextureDimension2D: 8192, maxTextureDimension3D: 2048,
            maxTextureArrayLayers: 256, maxBindGroups: 8, maxBindGroupsPlusVertexBuffers: 24,
            maxBindingsPerBindGroup: 16, maxBufferSize: 268435456, maxVertexBuffers: 8,
            maxVertexAttributes: 16, maxVertexBufferArrayStride: 2048,
            maxInterStageShaderComponents: 64, maxInterStageShaderVariables: 16,
            maxColorAttachments: 8, maxColorAttachmentBytesPerSample: 32,
            maxComputeWorkgroupStorageSize: 32768, maxComputeInvocationsPerWorkgroup: 256,
            maxComputeWorkgroupSizeX: 256, maxComputeWorkgroupSizeY: 256,
            maxComputeWorkgroupSizeZ: 64, maxComputeWorkgroupsPerDimension: 65535
          }};
          
          return {{
            features, limits, isFallbackAdapter: false,
            requestAdapterInfo: async () => ({{
              vendor: 'Google Inc.', architecture: '', device: 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti)',
              description: '', subgroupMinSize: 4, subgroupMaxSize: 128
            }}),
            requestDevice: async () => ({{
              features, limits,
              queue: {{ 
                submit: () => {{}}, 
                copyExternalImageToTexture: () => {{}}, 
                writeBuffer: () => {{}}, 
                writeTexture: () => {{}},
                onsubmittedworkdone: Promise.resolve()
              }},
              createBuffer: (desc) => ({{ 
                size: desc?.size || 0, usage: desc?.usage || 0,
                getMappedRange: () => new ArrayBuffer(desc?.size || 0), 
                unmap: () => {{}}, 
                mapAsync: async () => {{}} 
              }}),
              createTexture: (desc) => ({{ 
                width: desc?.size?.[0] || 1, height: desc?.size?.[1] || 1,
                depthOrArrayLayers: desc?.size?.[2] || 1,
                mipLevelCount: desc?.mipLevelCount || 1,
                sampleCount: desc?.sampleCount || 1,
                dimension: desc?.dimension || '2d',
                format: desc?.format || 'rgba8unorm',
                usage: desc?.usage || 0,
                createView: () => ({{}}), 
                destroy: () => {{}} 
              }}),
              createShaderModule: () => ({{ compilationInfo: async () => ({{ messages: [] }}) }}),
              createPipelineLayout: () => ({{}}),
              createRenderPipeline: () => ({{ getBindGroupLayout: () => ({{}}) }}),
              createComputePipeline: () => ({{ getBindGroupLayout: () => ({{}}) }}),
              createCommandEncoder: () => ({{
                beginRenderPass: () => ({{ 
                  setPipeline: () => {{}}, 
                  setBindGroup: () => {{}},
                  setVertexBuffer: () => {{}},
                  setIndexBuffer: () => {{}},
                  draw: () => {{}},
                  drawIndexed: () => {{}},
                  end: () => {{}} 
                }}),
                beginComputePass: () => ({{
                  setPipeline: () => {{}},
                  setBindGroup: () => {{}},
                  dispatchWorkgroups: () => {{}},
                  end: () => {{}}
                }}),
                copyBufferToBuffer: () => {{}},
                copyBufferToTexture: () => {{}},
                copyTextureToBuffer: () => {{}},
                copyTextureToTexture: () => {{}},
                clearBuffer: () => {{}},
                writeTimestamp: () => {{}},
                resolveQuerySet: () => {{}},
                finish: () => ({{}})
              }}),
              createBindGroup: () => ({{}}),
              createBindGroupLayout: () => ({{}}),
              createSampler: () => ({{}}),
              pushErrorScope: () => {{}},
              popErrorScope: async () => null,
              importExternalTexture: () => ({{}}),
              destroy: () => {{}},
              lost: Promise.resolve({{ reason: undefined, message: '' }})
            }})
          }};
        }};
        
        const fakeGPU = {{
          requestAdapter: async (options) => {{
            const adapter = createGPUAdapter();
            if (options?.powerPreference === 'low-power') adapter.isFallbackAdapter = true;
            return adapter;
          }},
          getPreferredCanvasFormat: () => 'bgra8unorm',
          wgslLanguageFeatures: new Set([
            'readonly_and_readwrite_storage_textures',
            'packed_4x8_integer_dot_product',
            'unrestricted_pointer_parameters'
          ])
        }};
        
        Object.defineProperty(navigator, 'gpu', {{ get: () => fakeGPU, configurable: true, enumerable: true }});
        Object.defineProperty(Navigator.prototype, 'gpu', {{ get: () => fakeGPU, configurable: true, enumerable: true }});
        log('WebGPU injected');
      }} catch (e) {{ log('WebGPU injection failed:', e.message); }}
      
      // ========== 2. 深度 Object 检测绕过 ==========
      try {{
        const originalGetOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
        const originalGetOwnPropertyDescriptors = Object.getOwnPropertyDescriptors;
        const originalGetOwnPropertyNames = Object.getOwnPropertyNames;
        const originalGetOwnPropertySymbols = Object.getOwnPropertySymbols;
        const originalHasOwnProperty = Object.prototype.hasOwnProperty;
        const originalDefineProperty = Object.defineProperty;
        const originalKeys = Object.keys;
        const originalValues = Object.values;
        const originalEntries = Object.entries;
        
        const protectedProps = ['webdriver', 'plugins', 'mimeTypes', 'languages', 
                               'hardwareConcurrency', 'deviceMemory', 'platform', 
                               'vendor', 'maxTouchPoints', 'gpu', 'mediaDevices', 
                               'connection', 'permissions', 'userAgentData'];
        const protectedObjects = [navigator, window.navigator, window.screen, window];
        
        // 重写 getOwnPropertyDescriptor
        Object.getOwnPropertyDescriptor = function(obj, prop) {{
          try {{
            const result = originalGetOwnPropertyDescriptor.call(Object, obj, prop);
            if (protectedObjects.includes(obj) && protectedProps.includes(prop) && result) {{
              return {{ 
                get: result.get || (() => obj[prop]), 
                set: result.set || undefined, 
                enumerable: true, 
                configurable: true 
              }};
            }}
            return result;
          }} catch (e) {{ 
            return originalGetOwnPropertyDescriptor.call(Object, obj, prop); 
          }}
        }};
        
        // 重写 getOwnPropertyDescriptors
        Object.getOwnPropertyDescriptors = function(obj) {{
          try {{
            const descriptors = originalGetOwnPropertyDescriptors.call(Object, obj);
            if (protectedObjects.includes(obj)) {{
              Object.keys(descriptors).forEach(key => {{
                if (protectedProps.includes(key)) {{
                  descriptors[key] = {{ 
                    get: descriptors[key].get || (() => obj[key]), 
                    set: descriptors[key].set || undefined, 
                    enumerable: true, 
                    configurable: true 
                  }};
                }}
              }});
            }}
            return descriptors;
          }} catch (e) {{ 
            return originalGetOwnPropertyDescriptors.call(Object, obj); 
          }}
        }};
        
        // 重写 hasOwnProperty
        Object.prototype.hasOwnProperty = function(v) {{
          try {{
            if (protectedObjects.includes(this) && protectedProps.includes(v)) return false;
            return originalHasOwnProperty.call(this, v);
          }} catch (e) {{ return false; }}
        }};
        
        // 重写 Object.keys
        Object.keys = function(obj) {{
          try {{
            if (protectedObjects.includes(obj)) {{
              return originalKeys(obj).filter(k => !protectedProps.includes(k));
            }}
            return originalKeys(obj);
          }} catch (e) {{ return []; }}
        }};
        
        // 重写 Object.getOwnPropertyNames
        Object.getOwnPropertyNames = function(obj) {{
          try {{
            if (protectedObjects.includes(obj)) {{
              return originalGetOwnPropertyNames.call(Object, obj).filter(k => !protectedProps.includes(k));
            }}
            return originalGetOwnPropertyNames.call(Object, obj);
          }} catch (e) {{ return []; }}
        }};
        
        // 保护 JSON.stringify
        const originalStringify = JSON.stringify;
        JSON.stringify = function(value, replacer, space) {{
          if (value === navigator) {{
            const navigatorCopy = {{}};
            Object.keys(value).forEach(key => {{
              if (!protectedProps.includes(key)) {{
                navigatorCopy[key] = value[key];
              }}
            }});
            return originalStringify.call(JSON, navigatorCopy, replacer, space);
          }}
          return originalStringify.call(JSON, value, replacer, space);
        }};
        
        log('Object descriptor protection enabled');
      }} catch (e) {{ }}
      
      // ========== 3. 多层 webdriver 移除（终极版） ==========
      try {{
        // 方法1: 直接删除
        delete navigator.webdriver;
        
        // 方法2: 定义在 navigator 上
        Object.defineProperty(navigator, 'webdriver', {{ 
          get: () => false, 
          configurable: true, 
          enumerable: true 
        }});
        
        // 方法3: 定义在 Navigator.prototype 上
        Object.defineProperty(Navigator.prototype, 'webdriver', {{ 
          get: () => false, 
          configurable: true, 
          enumerable: true 
        }});
        
        // 方法4: 定义在原型链上
        const navigatorProto = Object.getPrototypeOf(navigator);
        if (navigatorProto) {{
          Object.defineProperty(navigatorProto, 'webdriver', {{ 
            get: () => false, 
            configurable: true, 
            enumerable: true 
          }});
        }}
        
        // 方法5: 覆盖 Object.getPrototypeOf
        const originalGetPrototypeOf = Object.getPrototypeOf;
        Object.getPrototypeOf = function(obj) {{
          const proto = originalGetPrototypeOf.call(Object, obj);
          if (obj === navigator && proto) {{
            delete proto.webdriver;
            Object.defineProperty(proto, 'webdriver', {{ 
              get: () => false, 
              configurable: true, 
              enumerable: true 
            }});
          }}
          return proto;
        }};
        
        log('webdriver removed (ultimate)');
      }} catch (e) {{ }}
      
      // ========== 4. Canvas 指纹混淆（增强版） ==========
      try {{
        const originalGetContext = HTMLCanvasElement.prototype.getContext;
        const canvasNoise = Math.random();
        
        // 生成稳定的噪声
        const generateStableNoise = (width, height) => {{
          const seed = width * 31 + height * 17;
          return (Math.sin(seed) + 1) / 2;
        }};
        
        HTMLCanvasElement.prototype.getContext = function(type, ...args) {{
          const context = originalGetContext.apply(this, [type, ...args]);
          if (!context) return context;
          
          const canvas = this;
          const width = canvas.width || 300;
          const height = canvas.height || 150;
          const stableNoise = generateStableNoise(width, height);
          
          if (type === '2d') {{
            const randomOffset = () => (Math.random() - 0.5) * 0.0001;
            
            // 文本渲染方法
            ['fillText', 'strokeText', 'measureText'].forEach(method => {{
              const original = context[method];
              context[method] = function(...textArgs) {{
                if (method !== 'measureText' && Math.random() > 0.2) {{
                  textArgs[1] = (textArgs[1] || 0) + randomOffset();
                  textArgs[2] = (textArgs[2] || 0) + randomOffset();
                }}
                return original.apply(this, textArgs);
              }};
            }});
            
            // 路径方法
            ['moveTo', 'lineTo', 'bezierCurveTo', 'quadraticCurveTo', 'arc', 'arcTo'].forEach(method => {{
              const original = context[method];
              context[method] = function(...pathArgs) {{
                for (let i = 0; i < pathArgs.length; i++) {{
                  if (typeof pathArgs[i] === 'number' && Math.random() > 0.7) {{
                    pathArgs[i] += randomOffset() * 0.1;
                  }}
                }}
                return original.apply(this, pathArgs);
              }};
            }});
            
            // 图像数据获取
            const originalGetImageData = context.getImageData;
            context.getImageData = function(sx, sy, sw, sh) {{
              const imageData = originalGetImageData.apply(this, [sx, sy, sw, sh]);
              const noise = Math.floor(stableNoise * 3) - 1;
              for (let i = 0; i < imageData.data.length; i += 4) {{
                imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));
                imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));
                imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));
              }}
              return imageData;
            }};
            
            // toDataURL 防护
            const originalToDataURL = canvas.toDataURL;
            canvas.toDataURL = function(...args) {{
              const originalFillStyle = context.fillStyle;
              context.fillStyle = 'rgba(0,0,0,0.001)';
              context.fillRect(0, 0, 1, 1);
              context.fillStyle = originalFillStyle;
              return originalToDataURL.apply(this, args);
            }};
          }}
          return context;
        }};
        log('Canvas protection enabled (enhanced)');
      }} catch (e) {{ }}
      
      // ========== 5. WebGL/WebGL2 指纹伪装（增强版） ==========
      try {{
        const gpuVendor = fp.vendor;
        const gpuRenderer = 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)';
        
        const overrideWebGL = (WebGLClass) => {{
          if (!WebGLClass) return;
          
          const originalGetParameter = WebGLClass.prototype.getParameter;
          WebGLClass.prototype.getParameter = function(parameter) {{
            if (parameter === 37445) return gpuVendor;
            if (parameter === 37446) return gpuRenderer;
            return originalGetParameter.apply(this, arguments);
          }};
          
          const originalGetExtension = WebGLClass.prototype.getExtension;
          WebGLClass.prototype.getExtension = function(name) {{
            if (name === 'WEBGL_debug_renderer_info') return null;
            return originalGetExtension.apply(this, [name]);
          }};
        }};
        
        overrideWebGL(WebGLRenderingContext);
        overrideWebGL(WebGL2RenderingContext);
        log('WebGL protection enabled (enhanced)');
      }} catch (e) {{ }}
      
      // ========== 6. AudioContext 完整伪装 ==========
      try {{
        const createFakeAudioBuffer = () => ({{
          sampleRate: 48000 + Math.floor(Math.random() * 100),
          length: 1024,
          duration: 0.021333333333333333,
          numberOfChannels: 2,
          getChannelData: () => new Float32Array(1024).map(() => (Math.random() - 0.5) * 0.001),
          copyFromChannel: () => {{}},
          copyToChannel: () => {{}}
        }});

        const FakeAudioContext = class {{
          constructor() {{
            this.state = 'running';
            this.sampleRate = 48000;
            this.baseLatency = 0.01;
            this.outputLatency = 0.01;
            this.destination = {{
              maxChannelCount: 2, numberOfInputs: 1, numberOfOutputs: 0,
              channelCount: 2, channelCountMode: 'explicit', channelInterpretation: 'speakers'
            }};
          }}
          createAnalyser() {{
            return {{
              fftSize: 2048, frequencyBinCount: 1024,
              getFloatFrequencyData: (arr) => {{ for (let i = 0; i < arr.length; i++) arr[i] = -100 + Math.random() * 70; }},
              getFloatTimeDomainData: (arr) => {{ for (let i = 0; i < arr.length; i++) arr[i] = (Math.random() - 0.5) * 0.01; }},
              connect: () => {{}}, disconnect: () => {{}}
            }};
          }}
          createOscillator() {{ return {{ type: 'sine', frequency: {{ value: 440 }}, connect: () => {{}}, start: () => {{}}, stop: () => {{}} }}; }}
          createBufferSource() {{ return {{ buffer: null, playbackRate: {{ value: 1 }}, connect: () => {{}}, start: () => {{}}, stop: () => {{}} }}; }}
          createBuffer() {{ return createFakeAudioBuffer(); }}
          decodeAudioData() {{ return Promise.resolve(createFakeAudioBuffer()); }}
          createGain() {{ return {{ gain: {{ value: 1 }}, connect: () => {{}} }}; }}
          close() {{ this.state = 'closed'; return Promise.resolve(); }}
          suspend() {{ this.state = 'suspended'; return Promise.resolve(); }}
          resume() {{ this.state = 'running'; return Promise.resolve(); }}
        }};

        window.AudioContext = FakeAudioContext;
        window.webkitAudioContext = FakeAudioContext;
        log('AudioContext faked (complete)');
      }} catch (e) {{ }}
      
      // ========== 7. Chrome 对象完整伪装 ==========
      try {{
        window.chrome = {{
          app: {{
            isInstalled: false,
            InstallState: {{ DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' }},
            RunningState: {{ CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' }},
            getDetails: () => null, 
            getIsInstalled: () => false
          }},
          runtime: {{
            OnInstalledReason: {{ CHROME_UPDATE: 'chrome_update', INSTALL: 'install' }},
            OnRestartRequiredReason: {{ APP_UPDATE: 'app_update', OS_UPDATE: 'os_update' }},
            PlatformArch: {{ ARM: 'arm', X86_64: 'x86-64' }},
            PlatformOs: {{ WIN: 'win', MAC: 'mac', LINUX: 'linux', ANDROID: 'android', OPENBSD: 'openbsd' }},
            connect: () => ({{ 
              onDisconnect: {{ addListener: () => {{}} }}, 
              onMessage: {{ addListener: () => {{}} }}, 
              postMessage: () => {{}},
              disconnect: () => {{}}
            }}),
            sendMessage: () => {{}},
            getManifest: () => ({{}}),
            getURL: (path) => 'chrome-extension://' + path,
            reload: () => {{}},
            requestUpdateCheck: () => {{}},
            restart: () => {{}},
            restartAfterDelay: () => {{}}
          }},
          csi: () => ({{}}),
          loadTimes: () => ({{
            commitLoadTime: Date.now() / 1000 - Math.random() * 2,
            connectionInfo: 'http/1.1',
            finishDocumentLoadTime: Date.now() / 1000,
            finishLoadTime: Date.now() / 1000,
            firstPaintTime: Date.now() / 1000 - Math.random(),
            navigationType: 'Other',
            requestTime: Date.now() / 1000 - Math.random() * 3,
            startLoadTime: Date.now() / 1000 - Math.random() * 2.5,
          }})
        }};
        
        // 确保 window.chrome 也存在
        if (!window.navigator.chrome) {{
          window.navigator.chrome = window.chrome;
        }}
        
        log('Chrome object faked (complete)');
      }} catch (e) {{ }}
      
      // ========== 8. Plugins 伪装（增强版） ==========
      try {{
        const createFakePlugin = (name, filename, description, version) => {{
          const plugin = Object.create(Plugin.prototype);
          ['name', 'filename', 'description', 'version', 'length'].forEach(prop => {{
            Object.defineProperty(plugin, prop, {{ 
              get: () => ({{ name, filename, description, version: version || 'undefined', length: 0 }}[prop]),
              enumerable: true 
            }});
          }});
          plugin.item = () => null;
          plugin.namedItem = () => null;
          return plugin;
        }};
        
        // 随机版本号
        const randomVersion = () => '1.0.' + Math.floor(Math.random() * 10) + '.' + Math.floor(Math.random() * 100);
        
        const pluginsData = [
          createFakePlugin('Chrome PDF Plugin', 'internal-pdf-viewer', 'Portable Document Format', randomVersion()),
          createFakePlugin('Chrome PDF Viewer', 'mhjfbmdgcfjbbpaeojofohoefgiehjai', 'Portable Document Format', randomVersion()),
          createFakePlugin('Native Client', 'internal-nacl-plugin', '', randomVersion()),
          createFakePlugin('Widevine Content Decryption Module', 'widevinecdmadapter.dll', 'Widevine Content Decryption Module', randomVersion())
        ];
        
        const plugins = Object.create(PluginArray.prototype);
        pluginsData.forEach((p, i) => plugins[i] = p);
        Object.defineProperty(plugins, 'length', {{ get: () => pluginsData.length, enumerable: true }});
        plugins.item = function(i) {{ return this[i] || null; }};
        plugins.namedItem = function(name) {{
          for (let i = 0; i < this.length; i++) if (this[i].name === name) return this[i];
          return null;
        }};
        plugins.refresh = () => {{}};
        
        Object.defineProperty(navigator, 'plugins', {{ get: () => plugins, configurable: true, enumerable: true }});
        log('Plugins faked (enhanced)');
      }} catch (e) {{ }}
      
      // ========== 9. MimeTypes 伪装（增强版） ==========
      try {{
        const createFakeMimeType = (type, suffixes, description, plugin) => {{
          const mimeType = Object.create(MimeType.prototype);
          Object.defineProperty(mimeType, 'type', {{ get: () => type, enumerable: true }});
          Object.defineProperty(mimeType, 'suffixes', {{ get: () => suffixes, enumerable: true }});
          Object.defineProperty(mimeType, 'description', {{ get: () => description, enumerable: true }});
          Object.defineProperty(mimeType, 'enabledPlugin', {{ get: () => plugin, enumerable: true }});
          return mimeType;
        }};
        
        const mimeTypesData = [
          createFakeMimeType('application/pdf', 'pdf', 'Portable Document Format', navigator.plugins[1]),
          createFakeMimeType('application/x-google-chrome-pdf', 'pdf', 'Portable Document Format', navigator.plugins[1]),
          createFakeMimeType('application/x-nacl', '', 'Native Client module', navigator.plugins[2]),
          createFakeMimeType('application/x-pnacl', '', 'Portable Native Client module', navigator.plugins[2]),
          createFakeMimeType('application/octet-stream', '', '', null)
        ];
        
        const mimeTypes = Object.create(MimeTypeArray.prototype);
        mimeTypesData.forEach((m, i) => mimeTypes[i] = m);
        Object.defineProperty(mimeTypes, 'length', {{ get: () => mimeTypesData.length, enumerable: true }});
        mimeTypes.item = function(i) {{ return this[i] || null; }};
        mimeTypes.namedItem = function(name) {{
          for (let i = 0; i < this.length; i++) if (this[i].type === name) return this[i];
          return null;
        }};
        
        Object.defineProperty(navigator, 'mimeTypes', {{ get: () => mimeTypes, configurable: true, enumerable: true }});
        log('MimeTypes faked (enhanced)');
      }} catch (e) {{ }}
      
      // ========== 10. Navigator 属性伪装（增强版） ==========
      try {{
        const props = {{
          languages: fp.languages,
          hardwareConcurrency: fp.hardwareConcurrency,
          deviceMemory: fp.deviceMemory,
          platform: fp.platform,
          vendor: fp.vendor,
          maxTouchPoints: fp.maxTouchPoints,
          productSub: '20030107',
          vendorSub: '',
          product: 'Gecko',
          appCodeName: 'Mozilla',
          appName: 'Netscape',
          pdfViewerEnabled: true,
          webdriver: false,
          doNotTrack: Math.random() > 0.5 ? '1' : '0',
          standalone: false,
          onLine: true,
          cookieEnabled: true,
          javaEnabled: () => false,
          taintEnabled: () => false,
        }};
        
        Object.entries(props).forEach(([key, value]) => {{
          try {{ Object.defineProperty(navigator, key, {{ get: () => value, configurable: true, enumerable: true }}); }} catch (e) {{}}
        }});
        log('Navigator properties set');
      }} catch (e) {{ }}
      
      // 11. userAgentData 伪装
      try {{
        const chromeVersion = navigator.userAgent.match(/Chrome\\/(\\d+)/)?.[1] || '133';
        const uaPlatform = fp.platform === 'MacIntel' ? 'macOS' : 'Windows';
        const uaPlatformVersion = fp.platform === 'MacIntel' ? '14.0.0' : '15.0.0';
        const uaData = {{
          brands: [
            {{ brand: 'Not.A;Brand', version: '8' }},
            {{ brand: 'Chromium', version: chromeVersion }},
            {{ brand: 'Google Chrome', version: chromeVersion }}
          ],
          mobile: false,
          platform: uaPlatform,
          getHighEntropyValues: async (hints) => {{
            const highEntropyData = {{
              platform: uaPlatform,
              platformVersion: uaPlatformVersion,
              architecture: 'x86',
              bitness: '64',
              model: '',
              uaFullVersion: chromeVersion + '.0.0.0',
              fullVersionList: [
                {{ brand: 'Not.A;Brand', version: '8' }},
                {{ brand: 'Chromium', version: chromeVersion }},
                {{ brand: 'Google Chrome', version: chromeVersion }}
              ],
              wow64: false,
              formFactors: ['desktop']
            }};
            const result = {{}};
            hints.forEach(hint => {{ if (highEntropyData[hint] !== undefined) result[hint] = highEntropyData[hint]; }});
            return result;
          }},
          toJSON: () => ({{ brands: uaData.brands, mobile: uaData.mobile, platform: uaData.platform }})
        }};
        Object.defineProperty(navigator, 'userAgentData', {{ get: () => uaData, configurable: true, enumerable: true }});
        log('userAgentData faked');
      }} catch (e) {{ }}
      
      // 12. Permissions API
      try {{
        Object.defineProperty(navigator, 'permissions', {{
          get: () => ({{
            query: async (params) => {{
              const map = {{
                'notifications': 'default', 'camera': 'prompt', 'microphone': 'prompt',
                'geolocation': 'prompt', 'clipboard-read': 'prompt', 'clipboard-write': 'granted',
                'push': 'default', 'midi': 'prompt'
              }};
              return {{ state: map[params.name] || 'prompt', onchange: null }};
            }}
          }}),
          configurable: true, enumerable: true
        }});
      }} catch (e) {{ }}
      
      // 13. Notification API
      try {{
        window.Notification = class Notification {{
          constructor(title, options = {{}}) {{
            this.title = title;
            this.body = options.body || '';
            this.icon = options.icon || '';
            this.tag = options.tag || '';
            this.timestamp = Date.now();
          }}
          static get permission() {{ return 'default'; }}
          static requestPermission() {{ return Promise.resolve('default'); }}
          close() {{}}
        }};
      }} catch (e) {{ }}
      
      // 14. 窗口尺寸伪装
      try {{
        const outerWidthOffset = 16 + Math.floor(Math.random() * 8);
        const outerHeightOffset = 85 + Math.floor(Math.random() * 15);
        Object.defineProperty(window, 'outerWidth', {{ get: () => window.innerWidth + outerWidthOffset, configurable: true }});
        Object.defineProperty(window, 'outerHeight', {{ get: () => window.innerHeight + outerHeightOffset, configurable: true }});
        Object.defineProperty(window, 'devicePixelRatio', {{ get: () => fp.devicePixelRatio, configurable: true }});
        Object.defineProperty(window, 'screenX', {{ get: () => Math.floor(Math.random() * 50), configurable: true }});
        Object.defineProperty(window, 'screenY', {{ get: () => Math.floor(Math.random() * 50), configurable: true }});
      }} catch (e) {{ }}
      
      // 15. Screen 对象
      try {{
        Object.defineProperty(window.screen, 'pixelDepth', {{ get: () => fp.colorDepth, configurable: true }});
        Object.defineProperty(window.screen, 'colorDepth', {{ get: () => fp.colorDepth, configurable: true }});
        Object.defineProperty(window.screen, 'availWidth', {{ get: () => window.screen.width - 10, configurable: true }});
        Object.defineProperty(window.screen, 'availHeight', {{ get: () => window.screen.height - 40 - Math.floor(Math.random() * 50), configurable: true }});
        Object.defineProperty(window.screen, 'width', {{ get: () => fp.screenSize.width, configurable: true }});
        Object.defineProperty(window.screen, 'height', {{ get: () => fp.screenSize.height, configurable: true }});
      }} catch (e) {{ }}
      
      // 16. 电池 API
      try {{
        Object.defineProperty(navigator, 'getBattery', {{
          get: () => async () => ({{
            charging: true, chargingTime: 0, dischargingTime: Infinity,
            level: 0.95 + Math.random() * 0.05,
            addEventListener: () => {{}}, removeEventListener: () => {{}}
          }}),
          configurable: true
        }});
      }} catch (e) {{ }}
      
      // 17. 网络信息
      try {{
        Object.defineProperty(navigator, 'connection', {{
          get: () => ({{
            downlink: 8 + Math.random() * 5,
            effectiveType: ['4g', '4g', '4g', '3g'][Math.floor(Math.random() * 4)],
            rtt: 30 + Math.floor(Math.random() * 100),
            saveData: false,
            type: 'wifi',
            addEventListener: () => {{}}, removeEventListener: () => {{}}
          }}),
          configurable: true, enumerable: true
        }});
      }} catch (e) {{ }}
      
      // 18. iframe 检测绕过
      try {{
        Object.defineProperty(window, 'top', {{ get: () => window, configurable: false }});
        Object.defineProperty(window, 'parent', {{ get: () => window, configurable: false }});
        Object.defineProperty(window, 'frameElement', {{ get: () => null, configurable: false }});
      }} catch (e) {{ }}
      
      // 19. ChromeDriver 残留清理
      try {{
        ['cdc_adoQpoasnfa76pfcZLmcfl_Array', 'cdc_adoQpoasnfa76pfcZLmcfl_Promise', 
         '__webdriver_script_fn', '__$webdriverAsyncExecutor'].forEach(key => {{
          if (window[key]) try {{ delete window[key]; }} catch(e) {{}}
        }});
      }} catch (e) {{ }}
      
      // ========== 20. Document 属性 ==========
      try {{
        Object.defineProperty(document, 'hidden', {{ get: () => false, configurable: true }});
        Object.defineProperty(document, 'visibilityState', {{ get: () => 'visible', configurable: true }});
        Object.defineProperty(document, 'webkitHidden', {{ get: () => false, configurable: true }});
        Object.defineProperty(document, 'webkitVisibilityState', {{ get: () => 'visible', configurable: true }});
      }} catch (e) {{ }}
      
      // ========== 21. Function.prototype.toString 保护 ==========
      try {{
        const nativeToStringFunction = Function.prototype.toString;
        Function.prototype.toString = function() {{
          if (this === navigator.webdriver) return 'function webdriver() {{ [native code] }}';
          if (this === Function.prototype.toString) return 'function toString() {{ [native code] }}';
          const funcStr = nativeToStringFunction.call(this);
          if (funcStr.includes('[native code]')) return funcStr;
          return funcStr;
        }};
        log('Function.toString protected');
      }} catch (e) {{ }}
      
      // ========== 22. 时序噪声（防止通过性能检测发现自动化） ==========
      try {{
        const originalNow = performance.now;
        const noise = Math.random() * 10;
        performance.now = function() {{ return originalNow.call(performance) + noise; }};
        
        const originalDateNow = Date.now;
        Date.now = function() {{ return originalDateNow.call(Date) + Math.floor(Math.random() * 50); }};
        
        // 覆盖 console.time 和 console.timeEnd
        const originalTime = console.time;
        const originalTimeEnd = console.timeEnd;
        console.time = function(label) {{ return originalTime.call(console, label); }};
        console.timeEnd = function(label) {{ return originalTimeEnd.call(console, label); }};
        
        log('Timing noise added');
      }} catch (e) {{ }}
      
      // ========== 23. Alert 反检测（模拟人类反应时间） ==========
      try {{
        const originalAlert = window.alert;
        window.alert = function(message) {{
          const alertStartTime = Date.now();
          const result = originalAlert.call(this, message);
          const actualElapsed = Date.now() - alertStartTime;
          
          // 如果实际时间小于 50ms（说明是自动关闭的），模拟人类反应时间
          if (actualElapsed < 50) {{
            const humanDelay = 100 + Math.floor(Math.random() * 400);
            const targetTime = alertStartTime + humanDelay;
            while (Date.now() < targetTime) {{
              // 忙等待模拟人类反应
            }}
          }}
          return result;
        }};
        log('Alert anti-detection enabled');
      }} catch (e) {{ }}
      
      // ========== 24. MediaDevices 伪装 ==========
      try {{
        const fakeMediaDevices = {{
          enumerateDevices: async () => [
            {{ kind: 'audioinput', deviceId: 'default', label: 'Default', groupId: 'default' }},
            {{ kind: 'audiooutput', deviceId: 'default', label: 'Default', groupId: 'default' }},
            {{ kind: 'videoinput', deviceId: 'default', label: 'Default', groupId: 'default' }}
          ],
          getUserMedia: async () => ({{
            getTracks: () => [],
            addEventListener: () => {{}},
            removeEventListener: () => {{}}
          }}),
          getDisplayMedia: async () => ({{
            getTracks: () => [],
            addEventListener: () => {{}},
            removeEventListener: () => {{}}
          }}),
          addEventListener: () => {{}},
          removeEventListener: () => {{}}
        }};
        
        Object.defineProperty(navigator, 'mediaDevices', {{
          get: () => fakeMediaDevices,
          configurable: true,
          enumerable: true
        }});
        log('MediaDevices faked');
      }} catch (e) {{ }}
      
      // ========== 25. 删除可能暴露自动化的属性 ==========
      try {{
        const propsToDelete = ['bluetooth', 'clipboard', 'credentials', 'keyboard', 
                              'mediaCapabilities', 'presentation', 'scheduling', 
                              'storage', 'wakeLock', 'globalPrivacyControl'];
        propsToDelete.forEach(prop => {{
          try {{ 
            if (prop in navigator && navigator[prop] === undefined) {{
              delete navigator[prop]; 
            }}
          }} catch (e) {{}}
        }});
        log('Undefined properties cleaned');
      }} catch (e) {{ }}
      
      // ========== 26. Screen Orientation 伪装 ==========
      try {{
        Object.defineProperty(window.screen, 'orientation', {{
          get: () => ({{
            angle: 0,
            type: 'landscape-primary',
            addEventListener: () => {{}},
            removeEventListener: () => {{}},
            lock: async () => {{}},
            unlock: async () => {{}}
          }}),
          configurable: true
        }});
      }} catch (e) {{ }}
      
      // ========== 27. 覆盖 self 属性 ==========
      try {{
        Object.defineProperty(window, 'self', {{ get: () => window, configurable: false }});
      }} catch (e) {{ }}
      
      log('All stealth scripts injected successfully');
    }})();
  `;
}}

const stealthScript = generateStealthScript(fingerprint);

(async () => {{
  let browser;
  try {{
    browser = await {browser_type}.launch({{
      headless: {headless},
      args: [
        // ========== 核心反检测参数 ==========
        '--disable-blink-features=AutomationControlled',
        '--disable-dev-shm-usage',
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--disable-web-security',
        '--disable-features=IsolateOrigins,site-per-process,SitePerProcess',
        
        // ========== Cloudflare 绕过关键参数 ==========
        '--disable-background-networking',
        '--disable-sync',
        '--safebrowsing-disable-auto-update',
        '--disable-component-update',
        '--disable-features=InterestFeedContentSuggestions',
        '--disable-features=TranslateUI',
        '--disable-features=PasswordManager',
        '--disable-features=AutofillServerCommunication',
        '--disable-features=AutofillAddressSavePrompt',
        '--disable-features=AutofillCreditCardSavePrompt',
        '--disable-features=SecurePaymentConfirmation',
        '--disable-features=PrivacySandboxSettings4',
        '--disable-features=LazyFrameLoading',
        '--disable-features=LazyImageLoading',
        
        // ========== 禁用自动化特征 ==========
        '--disable-gpu',
        '--disable-extensions',
        '--disable-plugins',
        '--disable-default-apps',
        '--no-first-run',
        '--ignore-certificate-errors',
        '--ignore-ssl-errors',
        '--ignore-certificate-errors-spki-list',
        
        // ========== 后台行为优化 ==========
        '--disable-background-timer-throttling',
        '--disable-backgrounding-occluded-windows',
        '--disable-renderer-backgrounding',
        '--disable-ipc-flooding-protection',
        '--disable-hang-monitor',
        '--disable-prompt-on-repost',
        '--disable-popup-blocking',
        
        // ========== 性能和稳定性 ==========
        '--enable-features=NetworkService,NetworkServiceInProcess',
        '--disable-breakpad',
        '--no-crash-upload',
        '--log-level=3',
        '--disable-component-extensions-with-background-pages',
        '--disable-translate',
        '--disable-back-forward-cache',
        '--disable-prerender-back-forward-cache',
        '--disable-client-side-phishing-detection',
        '--disable-password-manager',
        '--enable-zero-copy',
        '--enable-gpu-rasterization',
        '--use-gl=swiftshader',
        
        // ========== 禁用可能暴露自动化的功能 ==========
        '--disable-bundled-ppapi-flash',
        '--disable-plugins-discovery',
        '--disable-flash-3d',
        '--disable-flash-stage3d',
        '--disable-speech-api',
        '--disable-voice-input',
        '--disable-file-system',
        
        // ========== 窗口和显示 ==========
        '--window-size=' + fp.screenSize.width + ',' + fp.screenSize.height,
        '--window-position=' + Math.floor(Math.random() * 100) + ',' + Math.floor(Math.random() * 100),
        '--force-device-scale-factor=' + fp.devicePixelRatio,
        
        // ========== 用户数据隔离 ==========
        '--disable-session-crashed-bubble',
        '--disable-infobars',
        '--disable-features=OptimizationHints,OptimizationHintsFetching,OptimizationTargetPrediction,OptimizationGuideModelDownloading',
      ],
      ignoreDefaultArgs: [
        '--enable-automation', 
        '--enable-blink-features=IdleDetection',
        '--enable-logging',
        '--log-level'
      ],
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
    userAgent: fingerprint.userAgent,
    viewport: fingerprint.screenSize,
    locale: fingerprint.locale,
    timezoneId: fingerprint.timezone,
    permissions: ['geolocation'],
    hasTouch: fingerprint.maxTouchPoints > 0,
    deviceScaleFactor: fingerprint.devicePixelRatio,
    colorScheme: 'light',
  }});
  
  // 设置额外的 HTTP 头（模拟真实浏览器）
  await context.setExtraHTTPHeaders({{
    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7',
    'Accept-Language': fingerprint.locale + ',en-US;q=0.9,en;q=0.8',
    'Accept-Encoding': 'gzip, deflate, br',
    'Cache-Control': 'max-age=0',
    'Sec-Ch-Ua': '"Not.A;Brand";v="8", "Chromium";v="' + (fingerprint.userAgent.match(/Chrome\/(\d+)/)?.[1] || '133') + '", "Google Chrome";v="' + (fingerprint.userAgent.match(/Chrome\/(\d+)/)?.[1] || '133') + '"',
    'Sec-Ch-Ua-Mobile': '?0',
    'Sec-Ch-Ua-Platform': '"' + (fingerprint.platform === 'MacIntel' ? 'macOS' : 'Windows') + '"',
    'Sec-Fetch-Dest': 'document',
    'Sec-Fetch-Mode': 'navigate',
    'Sec-Fetch-Site': 'none',
    'Sec-Fetch-User': '?1',
    'Upgrade-Insecure-Requests': '1',
    'DNT': '1'
  }});
  
  // 在所有页面加载前注入抗检测脚本
  await context.addInitScript(stealthScript);
  console.log('[Recorder] 抗检测脚本已添加到 context');
  
  const page = await context.newPage();
  
  // 使用 addInitScript 在每个页面加载前注入录制脚本
  // 这样可以确保页面导航后脚本仍然有效
  const recordingScript = `
    (function() {{
      // 每次页面加载时重置标志，确保新页面也能注册事件监听器
      window.__RECORDER_INJECTED__ = false;
      
      if (window.__RECORDER_INJECTED__) {{
        return;
      }}
      window.__RECORDER_INJECTED__ = true;
      
      console.log('[Recorder] 正在初始化录制脚本...');
      
      function logAction(actionType, selector, value, description) {{
        const action = {{
          action_type: actionType,
          selector: selector,
          value: value,
          timestamp: Date.now(),
          description: description,
          page_url: window.location.href
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
      
      // 点击事件监听
      document.addEventListener('click', function(e) {{
        const selector = getSelector(e.target);
        console.log('[Recorder Debug] 点击事件:', selector);
        logAction('click', selector, null, '点击 ' + selector);
      }}, false);
      
      {mouse_move_listener}
      
      // 页面滚动事件监听 (节流，每500ms记录一次)
      let lastScroll = 0;
      window.addEventListener('scroll', function(e) {{
        const now = Date.now();
        if (now - lastScroll > 500) {{  // 500ms 节流
          lastScroll = now;
          const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
          const scrollLeft = window.pageXOffset || document.documentElement.scrollLeft;
          const value = JSON.stringify({{ x: scrollLeft, y: scrollTop }});
          console.log('[Recorder Debug] 页面滚动:', value);
          logAction('scroll', 'window', value, '页面滚动到 ' + value);
        }}
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
      
      console.log('[Recorder] 所有事件监听器已注册');
    }})();
  `;
  
  await page.addInitScript(recordingScript);
  console.log('[Recorder] 初始化脚本已添加');
  
  // 监听新标签页创建 - 使用 context.on('page') 更可靠
  context.on('page', async (newPage) => {{
    console.log('[Recorder] 检测到新标签页创建');
    if (newPage) {{
      console.log('[Recorder] 新标签页 URL:', newPage.url());
      
      // 记录新标签页创建事件
      const newPageAction = {{
        action_type: 'newPage',
        selector: null,
        value: newPage.url(),
        timestamp: Date.now(),
        description: '创建新标签页: ' + newPage.url(),
        page_url: newPage.url()
      }};
      console.log('ACTION:' + JSON.stringify(newPageAction));
      
      try {{
        // 先注入抗检测脚本，再注入录制脚本
        await newPage.addInitScript(stealthScript);
        await newPage.addInitScript(recordingScript);
        console.log('[Recorder] 新标签页抗检测和录制脚本已添加');
        
        // 为新标签页添加 console 事件监听
        newPage.on('console', msg => {{
          const text = msg.text();
          console.log('[Recorder Console 新标签页]', text);
          if (text.startsWith('ACTION:')) {{
            console.log(text);
          }}
        }});
        
        // 监听新标签页的导航
        newPage.on('framenavigated', async (frame) => {{
          if (frame === newPage.mainFrame()) {{
            console.log('[Recorder] 新标签页导航至:', frame.url());
            try {{
              await newPage.evaluate(stealthScript);
              await newPage.evaluate(recordingScript);
              console.log('[Recorder] 新标签页抗检测和录制脚本已重新注入');
            }} catch (err) {{
              console.log('[Recorder] 新标签页重新注入失败:', err.message);
            }}
          }}
        }});
        
      }} catch (err) {{
        console.log('[Recorder] 为新标签页添加脚本失败:', err.message);
      }}
    }}
  }});
  
  // 监听 popup 窗口（window.open 方式）
  page.on('popup', async (popupPage) => {{
    console.log('[Recorder] 检测到 popup 窗口');
    if (popupPage) {{
      console.log('[Recorder] popup 窗口 URL:', popupPage.url());
      
      // 记录 popup 窗口创建事件
      const popupAction = {{
        action_type: 'newPage',
        selector: null,
        value: popupPage.url(),
        timestamp: Date.now(),
        description: '打开弹出窗口: ' + popupPage.url(),
        page_url: popupPage.url()
      }};
      console.log('ACTION:' + JSON.stringify(popupAction));
      
      try {{
        // 先注入抗检测脚本，再注入录制脚本
        await popupPage.addInitScript(stealthScript);
        await popupPage.addInitScript(recordingScript);
        console.log('[Recorder] popup 窗口抗检测和录制脚本已添加');
        
        // 为 popup 窗口添加 console 事件监听
        popupPage.on('console', msg => {{
          const text = msg.text();
          console.log('[Recorder Console popup]', text);
          if (text.startsWith('ACTION:')) {{
            console.log(text);
          }}
        }});
        
        // 监听 popup 窗口的导航
        popupPage.on('framenavigated', async (frame) => {{
          if (frame === popupPage.mainFrame()) {{
            console.log('[Recorder] popup 窗口导航至:', frame.url());
            try {{
              await popupPage.evaluate(stealthScript);
              await popupPage.evaluate(recordingScript);
              console.log('[Recorder] popup 窗口抗检测和录制脚本已重新注入');
            }} catch (err) {{
              console.log('[Recorder] popup 窗口重新注入失败:', err.message);
            }}
          }}
        }});
        
      }} catch (err) {{
        console.log('[Recorder] 为 popup 窗口添加脚本失败:', err.message);
      }}
    }}
  }});
  
  // 监听页面导航，在每次导航后重新注入脚本以确保稳定性
  page.on('framenavigated', async (frame) => {{
    if (frame === page.mainFrame()) {{
      console.log('[Recorder Debug] 页面已跳转至:', frame.url());
      // 重新注入抗检测和录制脚本以确保事件监听器在新的 DOM 中生效
      try {{
        await page.evaluate(stealthScript);
        await page.evaluate(recordingScript);
        console.log('[Recorder] 页面导航后抗检测和录制脚本已重新注入');
      }} catch (err) {{
        console.log('[Recorder] 重新注入脚本失败:', err.message);
      }}
    }}
  }});
  
  // ========== 人类行为模拟函数 ==========
  async function humanLikeMouseMove(targetX, targetY) {{
    const currentPos = await page.evaluate(() => ({{
      x: window.lastMouseX || Math.random() * window.innerWidth,
      y: window.lastMouseY || Math.random() * window.innerHeight
    }}));
    
    let startX = currentPos.x;
    let startY = currentPos.y;
    
    // 添加过冲效果
    let overshootRatio = 0.05 + Math.random() * 0.1;
    let targetXWithOvershoot = targetX + (targetX - startX) * overshootRatio;
    let targetYWithOvershoot = targetY + (targetY - startY) * overshootRatio;
    
    // 贝塞尔曲线控制点
    const distance = Math.sqrt((targetXWithOvershoot - startX) ** 2 + (targetYWithOvershoot - startY) ** 2);
    const cpDistance = distance * 0.3;
    const angle1 = Math.atan2(targetYWithOvershoot - startY, targetXWithOvershoot - startX) + Math.PI / 4;
    const angle2 = Math.atan2(targetYWithOvershoot - startY, targetXWithOvershoot - startX) - Math.PI / 4;
    
    const cp1x = startX + Math.cos(angle1) * cpDistance;
    const cp1y = startY + Math.sin(angle1) * cpDistance;
    const cp2x = targetXWithOvershoot - Math.cos(angle2) * cpDistance;
    const cp2y = targetYWithOvershoot - Math.sin(angle2) * cpDistance;
    
    const steps = Math.floor(Math.random() * 25) + 15; // 15-40步
    
    // 速度曲线
    for (let i = 0; i <= steps; i++) {{
      const t = i / steps;
      const easeOut = 1 - Math.pow(1 - t, 3);
      
      // 贝塞尔曲线计算
      const x = startX * Math.pow(1 - t, 3) + 
                cp1x * 3 * Math.pow(1 - t, 2) * t + 
                cp2x * 3 * (1 - t) * Math.pow(t, 2) + 
                targetXWithOvershoot * Math.pow(t, 3);
      const y = startY * Math.pow(1 - t, 3) + 
                cp1y * 3 * Math.pow(1 - t, 2) * t + 
                cp2y * 3 * (1 - t) * Math.pow(t, 2) + 
                targetYWithOvershoot * Math.pow(t, 3);
      
      await page.mouse.move(x, y);
      
      // 根据速度调整延迟
      const delay = Math.floor((Math.random() * 17) + 8) * (1 + Math.sin(t * Math.PI));
      await page.waitForTimeout(delay);
    }}
    
    // 修正过冲
    const correctionSteps = Math.floor(Math.random() * 5) + 3;
    for (let i = 1; i <= correctionSteps; i++) {{
      const t = i / correctionSteps;
      const x = targetXWithOvershoot + (targetX - targetXWithOvershoot) * t;
      const y = targetYWithOvershoot + (targetY - targetYWithOvershoot) * t;
      await page.mouse.move(x, y);
      await page.waitForTimeout(Math.floor(Math.random() * 10) + 10);
    }}
    
    // 保存最后位置
    await page.evaluate((x, y) => {{
      window.lastMouseX = x;
      window.lastMouseY = y;
    }}, targetX, targetY);
    
    // 随机停顿
    if (Math.random() > 0.6) {{
      await page.waitForTimeout(Math.floor(Math.random() * 150) + 50);
    }}
  }}
  
  async function humanLikeClick(selector) {{
    try {{
      const element = await page.$(selector);
      if (!element) return false;
      
      const box = await element.boundingBox();
      if (!box) return false;
      
      // 随机选择点击位置（偏向中心）
      const bias = 0.6;
      const targetX = box.x + box.width * (0.5 + (Math.random() - 0.5) * bias);
      const targetY = box.y + box.height * (0.5 + (Math.random() - 0.5) * bias);
      
      // 先移动到附近
      if (Math.random() > 0.3) {{
        const nearbyX = targetX + (Math.random() - 0.5) * 50;
        const nearbyY = targetY + (Math.random() - 0.5) * 50;
        await humanLikeMouseMove(nearbyX, nearbyY);
        await page.waitForTimeout(Math.floor(Math.random() * 200) + 100);
      }}
      
      // 精确移动到目标
      await humanLikeMouseMove(targetX, targetY);
      
      // 随机停顿后点击
      await page.waitForTimeout(Math.floor(Math.random() * 100) + 50);
      
      // 点击前微小抖动
      if (Math.random() > 0.8) {{
        await page.mouse.move(targetX + (Math.random() - 0.5) * 3, targetY + (Math.random() - 0.5) * 3);
        await page.waitForTimeout(20);
      }}
      
      // 执行点击
      await page.mouse.down();
      await page.waitForTimeout(Math.floor(Math.random() * 70) + 80); // 按下持续时间
      await page.mouse.up();
      
      // 点击后随机停顿
      await page.waitForTimeout(Math.floor(Math.random() * 300) + 200);
      
      return true;
    }} catch (e) {{
      return false;
    }}
  }}
  
  async function humanLikeScroll(direction = 'down', minDistance = 300, maxDistance = 800) {{
    const distance = Math.floor(Math.random() * (maxDistance - minDistance + 1)) + minDistance;
    const actualDistance = direction === 'up' ? -distance : distance;
    const steps = Math.floor(Math.random() * 10) + 5; // 5-15步
    const stepDistance = actualDistance / steps;
    
    for (let i = 0; i < steps; i++) {{
      const progress = i / steps;
      const currentStep = stepDistance * (1 + Math.sin(progress * Math.PI) * 0.3);
      
      await page.evaluate((d) => window.scrollBy({{ top: d, behavior: 'auto' }}), currentStep);
      
      // 滚动间随机停顿（模拟阅读）
      const delay = Math.floor(Math.random() * 100) + 50 + (Math.random() > 0.7 ? Math.floor(Math.random() * 400) + 200 : 0);
      await page.waitForTimeout(delay);
    }}
    
    // 滚动后随机停顿
    await page.waitForTimeout(Math.floor(Math.random() * 700) + 300);
  }}
  
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
    console.log('[Recorder] 页面 DOM 加载完成');
    
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
            url = url,
            timeout = DEFAULT_TIMEOUT_SECS * 1000,
            record_mouse_move = options.record_mouse_move.unwrap_or(false),
            mouse_move_listener = if options.record_mouse_move.unwrap_or(false) {
                r#"// 鼠标移动事件监听 (节流，每500ms记录一次)
      let lastMouseMove = 0;
      document.addEventListener('mousemove', function(e) {{
        const now = Date.now();
        if (now - lastMouseMove > 500) {{  // 500ms 节流
          lastMouseMove = now;
          const selector = getSelector(e.target);
          const value = JSON.stringify({{ x: e.clientX, y: e.clientY }});
          console.log('[Recorder Debug] 鼠标移动:', selector, value);
          logAction('mousemove', selector, value, '鼠标移动到 ' + selector);
        }}
      }}, false);"#
            } else {
                "// 鼠标移动录制已禁用"
            }
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
            "async function visitPage({ page, wallet, api }) {".to_string(),
            "    api.log('info', '开始执行录制脚本');".to_string(),
            "    const context = page.context();".to_string(),
            "".to_string(),
        ];

        let mut last_url: Option<String> = None;
        let mut page_index: usize = 0;
        
        // 用于去重 fill 操作：记录每个选择器最后出现的位置和值
        let mut fill_tracker: std::collections::HashMap<String, (usize, String)> = std::collections::HashMap::new();
        
        // 第一遍遍历：找出每个 fill 选择器最后一次的值
        for (idx, action) in session.actions.iter().enumerate() {
            if action.action_type == "fill" {
                if let (Some(selector), Some(value)) = (&action.selector, &action.value) {
                    fill_tracker.insert(selector.clone(), (idx, value.clone()));
                }
            }
        }
        
        // 第二遍遍历：生成代码，跳过非最后的 fill 操作
        for (idx, action) in session.actions.iter().enumerate() {
            // 对于 fill 操作，检查是否是该选择器最后一次出现
            if action.action_type == "fill" {
                if let (Some(selector), Some(_value)) = (&action.selector, &action.value) {
                    if let Some((last_idx, _)) = fill_tracker.get(selector) {
                        if *last_idx != idx {
                            // 不是最后一次出现，跳过
                            continue;
                        }
                    }
                }
            }
            
            if action.action_type == "navigate" {
                if let Some(url) = &action.value {
                    if last_url.as_ref() != Some(url) {
                        code_lines.push(format!("    await page.goto('{}');", url));
                        last_url = Some(url.clone());
                    }
                }
            } else if action.action_type == "newPage" {
                // 新标签页 - 使用 waitForEvent 等待并切换到新标签页
                if let Some(url) = &action.value {
                    page_index += 1;
                    code_lines.push(format!("    // 等待新标签页 {}", page_index));
                    code_lines.push(format!("    const newPage{} = await context.waitForEvent('page', {{ timeout: 30000 }});", page_index));
                    code_lines.push(format!("    await newPage{}.waitForLoadState('domcontentloaded');", page_index));
                    code_lines.push(format!("    const page{} = newPage{};", page_index, page_index));
                    last_url = Some(url.clone());
                }
            } else if let Some(line) = self.action_to_code(action, page_index) {
                code_lines.push(line);
            }
        }

        code_lines.push("".to_string());
        code_lines.push("    api.log('success', '脚本执行完成');".to_string());
        code_lines.push("    return { success: true };".to_string());
        code_lines.push("}".to_string());

        code_lines.join("\n")
    }
    
    fn action_to_code(&self, action: &RecordedAction, page_index: usize) -> Option<String> {
        let page_var = if page_index > 0 {
            format!("newPage{}", page_index)
        } else {
            "page".to_string()
        };
        
        match action.action_type.as_str() {
            "navigate" => {
                if let Some(url) = &action.value {
                    Some(format!("await {}.goto('{}');", page_var, url))
                } else {
                    None
                }
            }
            "click" => {
                if let Some(selector) = &action.selector {
                    Some(format!("await {}.click('{}');", page_var, selector.replace("'", "\\'")))
                } else {
                    None
                }
            }
            "fill" => {
                if let (Some(selector), Some(value)) = (&action.selector, &action.value) {
                    Some(format!("await {}.fill('{}', '{}');", 
                        page_var,
                        selector.replace("'", "\\'"),
                        value.replace("'", "\\'")))
                } else {
                    None
                }
            }
            "select" => {
                if let (Some(selector), Some(value)) = (&action.selector, &action.value) {
                    Some(format!("await {}.selectOption('{}', '{}');",
                        page_var,
                        selector.replace("'", "\\'"),
                        value.replace("'", "\\'")))
                } else {
                    None
                }
            }
            "hover" => {
                if let Some(selector) = &action.selector {
                    Some(format!("await {}.hover('{}');", page_var, selector.replace("'", "\\'")))
                } else {
                    None
                }
            }
            "screenshot" => {
                Some(format!("await {}.screenshot({{ path: 'screenshot.png' }});", page_var))
            }
            "press" => {
                if let Some(selector) = &action.selector {
                    let key = action.value.as_deref().unwrap_or("Enter");
                    Some(format!("await {}.press('{}', '{}');", page_var, selector.replace("'", "\\'"), key))
                } else {
                    None
                }
            }
            "mousemove" => {
                None // 鼠标移动在回放时通常不需要
            }
            "scroll" => {
                None // 页面滚动在回放时通常不需要，页面内容变化会自动滚动
            }
            _ => None,
        }
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
