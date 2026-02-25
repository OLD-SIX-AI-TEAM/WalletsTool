use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::fs;

pub mod executor;
pub mod recorder;

/// 浏览器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub user_agent: String,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub device_scale_factor: f64,
    pub locale: String,
    pub timezone_id: String,
    pub proxy_type: String,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub canvas_spoof: bool,
    pub webgl_spoof: bool,
    pub audio_spoof: bool,
    pub timezone_spoof: bool,
    pub geolocation_spoof: bool,
    pub font_spoof: bool,
    pub webrtc_spoof: bool,
    pub navigator_override: bool,
    pub webdriver_override: bool,
    pub headless: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            viewport_width: 1920,
            viewport_height: 1080,
            device_scale_factor: 1.0,
            locale: "en-US".to_string(),
            timezone_id: "America/New_York".to_string(),
            proxy_type: "direct".to_string(),
            proxy_host: None,
            proxy_port: None,
            proxy_username: None,
            proxy_password: None,
            canvas_spoof: true,
            webgl_spoof: true,
            audio_spoof: true,
            timezone_spoof: true,
            geolocation_spoof: true,
            font_spoof: true,
            webrtc_spoof: true,
            navigator_override: true,
            webdriver_override: true,
            headless: false,
        }
    }
}

/// 钱包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub id: String,
    pub name: String,
    pub address: String,
    pub private_key: String,
    pub chain_type: String,
}

/// 执行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParams {
    pub target_url: String,
    pub config: BrowserConfig,
    pub user_code: String,
    pub wallets: Vec<WalletInfo>,
    pub concurrency: i32,
    pub timeout: i32,
}

/// 执行配置（用于 executor 模块）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub target_url: String,
    pub config: BrowserConfig,
    pub user_code: String,
    pub wallets: Vec<WalletInfo>,
    pub concurrency: Option<usize>,
    pub timeout_secs: Option<u64>,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// 脚本模板
const SCRIPT_TEMPLATE: &str = r#"
const CONFIG = {{CONFIG}};
const WALLETS = {{WALLETS}};

