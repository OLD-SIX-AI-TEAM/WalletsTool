<script setup lang="ts">
import { ref, shallowRef, onMounted, nextTick, defineAsyncComponent, computed } from 'vue';
import { useThemeStore } from '../../../stores';
import { Message, Modal } from '@arco-design/web-vue';
import WalletManager from '../components/WalletManager.vue';
import EnvCheckOverlay from '../components/EnvCheckOverlay.vue';
import {
  IconSafe,
  IconComputer,
  IconCode,
  IconPlayCircle,
  IconPoweroff,
  IconFolder,
  IconSchedule,
  IconPushpin,
  IconApps,
  IconMinus,
  IconExpand,
  IconShrink
} from '@arco-design/web-vue/es/icon';
import { cliCheckService, type CliToolStatus } from '../services/cliCheckService';

const themeStore = useThemeStore();
const isDarkTheme = computed(() => themeStore.getEffectiveTheme() === 'dark');

// CLI 工具检查状态
const cliCheckLoading = ref(false);
const cliCheckResult = ref<CliToolStatus[]>([]);
const showCliCheckModal = ref(false);

// 环境检查状态
const isEnvReady = ref(false);
const envCheckFailed = ref(false);

// 页面加载时检查 localStorage 缓存
const checkLocalStorageCache = () => {
  const cachedResult = cliCheckService.getCachedResult();
  if (cachedResult && cachedResult.all_installed) {
    // 有有效缓存且检查通过，直接设置环境就绪
    console.log('[BrowserAutomation] 使用 localStorage 缓存，跳过环境检查');
    isEnvReady.value = true;
    return true;
  }
  return false;
};

// 立即检查缓存
const hasCache = checkLocalStorageCache();

const BrowserFarm = defineAsyncComponent(() => import('../components/BrowserFarm.vue'));
const ScriptEditor = defineAsyncComponent(() => import('../components/ScriptEditor.vue'));
const ExecutionPanel = defineAsyncComponent(() => import('../components/ExecutionPanel.vue'));
const TaskManager = defineAsyncComponent(() => import('../components/TaskManager.vue'));
const TaskMonitor = defineAsyncComponent(() => import('../components/TaskMonitor.vue'));
const ExtensionManager = defineAsyncComponent(() => import('../components/ExtensionManager.vue'));

const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
let appWindow = null;

if (isTauri) {
  import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
    appWindow = getCurrentWindow();
    Promise.resolve().then(() => {
      appWindow.emit('page-loaded');
    });
    // 监听窗口最大化状态变化
    appWindow.onResized(async () => {
      isMaximized.value = await appWindow.isMaximized();
    });
  });
}

const menuItems = [
  { id: 'wallets', label: '钱包管理', icon: IconSafe, component: WalletManager },
  { id: 'envs', label: '环境配置', icon: IconComputer, component: BrowserFarm },
  { id: 'extensions', label: '插件管理', icon: IconApps, component: ExtensionManager },
  { id: 'scripts', label: '脚本编辑', icon: IconCode, component: ScriptEditor },
  { id: 'tasks', label: '任务管理', icon: IconFolder, component: TaskManager },
  { id: 'monitor', label: '任务监控', icon: IconSchedule, component: TaskMonitor },
  { id: 'execution', label: '执行面板', icon: IconPlayCircle, component: ExecutionPanel },
];

const activeTab = ref('wallets');
const currentComponent = shallowRef(WalletManager);
const isExpanded = ref(true);
const isPinned = ref(true);
const isMaximized = ref(false);

onMounted(async () => {
  // 环境检查由 EnvCheckOverlay 组件处理
  // 页面加载时检查 CLI 工具（仅用于显示状态）
  await checkCliTools();
  
  // 初始化窗口最大化状态
  if (appWindow) {
    isMaximized.value = await appWindow.isMaximized();
  }
});

// 环境检查通过
const handleEnvReady = () => {
  // 延迟2秒后关闭遮罩层，让用户看到检查结果
  setTimeout(() => {
    isEnvReady.value = true;
    envCheckFailed.value = false;
  }, 1000);
};

// 环境检查失败
const handleEnvFailed = (tools: CliToolStatus[]) => {
  isEnvReady.value = false;
  envCheckFailed.value = true;
  Message.error('环境安装失败，请手动安装所需工具');
};

