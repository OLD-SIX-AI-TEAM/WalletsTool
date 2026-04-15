<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import {
  IconRecord,
  IconStop,
  IconPlayArrow,
  IconCode,
  IconDelete,
  IconToTop,
  IconPlus,
  IconRefresh,
  IconApps,
  IconSave
} from '@arco-design/web-vue/es/icon';
import { recorderService, recordingActions, recordingSession } from '../services/recorderService';
import { extensionService } from '../services/extensionService';
import { scriptService } from '../services/browserAutomationService';

const PROXY_TYPES = [
  { label: 'Direct (直接连接)', value: 'direct' },
  { label: 'HTTP', value: 'http' },
  { label: 'HTTPS', value: 'https' },
  { label: 'SOCKS5', value: 'socks5' }
];

const props = defineProps<{
  currentScript?: { id: number; name: string; content: string } | null;
}>();

const emit = defineEmits(['insert-code', 'close', 'script-saved', 'script-updated']);

const recordUrl = ref('');
const isRecording = ref(false);
const isBrowserOpen = ref(false);
const generatedCode = ref('');
const browserType = ref('chromium');

const availableExtensions = ref([]);
const selectedExtensions = ref([]);
const loadingExtensions = ref(false);

// 代理配置 - 默认使用 127.0.0.1:8595
const proxyType = ref('http');
const proxyHost = ref('127.0.0.1');
const proxyPort = ref('8595');
const proxyUsername = ref('');
const proxyPassword = ref('');

// 录制选项
const recordMouseMove = ref(false);

// 保存脚本对话框
const showSaveDialog = ref(false);
const scriptName = ref('');
const scriptDescription = ref('');
const savingScript = ref(false);

// 录制完成后选择对话框
const showActionChoiceDialog = ref(false);
const actionChoice = ref<'new' | 'overwrite'>('overwrite'); // 默认覆盖当前脚本

const loadExtensions = async () => {
  loadingExtensions.value = true;
  try {
    availableExtensions.value = await extensionService.getExtensions();
  } catch (error) {
    console.error('Failed to load extensions:', error);
  } finally {
    loadingExtensions.value = false;
  }
};

onMounted(() => {
  loadExtensions();
});

// 监听录制会话状态，当浏览器关闭时自动停止录制
watch(recordingSession, async (newSession, oldSession) => {
  // 当会话状态从 recording 变为 stopped 时，自动处理录制结果
  if (oldSession?.status === 'recording' && newSession?.status === 'stopped') {
    console.log('[ScriptRecorder] 检测到浏览器已关闭，自动停止录制');
    
    // 浏览器已关闭，需要获取生成的代码并显示保存对话框
    if (isRecording.value && !generatedCode.value) {
      try {
        // 调用 stopRecording 获取生成的代码
        const code = await recorderService.stopRecording();
        
        if (code) {
          generatedCode.value = code;
          // 自动生成默认脚本名称
          const url = new URL(recordUrl.value);
          const domain = url.hostname.replace(/^www\./, '');
          const timestamp = new Date().toISOString().slice(0, 10);
          scriptName.value = `${domain}_${timestamp}`;
          scriptDescription.value = `录制自 ${recordUrl.value}`;
          // 显示选择对话框（新建或覆盖）
          showActionChoiceDialog.value = true;
          Message.info('检测到浏览器已关闭，录制已自动停止');
        }
        
        isBrowserOpen.value = false;
        isRecording.value = false;
      } catch (error) {
        console.error('[ScriptRecorder] 自动停止录制失败:', error);
        Message.error('自动停止录制失败: ' + (error.message || error));
        isRecording.value = false;
        isBrowserOpen.value = false;
      }
    }
  }
});

const currentSession = computed(() => recordingSession.value);
const actionCount = computed(() => recordingActions.value.length);
const recordedActions = computed(() => recordingActions.value);

const actionTypeMap = {
  click: '点击',
  fill: '输入',
  navigate: '导航',
  select: '选择',
  hover: '悬停',
  screenshot: '截图',
  upload: '上传文件',
  evaluate: '执行脚本'
};

