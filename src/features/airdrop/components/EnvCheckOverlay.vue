<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue';
import { useThemeStore } from '../../../stores';
import { cliCheckService, type CliToolStatus } from '../services/cliCheckService';
import { invoke } from '@tauri-apps/api/core';
import { Message } from '@arco-design/web-vue';
import {
  IconCheckCircleFill,
  IconCloseCircleFill,
  IconLoading,
  IconTool
} from '@arco-design/web-vue/es/icon';

const themeStore = useThemeStore();
const isDarkTheme = computed(() => themeStore.currentTheme === 'dark');

// 组件状态
const isChecking = ref(true);
const isInstalling = ref(false);
const installProgress = ref('');
const tools = ref<CliToolStatus[]>([]);
const missingTools = ref<CliToolStatus[]>([]);
const installLogs = ref<string[]>([]);
const showLogs = ref(false);
const checkingToolName = ref<string>('');
const checkSuccess = ref(false);

// 预定义需要检查的工具列表
const requiredToolsList = ['Node.js', 'npm', 'npx', 'Playwright'];

// 检查是否完成
const checkComplete = computed(() => !isChecking.value && !isInstalling.value);

// 是否所有工具都已安装
const allToolsInstalled = computed(() => {
  // Playwright 是可选的，不强制要求
  const requiredTools = tools.value.filter(t => t.name !== 'Playwright');
  return requiredTools.length > 0 && requiredTools.every(t => t.installed);
});

// 是否可以进入页面
const canEnter = computed(() => allToolsInstalled.value);

const emit = defineEmits<{
  (e: 'ready'): void;
  (e: 'failed', tools: CliToolStatus[]): void;
}>();

// 检查环境
const checkEnvironment = async () => {
  isChecking.value = true;
  checkSuccess.value = false;
  installLogs.value = [];
  tools.value = [];
  
  addLog('开始检查环境...');
  
  try {
    // 先获取所有结果
    const result = await cliCheckService.checkTools(true);
    
    // 逐个显示检查结果，给用户更好的体验
    for (let i = 0; i < requiredToolsList.length; i++) {
      const toolName = requiredToolsList[i];
      checkingToolName.value = toolName;
      
      // 找到对应的检查结果
      const toolResult = result.tools.find(t => t.name === toolName);
      if (toolResult) {
        // 添加到显示列表
        tools.value.push(toolResult);
        
        const status = toolResult.installed ? '✓' : '✗';
        const version = toolResult.version ? `(${toolResult.version})` : '';
        addLog(`  ${status} ${toolName} ${version}`);
      }
      
      // 每个工具之间延迟，让用户看到逐个完成的效果
      if (i < requiredToolsList.length - 1) {
        await new Promise(resolve => setTimeout(resolve, 400));
      }
    }
    
    checkingToolName.value = '';
    
    // 小延迟让用户看到最后一个结果
    await new Promise(resolve => setTimeout(resolve, 300));
    
    // 找出未安装的工具（Playwright 可选）
    missingTools.value = result.tools.filter(t => !t.installed && t.name !== 'Playwright');
    
    addLog(`环境检查完成`);
    
    if (missingTools.value.length === 0) {
      addLog('所有必需工具已安装');
      // 停止检查状态，显示成功状态
      isChecking.value = false;
      checkSuccess.value = true;
      // 立即 emit，让父组件处理延迟关闭
      emit('ready');
      return;
    } else {
      // 检查完成但有缺失工具
      isChecking.value = false;
      const names = missingTools.value.map(t => t.name).join('、');
      addLog(`缺少工具: ${names}，需要安装`);
    }
  } catch (error) {
    console.error('环境检查失败:', error);
    addLog(`环境检查失败: ${error}`);
    Message.error('环境检查失败');
    isChecking.value = false;
  }
};

// 自动安装缺失的工具
const autoInstall = async () => {
  if (missingTools.value.length === 0) return;
  
  isInstalling.value = true;
  installProgress.value = '准备安装环境...';
  addLog('开始自动安装缺失的工具...');
  
  try {
    // 调用后端安装命令
    const result = await invoke<{ success: boolean; message: string; logs: string[] }>(
      'install_node_environment',
      {}
    );
    
    // 添加安装日志
    if (result.logs && result.logs.length > 0) {
      result.logs.forEach(log => addLog(log));
    }
    
    if (result.success) {
      addLog('安装完成，重新检查环境...');
      installProgress.value = '安装完成，验证中...';
      
      // 重新检查环境
      await checkEnvironment();
      
      if (allToolsInstalled.value) {
        Message.success('环境安装成功！');
      } else {
        Message.warning('部分工具安装失败，请手动安装');
        emit('failed', missingTools.value);
      }
    } else {
      addLog(`安装失败: ${result.message}`);
      Message.error(result.message || '安装失败');
      emit('failed', missingTools.value);
    }
  } catch (error) {
    console.error('自动安装失败:', error);
    const errorMsg = String(error);
    addLog(`安装失败: ${errorMsg}`);
    Message.error('自动安装失败，请手动安装所需工具');
    emit('failed', missingTools.value);
  } finally {
    isInstalling.value = false;
    installProgress.value = '';
  }
};

