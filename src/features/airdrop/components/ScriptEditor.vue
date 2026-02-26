<script setup lang="ts">
import { ref, onMounted, nextTick, watch, computed } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import {
  IconPlus,
  IconFile,
  IconPlayArrow,
  IconSave,
  IconCode,
  IconDelete,
  IconBook,
  IconFullscreen,
  IconFullscreenExit,
  IconToBottom,
  IconToTop,
  IconCopy,
  IconCheck,
  IconRecord,
  IconClose
} from '@arco-design/web-vue/es/icon';
import ApiHelper from './ApiHelper.vue';
import ScriptRecorder from './ScriptRecorder.vue';
import { scriptService, walletService, profileService } from '../services/browserAutomationService';
import { ScriptExecutor } from '../services/scriptExecutor';

const scripts = ref([]);

const activeScript = ref(null);
const scriptContent = ref('');
const isNewModalVisible = ref(false);
const newScriptName = ref('');
const showApiHelper = ref(true);
const isFullscreen = ref(false);
const copiedCode = ref(false);
const loading = ref(false);

// 每个脚本独立的执行状态
const scriptExecutionStates = ref<Map<string, {
  executing: boolean;
  logs: any[];
  status: any;
  showPanel: boolean;
  executor: ScriptExecutor | null;
}>>(new Map());

// 当前活跃脚本的执行状态
const executing = computed(() => {
  if (!activeScript.value) return false;
  return scriptExecutionStates.value.get(activeScript.value.id)?.executing ?? false;
});

const executionLogs = computed(() => {
  if (!activeScript.value) return [];
  const logs = scriptExecutionStates.value.get(activeScript.value.id)?.logs;
  return logs ?? [];
});

const executionStatus = computed(() => {
  if (!activeScript.value) return null;
  return scriptExecutionStates.value.get(activeScript.value.id)?.status ?? null;
});

const showExecutionPanel = computed(() => {
  if (!activeScript.value) return false;
  return scriptExecutionStates.value.get(activeScript.value.id)?.showPanel ?? false;
});

const editingScriptId = ref(null);
const editNameInput = ref(null);
const editNameValue = ref('');

// 右侧工具面板标签: 'api' | 'recorder' | null
const activeToolTab = ref(null);

const getErrorMessage = (error) => {
  if (!error) return '未知错误';
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message || '未知错误';
  if (typeof error === 'object' && typeof error.message === 'string') return error.message;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
};

// 加载脚本
const loadScripts = async () => {
  loading.value = true;
  try {
    scripts.value = await scriptService.getScripts();
  } catch (error) {
    Message.error('加载脚本失败: ' + getErrorMessage(error));
  } finally {
    loading.value = false;
  }
};

const startEditName = async (script, event) => {
  event?.stopPropagation();
  editingScriptId.value = script.id;
  editNameValue.value = script.name;
  await nextTick();
  const inputEl = Array.isArray(editNameInput.value) ? editNameInput.value[0] : editNameInput.value;
  inputEl?.focus?.();
  inputEl?.select?.();
};

const saveEditName = async () => {
  const trimmedName = editNameValue.value.trim();
  if (!trimmedName) {
    editingScriptId.value = null;
    return;
  }
  const script = scripts.value.find(s => s.id === editingScriptId.value);
  if (script) {
    try {
      await scriptService.updateScript({ id: script.id, name: trimmedName });
      script.name = trimmedName;
      Message.success('名称已更新');
    } catch (error) {
      Message.error('更新失败: ' + getErrorMessage(error));
    }
  }
  editingScriptId.value = null;
};

const cancelEditName = () => {
  editingScriptId.value = null;
};

const handleNameKeydown = (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    saveEditName();
  } else if (event.key === 'Escape') {
    event.preventDefault();
    cancelEditName();
  }
};

const handleSelectScript = (script) => {
  activeScript.value = script;
  scriptContent.value = script.content;
};

const handleSave = async () => {
  if (!activeScript.value) {
    Message.warning('请先选择或创建一个脚本');
    return;
  }
  
  try {
    await scriptService.updateScript({
      id: activeScript.value.id,
      content: scriptContent.value
    });
    activeScript.value.content = scriptContent.value;
    Message.success('脚本已保存');
  } catch (error) {
    Message.error('保存失败: ' + getErrorMessage(error));
  }
};

