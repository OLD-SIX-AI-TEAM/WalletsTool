import { invoke } from '@tauri-apps/api/core';
import { ref, shallowRef } from 'vue';

export const recordingSession = shallowRef<RecordingSession | null>(null);
export const recordingActions = ref<RecordedAction[]>([]);

export interface RecordingSession {
  id: string;
  url: string;
  startTime: Date;
  actions: RecordedAction[];
  status: 'recording' | 'stopped' | 'error';
}

export interface RecordedAction {
  type: 'click' | 'fill' | 'navigate' | 'select' | 'hover' | 'screenshot' | 'upload' | 'evaluate' | 'mousemove' | 'scroll' | 'press' | 'newPage';
  selector?: string;
  value?: string;
  timestamp: Date;
  description: string;
  pageUrl?: string;
}

export interface ProxyConfig {
  type: 'direct' | 'http' | 'https' | 'socks5';
  host?: string;
  port?: number;
  username?: string;
  password?: string;
}

export interface RecordingOptions {
  browserType?: 'chromium' | 'firefox' | 'webkit';
  headless?: boolean;
  viewportWidth?: number;
  viewportHeight?: number;
  includeComments?: boolean;
  extensions?: string[];
  proxy?: ProxyConfig;
}

// 后端返回的会话类型
interface BackendRecordingSession {
  id: string;
  url: string;
  start_time: number;
  actions: BackendRecordedAction[];
  status: string;
  generated_code?: string;
}

interface BackendRecordedAction {
  action_type: string;
  selector?: string;
  value?: string;
  timestamp: number;
  description: string;
  page_url?: string;
}

class PlaywrightRecorderBridge {
  private currentSession: RecordingSession | null = null;
  private sessionId: string | null = null;
  private currentExtensions: string[] = [];
  private currentProxy: ProxyConfig | null = null;
  private pollInterval: ReturnType<typeof setInterval> | null = null;

  async startSession(url: string, options: RecordingOptions = {}): Promise<RecordingSession> {
    // 先停止之前的轮询
    this.stopPolling();
    
    // 清空之前的会话数据
    this.currentSession = null;
    this.sessionId = null;
    recordingSession.value = null;
    recordingActions.value = [];
    
    const {
      browserType = 'chromium',
      headless = false,
      viewportWidth = 1280,
      viewportHeight = 720,
      proxy
    } = options;

    this.currentExtensions = options.extensions || [];
    this.currentProxy = proxy || null;

    try {
      console.log('[Recorder] 开始录制会话...');
      console.log('[Recorder] URL:', url);
      console.log('[Recorder] 浏览器:', browserType);
      console.log('[Recorder] 代理:', proxy);

      // 调用后端 Tauri 命令
      const sessionId = await invoke<string>('playwright_start_recording', {
        options: {
          url,
          browser_type: browserType,
          headless,
          viewport_width: viewportWidth,
          viewport_height: viewportHeight,
          proxy_type: proxy?.type,
          proxy_host: proxy?.host,
          proxy_port: proxy?.port,
          proxy_username: proxy?.username,
          proxy_password: proxy?.password
        }
      });

      console.log('[Recorder] 会话创建成功:', sessionId);

      this.sessionId = sessionId;
      this.currentSession = {
        id: sessionId,
        url,
        startTime: new Date(),
        actions: [{
          type: 'navigate',
          value: url,
          timestamp: new Date(),
          description: `导航到 ${url}`
        }],
        status: 'recording'
      };

      recordingSession.value = this.currentSession;
      recordingActions.value = this.currentSession.actions;

      // 启动轮询获取操作记录
      this.startPolling(sessionId);

      return this.currentSession;
    } catch (error) {
      console.error('[Recorder] 启动录制失败:', error);
      throw error;
    }
  }

  private startPolling(sessionId: string): void {
    this.pollInterval = setInterval(async () => {
      // 检查会话ID是否匹配，防止旧轮询更新错误的数据
      if (this.sessionId !== sessionId) {
        console.log('[Recorder] 会话ID不匹配，停止轮询');
        this.stopPolling();
        return;
      }
      
      try {
        const session = await invoke<BackendRecordingSession | null>('playwright_get_recording_session', {
          sessionId
        });

        if (session && this.currentSession) {
          // 更新操作列表
          this.currentSession.actions = session.actions.map(action => ({
            type: action.action_type as RecordedAction['type'],
            selector: action.selector,
            value: action.value,
            timestamp: new Date(action.timestamp),
            description: action.description,
            pageUrl: action.page_url
          }));

          recordingActions.value = this.currentSession.actions;

          // 如果会话已停止，停止轮询并更新状态
          if (session.status === 'stopped' && this.currentSession.status !== 'stopped') {
            console.log('[Recorder] 检测到会话已停止，更新状态');
            this.currentSession.status = 'stopped';
            recordingSession.value = { ...this.currentSession };
            this.stopPolling();
          }
        }
      } catch (error) {
        console.error('[Recorder] 轮询会话失败:', error);
      }
    }, 1000);

    // 5分钟后自动停止轮询
    setTimeout(() => {
      this.stopPolling();
    }, 5 * 60 * 1000);
  }
  
  private stopPolling(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = null;
      console.log('[Recorder] 轮询已停止');
    }
  }

  async stopSession(): Promise<string | null> {
    if (!this.currentSession || !this.sessionId) {
      return null;
    }

    try {
      console.log('[Recorder] 停止录制会话:', this.sessionId);
      
      // 停止轮询
      this.stopPolling();

      const code = await invoke<string | null>('playwright_stop_recording', {
        sessionId: this.sessionId
      });

      console.log('[Recorder] 录制已停止，生成代码:', code ? '成功' : '无代码');

      this.currentSession.status = 'stopped';
      this.sessionId = null;

      return code;
    } catch (error) {
      console.error('[Recorder] 停止录制失败:', error);
      this.currentSession.status = 'error';
      throw error;
    }
  }

  addAction(type: RecordedAction['type'], description: string, details: { selector?: string; value?: string } = {}): void {
    if (!this.currentSession) return;

    this.currentSession.actions.push({
      type,
      selector: details.selector,
      value: details.value,
      timestamp: new Date(),
      description
    });
  }

  getCurrentSession(): RecordingSession | null {
    return this.currentSession;
  }

  clearSession(): void {
    this.currentSession = null;
    this.sessionId = null;
    this.currentExtensions = [];
    this.currentProxy = null;
    recordingSession.value = null;
    recordingActions.value = [];
  }

  getExtensions(): string[] {
    return this.currentExtensions;
  }
}

export const playwrightRecorder = new PlaywrightRecorderBridge();

export const recorderService = {
  async startRecording(url: string, options?: RecordingOptions): Promise<RecordingSession> {
    return await playwrightRecorder.startSession(url, options);
  },

  async stopRecording(): Promise<string | null> {
    return await playwrightRecorder.stopSession();
  },

  getCurrentSession(): RecordingSession | null {
    return playwrightRecorder.getCurrentSession();
  },

  clearRecording(): void {
    playwrightRecorder.clearSession();
  },

  addAction(type: RecordedAction['type'], description: string, details?: { selector?: string; value?: string }): void {
    playwrightRecorder.addAction(type, description, details);
  }
};
