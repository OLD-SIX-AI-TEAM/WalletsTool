<script setup>
import { ref, reactive, computed, onMounted, onUnmounted, watch } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import {
  IconPlayArrow,
  IconStop,
  IconSettings,
  IconDelete,
  IconPlus,
  IconFile,
  IconRefresh,
  IconLoading,
  IconCheckCircle,
  IconCloseCircle,
  IconEye
} from '@arco-design/web-vue/es/icon';
import {
  scriptService,
  profileService,
  walletService,
  executionSessionService
} from '../services/browserAutomationService';

// 配置
const config = reactive({
  headless: false,
  timeout: 300,
  retryCount: 3,
  concurrentLimit: 1,
  enableProxy: true,
  enableFingerprint: true,
});

// 数据
const scripts = ref([]);
const profiles = ref([]);
const wallets = ref([]);
const executions = ref([]);
const selectedScript = ref(null);
const selectedProfile = ref(null);
const selectedWallets = ref([]);
const profileStrategy = ref('random');

// 执行状态
const isRunning = ref(false);
const progress = ref(0);
const elapsedTime = ref(0);
const logs = ref([]);
const activeTab = ref('queue');
let timer = null;
let currentExecutionIds = ref([]);

// 计算属性
const canStart = computed(() => {
  return (
    selectedScript.value &&
    selectedWallets.value.length > 0 &&
    !isRunning.value
  );
});

const stats = computed(() => {
  const total = selectedWallets.value.length;
  const completed = executions.value.filter(e => e.status === 'success' || e.status === 'failed').length;
  const success = executions.value.filter(e => e.status === 'success').length;
  const failed = executions.value.filter(e => e.status === 'failed').length;
  const running = executions.value.filter(e => e.status === 'running').length;
  const pending = total - completed - running;
  
  return { total, completed, success, failed, pending, running };
});

const formatTime = (seconds) => {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
};

const formatDuration = (ms) => {
  if (!ms) return '-';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
};

// 加载数据
const loadData = async () => {
  try {
    const [scriptsData, profilesData, walletsData] = await Promise.all([
      scriptService.getScripts(),
      profileService.getProfiles(),
      walletService.getWallets()
    ]);
    
    scripts.value = scriptsData;
    profiles.value = profilesData;
    wallets.value = walletsData;

    // 设置默认值
    if (scripts.value.length > 0 && !selectedScript.value) {
      selectedScript.value = scripts.value[0].id;
    }
    if (profiles.value.length > 0 && !selectedProfile.value) {
      selectedProfile.value = profiles.value[0].id;
    }
  } catch (error) {
    Message.error('加载数据失败: ' + error.message);
  }
};

// 添加日志
const addLog = (message, type = 'info') => {
  const time = new Date().toLocaleTimeString();
  logs.value.unshift({ time, message, type });
  if (logs.value.length > 500) {
    logs.value = logs.value.slice(0, 500);
  }
};

// 开始执行
const handleStart = async () => {
  if (!canStart.value) return;

  const script = scripts.value.find((s) => s.id === selectedScript.value);
  const selectedWalletList = wallets.value.filter((w) =>
    selectedWallets.value.includes(w.id)
  );

  if (!script || selectedWalletList.length === 0) {
    Message.error('请选择脚本和钱包');
    return;
  }

  // 根据策略选择环境
  let profileIds = [];
  if (profileStrategy.value === 'specific' && selectedProfile.value) {
    profileIds = [selectedProfile.value];
  } else if (profileStrategy.value === 'random') {
    profileIds = profiles.value.map(p => p.id);
  } else if (profileStrategy.value === 'sequential') {
    profileIds = profiles.value.map(p => p.id);
  }

  logs.value = [];
  progress.value = 0;
  elapsedTime.value = 0;
  isRunning.value = true;
  executions.value = [];
  currentExecutionIds.value = [];

  try {
    // 创建执行会话
    const session = await executionSessionService.createExecution({
      script_id: script.id,
      wallet_ids: selectedWallets.value,
      profile_ids: profileIds.length > 0 ? profileIds : undefined,
      parallel_mode: config.concurrentLimit > 1,
      max_parallel: config.concurrentLimit
    });
    
    // 保存执行ID列表
    currentExecutionIds.value = session.results.map(r => r.id);
    executions.value = session.results;
    
    // 启动计时器
    timer = setInterval(() => {
      elapsedTime.value++;
      updateProgress();
    }, 1000);

    addLog(`创建执行会话，共 ${session.total_tasks} 个任务`);
    
    // 启动所有执行任务（模拟执行）
    for (const result of session.results) {
      await executionSessionService.startExecution(result.id);
      // 启动模拟执行（后台运行）
      executionSessionService.simulateExecution(result.id).catch(e => {
        console.error('模拟执行失败:', e);
      });
    }
    
    addLog('所有任务已启动');
    
    // 开始轮询执行状态
    pollExecutionStatus();
    
  } catch (error) {
    isRunning.value = false;
    clearInterval(timer);
    Message.error('启动执行失败: ' + error.message);
    addLog('启动失败: ' + error.message, 'error');
  }
};