// 手动安装指导
const showManualInstallGuide = () => {
  showLogs.value = true;
  addLog('');
  addLog('=== 手动安装指南 ===');
  addLog('1. 访问 https://nodejs.org/ 下载并安装 LTS 版本 Node.js');
  addLog('2. 安装完成后，重启应用程序');
  addLog('');
  addLog('或使用包管理器安装:');
  addLog('- Windows: winget install OpenJS.NodeJS');
  addLog('- macOS: brew install node');
  addLog('- Linux: sudo apt install nodejs npm');
};

// 添加日志
const addLog = (log: string) => {
  const timestamp = new Date().toLocaleTimeString();
  installLogs.value.push(`[${timestamp}] ${log}`);
};

// 获取工具图标状态
const getToolStatus = (tool: CliToolStatus) => {
  if (tool.installed) {
    return { icon: IconCheckCircleFill, color: '#52c41a' };
  }
  return { icon: IconCloseCircleFill, color: '#f5222d' };
};

// 页面加载时检查
onMounted(() => {
  checkEnvironment();
});
</script>

<template>
  <Teleport to="body">
    <div 
      class="env-check-overlay" 
      :class="{ 'light-theme': !isDarkTheme }"
      v-if="!checkComplete || !canEnter || checkSuccess"
    >
      <div class="env-check-modal">
        <!-- 头部 -->
        <div class="modal-header">
          <div class="header-icon">
            <IconTool class="icon" />
          </div>
          <div class="header-content">
            <h3 class="title">环境检查</h3>
            <p class="subtitle">检查浏览器自动化所需的环境依赖</p>
          </div>
        </div>

        <!-- 工具列表 -->
        <div class="tools-list">
          <!-- 检查过程中逐个显示 -->
          <template v-if="isChecking || tools.length > 0">
            <!-- 已完成的工具 -->
            <div 
              v-for="tool in tools" 
              :key="tool.name"
              class="tool-item"
              :class="{ 
                'installed': tool.installed, 
                'missing': !tool.installed,
                'optional': tool.name === 'Playwright'
              }"
            >
              <div class="tool-status">
                <component 
                  :is="getToolStatus(tool).icon" 
                  class="status-icon"
                  :style="{ color: getToolStatus(tool).color }"
                />
              </div>
              <div class="tool-info">
                <span class="tool-name">{{ tool.name }}</span>
                <span class="tool-version" v-if="tool.version">{{ tool.version }}</span>
                <span class="tool-optional" v-if="tool.name === 'Playwright'">(可选)</span>
              </div>
            </div>
            
            <!-- 正在检查的工具 -->
            <div 
              v-if="checkingToolName && !tools.find(t => t.name === checkingToolName)"
              class="tool-item checking active"
            >
              <div class="tool-status">
                <IconLoading class="spin-icon" />
              </div>
              <div class="tool-info">
                <span class="tool-name">{{ checkingToolName }}</span>
                <span class="tool-checking">正在检查...</span>
                <span class="tool-optional" v-if="checkingToolName === 'Playwright'">(可选)</span>
              </div>
            </div>
            
            <!-- 待检查的工具 -->
            <div 
              v-for="toolName in requiredToolsList.filter(name => 
                !tools.find(t => t.name === name) && name !== checkingToolName
              )" 
              :key="toolName"
              class="tool-item checking"
            >
              <div class="tool-status">
                <div class="status-pending"></div>
              </div>
              <div class="tool-info">
                <span class="tool-name">{{ toolName }}</span>
                <span class="tool-optional" v-if="toolName === 'Playwright'">(可选)</span>
              </div>
            </div>
          </template>
        </div>

        <!-- 进度信息 -->
        <div class="progress-section" v-if="isInstalling || installProgress">
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: isInstalling ? '60%' : '100%' }"></div>
          </div>
          <p class="progress-text">{{ installProgress || '正在安装...' }}</p>
        </div>

        <!-- 日志区域 -->
        <div class="logs-section" v-if="showLogs || installLogs.length > 0">
          <div class="logs-header" @click="showLogs = !showLogs">
            <span>安装日志</span>
            <span class="toggle-icon">{{ showLogs ? '▼' : '▶' }}</span>
          </div>
          <div class="logs-content" v-show="showLogs">
            <div 
              v-for="(log, index) in installLogs" 
              :key="index"
              class="log-line"
            >
              {{ log }}
            </div>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="action-section">
          <template v-if="checkSuccess">
            <div class="success-status">
              <IconCheckCircleFill class="success-icon" />
              <span>环境检查通过</span>
            </div>
          </template>
          
          <template v-else-if="isChecking">
            <div class="checking-status">
              <IconLoading class="spin-icon large" />
              <span>正在检查环境...</span>
            </div>
          </template>
          
          <template v-else-if="missingTools.length > 0 && !isInstalling">
            <div class="missing-info">
              <p class="warning-text">
                <IconCloseCircleFill class="warning-icon" />
                检测到 {{ missingTools.length }} 个工具未安装
              </p>
            </div>
            <div class="action-buttons">
              <button class="btn btn-primary" @click="autoInstall">
                <IconTool class="btn-icon" />
                自动安装
              </button>
              <button class="btn btn-secondary" @click="showManualInstallGuide">
                手动安装
              </button>
            </div>
          </template>
          
          <template v-else-if="isInstalling">
            <div class="installing-status">
              <IconLoading class="spin-icon large" />
              <span>正在安装环境...</span>
              <p class="install-hint">这可能需要几分钟时间，请耐心等待</p>
            </div>
          </template>
          
          <template v-else-if="canEnter && !checkSuccess">
            <div class="success-status">
              <IconCheckCircleFill class="success-icon" />
              <span>环境检查通过</span>
            </div>
            <div class="action-buttons">
              <button class="btn btn-primary" @click="emit('ready')">
                <IconCheckCircleFill class="btn-icon" />
                进入系统
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
/* 遮罩层 */
.env-check-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* 弹窗 */
.env-check-modal {
  width: 420px;
  max-width: 90vw;
  background: var(--color-bg-2, #2a2a2b);
  border-radius: 16px;
  padding: 24px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
  animation: slideUp 0.3s ease;
  max-height: 80vh;
  overflow-y: auto;
}

@keyframes slideUp {
  from { 
    opacity: 0;
    transform: translateY(20px);
  }
  to { 
    opacity: 1;
    transform: translateY(0);
  }
}

/* 头部 */
.modal-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
}

