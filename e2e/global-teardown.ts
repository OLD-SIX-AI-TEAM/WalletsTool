import { type FullConfig } from '@playwright/test';

/**
 * 全局清理 - 在测试结束后执行
 * 可用于：关闭 Tauri 应用、清理测试数据等
 */
async function globalTeardown(config: FullConfig) {
  console.log('🧹 开始全局清理...');

  // 清理测试数据、关闭应用等

  console.log('✅ 全局清理完成');
}

export default globalTeardown;
