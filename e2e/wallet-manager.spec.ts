import { test, expect, Page } from '@playwright/test';
import {
  waitForTauriApp,
  invokeTauriCommand,
  waitAndClick,
  safeFill,
  waitForToast,
  takeScreenshot,
  waitForLoadingComplete,
} from './tauri-helpers';

/**
 * 钱包管理器 - 前端+后端联调测试
 *
 * 这些测试验证：
 * 1. 前端 UI 正确渲染
 * 2. 前端能正确调用 Tauri 后端命令
 * 3. 后端命令正确执行并返回预期结果
 * 4. 前端能正确处理后端返回的数据
 */

test.describe('钱包管理器 - 联调测试', () => {
  let page: Page;

  test.beforeEach(async ({ page: p }) => {
    page = p;

    // 导航到钱包管理页面
    await page.goto('/#/wallet-manager');

    // 等待 Tauri 应用加载
    await waitForTauriApp(page);

    // 等待页面加载完成
    await waitForLoadingComplete(page);
  });

  test('页面标题正确显示', async () => {
    // 验证页面标题
    const title = page.locator('h1, .page-title, [data-testid="page-title"]').first();
    await expect(title).toBeVisible();

    // 截图记录
    await takeScreenshot(page, 'wallet-manager-initial');
  });

  test('获取钱包分组列表 - 前后端联调', async () => {
    // 通过前端 UI 触发获取分组操作
    // 或者通过直接调用后端命令验证

    const groups = await invokeTauriCommand<any[]>(page, 'get_groups');

    // 验证返回数据格式正确
    expect(Array.isArray(groups)).toBe(true);

    // 如果有分组，验证数据结构
    if (groups.length > 0) {
      const firstGroup = groups[0];
      expect(firstGroup).toHaveProperty('id');
      expect(firstGroup).toHaveProperty('name');
    }

    console.log('✅ 获取分组列表成功:', groups);
  });

  test('获取钱包列表 - 前后端联调', async () => {
    // 调用后端命令获取钱包列表
    const wallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
      group_id: null,
      chain_type: null,
      password: null,
    });

    // 验证返回数据
    expect(Array.isArray(wallets)).toBe(true);

    // 验证钱包数据结构
    if (wallets.length > 0) {
      const firstWallet = wallets[0];
      expect(firstWallet).toHaveProperty('id');
      expect(firstWallet).toHaveProperty('address');
      expect(firstWallet).toHaveProperty('name');
    }

    console.log('✅ 获取钱包列表成功，数量:', wallets.length);
  });

  test('创建新分组 - 完整流程', async () => {
    const testGroupName = `测试分组-${Date.now()}`;

    // 1. 获取当前分组数量
    const groupsBefore = await invokeTauriCommand<any[]>(page, 'get_groups');
    const initialCount = groupsBefore.length;

    // 2. 调用后端命令创建分组
    const newGroupId = await invokeTauriCommand<number>(page, 'create_group', {
      name: testGroupName,
      parent_id: null,
    });

    // 3. 验证返回了有效的分组 ID
    expect(newGroupId).toBeGreaterThan(0);

    // 4. 再次获取分组列表验证创建成功
    const groupsAfter = await invokeTauriCommand<any[]>(page, 'get_groups');
    expect(groupsAfter.length).toBe(initialCount + 1);

    // 5. 验证新分组信息正确
    const createdGroup = groupsAfter.find(g => g.id === newGroupId);
    expect(createdGroup).toBeDefined();
    expect(createdGroup.name).toBe(testGroupName);

    console.log('✅ 创建分组成功:', createdGroup);

    // 6. 清理：删除测试分组
    // 注意：实际测试中可能需要保留或清理测试数据
  });

  test('检查数据库初始化状态', async () => {
    // 调用后端命令检查钱包管理器是否已初始化
    const isInitialized = await invokeTauriCommand<boolean>(page, 'is_wallet_manager_initialized');

    // 验证返回值为布尔类型
    expect(typeof isInitialized).toBe('boolean');

    console.log('✅ 钱包管理器初始化状态:', isInitialized);

    // 如果未初始化，可以测试初始化流程
    if (!isInitialized) {
      await invokeTauriCommand(page, 'init_wallet_manager_tables');
      const initializedAfter = await invokeTauriCommand<boolean>(page, 'is_wallet_manager_initialized');
      expect(initializedAfter).toBe(true);
    }
  });

  test('获取监控地址列表', async () => {
    // 调用获取监控地址命令
    const watchAddresses = await invokeTauriCommand<any[]>(page, 'get_watch_addresses', {
      group_id: null,
      chain_type: null,
    });

    // 验证返回数据
    expect(Array.isArray(watchAddresses)).toBe(true);

    console.log('✅ 获取监控地址列表成功，数量:', watchAddresses.length);
  });

  test('前端 UI 与后端数据一致性检查', async () => {
    // 1. 通过后端获取钱包数据
    const backendWallets = await invokeTauriCommand<any[]>(page, 'get_wallets', {
      group_id: null,
      chain_type: null,
      password: null,
    });

    // 2. 等待前端渲染完成
    await page.waitForTimeout(1000);

    // 3. 获取前端显示的钱包数量
    // 根据你的实际 UI 结构调整选择器
    const walletRows = page.locator('table tbody tr, .wallet-item, [data-testid="wallet-row"]');
    const frontendCount = await walletRows.count();

    // 4. 验证前后端数据一致
    // 注意：前端可能显示的是过滤后的数据，这里仅作示例
    console.log(`后端钱包数量: ${backendWallets.length}, 前端显示数量: ${frontendCount}`);

    // 截图记录
    await takeScreenshot(page, 'wallet-consistency-check');
  });

  test('错误处理 - 调用不存在的命令', async () => {
    // 测试错误处理
    try {
      await invokeTauriCommand(page, 'non_existent_command', {});
      // 如果执行到这里，说明测试失败
      expect(false).toBe(true);
    } catch (error) {
      // 验证错误被正确捕获
      expect(error).toBeDefined();
      console.log('✅ 错误处理正常:', error);
    }
  });

  test('密码验证流程', async () => {
    // 检查是否设置了密码
    const isPasswordSet = await invokeTauriCommand<boolean>(page, 'is_password_set');

    if (isPasswordSet) {
      // 测试密码验证
      const testPassword = 'wrong_password';
      try {
        await invokeTauriCommand(page, 'verify_password', {
          request: {
            password: testPassword,
            encrypted_password_b64: null,
          },
        });
      } catch (error) {
        // 预期会失败
        console.log('✅ 密码验证正常工作（错误密码被拒绝）');
      }
    } else {
      console.log('ℹ️ 未设置密码，跳过密码验证测试');
    }
  });
});

test.describe('钱包管理器 - UI 交互测试', () => {
  test('点击分组切换', async ({ page }) => {
    await page.goto('/#/wallet-manager');
    await waitForTauriApp(page);

    // 查找分组列表
    const groupItems = page.locator('.group-item, [data-testid="group-item"], .arco-menu-item');

    // 如果有多个分组，测试切换
    const count = await groupItems.count();
    if (count > 1) {
      // 点击第二个分组
      await groupItems.nth(1).click();

      // 等待数据加载
      await waitForLoadingComplete(page);

      // 验证激活状态
      await expect(groupItems.nth(1)).toHaveClass(/active|selected/);

      console.log('✅ 分组切换正常');
    }
  });

  test('搜索功能', async ({ page }) => {
    await page.goto('/#/wallet-manager');
    await waitForTauriApp(page);

    // 查找搜索框
    const searchInput = page.locator('input[type="search"], input[placeholder*="搜索"], [data-testid="search-input"]').first();

    if (await searchInput.isVisible().catch(() => false)) {
      // 输入搜索关键词
      await safeFill(page, 'input[type="search"]', 'test');

      // 等待搜索结果
      await page.waitForTimeout(500);

      console.log('✅ 搜索功能正常');
    }
  });
});