const getActionIcon = (type) => {
  const icons = {
    click: '👆',
    fill: '⌨️',
    navigate: '🔗',
    select: '📋',
    hover: '🖱️',
    screenshot: '📷',
    upload: '📎',
    evaluate: '⚙️'
  };
  return icons[type] || '📝';
};

// 自动补全 URL 协议
const normalizeUrl = (url: string): string => {
  const trimmed = url.trim();
  if (!trimmed) return '';
  
  // 如果已经有协议，直接返回
  if (/^https?:\/\//i.test(trimmed)) {
    return trimmed;
  }
  
  // 如果以 // 开头，添加 https:
  if (trimmed.startsWith('//')) {
    return 'https:' + trimmed;
  }
  
  // 默认添加 https://
  return 'https://' + trimmed;
};

const startRecording = async () => {
  if (!recordUrl.value.trim()) {
    Message.warning('请输入要录制的网址');
    return;
  }

  // 自动补全 URL
  const normalizedUrl = normalizeUrl(recordUrl.value);
  recordUrl.value = normalizedUrl;

  // 验证代理配置
  if (proxyType.value !== 'direct') {
    if (!proxyHost.value.trim()) {
      Message.warning('请输入代理主机地址');
      return;
    }
    if (!proxyPort.value.trim()) {
      Message.warning('请输入代理端口');
      return;
    }
  }

  try {
    isRecording.value = true;
    generatedCode.value = '';
    
    const extensionPaths = selectedExtensions.value.map(id => {
      const ext = availableExtensions.value.find(e => e.id === id);
      return ext?.path;
    }).filter(Boolean);
    
    // 构建代理配置
    const proxyConfig = proxyType.value !== 'direct' ? {
      type: proxyType.value,
      host: proxyHost.value.trim(),
      port: parseInt(proxyPort.value),
      username: proxyUsername.value.trim() || undefined,
      password: proxyPassword.value || undefined
    } : undefined;
    
    await recorderService.startRecording(normalizedUrl, {
      browserType: browserType.value,
      headless: false,
      viewportWidth: 1280,
      viewportHeight: 720,
      includeComments: true,
      extensions: extensionPaths,
      proxy: proxyConfig,
      recordMouseMove: recordMouseMove.value
    });
    
    isBrowserOpen.value = true;
    Message.success('浏览器已启动，请开始操作');
  } catch (error) {
    Message.error('启动浏览器失败: ' + (error.message || error));
    isRecording.value = false;
    isBrowserOpen.value = false;
  }
};

const stopRecording = async () => {
  try {
    const code = await recorderService.stopRecording();

    if (code) {
      generatedCode.value = code;
      // 自动生成默认脚本名称
      const url = new URL(recordUrl.value);
      const domain = url.hostname.replace(/^www\./, '');
      const timestamp = new Date().toISOString().slice(0, 10);
      scriptName.value = `${domain}_${timestamp}`;
      scriptDescription.value = `录制自 ${recordUrl.value}`;
      // 显示选择对话框（新建或覆盖）
      showActionChoiceDialog.value = true;
    }

    isBrowserOpen.value = false;
    isRecording.value = false;
    Message.success('录制已停止');
  } catch (error) {
    Message.error('停止录制失败: ' + (error.message || error));
    isRecording.value = false;
    isBrowserOpen.value = false;
  }
};

// 保存脚本到数据库
const saveScriptToDatabase = async () => {
  if (!scriptName.value.trim()) {
    Message.warning('请输入脚本名称');
    return;
  }

  if (!generatedCode.value) {
    Message.warning('没有可保存的脚本');
    return;
  }

  savingScript.value = true;
  try {
    const newScript = await scriptService.createScript({
      name: scriptName.value.trim(),
      description: scriptDescription.value,
      content: generatedCode.value,
      required_apis: ['page', 'wallet', 'api'],
      author: '录制生成',
      tags: ['录制', '自动化']
    });

    Message.success('脚本已保存到数据库');
    showSaveDialog.value = false;

    // 清空录制状态
    generatedCode.value = '';
    scriptName.value = '';
    scriptDescription.value = '';

    // 通知父组件脚本已保存
    emit('script-saved', newScript);

    // 关闭录制器
    emit('close');
  } catch (error) {
    Message.error('保存脚本失败: ' + (error.message || error));
  } finally {
    savingScript.value = false;
  }
};

