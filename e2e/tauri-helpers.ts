import { Page, expect } from '@playwright/test';

/**
 * Tauri 应用测试辅助函数
 * 用于与 Tauri 后端进行交互测试
 */

/**
 * 等待 Tauri 应用加载完成
 */
export async function waitForTauriApp(page: Page, timeout = 30000) {
  // 等待页面加载完成
  await page.waitForLoadState('networkidle', { timeout });

  // 等待 Tauri API 可用
  await page.waitForFunction(() => {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }, { timeout });

  console.log('✅ Tauri 应用已加载');
}

/**
 * 执行 Tauri 命令
 * 通过页面 evaluate 调用 Tauri API
 */
export async function invokeTauriCommand<T = any>(
  page: Page,
  command: string,
  args: Record<string, any> = {}
): Promise<T> {
  return await page.evaluate(async ({ cmd, args }) => {
    const { invoke } = (window as any).__TAURI__.core;
    return await invoke(cmd, args);
  }, { cmd: command, args });
}

/**
 * 监听 Tauri 事件
 */
export async function listenTauriEvent<T = any>(
  page: Page,
  eventName: string,
  callback: (payload: T) => void
) {
  return await page.evaluate(({ event }) => {
    const { listen } = (window as any).__TAURI__.event;
    return listen(event, (e: any) => {
      // 通过 console.log 传递事件数据
      console.log(`[TAURI_EVENT_${event}]`, JSON.stringify(e.payload));
    });
  }, { event: eventName });
}

/**
 * 等待特定元素出现并验证文本
 */
export async function expectElementText(
  page: Page,
  selector: string,
  expectedText: string | RegExp,
  timeout = 10000
) {
  const element = page.locator(selector);
  await expect(element).toBeVisible({ timeout });
  await expect(element).toHaveText(expectedText, { timeout });
}

/**
 * 安全地填写输入框（清除原有内容）
 */
export async function safeFill(page: Page, selector: string, value: string) {
  const input = page.locator(selector);
  await input.clear();
  await input.fill(value);
}

/**
 * 等待并点击元素
 */
export async function waitAndClick(page: Page, selector: string, timeout = 10000) {
  const element = page.locator(selector);
  await element.waitFor({ state: 'visible', timeout });
  await element.click();
}

/**
 * 等待 Toast/通知消息
 */
export async function waitForToast(
  page: Page,
  expectedText?: string | RegExp,
  timeout = 10000
) {
  // 根据你的 UI 框架调整选择器
  const toastSelector = '.p-toast-message, .arco-message, [role="alert"]';
  const toast = page.locator(toastSelector).first();

  await toast.waitFor({ state: 'visible', timeout });

  if (expectedText) {
    await expect(toast).toContainText(expectedText, { timeout });
  }

  return toast;
}

/**
 * 截图并保存
 */
export async function takeScreenshot(page: Page, name: string) {
  await page.screenshot({
    path: `./e2e/screenshots/${name}-${Date.now()}.png`,
    fullPage: true,
  });
}

/**
 * 模拟 Tauri 后端返回的数据
 * 用于前端单元测试或当后端不可用时
 */
export async function mockTauriCommand(
  page: Page,
  command: string,
  mockResponse: any
) {
  await page.addInitScript(({ cmd, response }) => {
    const originalInvoke = (window as any).__TAURI__?.core?.invoke;
    if (originalInvoke) {
      (window as any).__TAURI__.core.invoke = async (command: string, args: any) => {
        if (command === cmd) {
          return response;
        }
        return originalInvoke(command, args);
      };
    }
  }, { cmd: command, response: mockResponse });
}

/**
 * 检查页面是否有 JavaScript 错误
 */
export async function checkForConsoleErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];

  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      errors.push(msg.text());
    }
  });

  page.on('pageerror', (error) => {
    errors.push(error.message);
  });

  return errors;
}

/**
 * 等待加载状态完成
 */
export async function waitForLoadingComplete(page: Page, timeout = 30000) {
  // 等待加载动画消失
  const loadingSelectors = [
    '.p-progress-spinner',
    '.arco-spin',
    '.loading',
    '[data-testid="loading"]',
  ];

  for (const selector of loadingSelectors) {
    const loading = page.locator(selector);
    try {
      await loading.waitFor({ state: 'hidden', timeout: 5000 });
    } catch {
      // 忽略不存在的加载指示器
    }
  }
}