// 获取或创建脚本的执行状态
const getOrCreateExecutionState = (scriptId: string) => {
  if (!scriptExecutionStates.value.has(scriptId)) {
    scriptExecutionStates.value.set(scriptId, {
      executing: false,
      logs: [],
      status: null,
      showPanel: false,
      executor: null
    });
  }
  return scriptExecutionStates.value.get(scriptId)!;
};

// 获取当前脚本的执行状态
const getCurrentExecutionState = () => {
  if (!activeScript.value) return null;
  return getOrCreateExecutionState(activeScript.value.id);
};

const handleRun = async () => {
  if (!activeScript.value) {
    Message.warning('请先选择要运行的脚本');
    return;
  }

  // 关闭右侧工具面板（包括脚本录制器）
  activeToolTab.value = null;

  const state = getOrCreateExecutionState(activeScript.value.id);

  if (state.executing) {
    Message.warning('该脚本正在执行中，请先停止');
    return;
  }

  try {
    // 加载钱包列表（测试运行不强制要求钱包）
    const wallets = await walletService.getWallets();

    // 加载环境配置
    const profiles = await profileService.getProfiles();
    const profile = profiles.length > 0 ? profiles[0] : null;

    // 使用第一个钱包进行测试运行（如果有）
    const testWallet = wallets.length > 0 ? wallets[0] : null;

    // 重置执行状态
    state.executing = true;
    state.logs = [];
    state.status = null;
    state.showPanel = true;

    Message.loading('正在启动测试运行...');

    // 构建浏览器配置 (使用 camelCase 与 TypeScript 类型定义一致)
    const browserConfig = profile ? {
      userAgent: profile.userAgent || 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
      viewportWidth: profile.viewportWidth || 1920,
      viewportHeight: profile.viewportHeight || 1080,
      deviceScaleFactor: profile.deviceScaleFactor || 1,
      locale: profile.locale || 'en-US',
      timezoneId: profile.timezoneId || 'America/New_York',
      proxyType: profile.proxyType || 'direct',
      proxyHost: profile.proxyHost,
      proxyPort: profile.proxyPort,
      proxyUsername: profile.proxyUsername,
      proxyPassword: profile.proxyPassword,
      canvasSpoof: profile.canvasSpoof !== false,
      webglSpoof: profile.webglSpoof !== false,
      audioSpoof: profile.audioSpoof !== false,
      timezoneSpoof: profile.timezoneSpoof !== false,
      geolocationSpoof: profile.geolocationSpoof !== false,
      fontSpoof: profile.fontSpoof !== false,
      webrtcSpoof: profile.webrtcSpoof !== false,
      navigatorOverride: profile.navigatorOverride !== false,
      webdriverOverride: profile.webdriverOverride !== false,
      headless: false,
      targetUrl: profile?.targetUrl || 'https://example.com'
    } : {
      userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
      viewportWidth: 1920,
      viewportHeight: 1080,
      deviceScaleFactor: 1,
      locale: 'en-US',
      timezoneId: 'America/New_York',
      proxyType: 'direct',
      canvasSpoof: true,
      webglSpoof: true,
      audioSpoof: true,
      timezoneSpoof: true,
      geolocationSpoof: true,
      fontSpoof: true,
      webrtcSpoof: true,
      navigatorOverride: true,
      webdriverOverride: true,
      headless: false,
      targetUrl: 'https://example.com'
    };

    // 构建执行配置 - 如果没有钱包，创建一个虚拟钱包以确保浏览器能启动
    const walletsConfig = testWallet ? [{
      id: testWallet.id?.toString() || '1',
      name: testWallet.name || 'Test Wallet',
      address: testWallet.address,
      privateKey: '', // 将在后端解密
      chainType: testWallet.chainType || 'ethereum'
    }] : [{
      id: 'test-wallet',
      name: 'Test Wallet',
      address: '0x0000000000000000000000000000000000000000',
      privateKey: '',
      chainType: 'ethereum'
    }];

    const config = {
      targetUrl: profile?.targetUrl || 'https://example.com',
      config: browserConfig,
      userCode: scriptContent.value,
      wallets: walletsConfig,
      concurrency: 1,
      timeoutSecs: 300
    };

    console.log('[ScriptEditor] 执行配置:', JSON.stringify(config, null, 2));

    // 创建新的执行器并保存引用
    const executor = new ScriptExecutor();
    state.executor = executor;

    // 创建会话
    console.log('[ScriptEditor] 正在创建会话...');
    await executor.createSession(config);
    console.log('[ScriptEditor] 会话创建成功, sessionId:', executor.getSessionId());

    // 订阅日志
    await executor.subscribeLogs((log: any) => {
      const currentState = scriptExecutionStates.value.get(activeScript.value!.id);
      if (currentState) {
        currentState.logs.push(log);
        // 限制日志数量，避免内存溢出
        if (currentState.logs.length > 1000) {
          currentState.logs = currentState.logs.slice(-500);
        }
      }
    });

    // 启动执行
    console.log('[ScriptEditor] 正在启动执行...');
    await executor.startExecution(
      (status: any) => {
        console.log('[ScriptEditor] 状态更新:', status);
        const currentState = scriptExecutionStates.value.get(activeScript.value!.id);
        if (currentState) {
          currentState.status = status;
        }
      },
      (success: boolean, error?: string) => {
        console.log('[ScriptEditor] 执行完成, success:', success, 'error:', error);
        const currentState = scriptExecutionStates.value.get(activeScript.value!.id);
        if (currentState) {
          currentState.executing = false;
          // 更新状态为完成或失败
          if (currentState.status) {
            currentState.status.status = success ? 'completed' : 'failed';
            // 如果有运行中的钱包，将其计入失败或成功
            const runningCount = currentState.status.runningWallets || 0;
            if (!success && runningCount > 0) {
              currentState.status.failedWallets = (currentState.status.failedWallets || 0) + runningCount;
            } else if (success && runningCount > 0) {
              currentState.status.completedWallets = (currentState.status.completedWallets || 0) + runningCount;
            }
            // 确保运行中数量为0
            currentState.status.runningWallets = 0;
          }
        }
        if (success) {
          Message.success('测试运行完成');
        } else {
          Message.error('测试运行失败: ' + (error || '未知错误'));
        }
        // 清理资源
        executor?.cleanup().catch(console.error);
        if (currentState) {
          currentState.executor = null;
        }
      },
      1000
    );

    console.log('[ScriptEditor] 执行已启动');
    if (testWallet) {
      Message.success(`测试运行已启动，使用钱包: ${testWallet.name || testWallet.address.slice(0, 8)}...`);
    } else {
      Message.success('测试运行已启动（无钱包）');
    }

  } catch (error) {
    state.executing = false;
    state.executor = null;
    // 更新状态为失败
    if (state.status) {
      // 先读取运行中数量，再重置
      const runningCount = state.status.runningWallets || 0;
      state.status.status = 'failed';
      // 将运行中的任务计入失败
      if (runningCount > 0) {
        state.status.failedWallets = (state.status.failedWallets || 0) + runningCount;
      }
      // 确保运行中数量为0
      state.status.runningWallets = 0;
      // 如果没有总任务数，设置默认值
      if (!state.status.totalWallets || state.status.totalWallets === 0) {
        state.status.totalWallets = 1;
        state.status.failedWallets = 1;
      }
    } else {
      // 如果没有状态对象，创建一个失败状态
      state.status = {
        status: 'failed',
        totalWallets: 1,
        completedWallets: 0,
        failedWallets: 1,
        runningWallets: 0
      };
    }
    Message.error('测试运行失败: ' + getErrorMessage(error));
  }
};

