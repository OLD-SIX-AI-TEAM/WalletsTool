import { invoke } from '@tauri-apps/api/core';

export interface CliToolStatus {
  name: string;
  installed: boolean;
  version?: string;
  error?: string;
}

export interface CliCheckResult {
  all_installed: boolean;
  tools: CliToolStatus[];
}

class CliCheckService {
  private cachedResult: CliCheckResult | null = null;
  private lastCheckTime: number = 0;
  private readonly CACHE_DURATION = 60000; // 1分钟缓存

  async checkTools(forceRefresh = false): Promise<CliCheckResult> {
    // 检查缓存
    if (!forceRefresh && this.cachedResult) {
      const now = Date.now();
      if (now - this.lastCheckTime < this.CACHE_DURATION) {
        return this.cachedResult;
      }
    }

    try {
      const result = await invoke<CliCheckResult>('check_cli_tools');
      this.cachedResult = result;
      this.lastCheckTime = Date.now();
      return result;
    } catch (error) {
      console.error('检查 CLI 工具失败:', error);
      throw error;
    }
  }

  async checkNodeJs(): Promise<boolean> {
    const result = await this.checkTools();
    const nodeTool = result.tools.find(t => t.name === 'Node.js');
    return nodeTool?.installed ?? false;
  }

  async checkNpm(): Promise<boolean> {
    const result = await this.checkTools();
    const npmTool = result.tools.find(t => t.name === 'npm');
    return npmTool?.installed ?? false;
  }

  async checkNpx(): Promise<boolean> {
    const result = await this.checkTools();
    const npxTool = result.tools.find(t => t.name === 'npx');
    return npxTool?.installed ?? false;
  }

  async checkPlaywright(): Promise<boolean> {
    const result = await this.checkTools();
    const playwrightTool = result.tools.find(t => t.name === 'Playwright');
    return playwrightTool?.installed ?? false;
  }

  getInstallInstructions(missingTools: CliToolStatus[]): string {
    const instructions: string[] = [];
    
    const hasNode = missingTools.some(t => t.name === 'Node.js');
    const hasNpm = missingTools.some(t => t.name === 'npm');
    const hasNpx = missingTools.some(t => t.name === 'npx');
    const hasPlaywright = missingTools.some(t => t.name === 'Playwright');

    if (hasNode || hasNpm || hasNpx) {
      instructions.push('1. 安装 Node.js (包含 npm 和 npx):');
      instructions.push('   访问 https://nodejs.org/ 下载并安装 LTS 版本');
      instructions.push('   或使用包管理器:');
      instructions.push('   - Windows: winget install OpenJS.NodeJS');
      instructions.push('   - macOS: brew install node');
      instructions.push('   - Linux: sudo apt install nodejs npm');
    }

    if (hasPlaywright && !hasNode) {
      instructions.push('');
      instructions.push('2. 安装 Playwright (可选，首次使用时会自动安装):');
      instructions.push('   npm install -g playwright');
      instructions.push('   npx playwright install chromium');
    }

    return instructions.join('\n');
  }

  clearCache(): void {
    this.cachedResult = null;
    this.lastCheckTime = 0;
  }
}

export const cliCheckService = new CliCheckService();