// 轮询执行状态
const pollExecutionStatus = async () => {
  if (currentExecutionIds.value.length === 0 || !isRunning.value) return;
  
  try {
    const session = await executionSessionService.getExecution(currentExecutionIds.value);
    executions.value = session.results || [];
    
    // 更新进度
    progress.value = session.total_tasks > 0 
      ? Math.round((session.completed_tasks / session.total_tasks) * 100) 
      : 0;
    
    // 检查是否完成
    if (session.status === 'completed') {
      isRunning.value = false;
      clearInterval(timer);
      
      if (session.failed_count === 0) {
        Message.success(`执行完成！成功: ${session.success_count}`);
        addLog(`执行完成！成功: ${session.success_count}`, 'success');
      } else {
        Message.warning(`执行完成！成功: ${session.success_count}, 失败: ${session.failed_count}`);
        addLog(`执行完成！成功: ${session.success_count}, 失败: ${session.failed_count}`, 'warning');
      }
      return;
    }
    
    // 继续轮询
    setTimeout(pollExecutionStatus, 1000);
  } catch (error) {
    console.error('轮询状态失败:', error);
    setTimeout(pollExecutionStatus, 2000);
  }
};

// 更新进度
const updateProgress = () => {
  const total = selectedWallets.value.length;
  if (total === 0) {
    progress.value = 0;
    return;
  }
  const completed = executions.value.filter(e => 
    e.status === 'success' || e.status === 'failed'
  ).length;
  progress.value = Math.round((completed / total) * 100);
};

// 停止执行
const handleStop = async () => {
  if (currentExecutionIds.value.length > 0) {
    try {
      for (const id of currentExecutionIds.value) {
        await executionSessionService.cancelExecution(id);
      }
      addLog('任务已手动停止');
      Message.info('任务已停止');
    } catch (error) {
      Message.error('停止失败: ' + error.message);
    }
  }
  clearInterval(timer);
  isRunning.value = false;
};

// 清空日志
const clearLogs = () => {
  logs.value = [];
};

// 重置任务
const resetTasks = () => {
  Modal.confirm({
    title: '确认重置',
    content: '确定要清空当前任务队列吗？',
    onOk: () => {
      executions.value = [];
      logs.value = [];
      progress.value = 0;
      elapsedTime.value = 0;
      currentExecutionIds.value = [];
      Message.success('已重置');
    },
  });
};

// 全选/取消全选钱包
const toggleSelectAllWallets = () => {
  if (selectedWallets.value.length === wallets.value.length) {
    selectedWallets.value = [];
  } else {
    selectedWallets.value = wallets.value.map((w) => w.id);
  }
};

// 获取状态颜色
const getStatusColor = (status) => {
  const colors = {
    pending: 'var(--color-text-4)',
    running: 'rgb(var(--primary-6))',
    success: 'rgb(var(--success-6))',
    failed: 'rgb(var(--danger-6))',
    stopped: 'rgb(var(--warning-6))',
  };
  return colors[status] || colors.pending;
};

const getStatusText = (status) => {
  const texts = {
    pending: '等待中',
    running: '执行中',
    success: '成功',
    failed: '失败',
    stopped: '已停止',
  };
  return texts[status] || status;
};