const handleStopExecution = async () => {
  const state = getCurrentExecutionState();
  if (!state || !state.executing || !state.executor) return;

  try {
    // 使用当前执行器实例取消
    await state.executor.cancel();
    state.executing = false;
    state.executor = null;
    Message.success('已停止执行');
  } catch (error) {
    Message.error('停止失败: ' + getErrorMessage(error));
  }
};

const clearExecutionLogs = () => {
  const state = getCurrentExecutionState();
  if (state) {
    state.logs = [];
  }
};

const hideExecutionPanel = () => {
  const state = getCurrentExecutionState();
  if (state) {
    state.showPanel = false;
  }
};

const handleNewScript = () => {
  isNewModalVisible.value = true;
  newScriptName.value = '';
};

const confirmNewScript = async () => {
  if (!newScriptName.value.trim()) {
    Message.error('请输入脚本名称');
    return;
  }

  const defaultContent = `// ${newScriptName.value.trim()}
// 自定义 visitPage 逻辑
// 
// 可用参数通过 context 对象传入：
//   - manager: BrowserManager 实例 { browser, context, page, browserIndex }
//   - url: 目标 URL
//   - visitIndex: 当前访问序号
//   - totalVisits: 总访问次数
//   - wallet: 当前钱包信息 { name, address, private_key, chain_type }
//   - api: 工具 API 对象
//
// API 方法：
//   - api.log(level, message): 输出日志 (level: 'info' | 'warn' | 'error' | 'success')
//   - api.randomDelay(min, max): 随机延迟（毫秒）
//   - api.sleep(ms): 固定延迟（毫秒）
//   - api.humanLikeClick(page, selector): 模拟人类点击
//   - api.humanLikeScroll(page, options): 模拟人类滚动
//   - api.humanLikeMouseMove(page, x, y): 模拟人类鼠标移动
//
// Playwright page 方法：
//   - page.goto(url, options): 访问页面
//   - page.click(selector): 点击元素
//   - page.fill(selector, text): 填写输入框
//   - page.waitForSelector(selector): 等待元素出现
//   - page.evaluate(fn): 在页面中执行 JavaScript

async function visitPage({ manager, url, visitIndex, totalVisits, wallet, api }) {
  const { page, browserIndex } = manager;
  
  api.log('info', \`[\${browserIndex}] 钱包 \${wallet.name} 开始访问 \${url}\`);
  
  // 访问页面
  await page.goto(url, {
    waitUntil: 'domcontentloaded',
    timeout: 60000
  });
  
  // 等待页面加载
  await api.randomDelay(2000, 4000);
  
  // TODO: 在此添加您的自定义操作
  // 示例：
  // await api.humanLikeClick(page, '.claim-button');
  // await api.sleep(3000);
  
  api.log('success', '执行完成');
  return { success: true };
}`;

  try {
    const newScript = await scriptService.createScript({
      name: newScriptName.value.trim(),
      content: defaultContent,
      description: ''
    });
    
    scripts.value.push(newScript);
    handleSelectScript(newScript);
    isNewModalVisible.value = false;
    Message.success('创建成功');
  } catch (error) {
    Message.error('创建失败: ' + getErrorMessage(error));
  }
};

