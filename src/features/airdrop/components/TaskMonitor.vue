<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import {
  IconPlayArrow,
  IconStop,
  IconRefresh,
  IconDelete,
  IconCheckCircle,
  IconCloseCircle,
  IconLoading,
  IconSchedule,
  IconFile,
  IconSettings,
  IconHistory,
  IconEye
} from '@arco-design/web-vue/es/icon';
import { executionService, scriptService, profileService, walletService } from '../services/browserAutomationService';

// 执行记录
const executionHistory = ref([]);
const selectedRecord = ref(null);
const activeTab = ref('history');
const searchQuery = ref('');
const loading = ref(false);
const autoRefresh = ref(true);
let refreshTimer = null;

// 统计数据
const stats = computed(() => {
  const history = executionHistory.value;
  const total = history.length;
  const success = history.filter(r => r.status === 'success' || r.status === 'completed').length;
  const failed = history.filter(r => r.status === 'failed').length;
  const running = history.filter(r => r.status === 'running').length;
  const cancelled = history.filter(r => r.status === 'stopped' || r.status === 'cancelled').length;
  
  return {
    total,
    success,
    failed,
    running,
    cancelled,
    successRate: total > 0 ? Math.round((success / total) * 100) : 0
  };
});

// 过滤后的历史记录
const filteredHistory = computed(() => {
  let records = executionHistory.value.slice().reverse();
  if (!searchQuery.value) return records;
  
  const query = searchQuery.value.toLowerCase();
  return records.filter(r => 
    r.wallet_name?.toLowerCase().includes(query) ||
    r.wallet_address?.toLowerCase().includes(query) ||
    r.script_name?.toLowerCase().includes(query) ||
    r.profile_name?.toLowerCase().includes(query)
  );
});

// 加载执行记录
const loadHistory = async () => {
  loading.value = true;
  try {
    executionHistory.value = await executionService.getExecutions();
  } catch (error) {
    Message.error('加载执行记录失败: ' + error.message);
  } finally {
    loading.value = false;
  }
};

// 查看详情
const viewDetails = async (record) => {
  try {
    const detail = await executionService.getExecution(record.id);
    selectedRecord.value = detail;
  } catch (error) {
    Message.error('加载详情失败: ' + error.message);
  }
};

// 清空历史
const clearHistory = async () => {
  if (executionHistory.value.length === 0) {
    Message.info('执行记录为空');
    return;
  }
  
  Modal.warning({
    title: '确认清空',
    content: `确定要清空所有 ${executionHistory.value.length} 条执行记录吗？`,
    okText: '确认清空',
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        for (const record of executionHistory.value) {
          await executionService.deleteExecution(record.id);
        }
        executionHistory.value = [];
        selectedRecord.value = null;
        Message.success('执行记录已清空');
      } catch (error) {
        Message.error('清空失败: ' + error.message);
      }
    }
  });
};

// 删除单条记录
const deleteRecord = async (record, event) => {
  event.stopPropagation();
  Modal.warning({
    title: '确认删除',
    content: '确定要删除这条记录吗？',
    onOk: async () => {
      try {
        await executionService.deleteExecution(record.id);
        executionHistory.value = executionHistory.value.filter(r => r.id !== record.id);
        if (selectedRecord.value?.id === record.id) {
          selectedRecord.value = null;
        }
        Message.success('记录已删除');
      } catch (error) {
        Message.error('删除失败: ' + error.message);
      }
    }
  });
};

// 刷新
const refreshHistory = async () => {
  await loadHistory();
  Message.success('已刷新');
};

// 自动刷新
const startAutoRefresh = () => {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(() => {
    if (autoRefresh.value && !loading.value) {
      loadHistory();
    }
  }, 5000);
};

const stopAutoRefresh = () => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
};

// 格式化时间
const formatTime = (timestamp) => {
  if (!timestamp) return '-';
  return new Date(timestamp).toLocaleString('zh-CN');
};

// 格式化时长
const formatDuration = (ms) => {
  if (!ms) return '-';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const mins = Math.floor(ms / 60000);
  const secs = ((ms % 60000) / 1000).toFixed(0);
  return `${mins}分${secs}秒`;
};