.header-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: linear-gradient(135deg, #586cc7, #7b8fd4);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.header-icon .icon {
  font-size: 24px;
  color: white;
}

.header-content {
  flex: 1;
}

.title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-1, #fff);
}

.subtitle {
  margin: 4px 0 0;
  font-size: 13px;
  color: var(--color-text-3, rgba(255, 255, 255, 0.5));
}

/* 工具列表 */
.tools-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.tool-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 10px;
  background: var(--color-fill-2, rgba(255, 255, 255, 0.05));
  transition: all 0.2s;
}

.tool-item.installed {
  background: rgba(82, 196, 26, 0.1);
}

.tool-item.missing:not(.optional) {
  background: rgba(245, 34, 45, 0.1);
}

.tool-item.checking {
  opacity: 0.6;
}

.tool-item.checking.active {
  opacity: 1;
  background: rgba(88, 108, 199, 0.1);
}

.status-pending {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--color-fill-3, rgba(255, 255, 255, 0.2));
}

.tool-checking {
  font-size: 12px;
  color: #586cc7;
  font-style: italic;
}

.tool-status {
  flex-shrink: 0;
}

.status-icon {
  font-size: 20px;
}

.tool-info {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
}

.tool-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1, #fff);
}

.tool-version {
  font-size: 12px;
  color: var(--color-text-3, rgba(255, 255, 255, 0.5));
  background: var(--color-fill-3, rgba(255, 255, 255, 0.1));
  padding: 2px 8px;
  border-radius: 4px;
}

.tool-optional {
  font-size: 11px;
  color: var(--color-text-3, rgba(255, 255, 255, 0.4));
  font-style: italic;
}

.tool-loading {
  flex-shrink: 0;
}

/* 进度条 */
.progress-section {
  margin-bottom: 20px;
}

