import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// 类型定义
export interface LogMessage {
  sessionId: string;
  walletId?: string;
  level: 'info' | 'warn' | 'error' | 'success';
  message: string;
  timestamp: number;
}

export interface WalletExecutionStatus {
  walletId: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  startTime?: number;
  endTime?: number;
  error?: string;
}

export interface SessionStatus {
  sessionId: string;
  totalWallets: number;
  completedWallets: number;
  failedWallets: number;
  runningWallets: number;
  status: 'created' | 'running' | 'completed' | 'cancelled' | 'error';
  startTime: number;
  endTime?: number;
  wallets: WalletExecutionStatus[];
}

export interface BrowserConfig {
  userAgent: string;
  viewportWidth: number;
  viewportHeight: number;
  deviceScaleFactor: number;
  locale: string;
  timezoneId: string;
  proxyType: string;
  proxyHost?: string;
  proxyPort?: number;
  proxyUsername?: string;
  proxyPassword?: string;
  canvasSpoof: boolean;
  webglSpoof: boolean;
  audioSpoof: boolean;
  timezoneSpoof: boolean;
  geolocationSpoof: boolean;
  fontSpoof: boolean;
  webrtcSpoof: boolean;
  navigatorOverride: boolean;
  webdriverOverride: boolean;
  headless: boolean;
}

export interface WalletInfo {
  id: string;
  name: string;
  address: string;
  privateKey: string;
  chainType: string;
}

export interface ExecutionConfig {
  targetUrl: string;
  config: BrowserConfig;
  userCode: string;
  wallets: WalletInfo[];
  concurrency?: number;
  timeoutSecs?: number;
}

// 回调函数类型
export type LogCallback = (log: LogMessage) => void;
export type StatusCallback = (status: SessionStatus) => void;
export type CompleteCallback = (success: boolean, error?: string) => void;

/**
 * 脚本执行器 - 支持多并发、实时日志流、进度跟踪
 */
export class ScriptExecutor {
  private sessionId: string | null = null;
  private logUnlisten: UnlistenFn | null = null;
  private statusInterval: NodeJS.Timeout | null = null;
  private onLogCallback: LogCallback | null = null;
  private onStatusCallback: StatusCallback | null = null;
  private onCompleteCallback: CompleteCallback | null = null;
  private isRunning = false;

  /**
   * 创建执行会话
   */
  async createSession(config: ExecutionConfig): Promise<string> {
    try {
      this.sessionId = await invoke<string>('playwright_create_session', { config });
      return this.sessionId;
    } catch (error) {
      console.error('创建执行会话失败:', error);
      throw error;
    }
  }

  /**
   * 订阅日志流
   */
  async subscribeLogs(callback: LogCallback): Promise<void> {
    this.onLogCallback = callback;

    try {
      // 使用 Tauri 事件监听日志
      this.logUnlisten = await listen<LogMessage>('execution-log', (event) => {
        if (this.onLogCallback) {
          this.onLogCallback(event.payload);
        }
      });
    } catch (error) {
      console.error('订阅日志失败:', error);
      throw error;
    }
  }

  /**
   * 开始执行
   */
  async startExecution(
    onStatus?: StatusCallback,
    onComplete?: CompleteCallback,
    statusPollInterval = 1000
  ): Promise<void> {
    if (!this.sessionId) {
      throw new Error('未创建执行会话');
    }

    this.onStatusCallback = onStatus || null;
    this.onCompleteCallback = onComplete || null;
    this.isRunning = true;

    try {
      // 启动执行
      await invoke('playwright_start_execution', { sessionId: this.sessionId });

      // 启动状态轮询
      this.startStatusPolling(statusPollInterval);
    } catch (error) {
      this.isRunning = false;
      console.error('启动执行失败:', error);
      throw error;
    }
  }