// 格式化地址
const formatAddress = (address) => {
  if (!address) return '-';
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

// 获取状态颜色
const getStatusColor = (status) => {
  const colors = {
    pending: 'var(--color-text-4)',
    running: 'rgb(var(--primary-6))',
    success: 'rgb(var(--success-6))',
    completed: 'rgb(var(--success-6))',
    failed: 'rgb(var(--danger-6))',
    stopped: 'rgb(var(--warning-6))',
    cancelled: 'rgb(var(--warning-6))',
  };
  return colors[status] || colors.pending;
};

const getStatusText = (status) => {
  const texts = {
    pending: '等待中',
    running: '执行中',
    success: '成功',
    completed: '成功',
    failed: '失败',
    stopped: '已停止',
    cancelled: '已取消',
  };
  return texts[status] || status;
};

// 获取日志颜色
const getLogColor = (log) => {
  if (log.includes('成功') || log.includes('success')) return 'rgb(var(--success-6))';
  if (log.includes('失败') || log.includes('error') || log.includes('Error')) return 'rgb(var(--danger-6))';
  if (log.includes('警告') || log.includes('warn')) return 'rgb(var(--warning-6))';
  return 'var(--color-text-2)';
};

onMounted(() => {
  loadHistory();
  startAutoRefresh();
});

onUnmounted(() => {
  stopAutoRefresh();
});
</script>

<template>
  <div class="task-monitor">
    <!-- 统计概览 -->
    <div class="stats-overview">
      <div class="stat-card">
        <div class="stat-icon">
          <icon-history />
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-label">总执行次数</div>
        </div>
      </div>
      <div class="stat-card success">
        <div class="stat-icon">
          <icon-check-circle />
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.success }}</div>
          <div class="stat-label">成功</div>
        </div>
      </div>
      <div class="stat-card error">
        <div class="stat-icon">
          <icon-close-circle />
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.failed }}</div>
          <div class="stat-label">失败</div>
        </div>
      </div>
      <div class="stat-card" v-if="stats.running > 0">
        <div class="stat-icon" style="background: rgba(var(--primary-6), 0.1); color: rgb(var(--primary-6))">
          <icon-loading spin />
        </div>
        <div class="stat-content">
          <div class="stat-value" style="color: rgb(var(--primary-6))">{{ stats.running }}</div>
          <div class="stat-label">执行中</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon">
          <icon-schedule />
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ stats.successRate }}%</div>
          <div class="stat-label">成功率</div>
        </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="monitor-content">
      <!-- 执行记录列表 -->
      <div class="history-panel">
        <div class="panel-header">
          <div class="header-title">执行记录</div>
          <div class="header-actions">
            <a-input-search
              v-model="searchQuery"
              placeholder="搜索钱包/脚本..."
              size="small"
              class="search-input"
              allow-clear
            />
            <div class="action-divider"></div>
            <div class="auto-refresh-switch" :class="{ active: autoRefresh }">
              <a-switch v-model="autoRefresh" size="small" />
              <span class="switch-label">{{ autoRefresh ? '自动' : '手动' }}</span>
            </div>
            <a-button type="text" size="small" @click="refreshHistory" :loading="loading" title="刷新">
              <template #icon><icon-refresh /></template>
            </a-button>
            <a-button type="text" size="small" status="danger" @click="clearHistory" title="清空记录">
              <template #icon><icon-delete /></template>
            </a-button>
          </div>
        </div>

        <div class="history-list" v-loading="loading">
          <div
            v-for="record in filteredHistory"
            :key="record.id"
            class="history-item"
            :class="{ active: selectedRecord?.id === record.id }"
            @click="viewDetails(record)"
          >
            <div class="item-status" :style="{ color: getStatusColor(record.status) }">
              <icon-check-circle v-if="record.status === 'success' || record.status === 'completed'" />
              <icon-close-circle v-else-if="record.status === 'failed'" />
              <icon-loading v-else-if="record.status === 'running'" spin />
              <icon-schedule v-else />
            </div>
            <div class="item-info">
              <div class="item-wallet">{{ record.wallet_name || 'Unknown' }}</div>
              <div class="item-script">{{ record.script_name || 'Unknown Script' }}</div>
              <div class="item-time">{{ formatTime(record.started_at) }}</div>
            </div>
            <div class="item-meta">
              <a-tag :color="getStatusColor(record.status)" size="small">
                {{ getStatusText(record.status) }}
              </a-tag>
              <span class="item-duration">{{ formatDuration(record.duration_ms) }}</span>
              <icon-delete class="delete-btn" @click="(e) => deleteRecord(record, e)" />
            </div>
          </div>
          <div v-if="filteredHistory.length === 0" class="empty-history">
            <icon-history style="font-size: 32px; color: var(--color-text-4)" />
            <p>{{ searchQuery ? '未找到匹配的记录' : '暂无执行记录' }}</p>
          </div>
        </div>
      </div>

      <!-- 详情面板 -->
      <div class="details-panel" v-if="selectedRecord">
        <div class="panel-header">
          <span>执行详情</span>
          <a-button type="text" size="small" @click="selectedRecord = null">
            关闭
          </a-button>
        </div>

        <div class="details-content">
          <!-- 基本信息 -->
          <div class="detail-section">
            <div class="section-title">基本信息</div>
            <div class="detail-grid">
              <div class="detail-item">
                <span class="label">执行ID</span>
                <span class="value mono">{{ selectedRecord.id }}</span>
              </div>
              <div class="detail-item">
                <span class="label">钱包</span>
                <span class="value">{{ selectedRecord.wallet_name || '-' }}</span>
              </div>
              <div class="detail-item">
                <span class="label">地址</span>
                <span class="value mono">{{ formatAddress(selectedRecord.wallet_address) }}</span>
              </div>
              <div class="detail-item">
                <span class="label">脚本</span>
                <span class="value">{{ selectedRecord.script_name || '-' }}</span>
              </div>
              <div class="detail-item">
                <span class="label">环境</span>
                <span class="value">{{ selectedRecord.profile_name || '-' }}</span>
              </div>
              <div class="detail-item">
                <span class="label">状态</span>
                <a-tag :color="getStatusColor(selectedRecord.status)" size="small">
                  {{ getStatusText(selectedRecord.status) }}
                </a-tag>
              </div>
              <div class="detail-item">
                <span class="label">开始时间</span>
                <span class="value">{{ formatTime(selectedRecord.started_at) }}</span>
              </div>
              <div class="detail-item">
                <span class="label">结束时间</span>
                <span class="value">{{ formatTime(selectedRecord.completed_at) }}</span>
              </div>
              <div class="detail-item">
                <span class="label">执行时长</span>
                <span class="value">{{ formatDuration(selectedRecord.duration_ms) }}</span>
              </div>
              <div class="detail-item">
                <span class="label">重试次数</span>
                <span class="value">{{ selectedRecord.retry_count || 0 }}</span>
              </div>
            </div>
          </div>

          <!-- 错误信息 -->
          <div class="detail-section" v-if="selectedRecord.error_message">
            <div class="section-title" style="color: rgb(var(--danger-6))">错误信息</div>
            <div class="error-message">{{ selectedRecord.error_message }}</div>
          </div>

          <!-- 执行结果 -->
          <div class="detail-section" v-if="selectedRecord.result_data">
            <div class="section-title">执行结果</div>
            <pre class="result-data">{{ JSON.stringify(selectedRecord.result_data, null, 2) }}</pre>
          </div>

          <!-- 执行日志 -->
          <div class="detail-section" v-if="selectedRecord.logs">
            <div class="section-title">执行日志</div>
            <div class="log-container">
              <div
                v-for="(log, index) in selectedRecord.logs.split('\n')"
                :key="index"
                class="log-line"
                :style="{ color: getLogColor(log) }"
              >
                {{ log }}
              </div>
              <div v-if="!selectedRecord.logs" class="empty-logs">
                暂无日志
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="empty-details" v-else>
        <icon-eye style="font-size: 48px; color: var(--color-text-4)" />
        <p>点击左侧记录查看详情</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-monitor {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 统计概览 */
.stats-overview {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 10px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 15px;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.stat-card.success .stat-icon {
  background: rgba(var(--success-6), 0.1);
  color: rgb(var(--success-6));
}

.stat-card.error .stat-icon {
  background: rgba(var(--danger-6), 0.1);
  color: rgb(var(--danger-6));
}

.stat-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: var(--color-fill-2);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  color: var(--color-text-3);
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-1);
}