// 检查 CLI 工具
const checkCliTools = async () => {
  cliCheckLoading.value = true;
  try {
    const result = await cliCheckService.checkTools();
    cliCheckResult.value = result.tools;

    // 如果有未安装的工具，显示提示
    const missingTools = result.tools.filter(t => !t.installed && t.name !== 'Playwright');
    if (missingTools.length > 0) {
      const toolNames = missingTools.map(t => t.name).join('、');
      const instructions = cliCheckService.getInstallInstructions(missingTools);

      Modal.warning({
        title: '缺少必要的 CLI 工具',
        content: `检测到以下工具未安装：${toolNames}\n\n${instructions}`,
        okText: '我知道了',
        width: 600,
      });
    }
  } catch (error) {
    console.error('检查 CLI 工具失败:', error);
    Message.error('检查 CLI 工具失败，请确保系统环境正常');
  } finally {
    cliCheckLoading.value = false;
  }
};

const handleNavClick = (item) => {
  activeTab.value = item.id;
  currentComponent.value = item.component;
};

const handleMouseEnter = () => {
  if (!isPinned.value) {
    isExpanded.value = true;
  }
};

const handleMouseLeave = () => {
  if (!isPinned.value) {
    isExpanded.value = false;
  }
};

const togglePin = () => {
  isPinned.value = !isPinned.value;
  if (isPinned.value) {
    isExpanded.value = true;
  }
};

const closeWindow = async () => {
  if (appWindow) {
    await appWindow.destroy();
  } else if (typeof window !== 'undefined') {
    window.close();
  }
};

const minimizeWindow = async () => {
  if (appWindow) {
    await appWindow.minimize();
  }
};

const toggleMaximize = async () => {
  if (appWindow) {
    if (isMaximized.value) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
    isMaximized.value = !isMaximized.value;
  }
};

</script>

<template>
  <div class="browser-automation-layout" :class="{ 'light-theme': !isDarkTheme }">
    <!-- 环境检查遮罩层 -->
    <EnvCheckOverlay 
      v-if="!isEnvReady"
      @ready="handleEnvReady"
      @failed="handleEnvFailed"
    />
    
    <div class="layout-body">
      <!-- Sidebar Container -->
      <div class="sidebar-container" :class="{ expanded: isExpanded }">
        <!-- Collapsed Sidebar -->
        <div class="sidebar-collapsed" v-show="!isExpanded" @mouseenter="handleMouseEnter">
          <div class="nav-menu-collapsed">
            <div 
              v-for="item in menuItems" 
              :key="item.id" 
              class="nav-item-collapsed"
              :class="{ active: activeTab === item.id }"
              @click="handleNavClick(item)"
              :title="item.label"
            >
              <component :is="item.icon" class="nav-icon-collapsed" />
            </div>
          </div>

          <div class="sidebar-footer-collapsed">
            <div class="nav-item-collapsed maximize-btn" @click="toggleMaximize" :title="isMaximized ? '还原' : '最大化'">
              <component :is="isMaximized ? IconShrink : IconExpand" class="nav-icon-collapsed" />
            </div>
            <div class="nav-item-collapsed minimize-btn" @click="minimizeWindow" title="最小化">
              <IconMinus class="nav-icon-collapsed" />
            </div>
            <div class="nav-item-collapsed close-btn" @click="closeWindow" title="关闭窗口">
              <IconPoweroff class="nav-icon-collapsed" />
            </div>
          </div>
        </div>

        <!-- Expanded Sidebar -->
        <div class="sidebar-expanded" v-show="isExpanded" @mouseleave="handleMouseLeave">
          <div class="sidebar-header">
            <div 
              class="pin-btn"
              :class="{ pinned: isPinned }"
              @click="togglePin"
              :title="isPinned ? '取消固定' : '固定侧边栏'"
            >
              <IconPushpin class="pin-icon" :class="{ 'is-pinned': isPinned }" />
            </div>
          </div>

          <div class="nav-menu-expanded">
            <div 
              v-for="item in menuItems" 
              :key="item.id" 
              class="nav-item-expanded"
              :class="{ active: activeTab === item.id }"
              @click="handleNavClick(item)"
            >
              <component :is="item.icon" class="nav-icon-expanded" />
              <span class="nav-label-expanded">{{ item.label }}</span>
            </div>
          </div>

          <div class="sidebar-footer-expanded">
            <div class="nav-item-expanded maximize-btn" @click="toggleMaximize">
              <component :is="isMaximized ? IconShrink : IconExpand" class="nav-icon-expanded" />
              <span class="nav-label-expanded">{{ isMaximized ? '还原' : '最大化' }}</span>
            </div>
            <div class="nav-item-expanded minimize-btn" @click="minimizeWindow">
              <IconMinus class="nav-icon-expanded" />
              <span class="nav-label-expanded">最小化</span>
            </div>
            <div class="nav-item-expanded close-btn" @click="closeWindow">
              <IconPoweroff class="nav-icon-expanded" />
              <span class="nav-label-expanded">退出</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Main Content -->
      <div class="main-content">
        <header class="content-header" data-tauri-drag-region>
          <h2>{{ menuItems.find(i => i.id === activeTab)?.label }}</h2>
        </header>
        
        <div class="content-body">
          <component :is="currentComponent" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.browser-automation-layout {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: var(--color-bg-1);
  color: var(--color-text-1);
  overflow: hidden;
}