const handleDeleteScript = async (e, scriptId) => {
  e.stopPropagation();
  const script = scripts.value.find(s => s.id === scriptId);
  Modal.warning({
    title: '确认删除',
    content: `确定要删除脚本 "${script?.name || ''}" 吗？此操作不可恢复。`,
    onOk: async () => {
      try {
        await scriptService.deleteScript(scriptId);
        scripts.value = scripts.value.filter(s => s.id !== scriptId);
        if (activeScript.value && activeScript.value.id === scriptId) {
          activeScript.value = null;
          scriptContent.value = '';
        }
        Message.success('删除成功');
      } catch (error) {
        Message.error('删除失败: ' + getErrorMessage(error));
      }
    }
  });
};

// 导入脚本
const handleImportScript = async () => {
  try {
    const imported = await scriptService.importScript();
    if (imported) {
      // 保存到数据库
      const newScript = await scriptService.createScript({
        name: imported.name,
        content: imported.content,
        description: imported.description || ''
      });
      scripts.value.push(newScript);
      handleSelectScript(newScript);
      Message.success('导入成功');
    }
  } catch (error) {
    Message.error('导入失败: ' + getErrorMessage(error));
  }
};

// 导出脚本
const handleExportScript = async () => {
  if (!activeScript.value) {
    Message.warning('请先选择要导出的脚本');
    return;
  }
  try {
    await scriptService.exportScript(activeScript.value);
    Message.success('导出成功');
  } catch (error) {
    Message.error('导出失败: ' + getErrorMessage(error));
  }
};

// 复制脚本内容
const handleCopyScript = async () => {
  if (!activeScript.value) return;
  try {
    await navigator.clipboard.writeText(scriptContent.value);
    copiedCode.value = true;
    Message.success('已复制到剪贴板');
    setTimeout(() => {
      copiedCode.value = false;
    }, 2000);
  } catch (e) {
    Message.error('复制失败');
  }
};

const handleInsertCode = (code) => {
  if (activeScript.value) {
    scriptContent.value += '\n' + code;
    Message.success('代码已插入');
  }
};

// 处理录制脚本保存成功
const handleScriptSaved = (script) => {
  // 刷新脚本列表
  loadScripts();
  Message.success(`脚本 "${script.name}" 已保存到数据库`);
};

