import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';

export interface Script {
  id: number;
  name: string;
  content: string;
}

export interface Wallet {
  id: number;
  address: string;
  label: string;
  group: string;
  proxy: string;
}

export interface BrowserProfile {
  id: number;
  name: string;
  userAgent: string;
  viewport: string;
  proxy: string;
  canvasSpoof: boolean;
}

export interface ExecutionTask {
  id: string;
  wallet: Wallet;
  script: Script;
  profile: BrowserProfile;
  status: 'pending' | 'running' | 'success' | 'failed' | 'stopped';
  startTime?: number;
  endTime?: number;
  duration?: number;
  logs: string[];
  error?: string;
  txHash?: string;
}

export interface ExecutionConfig {
  headless: boolean;
  timeout: number;
  retryCount: number;
  concurrentLimit: number;
  enableProxy: boolean;
  enableFingerprint: boolean;
}

const STORAGE_KEYS = {
  scripts: 'browser_scripts',
  profiles: 'browser_profiles',
  executionHistory: 'browser_execution_history',
};

// 脚本管理
export const scriptService = {
  loadScripts(): Script[] {
    try {
      const saved = localStorage.getItem(STORAGE_KEYS.scripts);
      if (saved) {
        return JSON.parse(saved);
      }
    } catch (e) {
      console.error('Failed to load scripts:', e);
    }
    return [
      {
        id: 1,
        name: 'OKX Daily Claim',
        content: `// OKX Daily Claim Script
async function run({ page, wallet, api }) {
    api.log('info', '开始执行 OKX Daily Claim');
    
    // 1. 打开OKX官网
    await page.goto('https://www.okx.com');
    await api.waitForSelector('body');
    await api.randomDelay(2000, 4000);
    
    api.log('info', '页面加载完成');
    
    // 2. 连接钱包
    api.log('info', '连接 OKX Wallet...');
    // await api.connectOKXWallet({ chainId: '0x1' });
    
    api.log('success', '脚本执行完成');
    return { success: true };
}`,
      },
      {
        id: 2,
        name: 'Uniswap Swap',
        content: `// Uniswap V3 Swap Script
async function run({ page, wallet, api }) {
    api.log('info', '开始执行 Uniswap Swap');
    
    // 1. 连接钱包
    // await api.connectMetaMask({ expectedChainId: '0x1' });
    
    // 2. 打开Uniswap
    await page.goto('https://app.uniswap.org');
    await api.waitForSelector('body');
    await api.randomDelay(2000, 3000);
    
    api.log('info', 'Uniswap页面加载完成');
    
    api.log('success', '脚本执行完成');
    return { success: true };
}`,
      },
    ];
  },

  saveScripts(scripts: Script[]) {
    try {
      localStorage.setItem(STORAGE_KEYS.scripts, JSON.stringify(scripts));
    } catch (e) {
      console.error('Failed to save scripts:', e);
    }
  },

  async exportScript(script: Script) {
    try {
      const savePath = await save({
        filters: [{ name: 'JavaScript', extensions: ['js'] }],
        defaultPath: `${script.name.replace(/\s+/g, '_')}.js`,
      });

      if (savePath) {
        const content = new TextEncoder().encode(script.content);
        await writeFile(savePath, content);
        return true;
      }
    } catch (error) {
      console.error('Export script error:', error);
    }
    return false;
  },

  async importScript(): Promise<Script | null> {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'JavaScript', extensions: ['js'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });

      if (!selected) return null;

      const content = await readFile(selected);
      const decoder = new TextDecoder();
      const scriptContent = decoder.decode(content);

      // 从文件名提取脚本名称
      const fileName = selected.split(/[\\/]/).pop() || 'Imported Script';
      const name = fileName.replace(/\.js$/i, '').replace(/_/g, ' ');

      return {
        id: Date.now(),
        name,
        content: scriptContent,
      };
    } catch (error) {
      console.error('Import script error:', error);
      return null;
    }
  },
};

// 浏览器配置管理
export const profileService = {
  loadProfiles(): BrowserProfile[] {
    try {
      const saved = localStorage.getItem(STORAGE_KEYS.profiles);
      if (saved) {
        return JSON.parse(saved);
      }
    } catch (e) {
      console.error('Failed to load profiles:', e);
    }
    return [
      {
        id: 1,
        name: 'Default Profile',
        userAgent:
          'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        viewport: '1920x1080',
        proxy: 'Direct',
        canvasSpoof: true,
      },
      {
        id: 2,
        name: 'Mobile Profile',
        userAgent:
          'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
        viewport: '390x844',
        proxy: 'Direct',
        canvasSpoof: true,
      },
    ];
  },

  saveProfiles(profiles: BrowserProfile[]) {
    try {
      localStorage.setItem(STORAGE_KEYS.profiles, JSON.stringify(profiles));
    } catch (e) {
      console.error('Failed to save profiles:', e);
    }
  },
};

// 执行历史管理
export const historyService = {
  loadHistory(): ExecutionTask[] {
    try {
      const saved = localStorage.getItem(STORAGE_KEYS.executionHistory);
      if (saved) {
        return JSON.parse(saved);
      }
    } catch (e) {
      console.error('Failed to load history:', e);
    }
    return [];
  },

  saveHistory(history: ExecutionTask[]) {
    try {
      // 只保留最近100条记录
      const trimmed = history.slice(-100);
      localStorage.setItem(STORAGE_KEYS.executionHistory, JSON.stringify(trimmed));
    } catch (e) {
      console.error('Failed to save history:', e);
    }
  },

  addRecord(record: ExecutionTask) {
    const history = this.loadHistory();
    history.push(record);
    this.saveHistory(history);
  },

  clearHistory() {
    localStorage.removeItem(STORAGE_KEYS.executionHistory);
  },
};