// 取消保存
const cancelSave = () => {
  showSaveDialog.value = false;
  scriptName.value = '';
  scriptDescription.value = '';
};

// 处理录制完成后的选择
const handleActionChoiceConfirm = () => {
  showActionChoiceDialog.value = false;
  if (actionChoice.value === 'overwrite' && props.currentScript) {
    // 覆盖当前脚本
    overwriteCurrentScript();
  } else {
    // 新建脚本，显示保存对话框
    showSaveDialog.value = true;
  }
};

const cancelActionChoice = () => {
  showActionChoiceDialog.value = false;
  // 不清空数据，用户可以选择插入代码到编辑器
};

// 覆盖当前脚本
const overwriteCurrentScript = async () => {
  if (!props.currentScript) {
    Message.warning('没有可覆盖的当前脚本');
    showSaveDialog.value = true;
    return;
  }

  if (!generatedCode.value) {
    Message.warning('没有可保存的脚本');
    return;
  }

  savingScript.value = true;
  try {
    const updatedScript = await scriptService.updateScript({
      id: props.currentScript.id,
      content: generatedCode.value,
      description: `录制自 ${recordUrl.value}`
    });

    Message.success(`脚本 "${props.currentScript.name}" 已更新`);

    // 清空录制状态
    generatedCode.value = '';
    scriptName.value = '';
    scriptDescription.value = '';

    // 通知父组件脚本已更新
    emit('script-updated', updatedScript);

    // 关闭录制器
    emit('close');
  } catch (error) {
    Message.error('更新脚本失败: ' + (error.message || error));
  } finally {
    savingScript.value = false;
  }
};

const clearActions = () => {
  recorderService.clearRecording();
  generatedCode.value = '';
  Message.success('已清空录制记录');
};

const insertCode = () => {
  if (!generatedCode.value) {
    Message.warning('没有可插入的代码，请先录制操作');
    return;
  }
  emit('insert-code', generatedCode.value);
  Message.success('代码已插入到编辑器');
};

const copyCode = async () => {
  if (!generatedCode.value) {
    Message.warning('没有可复制的代码');
    return;
  }
  try {
    await navigator.clipboard.writeText(generatedCode.value);
    Message.success('代码已复制到剪贴板');
  } catch {
    Message.error('复制失败');
  }
};

const takeScreenshot = async () => {
  if (!isBrowserOpen.value) {
    Message.warning('请先启动浏览器');
    return;
  }
  try {
    const mcpPlaywright = window.__MCP_PLAYWRIGHT__;
    if (typeof window !== 'undefined' && mcpPlaywright) {
      await mcpPlaywright.playwright_screenshot({
        name: `recording-${Date.now()}`,
        fullPage: false
      });
    }
    recorderService.addAction('screenshot', '截图', {});
    Message.success('截图已保存');
  } catch (error) {
    Message.error('截图失败: ' + (error.message || error));
  }
};

const refreshPage = async () => {
  if (!isBrowserOpen.value) {
    Message.warning('请先启动浏览器');
    return;
  }
  try {
    const mcpPlaywright = window.__MCP_PLAYWRIGHT__;
    if (typeof window !== 'undefined' && mcpPlaywright) {
      await mcpPlaywright.playwright_navigate({
        url: recordUrl.value,
        browserType: browserType.value
      });
    }
    recorderService.addAction('navigate', '刷新页面', { value: recordUrl.value });
    Message.success('页面已刷新');
  } catch (error) {
    Message.error('刷新失败: ' + (error.message || error));
  }
};

onUnmounted(() => {
  if (isBrowserOpen.value) {
    recorderService.stopRecording();
  }
});
</script>