  /**
   * 启动状态轮询
   */
  private startStatusPolling(interval: number): void {
    if (this.statusInterval) {
      clearInterval(this.statusInterval);
    }

    this.statusInterval = setInterval(async () => {
      if (!this.isRunning || !this.sessionId) {
        this.stopStatusPolling();
        return;
      }

      try {
        const status = await this.getStatus();

        if (this.onStatusCallback) {
          this.onStatusCallback(status);
        }

        // 检查是否完成
        if (status.status === 'completed' || status.status === 'cancelled' || status.status === 'error') {
          this.isRunning = false;
          this.stopStatusPolling();

          if (this.onCompleteCallback) {
            const success = status.status === 'completed' && status.failedWallets === 0;
            const error = status.status === 'error' ? '执行出错' : undefined;
            this.onCompleteCallback(success, error);
          }
        }
      } catch (error) {
        console.error('获取状态失败:', error);
      }
    }, interval);
  }

  /**
   * 停止状态轮询
   */
  private stopStatusPolling(): void {
    if (this.statusInterval) {
      clearInterval(this.statusInterval);
      this.statusInterval = null;
    }
  }

  /**
   * 获取当前状态
   */
  async getStatus(): Promise<SessionStatus> {
    if (!this.sessionId) {
      throw new Error('未创建执行会话');
    }

    return await invoke<SessionStatus>('playwright_get_execution_status', { sessionId: this.sessionId });
  }

  /**
   * 取消执行
   */
  async cancel(): Promise<void> {
    if (!this.sessionId) {
      throw new Error('未创建执行会话');
    }

    try {
      await invoke('playwright_cancel_execution', { sessionId: this.sessionId });
      this.isRunning = false;
    } catch (error) {
      console.error('取消执行失败:', error);
      throw error;
    }
  }

  /**
   * 清理会话资源
   */
  async cleanup(): Promise<void> {
    if (this.sessionId) {
      try {
        await invoke('playwright_cleanup_session', { sessionId: this.sessionId });
      } catch (error) {
        console.error('清理会话失败:', error);
      }
    }

    this.dispose();
  }

  /**
   * 释放资源
   */
  dispose(): void {
    this.stopStatusPolling();

    if (this.logUnlisten) {
      this.logUnlisten();
      this.logUnlisten = null;
    }

    this.onLogCallback = null;
    this.onStatusCallback = null;
    this.onCompleteCallback = null;
    this.sessionId = null;
    this.isRunning = false;
  }

  /**
   * 获取会话ID
   */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * 是否正在运行
   */
  isExecuting(): boolean {
    return this.isRunning;
  }

  /**
   * 便捷方法：一键执行（创建会话、订阅日志、启动执行）
   */
  static async execute(
    config: ExecutionConfig,
    callbacks: {
      onLog?: LogCallback;
      onStatus?: StatusCallback;
      onComplete?: CompleteCallback;
    },
    options: {
      statusPollInterval?: number;
      autoCleanup?: boolean;
    } = {}
  ): Promise<ScriptExecutor> {
    const executor = new ScriptExecutor();

    try {
      // 创建会话
      await executor.createSession(config);

      // 订阅日志
      if (callbacks.onLog) {
        await executor.subscribeLogs(callbacks.onLog);
      }

      // 开始执行
      await executor.startExecution(
        callbacks.onStatus,
        async (success, error) => {
          if (callbacks.onComplete) {
            callbacks.onComplete(success, error);
          }
          if (options.autoCleanup !== false) {
            await executor.cleanup();
          }
        },
        options.statusPollInterval || 1000
      );

      return executor;
    } catch (error) {
      await executor.cleanup();
      throw error;
    }
  }
}

// 单例实例
let globalExecutor: ScriptExecutor | null = null;

/**
 * 获取全局执行器实例
 */
export function getGlobalExecutor(): ScriptExecutor {
  if (!globalExecutor) {
    globalExecutor = new ScriptExecutor();
  }
  return globalExecutor;
}

/**
 * 重置全局执行器
 */
export function resetGlobalExecutor(): void {
  if (globalExecutor) {
    globalExecutor.dispose();
    globalExecutor = null;
  }
}

export default ScriptExecutor;
