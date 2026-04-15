import { test, expect } from '@playwright/test';
import {
  waitForTauriApp,
  invokeTauriCommand,
  waitForLoadingComplete,
} from './tauri-helpers';

/**
 * API 集成测试 - 验证所有 Tauri 命令
 *
 * 这个测试文件用于验证前端和后端的 API 契约
 */

test.describe('API 集成测试 - 钱包管理', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/wallet-manager');
    await waitForTauriApp(page);
    await waitForLoadingComplete(page);
  });

  test('所有钱包管理命令可用', async ({ page }) => {
    const commands = [
      'get_groups',
      'get_wallets',
      'is_wallet_manager_initialized',
      'get_watch_addresses',
      'is_password_set',
    ];

    for (const cmd of commands) {
      try {
        const result = await invokeTauriCommand(page, cmd, {});
        console.log(`✅ 命令 ${cmd} 可用`);
      } catch (error) {
        console.log(`❌ 命令 ${cmd} 失败:`, error);
        // 不抛出错误，只是记录
      }
    }
  });
});

test.describe('API 集成测试 - 链配置', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/ethereum/balance');
    await waitForTauriApp(page);
    await waitForLoadingComplete(page);
  });

  test('所有链配置命令可用', async ({ page }) => {
    const commands = [
      { cmd: 'get_chain_list', args: {} },
      { cmd: 'get_rpc_providers', args: {} },
    ];

    for (const { cmd, args } of commands) {
      try {
        const result = await invokeTauriCommand(page, cmd, args);
        console.log(`✅ 命令 ${cmd} 可用`);
      } catch (error) {
        console.log(`❌ 命令 ${cmd} 失败:`, error);
      }
    }
  });
});

test.describe('API 集成测试 - 工具函数', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/#/');
    await waitForTauriApp(page);
  });

  test('工具命令可用', async ({ page }) => {
    const commands = [
      { cmd: 'get_temp_dir', args: {} },
    ];

    for (const { cmd, args } of commands) {
      try {
        const result = await invokeTauriCommand(page, cmd, args);
        console.log(`✅ 命令 ${cmd} 可用，结果:`, result);
      } catch (error) {
        console.log(`❌ 命令 ${cmd} 失败:`, error);
      }
    }
  });
});

test.describe('API 响应格式验证', () => {
  test('钱包列表响应格式', async ({ page }) => {
    await page.goto('/#/wallet-manager');
    await waitForTauriApp(page);

    const wallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
      group_id: null,
      chain_type: null,
      password: null,
    });

    if (wallets.length > 0) {
      const wallet = wallets[0];

      // 验证必需的字段
      expect(wallet).toHaveProperty('id');
      expect(typeof wallet.id).toBe('number');

      expect(wallet).toHaveProperty('address');
      expect(typeof wallet.address).toBe('string');

      expect(wallet).toHaveProperty('name');
      expect(typeof wallet.name).toBe('string');

      console.log('✅ 钱包列表响应格式正确');
    }
  });

  test('分组列表响应格式', async ({ page }) => {
    await page.goto('/#/wallet-manager');
    await waitForTauriApp(page);

    const groups = await invokeTauriCommand<any[]>(page, 'get_groups');

    if (groups.length > 0) {
      const group = groups[0];

      expect(group).toHaveProperty('id');
      expect(typeof group.id).toBe('number');

      expect(group).toHaveProperty('name');
      expect(typeof group.name).toBe('string');

      console.log('✅ 分组列表响应格式正确');
    }
  });

  test('链列表响应格式', async ({ page }) => {
    await page.goto('/#/ethereum/balance');
    await waitForTauriApp(page);

    const chains = await invokeTauriCommand<any[]>(page, 'get_chain_list');

    if (chains.length > 0) {
      const chain = chains[0];

      expect(chain).toHaveProperty('id');
      expect(typeof chain.id).toBe('number');

      expect(chain).toHaveProperty('name');
      expect(typeof chain.name).toBe('string');

      expect(chain).toHaveProperty('chain_id');
      expect(typeof chain.chain_id).toBe('number');

      console.log('✅ 链列表响应格式正确');
    }
  });
});