<template>
  <div class="script-recorder">
    <div class="recorder-header">
      <h3>
        <icon-record :style="{ color: isRecording ? 'rgb(var(--danger-6))' : 'inherit' }" />
        脚本录制
      </h3>
      <a-button type="text" size="small" @click="emit('close')">
        <template #icon><icon-delete /></template>
      </a-button>
    </div>

    <div class="recorder-config">
      <div class="config-row">
        <label>目标网址:</label>
        <a-input 
          v-model="recordUrl" 
          placeholder="https://example.com"
          :disabled="isRecording"
          @press-enter="startRecording"
        >
          <template #prefix>🔗</template>
        </a-input>
      </div>
      
      <div class="config-row inline">
        <label>浏览器:</label>
        <a-select v-model="browserType" :disabled="isRecording" style="width: 120px">
          <a-option value="chromium">Chromium</a-option>
          <a-option value="firefox">Firefox</a-option>
          <a-option value="webkit">WebKit</a-option>
        </a-select>
        
        <span class="switch-label" style="margin-left: 16px; color: var(--color-text-3); font-size: 12px;">录制鼠标:</span>
        <a-switch v-model="recordMouseMove" :disabled="isRecording" size="small" style="margin-left: 4px;">
          <template #checked>开</template>
          <template #unchecked>关</template>
        </a-switch>
      </div>
      
      <div class="config-row" v-if="availableExtensions.length > 0">
        <label>
          <icon-apps style="margin-right: 4px;" />
          加载插件:
        </label>
        <a-select 
          v-model="selectedExtensions" 
          :disabled="isRecording"
          multiple
          placeholder="选择要加载的浏览器插件"
          style="width: 100%"
        >
          <a-option 
            v-for="ext in availableExtensions" 
            :key="ext.id" 
            :value="ext.id"
            :disabled="!ext.enabled"
          >
            <span>{{ ext.name }}</span>
            <span v-if="ext.version" style="color: var(--color-text-3); margin-left: 8px;">
              v{{ ext.version }}
            </span>
            <a-tag v-if="!ext.enabled" size="small" color="gray" style="margin-left: 8px;">未启用</a-tag>
          </a-option>
        </a-select>
      </div>
      
      <div class="config-row">
        <label>🌐 代理配置:</label>
        <a-select v-model="proxyType" :disabled="isRecording" style="width: 100%; margin-bottom: 8px;">
          <a-option v-for="type in PROXY_TYPES" :key="type.value" :value="type.value">{{ type.label }}</a-option>
        </a-select>
        
        <div v-if="proxyType !== 'direct'" class="proxy-config-panel">
          <a-row :gutter="8">
            <a-col :span="16">
              <a-input 
                v-model="proxyHost" 
                placeholder="代理主机 (如: 127.0.0.1 或 proxy.example.com)"
                :disabled="isRecording"
                size="small"
              />
            </a-col>
            <a-col :span="8">
              <a-input 
                v-model="proxyPort" 
                placeholder="端口"
                :disabled="isRecording"
                size="small"
              />
            </a-col>
          </a-row>
          <a-row :gutter="8" style="margin-top: 8px;">
            <a-col :span="12">
              <a-input 
                v-model="proxyUsername" 
                placeholder="用户名 (可选)"
                :disabled="isRecording"
                size="small"
              />
            </a-col>
            <a-col :span="12">
              <a-input-password 
                v-model="proxyPassword" 
                placeholder="密码 (可选)"
                :disabled="isRecording"
                size="small"
              />
            </a-col>
          </a-row>
        </div>
      </div>
    </div>

    <div class="recorder-controls">
      <a-space>
        <a-button 
          v-if="!isRecording"
          type="primary"
          @click="startRecording"
        >
          <template #icon><icon-play-arrow /></template>
          开始录制
        </a-button>
        
        <a-button 
          v-else
          type="primary"
          status="danger"
          @click="stopRecording"
        >
          <template #icon><icon-stop /></template>
          停止录制
        </a-button>
        
        <a-button 
          v-if="isBrowserOpen"
          @click="refreshPage"
        >
          <template #icon><icon-refresh /></template>
          刷新
        </a-button>
        
        <a-button 
          v-if="isBrowserOpen"
          @click="takeScreenshot"
        >
          <template #icon>📷</template>
          截图
        </a-button>
      </a-space>
    </div>

    <div class="recorder-actions" v-if="recordedActions.length > 0">
      <div class="actions-header">
        <span>录制操作 ({{ actionCount }})</span>
        <a-button type="text" size="small" @click="clearActions">
          <template #icon><icon-delete /></template>
          清空
        </a-button>
      </div>
      
      <div class="actions-list">
        <div 
          v-for="(action, index) in recordedActions" 
          :key="index"
          class="action-item"
        >
          <span class="action-icon">{{ getActionIcon(action.type) }}</span>
          <span class="action-desc">{{ action.description }}</span>
          <span class="action-time">{{ new Date(action.timestamp).toLocaleTimeString() }}</span>
        </div>
      </div>
    </div>

    <div class="generated-code" v-if="generatedCode">
      <div class="code-header">
        <span><icon-code /> 生成的代码</span>
        <a-space>
          <a-button type="text" size="small" @click="copyCode">
            复制
          </a-button>
          <a-button type="primary" size="small" @click="insertCode">
            <template #icon><icon-plus /></template>
            插入到编辑器
          </a-button>
        </a-space>
      </div>

      <div class="code-preview">
        <pre><code>{{ generatedCode }}</code></pre>
      </div>
    </div>

    <!-- 录制完成后选择对话框 -->
    <a-modal
      v-model:visible="showActionChoiceDialog"
      title="录制完成"
      :mask-closable="false"
      :esc-to-close="false"
      @ok="handleActionChoiceConfirm"
      @cancel="cancelActionChoice"
      :ok-text="actionChoice === 'overwrite' && currentScript ? '覆盖当前脚本' : '新建脚本'"
      cancel-text="取消"
      width="500px"
    >
      <div class="action-choice-content">
        <p class="choice-description">请选择如何处理录制的脚本：</p>
        <a-radio-group v-model="actionChoice" direction="vertical" class="choice-radio-group">
          <a-radio value="overwrite" :disabled="!currentScript">
            <div class="radio-option">
              <span class="radio-title">覆盖当前脚本</span>
              <span class="radio-desc" v-if="currentScript">
                更新 "{{ currentScript.name }}" 的内容
              </span>
              <span class="radio-desc disabled" v-else>
                没有当前选中的脚本
              </span>
            </div>
          </a-radio>
          <a-radio value="new">
            <div class="radio-option">
              <span class="radio-title">新建脚本</span>
              <span class="radio-desc">创建一个新的脚本文件</span>
            </div>
          </a-radio>
        </a-radio-group>
      </div>
    </a-modal>

    <!-- 保存脚本对话框 -->
    <a-modal
      v-model:visible="showSaveDialog"
      title="保存录制脚本"
      :mask-closable="false"
      :esc-to-close="false"
      @ok="saveScriptToDatabase"
      @cancel="cancelSave"
      :ok-loading="savingScript"
      ok-text="保存到数据库"
      cancel-text="取消"
      width="700px"
      :footer="true"
    >
      <a-form :model="{ name: scriptName, description: scriptDescription }" layout="vertical">
        <a-form-item label="脚本名称" required>
          <a-input
            v-model="scriptName"
            placeholder="请输入脚本名称"
            :max-length="100"
            show-word-limit
          />
        </a-form-item>
        <a-form-item label="脚本描述">
          <a-textarea
            v-model="scriptDescription"
            placeholder="请输入脚本描述（可选）"
            :auto-size="{ minRows: 2, maxRows: 4 }"
            :max-length="500"
            show-word-limit
          />
        </a-form-item>
        <a-form-item label="脚本内容预览" class="code-preview-full-width">
          <div class="code-preview-mini">
            <pre><code>{{ generatedCode }}</code></pre>
          </div>
        </a-form-item>
      </a-form>
    </a-modal>

    <div class="recorder-tips" v-if="!isRecording && recordedActions.length === 0">
      <div class="tip-item">
        <span class="tip-icon">💡</span>
        <span>输入目标网址，点击"开始录制"启动浏览器</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">🖱️</span>
        <span>在浏览器中的操作将被自动记录</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">📝</span>
        <span>停止录制后可生成 Playwright 脚本代码</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.script-recorder {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-2);
  border-radius: 8px;
  overflow: hidden;
}

