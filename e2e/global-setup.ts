import { chromium, type FullConfig } from '@playwright/test';

/**
 * 全局设置 - 在测试开始前执行
 * 可用于：启动 Tauri 应用、初始化测试数据等
 */
async function globalSetup(config: FullConfig) {
  console.log('🚀 开始全局设置...');

  // 可以在这里启动 Tauri 应用或进行其他初始化
  // 例如：
  // 1. 启动 Tauri dev server
  // 2. 初始化测试数据库
  // 3. 创建测试用的钱包数据

  console.log('✅ 全局设置完成');
}

export default globalSetup;