// 处理录制脚本更新成功
const handleScriptUpdated = (script) => {
  // 更新当前脚本内容
  if (activeScript.value && activeScript.value.id === script.id) {
    activeScript.value.content = script.content;
    scriptContent.value = script.content;
  }
  // 刷新脚本列表
  loadScripts();
  Message.success(`脚本 "${script.name}" 已更新`);
};

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value;
};

// 切换右侧工具面板标签
const setToolTab = (tab) => {
  if (activeToolTab.value === tab) {
    activeToolTab.value = null; // 再次点击关闭
  } else {
    activeToolTab.value = tab;
  }
};

// 键盘快捷键
const handleKeydown = (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault();
    handleSave();
  }
};

onMounted(() => {
  loadScripts();
  window.addEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div class="script-editor" :class="{ fullscreen: isFullscreen }">
    <div class="script-list" v-if="!isFullscreen">
      <div class="list-header">
        <h3>脚本列表</h3>
        <a-space>
          <a-button type="secondary" size="small" @click="handleImportScript" title="导入脚本">
            <template #icon><IconToBottom /></template>
          </a-button>
          <a-button type="primary" size="small" @click="handleNewScript" title="创建新脚本" style="margin-left: 10px;">
            <template #icon><IconPlus /></template>
          </a-button>
        </a-space>
      </div>

      <div class="list-content" v-loading="loading">
        <div
          v-for="script in scripts"
          :key="script.id"
          class="script-item"
          :class="{ active: activeScript && activeScript.id === script.id }"
          @click="handleSelectScript(script)"
        >
          <div class="item-main">
            <IconCode />
            <template v-if="editingScriptId === script.id">
              <input
                ref="editNameInput"
                v-model="editNameValue"
                class="name-edit-input"
                @blur="saveEditName"
                @keydown="handleNameKeydown"
                @click.stop
              />
            </template>
            <template v-else>
              <span class="script-name editable" @click="(e) => startEditName(script, e)" title="点击编辑名称">
                {{ script.name }}
              </span>
            </template>
          </div>
          <div class="item-actions">
            <IconDelete class="delete-icon" @click="(e) => handleDeleteScript(e, script.id)" />
          </div>
        </div>
        <div v-if="!scripts || scripts.length === 0" class="empty-scripts">
          暂无脚本，点击 + 创建新脚本
        </div>
      </div>
    </div>

    <div class="editor-main" v-if="activeScript" :style="{ width: isFullscreen ? '100%' : (activeToolTab ? 'calc(100% - 270px - 400px)' : 'calc(100% - 270px)'), flex: activeToolTab ? 'none' : 1 }">
      <div class="editor-area">
        <div class="editor-toolbar">
          <div class="file-info">
            <IconFile />
            <span>{{ activeScript.name }}</span>
          </div>
          <div class="actions">
            <a-tooltip content="API 参考文档">
              <a-button type="text" size="small" @click="setToolTab('api')" :status="activeToolTab === 'api' ? 'primary' : 'normal'" v-if="!isFullscreen">
                <template #icon><IconBook /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip content="脚本录制">
              <a-button type="text" size="small" @click="setToolTab('recorder')" :status="activeToolTab === 'recorder' ? 'warning' : 'normal'" v-if="!isFullscreen">
                <template #icon><IconRecord /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip content="全屏编辑">
              <a-button type="text" size="small" @click="toggleFullscreen">
                <template #icon><IconFullscreen v-if="!isFullscreen" /><IconFullscreenExit v-else /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip content="导出脚本">
              <a-button type="text" size="small" @click="handleExportScript">
                <template #icon><IconToTop /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip content="复制代码">
              <a-button type="text" size="small" @click="handleCopyScript">
                <template #icon><IconCheck v-if="copiedCode" /><IconCopy v-else /></template>
              </a-button>
            </a-tooltip>
            <a-button
              v-if="!executing"
              type="secondary"
              size="small"
              @click="handleRun"
            >
              <template #icon><IconPlayArrow /></template>
              测试运行
            </a-button>
            <a-button
              v-else
              type="primary"
              status="danger"
              size="small"
              @click="handleStopExecution"
            >
              <template #icon><IconRecord /></template>
              停止执行
            </a-button>
            <a-button type="primary" size="small" @click="handleSave">
              <template #icon><IconSave /></template>
              保存
            </a-button>
          </div>
        </div>

        <div class="code-container">
          <textarea
            v-model="scriptContent"
            class="code-input"
            spellcheck="false"
            placeholder="在此编写 Playwright 脚本..."
          ></textarea>
        </div>

        <div class="editor-footer">
          <div class="script-tips">
            <span>提示: 使用 api. 调用自定义方法，如 api.connectMetaMask() | 按 Ctrl+S 保存</span>
          </div>
        </div>
      </div>
    </div>

    <div class="tool-panel" v-if="activeToolTab && !isFullscreen && activeScript">
      <ApiHelper v-if="activeToolTab === 'api'" @insert-code="handleInsertCode" />
      <ScriptRecorder v-if="activeToolTab === 'recorder'" :current-script="activeScript" @insert-code="handleInsertCode" @close="activeToolTab = null" @script-saved="handleScriptSaved" @script-updated="handleScriptUpdated" />
    </div>

    <!-- 执行面板 -->
    <div class="execution-panel" v-if="showExecutionPanel && !isFullscreen">
      <div class="execution-header">
        <div class="execution-title">
          <span>执行日志</span>
          <a-tag v-if="executing" color="blue" size="small">运行中</a-tag>
          <a-tag v-else-if="executionStatus?.status === 'completed'" color="green" size="small">已完成</a-tag>
          <a-tag v-else-if="executionStatus?.status === 'cancelled'" color="orange" size="small">已取消</a-tag>
        </div>
        <div class="execution-actions">
          <a-button type="text" size="mini" @click="clearExecutionLogs">
            <template #icon><IconDelete /></template>
          </a-button>
          <a-button type="text" size="mini" @click="hideExecutionPanel">
            <IconClose />
          </a-button>
        </div>
      </div>

      <div class="execution-progress" v-if="executionStatus">
        <div class="progress-stats">
          <span>总任务: {{ executionStatus.totalWallets }}</span>
          <span class="success">成功: {{ executionStatus.completedWallets }}</span>
          <span class="error">失败: {{ executionStatus.failedWallets }}</span>
          <span class="running" v-if="executionStatus.runningWallets > 0">运行中: {{ executionStatus.runningWallets }}</span>
        </div>
        <a-progress
          :percent="executionStatus.totalWallets > 0 ? Math.round(((executionStatus.completedWallets + executionStatus.failedWallets) / executionStatus.totalWallets) * 100) / 100 : 0"
          :status="executing ? 'normal' : 'success'"
          size="small"
        />
      </div>

      <div class="execution-logs" ref="logContainer">
        <div
          v-for="(log, index) in executionLogs"
          :key="index"
          class="log-item"
          :class="log.level"
        >
          <span class="log-time">{{ new Date(log.timestamp).toLocaleTimeString() }}</span>
          <span class="log-level" :class="log.level">[{{ log.level.toUpperCase() }}]</span>
          <span class="log-wallet" v-if="log.walletId">[{{ log.walletId.slice(0, 8) }}]</span>
          <span class="log-message">{{ log.message }}</span>
        </div>
        <div v-if="!executionLogs || executionLogs.length === 0" class="empty-logs">
          暂无日志，等待执行开始...
        </div>
      </div>
    </div>

    <div class="empty-state" v-if="!activeScript">
      <IconCode style="font-size: 48px; color: var(--color-text-4)" />
      <p>请选择左侧脚本进行编辑，或创建新脚本</p>
      <a-space>
        <a-button type="primary" @click="handleNewScript">创建新脚本</a-button>
        <a-button type="outline" @click="handleImportScript" style="margin-left: 10px;">导入脚本</a-button>
      </a-space>
    </div>

    <!-- New Script Modal -->
    <a-modal v-model:visible="isNewModalVisible" title="新建脚本" @ok="confirmNewScript">
      <a-form-item label="脚本名称">
        <a-input v-model="newScriptName" placeholder="e.g., My Airdrop Task" @press-enter="confirmNewScript" />
      </a-form-item>
    </a-modal>
  </div>
</template>

<style scoped>
.script-editor {
  height: 100%;
  display: flex;
  gap: 10px;
}

.script-editor.fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  background: var(--color-bg-1);
  padding: 20px;
}

.script-list {
  width: 250px;
  background: var(--color-bg-2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
}

.list-header {
  padding: 10px 15px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.list-header h3 {
  margin: 0;
  font-size: 14px;
  color: var(--color-text-2);
}

.list-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

.script-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  border-radius: 6px;
  cursor: pointer;
  color: var(--color-text-2);
  transition: all 0.2s;
}

.item-main {
  display: flex;
  align-items: center;
  gap: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.script-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.script-name.editable {
  cursor: text;
  padding: 2px 4px;
  margin: -2px -4px;
  border-radius: 4px;
  transition: all 0.2s;
}

.script-name.editable:hover {
  background: var(--color-fill-2);
}

.name-edit-input {
  background: var(--color-bg-1);
  border: 1px solid rgb(var(--primary-6));
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 14px;
  color: var(--color-text-1);
  outline: none;
  line-height: 1.2;
  max-width: 150px;
}

.name-edit-input:focus {
  border-color: rgb(var(--primary-6));
  box-shadow: 0 0 0 2px rgba(var(--primary-6), 0.2);
}

.item-actions {
  opacity: 0;
  transition: opacity 0.2s;
}

.script-item:hover {
  background: var(--color-fill-2);
  color: var(--color-text-1);
}

.script-item:hover .item-actions {
  opacity: 1;
}

.delete-icon {
  cursor: pointer;
}

.delete-icon:hover {
  color: rgb(var(--danger-6));
}

.script-item.active {
  background: rgba(var(--primary-6), 0.1);
  color: rgb(var(--primary-6));
}

.empty-scripts {
  text-align: center;
  padding: 30px 20px;
  color: var(--color-text-3);
  font-size: 12px;
}

.editor-main {
  display: flex;
  flex-direction: column;
  transition: width 0.3s ease;
}

.editor-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-2);
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

.editor-toolbar {
  padding: 8px 15px;
  background: var(--color-bg-3);
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.file-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--color-text-3);
}

.actions {
  display: flex;
  gap: 8px;
}

.code-container {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.code-input {
  width: 100%;
  height: 100%;
  background: var(--color-bg-1);
  color: var(--color-text-1);
  border: none;
  padding: 15px;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 14px;
  line-height: 1.6;
  resize: none;
  outline: none;
  tab-size: 2;
  overflow-y: auto;
}

/* 滚动条样式 - 适配深浅主题 */
.code-input::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.code-input::-webkit-scrollbar-track {
  background: var(--color-fill-2);
  border-radius: 4px;
}

.code-input::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 4px;
}

.code-input::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox 滚动条 */
.code-input {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) var(--color-fill-2);
}