// ==================== 工具函数 ====================
function randomDelay(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function randomChoice(array) {
  return array[Math.floor(Math.random() * array.length)];
}

// ==================== 指纹配置池 ====================
const USER_AGENTS = [
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36',
];

const SCREEN_SIZES = [
  { width: 1920, height: 1080 },
  { width: 2560, height: 1440 },
  { width: 1366, height: 768 },
  { width: 1440, height: 900 },
];

const TIMEZONES = [
  'Asia/Shanghai', 'Asia/Tokyo', 'Asia/Singapore',
  'America/New_York', 'America/Los_Angeles', 'America/Chicago',
  'Europe/London', 'Europe/Paris', 'Europe/Berlin',
];

const LOCALES = ['zh-CN', 'zh-TW', 'en-US', 'en-GB', 'ja-JP', 'ko-KR', 'de-DE', 'fr-FR'];

const HARDWARE_CONCURRENCY_OPTIONS = [2, 4, 6, 8, 12, 16];
const DEVICE_MEMORY_OPTIONS = [2, 4, 6, 8, 12, 16];

// ==================== 指纹生成器 ====================
function generateFingerprint() {
  const userAgent = CONFIG.user_agent || randomChoice(USER_AGENTS);
  const screenSize = {
    width: CONFIG.viewport_width || 1920,
    height: CONFIG.viewport_height || 1080
  };
  const platform = userAgent.includes('Macintosh') ? 'MacIntel' :
                   userAgent.includes('Linux') ? 'Linux x86_64' : 'Win32';
  const locale = CONFIG.locale || randomChoice(LOCALES);
  const timezone = CONFIG.timezone_id || randomChoice(TIMEZONES);

  return {
    userAgent,
    screenSize,
    platform,
    locale,
    timezone,
    hardwareConcurrency: randomChoice(HARDWARE_CONCURRENCY_OPTIONS),
    deviceMemory: randomChoice(DEVICE_MEMORY_OPTIONS),
    languages: [locale, 'en-US', 'en'],
    vendor: 'Google Inc.',
    colorScheme: 'light',
    devicePixelRatio: CONFIG.device_scale_factor || 1,
    colorDepth: 24,
    touchConfig: { maxTouchPoints: 0, hasTouch: false },
    screenOrientation: { angle: 0, type: 'landscape-primary' },
  };
}

// ==================== 反检测脚本生成器 ====================
function generateUltimateStealthScript(fingerprint) {
  const { hardwareConcurrency, deviceMemory, platform, languages, vendor, colorDepth } = fingerprint;

  return \`
    (() => {
      'use strict';
      const DEBUG = false;
      const log = (...args) => { if (DEBUG) console.log('[Stealth]', ...args); };

      // 1. 多层 webdriver 移除
      try {
        delete navigator.webdriver;
        Object.defineProperty(navigator, 'webdriver', {
          get: () => false,
          configurable: true,
          enumerable: true
        });
        Object.defineProperty(Navigator.prototype, 'webdriver', {
          get: () => false,
          configurable: true,
          enumerable: true
        });
        log('webdriver removed');
      } catch (e) { }

      // 2. Navigator 属性伪装
      try {
        const props = {
          languages: \${JSON.stringify(languages)},
          hardwareConcurrency: \${hardwareConcurrency},
          deviceMemory: \${deviceMemory},
          platform: '\${platform}',
          vendor: '\${vendor}',
          maxTouchPoints: 0,
          productSub: '20030107',
          pdfViewerEnabled: true,
          webdriver: false,
        };

        Object.entries(props).forEach(([key, value]) => {
          try {
            Object.defineProperty(navigator, key, { get: () => value, configurable: true, enumerable: true });
          } catch (e) {}
        });
        log('Navigator properties set');
      } catch (e) { }

      // 3. Canvas 指纹混淆
      try {
        const originalGetContext = HTMLCanvasElement.prototype.getContext;
        HTMLCanvasElement.prototype.getContext = function(type, ...args) {
          const context = originalGetContext.apply(this, [type, ...args]);
          if (!context) return context;
          
          if (type === '2d') {
            const randomOffset = () => (Math.random() - 0.5) * 0.0001;
            
            ['fillText', 'strokeText', 'measureText'].forEach(method => {
              const original = context[method];
              context[method] = function(...textArgs) {
                if (method !== 'measureText' && Math.random() > 0.2) {
                  textArgs[1] = (textArgs[1] || 0) + randomOffset();
                  textArgs[2] = (textArgs[2] || 0) + randomOffset();
                }
                return original.apply(this, textArgs);
              };
            });

            const originalGetImageData = context.getImageData;
            context.getImageData = function(sx, sy, sw, sh) {
              const imageData = originalGetImageData.apply(this, [sx, sy, sw, sh]);
              const noise = Math.floor(Math.random() * 3) - 1;
              for (let i = 0; i < imageData.data.length; i += 4) {
                imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));
                imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));
                imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));
              }
              return imageData;
            };
          }
          return context;
        };
        log('Canvas fingerprinting protection enabled');
      } catch (e) { }

      // 4. WebGL 指纹伪装
      try {
        const overrideWebGL = (WebGLClass) => {
          if (!WebGLClass) return;
          const originalGetParameter = WebGLClass.prototype.getParameter;
          WebGLClass.prototype.getParameter = function(parameter) {
            if (parameter === 37445) return 'Google Inc.';
            if (parameter === 37446) return 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti)';
            return originalGetParameter.apply(this, arguments);
          };
          const originalGetExtension = WebGLClass.prototype.getExtension;
          WebGLClass.prototype.getExtension = function(name) {
            if (name === 'WEBGL_debug_renderer_info') return null;
            return originalGetExtension.apply(this, [name]);
          };
        };
        overrideWebGL(WebGLRenderingContext);
        overrideWebGL(WebGL2RenderingContext);
        log('WebGL fingerprinting protection enabled');
      } catch (e) { }

      // 5. Chrome 对象伪装
      try {
        window.navigator.chrome = {
          app: {
            isInstalled: false,
            getDetails: () => null,
            getIsInstalled: () => false
          },
          runtime: {
            OnInstalledReason: { CHROME_UPDATE: 'chrome_update', INSTALL: 'install' },
            PlatformOs: { WIN: 'win', MAC: 'mac', LINUX: 'linux' },
          },
          csi: () => ({}),
          loadTimes: () => ({
            commitLoadTime: Date.now() / 1000 - Math.random() * 2,
            connectionInfo: 'http/1.1',
            firstPaintTime: Date.now() / 1000 - Math.random(),
          })
        };
        log('Chrome object faked');
      } catch (e) { }

      // 6. Plugins 伪装
      try {
        const plugins = Object.create(PluginArray.prototype);
        Object.defineProperty(plugins, 'length', { get: () => 3, enumerable: true });
        plugins.item = function(i) { return null; };
        plugins.namedItem = function(name) { return null; };
        plugins.refresh = () => {};
        Object.defineProperty(navigator, 'plugins', { get: () => plugins, configurable: true, enumerable: true });
        log('Plugins faked');
      } catch (e) { }

      // 7. 窗口尺寸伪装
      try {
        Object.defineProperty(window, 'outerWidth', { 
          get: () => window.innerWidth + 16, 
          configurable: true 
        });
        Object.defineProperty(window, 'outerHeight', { 
          get: () => window.innerHeight + 85, 
          configurable: true 
        });
        Object.defineProperty(window, 'devicePixelRatio', { 
          get: () => \${fingerprint.devicePixelRatio}, 
          configurable: true 
        });
        log('Window dimensions faked');
      } catch (e) { }

      // 8. Screen 对象
      try {
        Object.defineProperty(window.screen, 'pixelDepth', { get: () => \${colorDepth}, configurable: true });
        Object.defineProperty(window.screen, 'colorDepth', { get: () => \${colorDepth}, configurable: true });
        Object.defineProperty(window.screen, 'width', { get: () => \${fingerprint.screenSize.width}, configurable: true });
        Object.defineProperty(window.screen, 'height', { get: () => \${fingerprint.screenSize.height}, configurable: true });
        log('Screen properties set');
      } catch (e) { }

      log('All stealth scripts injected successfully');
    })();
  \`;
}

// ==================== 行为模拟 ====================
async function humanLikeMouseMove(page, endX, endY, options = {}) {
  const { minSteps = 15, maxSteps = 40, minDelay = 8, maxDelay = 25 } = options;
  
  const currentPos = await page.evaluate(() => ({
    x: window.lastMouseX || Math.random() * window.innerWidth,
    y: window.lastMouseY || Math.random() * window.innerHeight
  }));
  
  let startX = currentPos.x;
  let startY = currentPos.y;
  
  const steps = Math.floor(Math.random() * (maxSteps - minSteps + 1)) + minSteps;
  
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const x = startX + (endX - startX) * t;
    const y = startY + (endY - startY) * t;
    
    await page.mouse.move(x, y);
    await sleep(randomDelay(minDelay, maxDelay));
  }
  
  await page.evaluate((x, y) => {
    window.lastMouseX = x;
    window.lastMouseY = y;
  }, endX, endY);
}

async function humanLikeClick(page, selector, options = {}) {
  try {
    const element = await page.$(selector);
    if (!element) return false;
    
    const box = await element.boundingBox();
    if (!box) return false;
    
    const targetX = box.x + box.width * (0.5 + (Math.random() - 0.5) * 0.6);
    const targetY = box.y + box.height * (0.5 + (Math.random() - 0.5) * 0.6);
    
    await humanLikeMouseMove(page, targetX, targetY);
    await sleep(randomDelay(50, 150));
    
    await page.mouse.down();
    await sleep(randomDelay(80, 150));
    await page.mouse.up();
    
    await sleep(randomDelay(200, 500));
    return true;
  } catch (e) {
    return false;
  }
}

async function humanLikeScroll(page, options = {}) {
  const { direction = 'down', minDistance = 300, maxDistance = 800, minSteps = 5, maxSteps = 15 } = options;
  
  const distance = randomDelay(minDistance, maxDistance) * (direction === 'up' ? -1 : 1);
  const steps = Math.floor(Math.random() * (maxSteps - minSteps + 1)) + minSteps;
  const stepDistance = distance / steps;
  
  for (let i = 0; i < steps; i++) {
    await page.evaluate((d) => window.scrollBy({ top: d, behavior: 'auto' }), stepDistance);
    await sleep(randomDelay(50, 150));
  }
  
  await sleep(randomDelay(300, 1000));
}

// ==================== 浏览器管理 ====================
class BrowserManager {
  constructor(browserIndex) {
    this.browserIndex = browserIndex;
    this.browser = null;
    this.context = null;
    this.page = null;
    this.fingerprint = null;
  }

  async createBrowser() {
    const { chromium } = require('playwright');
    this.fingerprint = generateFingerprint();

    const launchOptions = {
      headless: CONFIG.headless,
      args: [
        '--disable-blink-features=AutomationControlled',
        '--disable-dev-shm-usage',
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--disable-web-security',
        '--disable-features=IsolateOrigins,site-per-process',
        '--disable-gpu',
        '--disable-extensions',
        '--no-first-run',
        '--ignore-certificate-errors',
        \`--window-size=\${this.fingerprint.screenSize.width},\${this.fingerprint.screenSize.height}\`,
      ],
      ignoreDefaultArgs: ['--enable-automation', '--enable-blink-features=IdleDetection']
    };

    // 代理配置
    if (CONFIG.proxy_type !== 'direct' && CONFIG.proxy_host) {
      const proxyUrl = CONFIG.proxy_username 
        ? \`http://\${CONFIG.proxy_username}:\${CONFIG.proxy_password}@\${CONFIG.proxy_host}:\${CONFIG.proxy_port}\`
        : \`http://\${CONFIG.proxy_host}:\${CONFIG.proxy_port}\`;
      launchOptions.proxy = { server: proxyUrl };
    }

    this.browser = await chromium.launch(launchOptions);

    this.context = await this.browser.newContext({
      userAgent: this.fingerprint.userAgent,
      viewport: this.fingerprint.screenSize,
      locale: this.fingerprint.locale,
      timezoneId: this.fingerprint.timezone,
      deviceScaleFactor: this.fingerprint.devicePixelRatio,
    });

    await this.context.addInitScript(generateUltimateStealthScript(this.fingerprint));
    this.page = await this.context.newPage();

    return { browser: this.browser, page: this.page };
  }

  async closeBrowser() {
    if (this.browser) {
      await this.browser.close();
      this.browser = null;
      this.context = null;
      this.page = null;
    }
  }
}

// ==================== 用户自定义代码 ====================
{{USER_CODE}}

// ==================== 主程序 ====================
async function runBrowserInstance(browserIndex, visitCount, wallet) {
  let successCount = 0;
  let failCount = 0;
  
  console.log(`[Browser \${browserIndex}] Starting for wallet: \${wallet.name}`);

  const manager = new BrowserManager(browserIndex);
  await manager.createBrowser();

  try {
    for (let i = 1; i <= visitCount; i++) {
      const context = {
        manager,
        url: CONFIG.target_url,
        visitIndex: i,
        totalVisits: visitCount,
        wallet: wallet,
        api: {
          log: (level, message) => console.log(`[\${level.toUpperCase()}] \${message}`),
          randomDelay,
          sleep,
          humanLikeClick,
          humanLikeScroll,
          humanLikeMouseMove,
        }
      };

      try {
        const result = await visitPage(context);
        if (result && result.success) {
          successCount++;
        } else {
          failCount++;
        }
      } catch (error) {
        console.error(`[Browser \${browserIndex}] visitPage error:`, error.message);
        failCount++;
      }

      if (i < visitCount) {
        await sleep(randomDelay(5000, 10000));
      }
    }
  } catch (error) {
    console.error(`[Browser \${browserIndex}] Error:`, error);
  } finally {
    await manager.closeBrowser();
  }

  return { successCount, failCount };
}

async function main() {
  console.log('========================================');
  console.log('自动化脚本执行器');
  console.log('========================================');
  
  const concurrency = Math.min(CONFIG.concurrency || 1, WALLETS.length);
  
  console.log(`Target URL: \${CONFIG.target_url}`);
  console.log(`Wallets: \${WALLETS.length}`);
  console.log(`Concurrency: \${concurrency}`);
  console.log('========================================\n');

  const promises = [];
  for (let i = 0; i < WALLETS.length; i++) {
    const wallet = WALLETS[i];
    promises.push(runBrowserInstance(i + 1, 1, wallet));
  }

  const results = await Promise.all(promises);
  
  const totalSuccess = results.reduce((sum, r) => sum + r.successCount, 0);
  const totalFail = results.reduce((sum, r) => sum + r.failCount, 0);

  console.log('\n========================================');
  console.log('执行完成');
  console.log(`成功: \${totalSuccess}, 失败: \${totalFail}`);
  console.log('========================================');
}

main().catch(console.error);
"#;

/// 构建完整脚本（用于 ExecutionParams）
pub fn build_script(params: &ExecutionParams) -> String {
    let config_json = serde_json::to_string(&params.config).unwrap_or_default();
    let wallets_json = serde_json::to_string(&params.wallets).unwrap_or_default();
    
    SCRIPT_TEMPLATE
        .replace("{{CONFIG}}", &config_json)
        .replace("{{WALLETS}}", &wallets_json)
        .replace("{{USER_CODE}}", &params.user_code)
}

/// 构建完整脚本（用于 ExecutionConfig）
pub fn build_script_from_config(config: &ExecutionConfig, wallets: Vec<WalletInfo>) -> Result<String, String> {
    let config_json = serde_json::to_string(&config.config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    let wallets_json = serde_json::to_string(&wallets)
        .map_err(|e| format!("序列化钱包失败: {}", e))?;
    
    Ok(SCRIPT_TEMPLATE
        .replace("{{CONFIG}}", &config_json)
        .replace("{{WALLETS}}", &wallets_json)
        .replace("{{USER_CODE}}", &config.user_code))
}

/// 执行 Playwright 脚本
pub async fn execute_script(params: ExecutionParams) -> ExecutionResult {
    let start_time = std::time::Instant::now();
    
    // 1. 构建脚本
    let script = build_script(&params);
    
    // 2. 创建临时文件
    let temp_dir = std::env::temp_dir();
    let script_file_name = format!("playwright_script_{}.js", uuid::Uuid::new_v4());
    let script_path = temp_dir.join(&script_file_name);
    
    if let Err(e) = fs::write(&script_path, script) {
        return ExecutionResult {
            success: false,
            output: String::new(),
            error: Some(format!("写入脚本失败: {}", e)),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        };
    }
    
    // 3. 执行 Node.js 脚本
    let output_result = Command::new("node")
        .arg(&script_path)
        .current_dir(&temp_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    
    // 4. 清理临时文件
    let _ = fs::remove_file(&script_path);
    
    // 5. 处理结果
    match output_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if output.status.success() {
                ExecutionResult {
                    success: true,
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                }
            } else {
                ExecutionResult {
                    success: false,
                    output: stdout,
                    error: Some(format!("执行错误: {}", stderr)),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
        }
        Err(e) => {
            ExecutionResult {
                success: false,
                output: String::new(),
                error: Some(format!("启动执行失败: {}", e)),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            }
        }
    }
}

#[tauri::command]
pub async fn execute_playwright_script(params: ExecutionParams) -> Result<ExecutionResult, String> {
    Ok(execute_script(params).await)
}
