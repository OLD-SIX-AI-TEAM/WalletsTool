import { invoke } from '@tauri-apps/api/core';
import { scriptService, profileService, walletService } from './browserAutomationService';

export interface ExecutionConfig {
  scriptId: number;
  profileId: number;
  walletIds: number[];
  targetUrl: string;
  concurrency?: number;
  timeout?: number;
}

export interface ExecutionResult {
  success: boolean;
  output: string;
  error?: string;
  execution_time_ms: number;
}

export interface BrowserConfig {
  user_agent: string;
  viewport_width: number;
  viewport_height: number;
  device_scale_factor: number;
  locale: string;
  timezone_id: string;
  proxy_type: string;
  proxy_host?: string;
  proxy_port?: number;
  proxy_username?: string;
  proxy_password?: string;
  canvas_spoof: boolean;
  webgl_spoof: boolean;
  audio_spoof: boolean;
  timezone_spoof: boolean;
  geolocation_spoof: boolean;
  font_spoof: boolean;
  webrtc_spoof: boolean;
  navigator_override: boolean;
  webdriver_override: boolean;
  headless: boolean;
  target_url?: string;
}

export interface WalletInfo {
  name: string;
  address: string;
  private_key: string;
  chain_type: string;
}

export interface ExecutionParams {
  target_url: string;
  config: BrowserConfig;
  user_code: string;
  wallets: WalletInfo[];
  concurrency: number;
  timeout: number;
}

/**
 * 脚本构建器 - 整合环境配置和用户脚本，生成完整执行脚本
 */
export class ScriptBuilder {
  /**
   * 构建并执行脚本
   */
  static async execute(config: ExecutionConfig): Promise<ExecutionResult> {
    try {
      // 1. 获取用户脚本
      const script = await scriptService.getScript(config.scriptId);
      if (!script) {
        throw new Error('脚本不存在');
      }

      // 2. 获取环境配置
      const profile = await profileService.getProfile(config.profileId);
      if (!profile) {
        throw new Error('环境配置不存在');
      }

      // 3. 获取钱包信息（包含解密后的私钥）
      const wallets: WalletInfo[] = [];
      for (const walletId of config.walletIds) {
        try {
          // 获取钱包基本信息
          const wallet = await walletService.getWallet(walletId);
          if (!wallet) {
            console.warn(`钱包 ${walletId} 不存在，跳过`);
            continue;
          }

          // 获取解密后的私钥
          const privateKey = await walletService.getWalletPrivateKey(walletId);
          
          wallets.push({
            name: wallet.name,
            address: wallet.address,
            private_key: privateKey,
            chain_type: wallet.chain_type,
          });
        } catch (error) {
          console.error(`获取钱包 ${walletId} 信息失败:`, error);
        }
      }

      if (wallets.length === 0) {
        throw new Error('没有有效的钱包可以执行');
      }

      // 4. 构建执行参数
      const executionParams: ExecutionParams = {
        target_url: config.targetUrl,
        config: {
          user_agent: profile.user_agent || '',
          viewport_width: profile.viewport_width || 1920,
          viewport_height: profile.viewport_height || 1080,
          device_scale_factor: profile.device_scale_factor || 1,
          locale: profile.locale || 'en-US',
          timezone_id: profile.timezone_id || 'America/New_York',
          proxy_type: profile.proxy_type || 'direct',
          proxy_host: profile.proxy_host || undefined,
          proxy_port: profile.proxy_port || undefined,
          proxy_username: profile.proxy_username || undefined,
          proxy_password: profile.proxy_password || undefined,
          canvas_spoof: profile.canvas_spoof ?? true,
          webgl_spoof: profile.webgl_spoof ?? true,
          audio_spoof: profile.audio_spoof ?? true,
          timezone_spoof: profile.timezone_spoof ?? true,
          geolocation_spoof: profile.geolocation_spoof ?? true,
          font_spoof: profile.font_spoof ?? true,
          webrtc_spoof: profile.webrtc_spoof ?? true,
          navigator_override: profile.navigator_override ?? true,
          webdriver_override: profile.webdriver_override ?? true,
          headless: profile.headless ?? false,
        },
        user_code: script.content,
        wallets,
        concurrency: config.concurrency || 1,
        timeout: config.timeout || 300,
      };

      // 5. 调用后端执行
      const result = await invoke<ExecutionResult>('execute_playwright_script', {
        params: executionParams,
      });

      return result;
    } catch (error) {
      return {
        success: false,
        output: '',
        error: String(error),
        execution_time_ms: 0,
      };
    }
  }

  /**
   * 仅构建脚本（用于预览或调试）
   */
  static async buildScriptOnly(config: ExecutionConfig): Promise<string> {
    try {
      const script = await scriptService.getScript(config.scriptId);
      const profile = await profileService.getProfile(config.profileId);
      
      if (!script || !profile) {
        throw new Error('脚本或配置不存在');
      }

      // 构建参数（不包含私钥）
      const params = {
        target_url: config.targetUrl,
        config: {
          user_agent: profile.user_agent || '',
          viewport_width: profile.viewport_width || 1920,
          viewport_height: profile.viewport_height || 1080,
          proxy_type: profile.proxy_type || 'direct',
          // ... 其他配置
        },
        user_code: script.content,
        wallets: [], // 预览时不包含钱包
        concurrency: config.concurrency || 1,
        timeout: config.timeout || 300,
      };

      // 这里可以调用一个后端命令来只构建不执行
      // 暂时返回简化版本
      return `// 生成的脚本预览
// 目标URL: ${params.target_url}
// 环境: ${profile.name}
// 脚本: ${script.name}

// 用户代码:
${script.content}`;
    } catch (error) {
      return `构建失败: ${error}`;
    }
  }
}

export default ScriptBuilder;