.editor-footer {
  padding: 8px 15px;
  background: var(--color-bg-3);
  border-top: 1px solid var(--color-border);
}

.script-tips {
  font-size: 12px;
  color: var(--color-text-4);
}

.tool-panel {
  width: 400px;
  flex-shrink: 0;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-bg-2);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-4);
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 2px dashed var(--color-border);
  gap: 10px;
}

/* 执行面板样式 */
.execution-panel {
  width: 400px;
  flex-shrink: 0;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-bg-2);
  display: flex;
  flex-direction: column;
}

.execution-header {
  padding: 12px 15px;
  background: var(--color-bg-3);
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.execution-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}

.execution-actions {
  display: flex;
  gap: 4px;
}

.execution-progress {
  padding: 12px 15px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-1);
}

.progress-stats {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;
  font-size: 12px;
  color: var(--color-text-3);
}

.progress-stats .success {
  color: rgb(var(--success-6));
}

.progress-stats .error {
  color: rgb(var(--danger-6));
}

.progress-stats .running {
  color: rgb(var(--primary-6));
}

.execution-logs {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  background: var(--color-bg-1);
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.6;
}

.log-item {
  padding: 4px 0;
  border-bottom: 1px solid var(--color-border-2);
  word-break: break-all;
}

.log-item:last-child {
  border-bottom: none;
}

.log-time {
  color: var(--color-text-4);
  margin-right: 8px;
}

.log-level {
  font-weight: 500;
  margin-right: 8px;
}

.log-level.info {
  color: rgb(var(--primary-6));
}

.log-level.success {
  color: rgb(var(--success-6));
}

.log-level.warn {
  color: rgb(var(--warning-6));
}

.log-level.error {
  color: rgb(var(--danger-6));
}

.log-wallet {
  color: var(--color-text-3);
  margin-right: 8px;
}

.log-message {
  color: var(--color-text-2);
}

.empty-logs {
  text-align: center;
  padding: 40px 20px;
  color: var(--color-text-4);
  font-style: italic;
}
</style>