// 查看执行详情
const viewExecutionDetail = (execution) => {
  Modal.info({
    title: '执行详情',
    content: `
      <div style="line-height: 1.8; max-height: 400px; overflow-y: auto;">
        <p><strong>钱包:</strong> ${execution.wallet_name || '-'}</p>
        <p><strong>地址:</strong> ${execution.wallet_address || '-'}</p>
        <p><strong>状态:</strong> ${getStatusText(execution.status)}</p>
        <p><strong>开始时间:</strong> ${execution.started_at ? new Date(execution.started_at).toLocaleString('zh-CN') : '-'}</p>
        <p><strong>结束时间:</strong> ${execution.completed_at ? new Date(execution.completed_at).toLocaleString('zh-CN') : '-'}</p>
        <p><strong>执行时长:</strong> ${formatDuration(execution.duration_ms)}</p>
        <p><strong>日志:</strong></p>
        <pre style="background: var(--color-bg-2); padding: 10px; border-radius: 4px; font-size: 12px; overflow-x: auto;">${execution.logs || '无日志'}</pre>
        ${execution.error_message ? `<p style="color: rgb(var(--danger-6));"><strong>错误:</strong> ${execution.error_message}</p>` : ''}
      </div>
    `
  });
};

onMounted(() => {
  loadData();
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <div class="execution-panel">
    <!-- 配置面板 -->
    <div class="config-panel">
      <div class="panel-section">
        <div class="section-title">
          <icon-file />
          <span>选择脚本</span>
        </div>
        <a-select
          v-model="selectedScript"
          placeholder="选择要执行的脚本"
          style="width: 100%"
        >
          <a-option
            v-for="script in scripts"
            :key="script.id"
            :value="script.id"
            :label="script.name"
          >
            {{ script.name }}
          </a-option>
        </a-select>
      </div>

      <div class="panel-section">
        <div class="section-title">
          <icon-settings />
          <span>环境分配策略</span>
        </div>
        <a-radio-group v-model="profileStrategy" type="button" size="small" style="width: 100%">
          <a-radio value="random">随机</a-radio>
          <a-radio value="sequential">顺序</a-radio>
          <a-radio value="specific">指定</a-radio>
        </a-radio-group>
        
        <a-select
          v-if="profileStrategy === 'specific'"
          v-model="selectedProfile"
          placeholder="选择浏览器环境"
          style="width: 100%; margin-top: 10px"
        >
          <a-option
            v-for="profile in profiles"
            :key="profile.id"
            :value="profile.id"
            :label="profile.name"
          >
            {{ profile.name }}
          </a-option>
        </a-select>
      </div>

      <div class="panel-section">
        <div class="section-title">
          <span>选择钱包 ({{ selectedWallets.length }}/{{ wallets.length }})</span>
          <a-button type="text" size="mini" @click="toggleSelectAllWallets">
            {{ selectedWallets.length === wallets.length ? '取消全选' : '全选' }}
          </a-button>
        </div>
        <div class="wallet-list">
          <div
            v-for="wallet in wallets"
            :key="wallet.id"
            class="wallet-item"
            :class="{ selected: selectedWallets.includes(wallet.id) }"
            @click="
              selectedWallets.includes(wallet.id)
                ? selectedWallets.splice(selectedWallets.indexOf(wallet.id), 1)
                : selectedWallets.push(wallet.id)
            "
          >
            <a-checkbox :model-value="selectedWallets.includes(wallet.id)">
              <div class="wallet-info">
                <div class="wallet-label">{{ wallet.name || wallet.label }}</div>
                <div class="wallet-address">{{ wallet.address.slice(0, 8) }}...{{ wallet.address.slice(-4) }}</div>
              </div>
            </a-checkbox>
          </div>
          <div v-if="wallets.length === 0" class="empty-wallets">
            暂无钱包，请先在"钱包管理"中添加
          </div>
        </div>
      </div>

      <div class="panel-section">
        <div class="section-title">执行配置</div>
        <div class="config-options">
          <div class="config-switch-item">
            <a-switch v-model="config.headless" size="small" />
            <span class="switch-label">无头模式</span>
          </div>
          <div class="config-switch-item">
            <a-switch v-model="config.enableProxy" size="small" />
            <span class="switch-label">使用代理</span>
          </div>
          <div class="config-switch-item full-width">
            <a-switch v-model="config.enableFingerprint" size="small" />
            <span class="switch-label">指纹保护</span>
          </div>
          <div class="config-input-item">
            <span class="input-label">并发数</span>
            <a-input-number v-model="config.concurrentLimit" :min="1" :max="5" size="small" style="width: 60px" />
          </div>
          <div class="config-input-item">
            <span class="input-label">重试次数</span>
            <a-input-number v-model="config.retryCount" :min="0" :max="5" size="small" style="width: 60px" />
          </div>
        </div>
      </div>
    </div>

    <!-- 执行面板 -->
    <div class="execution-main">
      <!-- 控制栏 -->
      <div class="control-bar">
        <div class="status-overview">
          <div class="stat-card">
            <div class="stat-value">{{ formatTime(elapsedTime) }}</div>
            <div class="stat-label">执行时间</div>
          </div>
          <div class="stat-card">
            <div class="stat-value">{{ stats.total }}</div>
            <div class="stat-label">总任务</div>
          </div>
          <div class="stat-card success">
            <div class="stat-value">{{ stats.success }}</div>
            <div class="stat-label">成功</div>
          </div>
          <div class="stat-card error">
            <div class="stat-value">{{ stats.failed }}</div>
            <div class="stat-label">失败</div>
          </div>
          <div class="stat-card" v-if="stats.running > 0">
            <div class="stat-value" style="color: rgb(var(--primary-6))">{{ stats.running }}</div>
            <div class="stat-label">执行中</div>
          </div>
        </div>

        <div class="control-actions">
          <a-button
            type="primary"
            status="success"
            size="large"
            :disabled="!canStart"
            :loading="isRunning"
            @click="handleStart"
          >
            <template #icon><icon-play-arrow /></template>
            开始执行
          </a-button>
          <a-button
            type="primary"
            status="danger"
            size="large"
            :disabled="!isRunning"
            @click="handleStop"
          >
            <template #icon><icon-stop /></template>
            停止
          </a-button>
          <a-button type="outline" size="large" @click="resetTasks">
            <template #icon><icon-refresh /></template>
            重置
          </a-button>
        </div>
      </div>

      <!-- 进度条 -->
      <div class="progress-section">
        <div class="progress-info">
          <span>执行进度</span>
          <span>{{ progress }}%</span>
        </div>
        <a-progress
          :percent="progress"
          :show-text="false"
          :status="progress === 100 ? 'success' : isRunning ? 'active' : 'normal'"
        />
      </div>

      <!-- 标签页 -->
      <a-tabs v-model:active-key="activeTab" type="card" class="execution-tabs">
        <a-tab-pane key="queue" title="执行队列">
          <div class="task-queue">
            <div
              v-for="execution in executions"
              :key="execution.id"
              class="queue-item"
              :class="execution.status"
            >
              <div class="item-status" :style="{ color: getStatusColor(execution.status) }">
                <icon-loading v-if="execution.status === 'running'" spin />
                <icon-play-arrow v-else-if="execution.status === 'pending'" />
                <icon-check-circle v-else-if="execution.status === 'completed'" />
                <icon-close-circle v-else-if="execution.status === 'failed'" />
              </div>
              <div class="item-info">
                <div class="item-wallet">{{ execution.wallet_name || 'Unknown' }}</div>
                <div class="item-address">
                  {{ execution.wallet_address ? execution.wallet_address.slice(0, 6) + '...' + execution.wallet_address.slice(-4) : '-' }}
                </div>
              </div>
              <div class="item-meta">
                <a-tag :color="getStatusColor(execution.status)" size="small">
                  {{ getStatusText(execution.status) }}
                </a-tag>
                <span class="item-duration">{{ formatDuration(execution.duration_ms) }}</span>
                <a-button type="text" size="mini" @click="viewExecutionDetail(execution)">
                  <template #icon><icon-eye /></template>
                </a-button>
              </div>
            </div>
            <div v-if="executions.length === 0" class="empty-queue">
              <icon-file style="font-size: 32px; color: var(--color-text-4)" />
              <p>请选择脚本、配置和钱包后点击"开始执行"</p>
            </div>
          </div>
        </a-tab-pane>

        <a-tab-pane key="logs" title="执行日志">
          <div class="log-panel">
            <div class="log-header">
              <span>实时日志</span>
              <a-button type="text" size="mini" @click="clearLogs" :disabled="logs.length === 0">
                清空
              </a-button>
            </div>
            <div class="log-container">
              <div v-for="(log, index) in logs" :key="index" class="log-line" :class="log.type">
                <span class="log-time">[{{ log.time }}]</span>
                <span class="log-message">{{ log.message }}</span>
              </div>
              <div v-if="logs.length === 0" class="empty-logs">暂无日志</div>
            </div>
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>
  </div>
</template>

<style scoped>
.execution-panel {
  height: 100%;
  display: flex;
  gap: 10px;
}

/* 配置面板 */
.config-panel {
  width: 300px;
  background: var(--color-bg-2);
  border-radius: 8px;
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border: 1px solid var(--color-border);
  overflow-y: auto;
}

.panel-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
  font-size: 14px;
  color: var(--color-text-1);
  justify-content: space-between;
}

.wallet-list {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px;
}

.wallet-item {
  padding: 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.wallet-item:hover {
  background: var(--color-fill-2);
}

.wallet-item.selected {
  background: rgba(var(--primary-6), 0.1);
}

.wallet-info {
  display: flex;
  flex-direction: column;
}

.wallet-label {
  font-size: 13px;
  color: var(--color-text-1);
}

.wallet-address {
  font-size: 11px;
  color: var(--color-text-3);
  font-family: monospace;
}

.empty-wallets {
  text-align: center;
  padding: 20px;
  color: var(--color-text-3);
  font-size: 12px;
}

/* 执行配置选项网格布局 */
.config-options {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.config-switch-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.config-switch-item.full-width {
  grid-column: span 2;
}

.switch-label {
  font-size: 13px;
  color: var(--color-text-2);
  white-space: nowrap;
}

.config-input-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.input-label {
  font-size: 13px;
  color: var(--color-text-2);
}

/* 执行主面板 */
.execution-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.control-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 20px;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.status-overview {
  display: flex;
  gap: 10px;
}

.stat-card {
  text-align: center;
  padding: 10px 20px;
  border-radius: 6px;
  background: var(--color-fill-2);
}

.stat-card.success .stat-value {
  color: rgb(var(--success-6));
}

.stat-card.error .stat-value {
  color: rgb(var(--danger-6));
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-1);
}

.stat-label {
  font-size: 12px;
  color: var(--color-text-3);
  margin-top: 4px;
}

.control-actions {
  display: flex;
  gap: 10px;
}

/* 进度条 */
.progress-section {
  padding: 0 5px;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
  font-size: 13px;
  color: var(--color-text-3);
}

/* 标签页 */
.execution-tabs {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  overflow: hidden;
}

.execution-tabs :deep(.arco-tabs-content) {
  height: calc(100% - 46px);
  padding: 0;
}

.execution-tabs :deep(.arco-tab-pane) {
  height: 100%;
}

/* 任务队列 */
.task-queue {
  height: 100%;
  overflow-y: auto;
  padding: 10px;
}

.queue-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-bottom: 1px solid var(--color-border);
  transition: all 0.2s;
}

.queue-item:hover {
  background: var(--color-fill-2);
}

.item-status {
  width: 24px;
  display: flex;
  justify-content: center;
}

.item-info {
  flex: 1;
}

.item-wallet {
  font-size: 14px;
  color: var(--color-text-1);
}

.item-address {
  font-size: 12px;
  color: var(--color-text-3);
  font-family: monospace;
}

.item-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.item-duration {
  font-size: 12px;
  color: var(--color-text-3);
  min-width: 50px;
  text-align: right;
}

.empty-queue {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-4);
  gap: 10px;
}

/* 日志面板 */
.log-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.log-header {
  padding: 12px 15px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 500;
}

.log-container {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  background: var(--color-bg-1);
  font-family: 'Fira Code', monospace;
  font-size: 12px;
}

.log-line {
  margin-bottom: 4px;
  line-height: 1.5;
  color: var(--color-text-2);
}

.log-line.success {
  color: rgb(var(--success-6));
}

.log-line.error {
  color: rgb(var(--danger-6));
}

.log-line.warning {
  color: rgb(var(--warning-6));
}

.log-time {
  color: var(--color-text-4);
  margin-right: 8px;
}

.empty-logs {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-4);
}
</style>