// Playwright执行器
class PlaywrightExecutor {
  private abortController: AbortController | null = null;
  private isRunning = false;

  async execute(
    task: ExecutionTask,
    onLog: (log: string) => void,
    config: ExecutionConfig
  ): Promise<{ success: boolean; error?: string; txHash?: string }> {
    this.isRunning = true;
    this.abortController = new AbortController();

    try {
      // 创建临时脚本文件
      const scriptContent = this.generateRunnerScript(task, config);

      // 使用Tauri的shell插件执行Node.js脚本
      // 注意：这里需要Node.js环境和Playwright安装
      onLog(`[${new Date().toLocaleTimeString()}] 启动浏览器...`);

      // 模拟执行过程（实际实现需要后端支持）
      await this.simulateExecution(task, onLog, config);

      return { success: true };
    } catch (error: any) {
      return { success: false, error: error.message };
    } finally {
      this.isRunning = false;
      this.abortController = null;
    }
  }

  private generateRunnerScript(task: ExecutionTask, config: ExecutionConfig): string {
    return `
const { chromium } = require('playwright');

async function run() {
  const browser = await chromium.launch({
    headless: ${config.headless},
    args: ['--disable-blink-features=AutomationControlled']
  });
  
  const context = await browser.newContext({
    userAgent: '${task.profile.userAgent}',
    viewport: { width: 1280, height: 800 }
  });
  
  const page = await context.newPage();
  
  // User script
  ${task.script.content}
  
  await browser.close();
}

run().catch(console.error);
    `.trim();
  }

  private async simulateExecution(
    task: ExecutionTask,
    onLog: (log: string) => void,
    config: ExecutionConfig
  ): Promise<void> {
    const steps = [
      '初始化浏览器环境...',
      `设置 User-Agent: ${task.profile.userAgent.slice(0, 50)}...`,
      '启动浏览器...',
      `导航到目标页面...`,
      '等待页面加载...',
      '执行脚本逻辑...',
      '等待用户操作...',
      '完成任务...',
    ];

    for (const step of steps) {
      if (this.abortController?.signal.aborted) {
        throw new Error('任务已取消');
      }

      await new Promise((resolve) => setTimeout(resolve, 1000 + Math.random() * 2000));
      onLog(`[${new Date().toLocaleTimeString()}] ${step}`);
    }

    // 模拟成功率
    if (Math.random() > 0.9) {
      throw new Error('模拟执行失败');
    }
  }

  stop() {
    if (this.abortController) {
      this.abortController.abort();
    }
    this.isRunning = false;
  }

  getIsRunning() {
    return this.isRunning;
  }
}

export const executor = new PlaywrightExecutor();

// 批量执行管理
export class BatchExecutor {
  private tasks: ExecutionTask[] = [];
  private currentIndex = 0;
  private isRunning = false;
  private config: ExecutionConfig;
  private onTaskUpdate: (task: ExecutionTask) => void;
  private onLog: (taskId: string, log: string) => void;

  constructor(
    config: ExecutionConfig,
    onTaskUpdate: (task: ExecutionTask) => void,
    onLog: (taskId: string, log: string) => void
  ) {
    this.config = config;
    this.onTaskUpdate = onTaskUpdate;
    this.onLog = onLog;
  }

  setTasks(tasks: ExecutionTask[]) {
    this.tasks = tasks;
    this.currentIndex = 0;
  }

  async start() {
    if (this.isRunning) return;
    this.isRunning = true;

    while (this.currentIndex < this.tasks.length && this.isRunning) {
      const task = this.tasks[this.currentIndex];
      await this.executeTask(task);
      this.currentIndex++;

      // 执行间隔
      if (this.currentIndex < this.tasks.length) {
        await new Promise((resolve) => setTimeout(resolve, 2000));
      }
    }

    this.isRunning = false;
  }

  private async executeTask(task: ExecutionTask) {
    task.status = 'running';
    task.startTime = Date.now();
    this.onTaskUpdate({ ...task });

    this.onLog(task.id, `[${new Date().toLocaleTimeString()}] 开始执行任务: ${task.wallet.label}`);

    try {
      const result = await executor.execute(
        task,
        (log) => this.onLog(task.id, log),
        this.config
      );

      task.endTime = Date.now();
      task.duration = task.endTime - task.startTime;

      if (result.success) {
        task.status = 'success';
        task.txHash = result.txHash;
        this.onLog(task.id, `[${new Date().toLocaleTimeString()}] 任务执行成功`);
      } else {
        task.status = 'failed';
        task.error = result.error;
        this.onLog(task.id, `[${new Date().toLocaleTimeString()}] 任务执行失败: ${result.error}`);
      }
    } catch (error: any) {
      task.endTime = Date.now();
      task.duration = task.endTime - (task.startTime || Date.now());
      task.status = 'failed';
      task.error = error.message;
      this.onLog(task.id, `[${new Date().toLocaleTimeString()}] 任务执行失败: ${error.message}`);
    }

    this.onTaskUpdate({ ...task });
    historyService.addRecord(task);
  }

  stop() {
    this.isRunning = false;
    executor.stop();
  }

  getProgress() {
    if (this.tasks.length === 0) return 0;
    return Math.round((this.currentIndex / this.tasks.length) * 100);
  }

  getStats() {
    const completed = this.tasks.filter(
      (t) => t.status === 'success' || t.status === 'failed'
    ).length;
    const success = this.tasks.filter((t) => t.status === 'success').length;
    const failed = this.tasks.filter((t) => t.status === 'failed').length;

    return {
      total: this.tasks.length,
      completed,
      success,
      failed,
      pending: this.tasks.filter((t) => t.status === 'pending').length,
      running: this.tasks.filter((t) => t.status === 'running').length,
    };
  }
}