.layout-body {
  display: flex;
  flex: 1;
  height: 100vh;
  overflow: hidden;
}

/* Sidebar Container */
.sidebar-container {
  width: 60px;
  flex-shrink: 0;
  transition: width 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}

.sidebar-container.expanded {
  width: 180px;
}

/* Collapsed Sidebar */
.sidebar-collapsed {
  width: 60px;
  height: 100%;
  background: var(--color-bg-2);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 0;
}

.nav-menu-collapsed {
  flex: 1;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.nav-item-collapsed {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-3);
  box-sizing: border-box;
}

.nav-icon-collapsed {
  font-size: 20px;
  width: 20px;
  height: 20px;
}

.nav-item-collapsed:hover {
  background: var(--color-fill-2);
  color: var(--color-text-1);
}

.nav-item-collapsed.active {
  background: rgba(var(--primary-6), 0.1);
  color: rgb(var(--primary-6));
}

.sidebar-footer-collapsed {
  padding-bottom: 10px;
}

/* Expanded Sidebar */
.sidebar-expanded {
  width: 180px;
  height: 100%;
  background: var(--color-bg-2);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  padding: 0 0 20px 0;
}

.sidebar-header {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 10px;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 8px;
}

.pin-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-3);
}

.pin-btn:hover {
  background: var(--color-fill-2);
  color: var(--color-text-1);
}

.pin-btn.pinned {
  color: rgb(var(--primary-6));
}

.pin-btn.pinned:hover {
  background: rgba(var(--primary-6), 0.1);
}

.pin-icon {
  font-size: 16px;
  width: 16px;
  height: 16px;
  transition: all 0.2s;
}

.pin-icon.is-pinned {
  transform: rotate(-45deg);
  color: rgb(var(--primary-6));
}

.nav-menu-expanded {
  flex: 1;
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 10px;
  box-sizing: border-box;
}

.nav-item-expanded {
  display: flex;
  align-items: center;
  height: 44px;
  padding: 0 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-3);
  white-space: nowrap;
  box-sizing: border-box;
}

.nav-icon-expanded {
  font-size: 20px;
  width: 20px;
  height: 20px;
  flex-shrink: 0;
}

.nav-label-expanded {
  margin-left: 12px;
  font-size: 14px;
  font-weight: 500;
}

.nav-item-expanded:hover {
  background: var(--color-fill-2);
  color: var(--color-text-1);
}

.nav-item-expanded.active {
  background: rgba(var(--primary-6), 0.1);
  color: rgb(var(--primary-6));
}

.sidebar-footer-expanded {
  width: 100%;
  padding: 0 10px 10px;
  box-sizing: border-box;
}

.close-btn:hover {
  color: rgb(var(--danger-6));
  background: rgba(var(--danger-6), 0.1);
}

.minimize-btn:hover {
  color: rgb(var(--primary-6));
  background: rgba(var(--primary-6), 0.1);
}

.maximize-btn:hover {
  color: rgb(var(--success-6));
  background: rgba(var(--success-6), 0.1);
}

/* Main Content */
.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-1);
  overflow: hidden;
}

.content-header {
  height: 44px;
  padding: 0 24px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  background: var(--color-bg-2);
}

.content-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
  color: var(--color-text-1);
}

.content-body {
  flex: 1;
  padding: 10px;
  overflow: hidden;
}

.light-theme {
  background: #f5f7fa;
}

.light-theme .sidebar-container {
  background: rgba(255, 255, 255, 0.95);
}