.recorder-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-3);
}

.recorder-header h3 {
  margin: 0;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--color-text-1);
}

.recorder-config {
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}

.config-row {
  margin-bottom: 12px;
}

.config-row:last-child {
  margin-bottom: 0;
}

.config-row label {
  display: block;
  font-size: 12px;
  color: var(--color-text-3);
  margin-bottom: 6px;
}

.config-row.inline {
  display: flex;
  align-items: center;
  gap: 10px;
}

.config-row.inline label {
  margin-bottom: 0;
}

.recorder-controls {
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-1);
}

.recorder-actions {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 150px;
  max-height: 250px;
  border-bottom: 1px solid var(--color-border);
}

.actions-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  background: var(--color-bg-3);
  font-size: 12px;
  color: var(--color-text-2);
}

.actions-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

/* 自定义滚动条样式 */
.actions-list::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.actions-list::-webkit-scrollbar-track {
  background: var(--color-fill-2);
  border-radius: 4px;
}

.actions-list::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 4px;
}

.actions-list::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox */
.actions-list {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) var(--color-fill-2);
}

.action-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--color-bg-1);
  margin-bottom: 6px;
  font-size: 13px;
}

.action-item:last-child {
  margin-bottom: 0;
}

.action-icon {
  font-size: 14px;
}