.progress-bar {
  height: 4px;
  background: var(--color-fill-3, rgba(255, 255, 255, 0.1));
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #586cc7, #7b8fd4);
  border-radius: 2px;
  transition: width 0.3s ease;
  animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
  0% { opacity: 1; }
  50% { opacity: 0.7; }
  100% { opacity: 1; }
}

.progress-text {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--color-text-3, rgba(255, 255, 255, 0.5));
  text-align: center;
}

/* 日志区域 */
.logs-section {
  margin-bottom: 20px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  overflow: hidden;
}

.logs-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--color-fill-2, rgba(255, 255, 255, 0.05));
  font-size: 12px;
  color: var(--color-text-2, rgba(255, 255, 255, 0.7));
  cursor: pointer;
  user-select: none;
}

.logs-header:hover {
  background: var(--color-fill-3, rgba(255, 255, 255, 0.08));
}

.toggle-icon {
  font-size: 10px;
}

.logs-content {
  max-height: 150px;
  overflow-y: auto;
  padding: 12px;
  background: var(--color-bg-3, #1a1a1b);
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  line-height: 1.6;
}

.log-line {
  color: var(--color-text-2, rgba(255, 255, 255, 0.7));
  word-break: break-all;
}

/* 操作区域 */
.action-section {
  text-align: center;
}

.checking-status,
.installing-status,
.success-status {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 20px;
}

.checking-status span,
.installing-status span,
.success-status span {
  font-size: 14px;
  color: var(--color-text-1, #fff);
}

.success-icon {
  font-size: 48px;
  color: #52c41a;
}

.install-hint {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-3, rgba(255, 255, 255, 0.5));
}

.missing-info {
  margin-bottom: 16px;
}

.warning-text {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 0;
  font-size: 14px;
  color: #faad14;
}

.warning-icon {
  font-size: 18px;
}

.action-buttons {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 20px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
  outline: none;
}

.btn-primary {
  background: linear-gradient(135deg, #586cc7, #7b8fd4);
  color: white;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(88, 108, 199, 0.4);
}

.btn-primary:active {
  transform: translateY(0);
}

.btn-secondary {
  background: var(--color-fill-2, rgba(255, 255, 255, 0.1));
  color: var(--color-text-1, #fff);
}

.btn-secondary:hover {
  background: var(--color-fill-3, rgba(255, 255, 255, 0.15));
}

.btn-icon {
  font-size: 16px;
}

/* 旋转动画 */
.spin-icon {
  animation: spin 1s linear infinite;
}

.spin-icon.large {
  font-size: 32px;
  color: #586cc7;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 浅色主题适配 */
.light-theme .env-check-modal {
  background: #ffffff;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.light-theme .modal-header {
  border-color: rgba(0, 0, 0, 0.08);
}

.light-theme .title {
  color: #1f1f1f;
}

.light-theme .subtitle {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .tool-item {
  background: rgba(0, 0, 0, 0.04);
}

.light-theme .tool-item.installed {
  background: rgba(82, 196, 26, 0.1);
}

.light-theme .tool-item.missing:not(.optional) {
  background: rgba(245, 34, 45, 0.08);
}

.light-theme .tool-item.checking.active {
  background: rgba(88, 108, 199, 0.08);
}

.light-theme .status-pending {
  border-color: rgba(0, 0, 0, 0.15);
}

.light-theme .tool-name {
  color: #1f1f1f;
}

.light-theme .tool-version {
  color: rgba(0, 0, 0, 0.5);
  background: rgba(0, 0, 0, 0.06);
}

.light-theme .tool-optional {
  color: rgba(0, 0, 0, 0.4);
}

.light-theme .progress-bar {
  background: rgba(0, 0, 0, 0.08);
}

.light-theme .progress-text {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .logs-section {
  border-color: rgba(0, 0, 0, 0.08);
}

.light-theme .logs-header {
  background: rgba(0, 0, 0, 0.04);
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .logs-header:hover {
  background: rgba(0, 0, 0, 0.06);
}

.light-theme .logs-content {
  background: #f5f5f5;
}

.light-theme .log-line {
  color: rgba(0, 0, 0, 0.7);
}

.light-theme .checking-status span,
.light-theme .installing-status span,
.light-theme .success-status span {
  color: #1f1f1f;
}

.light-theme .install-hint {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .btn-secondary {
  background: rgba(0, 0, 0, 0.06);
  color: #1f1f1f;
}

.light-theme .btn-secondary:hover {
  background: rgba(0, 0, 0, 0.1);
}
</style>