.light-theme .sidebar-collapsed,
.light-theme .sidebar-expanded {
  background: rgba(255, 255, 255, 0.95);
  border-color: rgba(0, 0, 0, 0.08);
}

.light-theme .sidebar-header {
  border-color: rgba(0, 0, 0, 0.08);
}

.light-theme .pin-btn {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .pin-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.85);
}

.light-theme .pin-btn.pinned {
  color: #586cc7;
}

.light-theme .pin-btn.pinned:hover {
  background: rgba(88, 108, 199, 0.1);
}

.light-theme .nav-item-collapsed,
.light-theme .nav-item-expanded {
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .nav-item-collapsed:hover,
.light-theme .nav-item-expanded:hover {
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.85);
}

.light-theme .nav-item-collapsed.active,
.light-theme .nav-item-expanded.active {
  background: rgba(88, 108, 199, 0.1);
  color: #586cc7;
}

.light-theme .close-btn:hover {
  background: rgba(255, 100, 100, 0.15);
  color: #e74c3c;
}

.light-theme .minimize-btn:hover {
  background: rgba(88, 108, 199, 0.1);
  color: #586cc7;
}

.light-theme .maximize-btn:hover {
  background: rgba(82, 196, 26, 0.1);
  color: #52c41a;
}

.light-theme .main-content {
  background: #f5f7fa;
}

.light-theme .content-header {
  background: rgba(255, 255, 255, 0.95);
  border-color: rgba(0, 0, 0, 0.08);
}

.light-theme .content-header h2 {
  color: #2c3e50;
}

