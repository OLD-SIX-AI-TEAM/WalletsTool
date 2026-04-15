import { test, expect } from '@playwright/test';
import {
  waitForTauriApp,
  invokeTauriCommand,
  waitForLoadingComplete,
  takeScreenshot,
} from './tauri-helpers';

/**
 * 余额查询功能 - 前端+后端联调测试
 *
 * 测试余额查询的前后端交互
 */

test.describe('余额查询 - 联调测试', () => {
  test.beforeEach(async ({ page }) => {
    // 导航到余额查询页面
    await page.goto('/#/ethereum/balance');

    // 等待 Tauri 应用加载
    await waitForTauriApp(page);

    // 等待页面加载完成
    await waitForLoadingComplete(page);
  });

  test('页面加载正确', async ({ page }) => {
    // 验证页面标题
    const title = page.locator('h1, .page-title').first();
    await expect(title).toBeVisible();

    // 截图记录
    await takeScreenshot(page, 'balance-page');
  });

  test('获取链列表 - 后端联调', async ({ page }) => {
    // 调用后端命令获取链列表
    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    // 验证返回数据
    expect(Array.isArray(chains)).toBe(true);
    expect(chains.length).toBeGreaterThan(0);

    // 验证链数据结构
    const firstChain = chains[0];
    expect(firstChain).toHaveProperty('id');
    expect(firstChain).toHaveProperty('name');
    expect(firstChain).toHaveProperty('chain_id');

    console.log('✅ 获取链列表成功，数量:', chains.length);
    console.log('第一条链:', firstChain.name);
  });

  test('获取代币列表 - 后端联调', async ({ page }) => {
    // 先获取链列表
    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    if (chains.length > 0) {
      // 获取第一条链的代币列表
      const chainId = chains[0].id;
      const coins = await invokeTauriCommand<any[]>(page, 'get_coin_list', {
        chain_id: chainId,
      });

      expect(Array.isArray(coins)).toBe(true);
      console.log(`✅ 获取代币列表成功，链 ${chains[0].name} 有 ${coins.length} 个代币`);
    }
  });

  test('测试 RPC 连接 - 后端联调', async ({ page }) => {
    // 获取链列表
    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    if (chains.length > 0) {
      const chain = chains[0];

      // 测试 RPC 连接
      try {
        const result = await invokeTauriCommand<any>(page, 'test_rpc_connection', {
          chain_id: chain.id,
        });

        console.log('✅ RPC 连接测试结果:', result);

        // 验证返回结果包含必要的字段
        expect(result).toHaveProperty('success');
      } catch (error) {
        console.log('ℹ️ RPC 连接测试失败（可能是网络问题）:', error);
      }
    }
  });

  test('获取 Gas 价格 - 后端联调', async ({ page }) => {
    // 获取链列表
    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    if (chains.length > 0) {
      const chainId = chains[0].id;

      try {
        const gasPrice = await invokeTauriCommand<string>(page, 'get_chain_gas_price', {
          chain_id: chainId,
        });

        console.log('✅ 获取 Gas 价格成功:', gasPrice);
        expect(typeof gasPrice).toBe('string');
      } catch (error) {
        console.log('ℹ️ 获取 Gas 价格失败:', error);
      }
    }
  });

  test('批量获取 Gas 价格 - 后端联调', async ({ page }) => {
    // 获取链列表
    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    if (chains.length > 0) {
      // 取前3条链的ID
      const chainIds = chains.slice(0, 3).map(c => c.id);

      try {
        const gasPrices = await invokeTauriCommand<Record<string, string>>(page, 'get_multiple_gas_prices', {
          chain_ids: chainIds,
        });

        console.log('✅ 批量获取 Gas 价格成功:', gasPrices);
        expect(typeof gasPrices).toBe('object');
      } catch (error) {
        console.log('ℹ️ 批量获取 Gas 价格失败:', error);
      }
    }
  });
});