.stat-label {
  font-size: 12px;
  color: var(--color-text-3);
}

/* 主内容区 */
.monitor-content {
  flex: 1;
  display: flex;
  gap: 10px;
  min-height: 0;
}

.history-panel {
  width: 400px;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  padding: 12px 15px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.header-title {
  font-weight: 500;
  font-size: 14px;
  color: var(--color-text-1);
  white-space: nowrap;
  flex-shrink: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex: 1;
  justify-content: flex-end;
}

.search-input {
  width: 140px;
  transition: width 0.2s;
}

.search-input:focus-within {
  width: 160px;
}

.action-divider {
  width: 1px;
  height: 20px;
  background: var(--color-border);
  margin: 0 4px;
}

.auto-refresh-switch {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 4px;
  background: var(--color-fill-2);
  transition: all 0.2s;
  cursor: pointer;
}

.auto-refresh-switch:hover {
  background: var(--color-fill-3);
}

.auto-refresh-switch.active {
  background: rgba(var(--primary-6), 0.15);
}

.auto-refresh-switch.active .switch-label {
  color: rgb(var(--primary-6));
}

.switch-label {
  font-size: 12px;
  color: var(--color-text-3);
  font-weight: 400;
  white-space: nowrap;
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

.history-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  border-bottom: 1px solid var(--color-border);
}

.history-item:hover {
  background: var(--color-fill-2);
}

.history-item.active {
  background: rgba(var(--primary-6), 0.1);
}

.item-status {
  width: 24px;
  display: flex;
  justify-content: center;
}

.item-info {
  flex: 1;
  min-width: 0;
}

.item-wallet {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-script {
  font-size: 12px;
  color: var(--color-text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-time {
  font-size: 11px;
  color: var(--color-text-4);
}

.item-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}

.item-duration {
  font-size: 11px;
  color: var(--color-text-4);
}

.delete-btn {
  cursor: pointer;
  color: var(--color-text-4);
  opacity: 0;
  transition: all 0.2s;
}

.history-item:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  color: rgb(var(--danger-6));
}

.empty-history {
  padding: 40px 20px;
  text-align: center;
  color: var(--color-text-4);
}

/* 详情面板 */
.details-panel {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.details-content {
  flex: 1;
  overflow-y: auto;
  padding: 15px;
}

.detail-section {
  margin-bottom: 10px;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-2);
  margin-bottom: 10px;
  text-transform: uppercase;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.detail-item .label {
  font-size: 11px;
  color: var(--color-text-4);
}

.detail-item .value {
  font-size: 13px;
  color: var(--color-text-1);
}

.detail-item .value.mono {
  font-family: monospace;
}

.error-message {
  padding: 12px;
  background: rgba(var(--danger-6), 0.1);
  border: 1px solid rgba(var(--danger-6), 0.2);
  border-radius: 6px;
  color: rgb(var(--danger-6));
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}

.result-data {
  padding: 12px;
  background: var(--color-bg-1);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  font-size: 12px;
  font-family: 'Fira Code', monospace;
  overflow-x: auto;
  max-height: 200px;
  overflow-y: auto;
}

.log-container {
  background: var(--color-bg-1);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 12px;
  max-height: 300px;
  overflow-y: auto;
  font-family: 'Fira Code', monospace;
  font-size: 12px;
}

.log-line {
  margin-bottom: 4px;
  line-height: 1.5;
}

.empty-logs {
  color: var(--color-text-4);
  font-style: italic;
}

.empty-details {
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
</style>
