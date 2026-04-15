import { defineConfig, devices } from '@playwright/test';
import path from 'path';

/**
 * Playwright 配置文件 - 用于测试 Tauri 应用
 * @see https://playwright.dev/docs/test-configuration
 */
export default defineConfig({
  testDir: './e2e',

  /* 每个测试的超时时间 */
  timeout: 60 * 1000, // 60秒

  /* 全局设置 */
  globalSetup: require.resolve('./e2e/global-setup.ts'),
  globalTeardown: require.resolve('./e2e/global-teardown.ts'),

  expect: {
    /**
     * 断言超时时间
     */
    timeout: 10000,
  },

  /* 并发运行测试 */
  fullyParallel: false, // Tauri 应用建议串行运行

  /* 失败时保留输出 */
  forbidOnly: !!process.env.CI,

  /* 重试次数 */
  retries: process.env.CI ? 2 : 0,

  /* 并行工作进程数 */
  workers: 1, // Tauri 应用建议单进程

  /* 报告器配置 */
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
  ],

  /* 共享配置 */
  use: {
    /* 基础 URL */
    baseURL: 'http://localhost:1420',

    /* 收集追踪信息 */
    trace: 'on-first-retry',

    /* 截图 */
    screenshot: 'only-on-failure',

    /* 视频录制 */
    video: 'on-first-retry',

    /* 视口大小 */
    viewport: { width: 1280, height: 720 },

    /* 动作超时 */
    actionTimeout: 15000,
  },

  /* 项目配置 */
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Tauri 应用使用特定的 user agent
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.0',
      },
    },
  ],

  /* 本地开发服务器配置 */
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
