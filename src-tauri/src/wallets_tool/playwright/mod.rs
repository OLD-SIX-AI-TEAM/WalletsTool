use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::fs;

pub mod executor;
pub mod recorder;

/// 浏览器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub target_url: Option<String>,
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
            target_url: None,
        }
    }
}

/// 钱包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletInfo {
    pub id: String,
    pub name: String,
    pub address: String,
    pub private_key: String,
    pub chain_type: String,
}

/// 执行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

  // 使用字符串拼接而不是模板字符串，避免嵌套模板字符串的问题
  return '(() => {\n' +
    "  'use strict';\n" +
    '  const DEBUG = false;\n' +
    "  const log = (...args) => { if (DEBUG) console.log('[Stealth]', ...args); };\n" +
    '\n' +
    '  // 1. 多层 webdriver 移除\n' +
    '  try {\n' +
    '    delete navigator.webdriver;\n' +
    "    Object.defineProperty(navigator, 'webdriver', {\n" +
    '      get: () => false,\n' +
    '      configurable: true,\n' +
    '      enumerable: true\n' +
    '    });\n' +
    "    Object.defineProperty(Navigator.prototype, 'webdriver', {\n" +
    '      get: () => false,\n' +
    '      configurable: true,\n' +
    '      enumerable: true\n' +
    '    });\n' +
    "    log('webdriver removed');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 2. Navigator 属性伪装\n' +
    '  try {\n' +
    '    const props = {\n' +
    '      languages: ' + JSON.stringify(languages) + ',\n' +
    '      hardwareConcurrency: ' + hardwareConcurrency + ',\n' +
    '      deviceMemory: ' + deviceMemory + ',\n' +
    "      platform: '" + platform + "',\n" +
    "      vendor: '" + vendor + "',\n" +
    '      maxTouchPoints: 0,\n' +
    "      productSub: '20030107',\n" +
    '      pdfViewerEnabled: true,\n' +
    '      webdriver: false,\n' +
    '    };\n' +
    '\n' +
    '    Object.entries(props).forEach(([key, value]) => {\n' +
    '      try {\n' +
    "        Object.defineProperty(navigator, key, { get: () => value, configurable: true, enumerable: true });\n" +
    '      } catch (e) {}\n' +
    '    });\n' +
    "    log('Navigator properties set');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 3. Canvas 指纹混淆\n' +
    '  try {\n' +
    "    const originalGetContext = HTMLCanvasElement.prototype.getContext;\n" +
    '    HTMLCanvasElement.prototype.getContext = function(type, ...args) {\n' +
    '      const context = originalGetContext.apply(this, [type, ...args]);\n' +
    '      if (!context) return context;\n' +
    '      \n' +
    "      if (type === '2d') {\n" +
    '        const randomOffset = () => (Math.random() - 0.5) * 0.0001;\n' +
    '        \n' +
    "        ['fillText', 'strokeText', 'measureText'].forEach(method => {\n" +
    '          const original = context[method];\n' +
    '          context[method] = function(...textArgs) {\n' +
    '            if (method !== \'measureText\' && Math.random() > 0.2) {\n' +
    '              textArgs[1] = (textArgs[1] || 0) + randomOffset();\n' +
    '              textArgs[2] = (textArgs[2] || 0) + randomOffset();\n' +
    '            }\n' +
    '            return original.apply(this, textArgs);\n' +
    '          };\n' +
    '        });\n' +
    '\n' +
    '        const originalGetImageData = context.getImageData;\n' +
    '        context.getImageData = function(sx, sy, sw, sh) {\n' +
    '          const imageData = originalGetImageData.apply(this, [sx, sy, sw, sh]);\n' +
    '          const noise = Math.floor(Math.random() * 3) - 1;\n' +
    '          for (let i = 0; i < imageData.data.length; i += 4) {\n' +
    '            imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));\n' +
    '            imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));\n' +
    '            imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));\n' +
    '          }\n' +
    '          return imageData;\n' +
    '        };\n' +
    '      }\n' +
    '      return context;\n' +
    '    };\n' +
    "    log('Canvas fingerprinting protection enabled');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 4. WebGL 指纹伪装\n' +
    '  try {\n' +
    '    const overrideWebGL = (WebGLClass) => {\n' +
    '      if (!WebGLClass) return;\n' +
    '      const originalGetParameter = WebGLClass.prototype.getParameter;\n' +
    '      WebGLClass.prototype.getParameter = function(parameter) {\n' +
    "        if (parameter === 37445) return 'Google Inc.';\n" +
    "        if (parameter === 37446) return 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti)';\n" +
    '        return originalGetParameter.apply(this, arguments);\n' +
    '      };\n' +
    '      const originalGetExtension = WebGLClass.prototype.getExtension;\n' +
    '      WebGLClass.prototype.getExtension = function(name) {\n' +
    "        if (name === 'WEBGL_debug_renderer_info') return null;\n" +
    '        return originalGetExtension.apply(this, [name]);\n' +
    '      };\n' +
    '    };\n' +
    '    overrideWebGL(WebGLRenderingContext);\n' +
    '    overrideWebGL(WebGL2RenderingContext);\n' +
    "    log('WebGL fingerprinting protection enabled');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 5. Chrome 对象伪装\n' +
    '  try {\n' +
    '    window.navigator.chrome = {\n' +
    '      app: {\n' +
    '        isInstalled: false,\n' +
    '        getDetails: () => null,\n' +
    '        getIsInstalled: () => false\n' +
    '      },\n' +
    '      runtime: {\n' +
    "        OnInstalledReason: { CHROME_UPDATE: 'chrome_update', INSTALL: 'install' },\n" +
    "        PlatformOs: { WIN: 'win', MAC: 'mac', LINUX: 'linux' },\n" +
    '      },\n' +
    '      csi: () => ({}),\n' +
    '      loadTimes: () => ({\n' +
    '        commitLoadTime: Date.now() / 1000 - Math.random() * 2,\n' +
    "        connectionInfo: 'http/1.1',\n" +
    '        firstPaintTime: Date.now() / 1000 - Math.random(),\n' +
    '      })\n' +
    '    };\n' +
    "    log('Chrome object faked');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 6. Plugins 伪装\n' +
    '  try {\n' +
    '    const plugins = Object.create(PluginArray.prototype);\n' +
    "    Object.defineProperty(plugins, 'length', { get: () => 3, enumerable: true });\n" +
    '    plugins.item = function(i) { return null; };\n' +
    '    plugins.namedItem = function(name) { return null; };\n' +
    '    plugins.refresh = () => {};\n' +
    "    Object.defineProperty(navigator, 'plugins', { get: () => plugins, configurable: true, enumerable: true });\n" +
    "    log('Plugins faked');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 7. 窗口尺寸伪装\n' +
    '  try {\n' +
    '    Object.defineProperty(window, \'outerWidth\', { \n' +
    '      get: () => window.innerWidth + 16, \n' +
    '      configurable: true \n' +
    '    });\n' +
    '    Object.defineProperty(window, \'outerHeight\', { \n' +
    '      get: () => window.innerHeight + 85, \n' +
    '      configurable: true \n' +
    '    });\n' +
    '    Object.defineProperty(window, \'devicePixelRatio\', { \n' +
    '      get: () => ' + fingerprint.devicePixelRatio + ', \n' +
    '      configurable: true \n' +
    '    });\n' +
    "    log('Window dimensions faked');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    '  // 8. Screen 对象\n' +
    '  try {\n' +
    '    Object.defineProperty(window.screen, \'pixelDepth\', { get: () => ' + colorDepth + ', configurable: true });\n' +
    '    Object.defineProperty(window.screen, \'colorDepth\', { get: () => ' + colorDepth + ', configurable: true });\n' +
    '    Object.defineProperty(window.screen, \'width\', { get: () => ' + fingerprint.screenSize.width + ', configurable: true });\n' +
    '    Object.defineProperty(window.screen, \'height\', { get: () => ' + fingerprint.screenSize.height + ', configurable: true });\n' +
    "    log('Screen properties set');\n" +
    '  } catch (e) { }\n' +
    '\n' +
    "  log('All stealth scripts injected successfully');\n" +
    '})();';
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
        `--window-size=${this.fingerprint.screenSize.width},${this.fingerprint.screenSize.height}`,
      ],
      ignoreDefaultArgs: ['--enable-automation', '--enable-blink-features=IdleDetection']
    };

    // 代理配置
    if (CONFIG.proxy_type !== 'direct' && CONFIG.proxy_host) {
      const proxyUrl = CONFIG.proxy_username 
        ? `http://${CONFIG.proxy_username}:${CONFIG.proxy_password}@${CONFIG.proxy_host}:${CONFIG.proxy_port}`
        : `http://${CONFIG.proxy_host}:${CONFIG.proxy_port}`;
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
  
  console.log(`[Browser ${browserIndex}] Starting for wallet: ${wallet.name}`);

  const manager = new BrowserManager(browserIndex);
  await manager.createBrowser();

  try {
    for (let i = 1; i <= visitCount; i++) {
      // 构建 context 对象，添加调试信息
      const context = {
        manager,
        url: CONFIG.targetUrl,
        visitIndex: i,
        totalVisits: visitCount,
        wallet: wallet,
        api: {
          log: (level, message) => console.log(`[${level.toUpperCase()}] ${message}`),
          randomDelay,
          sleep,
          humanLikeClick,
          humanLikeScroll,
          humanLikeMouseMove,
        }
      };
      
      // 调试：打印 context 对象结构
      console.log(`[Browser ${browserIndex}] Context keys:`, Object.keys(context));
      console.log(`[Browser ${browserIndex}] api defined:`, !!context.api);
      console.log(`[Browser ${browserIndex}] api.log defined:`, !!(context.api && context.api.log));

      try {
        // 检查 visitPage 函数是否存在
        if (typeof visitPage !== 'function') {
          throw new Error('visitPage function is not defined. Please define visitPage function in your script.');
        }
        
        // 调试：在传入前再次确认 context.api 存在
        if (!context.api) {
          console.error(`[Browser ${browserIndex}] ERROR: context.api is undefined before calling visitPage`);
          console.error(`[Browser ${browserIndex}] context keys:`, Object.keys(context));
          throw new Error('context.api is undefined');
        }
        if (typeof context.api.log !== 'function') {
          console.error(`[Browser ${browserIndex}] ERROR: context.api.log is not a function`);
          console.error(`[Browser ${browserIndex}] context.api:`, context.api);
          throw new Error('context.api.log is not a function');
        }
        
        const result = await visitPage(context);
        if (result && result.success) {
          successCount++;
        } else {
          failCount++;
          console.error(`[Browser ${browserIndex}] visitPage returned non-success result:`, result);
        }
      } catch (error) {
        console.error(`[Browser ${browserIndex}] visitPage error:`, error.message);
        console.error(`[Browser ${browserIndex}] Error stack:`, error.stack);
        failCount++;
      }

      if (i < visitCount) {
        await sleep(randomDelay(5000, 10000));
      }
    }
  } catch (error) {
    console.error(`[Browser ${browserIndex}] Error:`, error);
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
  
  console.log(`Target URL: ${CONFIG.targetUrl}`);
  console.log(`Wallets: ${WALLETS.length}`);
  console.log(`Concurrency: ${concurrency}`);
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
  console.log(`成功: ${totalSuccess}, 失败: ${totalFail}`);
  console.log('========================================');
  
  // 如果有失败的任务，返回非零退出码
  if (totalFail > 0) {
    console.error(`执行失败: ${totalFail} 个任务执行失败`);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error('执行出错:', error);
  process.exit(1);
});
"#;

/// 构建完整脚本（用于 ExecutionParams）
pub fn build_script(params: &ExecutionParams) -> String {
    // 克隆 config 并设置 target_url
    let mut browser_config = params.config.clone();
    browser_config.target_url = Some(params.target_url.clone());
    
    let config_json = serde_json::to_string(&browser_config).unwrap_or_default();
    let wallets_json = serde_json::to_string(&params.wallets).unwrap_or_default();
    
    SCRIPT_TEMPLATE
        .replace("{{CONFIG}}", &config_json)
        .replace("{{WALLETS}}", &wallets_json)
        .replace("{{USER_CODE}}", &params.user_code)
}

/// 构建完整脚本（用于 ExecutionConfig）
pub fn build_script_from_config(config: &ExecutionConfig, wallets: Vec<WalletInfo>) -> Result<String, String> {
    // 克隆 config.config 并设置 target_url
    let mut browser_config = config.config.clone();
    browser_config.target_url = Some(config.target_url.clone());
    
    let config_json = serde_json::to_string(&browser_config)
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
