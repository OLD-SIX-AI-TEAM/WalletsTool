# Tauri 应用 E2E 测试

这个目录包含 WalletsTool Tauri 应用的前端+后端联调测试。

## 测试架构

```
e2e/
├── playwright.config.ts      # Playwright 配置文件
├── global-setup.ts           # 全局测试前设置
├── global-teardown.ts        # 全局测试后清理
├── tauri-helpers.ts          # Tauri 测试辅助函数
├── wallet-manager.spec.ts    # 钱包管理器测试
├── balance-query.spec.ts     # 余额查询测试
└── api-integration.spec.ts   # API 集成测试
```

## 运行测试

### 1. 安装 Playwright 浏览器

```bash
npx playwright install chromium
```

### 2. 运行所有测试

```bash
# 无头模式运行（CI 推荐）
npm run test:e2e

# 有界面模式运行（开发调试）
npm run test:e2e:headed

# UI 模式（交互式调试）
npm run test:e2e:ui

# 调试模式
npm run test:e2e:debug
```

### 3. 运行特定测试文件

```bash
npx playwright test wallet-manager.spec.ts
```

### 4. 查看测试报告

```bash
npm run test:e2e:report
```

## 测试类型

### 1. 前端+后端联调测试

测试前端页面和后端 Rust 命令的完整交互流程：

- `wallet-manager.spec.ts` - 钱包管理器功能测试
- `balance-query.spec.ts` - 余额查询功能测试

### 2. API 集成测试

验证所有 Tauri 命令的可用性和响应格式：

- `api-integration.spec.ts` - 所有 API 的契约测试

## 关键辅助函数

### invokeTauriCommand

通过 Playwright 调用 Tauri 后端命令：

```typescript
const wallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
  group_id: null,
  chain_type: null,
  password: null,
});
```

### waitForTauriApp

等待 Tauri 应用完全加载：

```typescript
await waitForTauriApp(page);
```

### waitForLoadingComplete

等待页面加载状态完成：

```typescript
await waitForLoadingComplete(page);
```

## 编写新测试

### 基本模板

```typescript
import { test, expect } from '@playwright/test';
import {
  waitForTauriApp,
  invokeTauriCommand,
  waitForLoadingComplete,
} from './tauri-helpers';

test.describe('功能名称', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/your-page');
    await waitForTauriApp(page);
    await waitForLoadingComplete(page);
  });

  test('测试名称', async ({ page }) => {
    // 调用后端命令
    const result = await invokeTauriCommand(page, 'your_command', { arg: 'value' });

    // 验证结果
    expect(result).toBeDefined();

    // 验证 UI
    const element = page.locator('.your-selector');
    await expect(element).toBeVisible();
  });
});
```

## 注意事项

1. **测试数据**：测试会创建真实数据，建议在测试环境中运行
2. **并发**：Tauri 应用测试建议串行运行（workers: 1）
3. **超时**：网络操作可能需要更长的超时时间
4. **截图**：失败测试会自动截图保存到 `playwright-report/`

## CI/CD 集成

在 GitHub Actions 中使用：

```yaml
- name: Run E2E Tests
  run: |
    npm install
    npx playwright install chromium
    npm run test:e2e
```