.action-desc {
  flex: 1;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.action-time {
  font-size: 11px;
  color: var(--color-text-4);
}

.generated-code {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 200px;
}

.code-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  background: var(--color-bg-3);
  font-size: 12px;
  color: var(--color-text-2);
}

.code-preview {
  flex: 1;
  overflow: auto;
  background: var(--color-bg-1);
  padding: 12px;
}

/* 自定义滚动条样式 */
.code-preview::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.code-preview::-webkit-scrollbar-track {
  background: var(--color-fill-2);
  border-radius: 4px;
}

.code-preview::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 4px;
}

.code-preview::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox */
.code-preview {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) var(--color-fill-2);
}

.code-preview pre {
  margin: 0;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-1);
  white-space: pre-wrap;
  word-break: break-all;
}

.recorder-tips {
  padding: 20px;
  background: var(--color-bg-1);
}

.tip-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
  font-size: 13px;
  color: var(--color-text-3);
}

.tip-icon {
  font-size: 16px;
}

.proxy-config-panel {
  padding: 12px;
  background: var(--color-bg-1);
  border-radius: 6px;
  border: 1px solid var(--color-border);
}

.code-preview-full-width {
  width: 100%;
}

.code-preview-mini {
  max-height: 300px;
  overflow: auto;
  background: var(--color-bg-1);
  padding: 12px;
  border-radius: 4px;
  border: 1px solid var(--color-border);
  width: 100%;
  box-sizing: border-box;
}

/* 自定义滚动条样式 */
.code-preview-mini::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.code-preview-mini::-webkit-scrollbar-track {
  background: var(--color-fill-2);
  border-radius: 4px;
}

.code-preview-mini::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 4px;
}

.code-preview-mini::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox */
.code-preview-mini {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) var(--color-fill-2);
}

.code-preview-mini pre {
  margin: 0;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-2);
  white-space: pre-wrap;
  word-break: break-all;
  width: 100%;
}

.code-preview-mini code {
  display: block;
  width: 100%;
}

.action-choice-content {
  padding: 16px 8px;
}

.choice-description {
  margin: 0 0 20px 0;
  font-size: 14px;
  color: var(--color-text-2);
}

.choice-radio-group {
  width: 100%;
}

.choice-radio-group :deep(.arco-radio) {
  margin-bottom: 10px;
  width: 100%;
}

.radio-option {
  display: flex;
  flex-direction: column;
  margin-left: 8px;
}

.radio-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.radio-desc {
  font-size: 12px;
  color: var(--color-text-3);
  margin-top: 4px;
}

.radio-desc.disabled {
  color: var(--color-text-4);
}
</style>