/* 暗黑主题样式覆盖 */
.browser-automation-layout[data-theme="dark"] .sidebar-collapsed,
.browser-automation-layout[data-theme="dark"] .sidebar-expanded,
:root[data-theme="dark"] .browser-automation-layout .sidebar-collapsed,
:root[data-theme="dark"] .browser-automation-layout .sidebar-expanded {
  background: linear-gradient(180deg, #161b22 0%, #0d1117 100%) !important;
  border-right: 1px solid rgba(88, 108, 199, 0.2) !important;
}

.browser-automation-layout[data-theme="dark"] .sidebar-header,
:root[data-theme="dark"] .browser-automation-layout .sidebar-header {
  border-bottom: 1px solid rgba(88, 108, 199, 0.2) !important;
}

.browser-automation-layout[data-theme="dark"] .nav-item-collapsed,
.browser-automation-layout[data-theme="dark"] .nav-item-expanded,
:root[data-theme="dark"] .browser-automation-layout .nav-item-collapsed,
:root[data-theme="dark"] .browser-automation-layout .nav-item-expanded {
  color: var(--color-text-2, #c9d1d9);
}

.browser-automation-layout[data-theme="dark"] .nav-item-collapsed:hover,
.browser-automation-layout[data-theme="dark"] .nav-item-expanded:hover,
:root[data-theme="dark"] .browser-automation-layout .nav-item-collapsed:hover,
:root[data-theme="dark"] .browser-automation-layout .nav-item-expanded:hover {
  background: rgba(88, 108, 199, 0.15) !important;
  color: var(--color-text-1, #e8eaf6) !important;
}

.browser-automation-layout[data-theme="dark"] .nav-item-collapsed.active,
.browser-automation-layout[data-theme="dark"] .nav-item-expanded.active,
:root[data-theme="dark"] .browser-automation-layout .nav-item-collapsed.active,
:root[data-theme="dark"] .browser-automation-layout .nav-item-expanded.active {
  background: rgba(91, 138, 255, 0.2) !important;
  color: #5b8aff !important;
}

.browser-automation-layout[data-theme="dark"] .pin-btn,
:root[data-theme="dark"] .browser-automation-layout .pin-btn {
  color: var(--color-text-3, #9aa3c2);
}

.browser-automation-layout[data-theme="dark"] .pin-btn:hover,
:root[data-theme="dark"] .browser-automation-layout .pin-btn:hover {
  background: rgba(88, 108, 199, 0.15) !important;
  color: var(--color-text-1, #e8eaf6) !important;
}

.browser-automation-layout[data-theme="dark"] .pin-btn.pinned,
:root[data-theme="dark"] .browser-automation-layout .pin-btn.pinned {
  color: #5b8aff !important;
}

.browser-automation-layout[data-theme="dark"] .pin-btn.pinned:hover,
:root[data-theme="dark"] .browser-automation-layout .pin-btn.pinned:hover {
  background: rgba(91, 138, 255, 0.2) !important;
}

.browser-automation-layout[data-theme="dark"] .content-header,
:root[data-theme="dark"] .browser-automation-layout .content-header {
  background: linear-gradient(135deg, #161b22 0%, #0d1117 100%) !important;
  border-bottom: 1px solid rgba(88, 108, 199, 0.2) !important;
}

.browser-automation-layout[data-theme="dark"] .content-header h2,
:root[data-theme="dark"] .browser-automation-layout .content-header h2 {
  color: var(--color-text-1, #e8eaf6) !important;
}

.browser-automation-layout[data-theme="dark"] .close-btn:hover,
:root[data-theme="dark"] .browser-automation-layout .close-btn:hover {
  color: #ef4444 !important;
  background: rgba(239, 68, 68, 0.15) !important;
}

.browser-automation-layout[data-theme="dark"] .minimize-btn:hover,
:root[data-theme="dark"] .browser-automation-layout .minimize-btn:hover {
  color: #5b8aff !important;
  background: rgba(91, 138, 255, 0.15) !important;
}

.browser-automation-layout[data-theme="dark"] .maximize-btn:hover,
:root[data-theme="dark"] .browser-automation-layout .maximize-btn:hover {
  color: #14b866 !important;
  background: rgba(20, 184, 102, 0.15) !important;
}

.browser-automation-layout[data-theme="dark"] .main-content,
:root[data-theme="dark"] .browser-automation-layout .main-content {
  background: linear-gradient(135deg, #0d1117 0%, #111827 50%, #0f172a 100%) !important;
}

.browser-automation-layout[data-theme="dark"] .content-body,
:root[data-theme="dark"] .browser-automation-layout .content-body {
  background: transparent !important;
}

/* 覆盖子组件的背景色 - 使用 :deep() 穿透 scoped */
.browser-automation-layout[data-theme="dark"] :deep(.toolbar),
:root[data-theme="dark"] .browser-automation-layout :deep(.toolbar) {
  background: linear-gradient(135deg, #1c2128 0%, #161b22 100%) !important;
  border-color: rgba(88, 108, 199, 0.2) !important;
}

.browser-automation-layout[data-theme="dark"] :deep(.table-wrapper),
.browser-automation-layout[data-theme="dark"] :deep(.browser-farm),
.browser-automation-layout[data-theme="dark"] :deep(.script-editor),
.browser-automation-layout[data-theme="dark"] :deep(.extension-manager),
.browser-automation-layout[data-theme="dark"] :deep(.task-manager),
.browser-automation-layout[data-theme="dark"] :deep(.task-monitor),
.browser-automation-layout[data-theme="dark"] :deep(.execution-panel),
:root[data-theme="dark"] .browser-automation-layout :deep(.table-wrapper),
:root[data-theme="dark"] .browser-automation-layout :deep(.browser-farm),
:root[data-theme="dark"] .browser-automation-layout :deep(.script-editor),
:root[data-theme="dark"] .browser-automation-layout :deep(.extension-manager),
:root[data-theme="dark"] .browser-automation-layout :deep(.task-manager),
:root[data-theme="dark"] .browser-automation-layout :deep(.task-monitor),
:root[data-theme="dark"] .browser-automation-layout :deep(.execution-panel) {
  background: linear-gradient(135deg, #161b22 0%, #0d1117 100%) !important;
  border-color: rgba(88, 108, 199, 0.2) !important;
}

/* 覆盖所有使用 var(--color-bg-2) 的元素 */
.browser-automation-layout[data-theme="dark"] :deep([class*="wrapper"]),
.browser-automation-layout[data-theme="dark"] :deep([class*="container"]),
.browser-automation-layout[data-theme="dark"] :deep([class*="panel"]),
:root[data-theme="dark"] .browser-automation-layout :deep([class*="wrapper"]),
:root[data-theme="dark"] .browser-automation-layout :deep([class*="container"]),
:root[data-theme="dark"] .browser-automation-layout :deep([class*="panel"]) {
  background-color: #161b22 !important;
}

/* 强制覆盖所有子组件的背景色 */
.browser-automation-layout[data-theme="dark"] :deep(*),
:root[data-theme="dark"] .browser-automation-layout :deep(*) {
  --color-bg-1: #0d1117 !important;
  --color-bg-2: #161b22 !important;
  --color-bg-3: #1c2128 !important;
}
</style>
