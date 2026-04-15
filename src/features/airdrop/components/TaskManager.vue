<script setup>
import { ref, reactive, onMounted, computed } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { 
  IconPlus, 
  IconEdit, 
  IconDelete, 
  IconPlayArrow,
  IconClockCircle,
  IconSave,
  IconToBottom,
  IconToTop,
  IconPause,
  IconEye
} from '@arco-design/web-vue/es/icon';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';
import { taskService, walletService, profileService, scriptService } from '../services/browserAutomationService';

const tasks = ref([]);
const wallets = ref([]);
const profiles = ref([]);
const scripts = ref([]);
const isModalVisible = ref(false);
const isEditing = ref(false);
const editingId = ref(null);
const loading = ref(false);
const cronError = ref('');

const form = reactive({
  name: '',
  description: '',
  schedule_type: 'once',
  cron_expression: '',
  interval_seconds: 3600,
  script_id: null,
  wallet_ids: [],
  profile_strategy: 'random',
  specific_profile_id: null,
  parallel_mode: false,
  max_parallel: 1,
  retry_count: 3,
  retry_interval: 5000,
  enabled: true
});

const scheduleTypeOptions = [
  { label: '单次执行', value: 'once' },
  { label: '间隔执行', value: 'interval' },
  { label: 'Cron定时', value: 'cron' }
];

const profileStrategyOptions = [
  { label: '随机分配', value: 'random', description: '每次执行随机选择环境' },
  { label: '顺序分配', value: 'sequential', description: '按顺序循环使用环境' },
  { label: '指定环境', value: 'specific', description: '使用指定环境执行' }
];

const columns = [
  { title: '任务名称', dataIndex: 'name', ellipsis: true },
  { title: '调度类型', slotName: 'schedule_type', width: 120, align: 'center' },
  { title: '下次执行', slotName: 'next_run', width: 160 },
  { title: '状态', slotName: 'status', width: 100, align: 'center' },
  { title: '操作', slotName: 'action', width: 240, align: 'center' }
];

const getScheduleTypeLabel = (type) => {
  const map = { once: '单次', interval: '间隔', cron: 'Cron' };
  return map[type] || type;
};

const getScheduleTypeColor = (type) => {
  const map = { once: 'gray', interval: 'blue', cron: 'purple' };
  return map[type] || 'gray';
};

const formatNextRun = (dateStr) => {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  const now = new Date();
  const diff = date - now;
  
  if (diff < 0) return '已过期';
  if (diff < 60000) return '即将执行';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟后`;
  
  return date.toLocaleString('zh-CN', { 
    month: 'short', 
    day: 'numeric', 
    hour: '2-digit', 
    minute: '2-digit' 
  });
};

const validateCron = (expression) => {
  if (!expression) return true;
  // 基础Cron格式验证
  const parts = expression.trim().split(/\s+/);
  if (parts.length !== 5 && parts.length !== 6) {
    return false;
  }
  return true;
};

const loadTasks = async () => {
  loading.value = true;
  try {
    tasks.value = await taskService.getTasks();
  } catch (error) {
    Message.error('加载任务失败: ' + error.message);
  } finally {
    loading.value = false;
  }
};

const loadWallets = async () => {
  try {
    wallets.value = await walletService.getWallets();
  } catch (error) {
    console.error('加载钱包失败:', error);
  }
};

const loadProfiles = async () => {
  try {
    profiles.value = await profileService.getProfiles();
  } catch (error) {
    console.error('加载环境失败:', error);
  }
};

const loadScripts = async () => {
  try {
    scripts.value = await scriptService.getScripts();
  } catch (error) {
    console.error('加载脚本失败:', error);
  }
};

const resetForm = () => {
  form.name = '';
  form.description = '';
  form.schedule_type = 'once';
  form.cron_expression = '';
  form.interval_seconds = 3600;
  form.script_id = null;
  form.wallet_ids = [];
  form.profile_strategy = 'random';
  form.specific_profile_id = null;
  form.parallel_mode = false;
  form.max_parallel = 1;
  form.retry_count = 3;
  form.retry_interval = 5000;
  form.enabled = true;
  cronError.value = '';
};

const handleAdd = () => {
  isEditing.value = false;
  editingId.value = null;
  resetForm();
  isModalVisible.value = true;
};

const handleEdit = (record) => {
  isEditing.value = true;
  editingId.value = record.id;
  
  form.name = record.name;
  form.description = record.description || '';
  form.schedule_type = record.schedule_type;
  form.script_id = record.script_id;
  form.wallet_ids = record.wallet_ids || [];
  form.profile_strategy = record.profile_strategy;
  form.specific_profile_id = record.specific_profile_id;
  form.parallel_mode = record.parallel_mode;
  form.max_parallel = record.max_parallel;
  form.retry_count = record.retry_count;
  form.retry_interval = record.retry_interval;
  form.enabled = record.enabled;
  
  // 解析调度配置
  try {
    const scheduleConfig = JSON.parse(record.schedule_config || '{}');
    form.cron_expression = scheduleConfig.cron || '';
    form.interval_seconds = scheduleConfig.interval || 3600;
  } catch {
    form.cron_expression = '';
    form.interval_seconds = 3600;
  }
  
  isModalVisible.value = true;
};

const handleDelete = (record) => {
  Modal.warning({
    title: '确认删除',
    content: `确定要删除任务 "${record.name}" 吗？`,
    onOk: async () => {
      try {
        await taskService.deleteTask(record.id);
        await loadTasks();
        Message.success('删除成功');
      } catch (error) {
        Message.error('删除失败: ' + error.message);
      }
    }
  });
};

const handleToggle = async (record) => {
  try {
    await taskService.toggleTask(record.id, !record.enabled);
    await loadTasks();
    Message.success(record.enabled ? '任务已禁用' : '任务已启用');
  } catch (error) {
    Message.error('操作失败: ' + error.message);
  }
};

const handleRunNow = async (record) => {
  try {
    await taskService.runTaskNow(record.id);
    Message.success(`任务 "${record.name}" 已添加到执行队列`);
  } catch (error) {
    Message.error('执行失败: ' + error.message);
  }
};

const handleSubmit = async () => {
  if (!form.name) {
    Message.error('请输入任务名称');
    return;
  }
  
  if (!form.script_id) {
    Message.error('请选择执行脚本');
    return;
  }
  
  if (form.wallet_ids.length === 0) {
    Message.error('请至少选择一个钱包');
    return;
  }
  
  if (form.schedule_type === 'cron') {
    if (!form.cron_expression) {
      Message.error('请输入Cron表达式');
      return;
    }
    if (!validateCron(form.cron_expression)) {
      Message.error('Cron表达式格式不正确');
      return;
    }
  }
  
  if (form.schedule_type === 'interval' && form.interval_seconds < 60) {
    Message.error('间隔时间不能少于60秒');
    return;
  }
  
  if (form.profile_strategy === 'specific' && !form.specific_profile_id) {
    Message.error('请选择指定环境');
    return;
  }

  const scheduleConfig = {};
  if (form.schedule_type === 'cron') {
    scheduleConfig.cron = form.cron_expression;
  } else if (form.schedule_type === 'interval') {
    scheduleConfig.interval = form.interval_seconds;
  }

  const request = {
    name: form.name,
    description: form.description,
    schedule_type: form.schedule_type,
    schedule_config: scheduleConfig,
    script_id: form.script_id,
    wallet_ids: form.wallet_ids,
    profile_strategy: form.profile_strategy,
    specific_profile_id: form.specific_profile_id,
    parallel_mode: form.parallel_mode,
    max_parallel: form.max_parallel,
    retry_count: form.retry_count,
    retry_interval: form.retry_interval,
    enabled: form.enabled
  };

  try {
    if (isEditing.value) {
      await taskService.updateTask(editingId.value, request);
      Message.success('更新成功');
    } else {
      await taskService.createTask(request);
      Message.success('创建成功');
    }
    
    isModalVisible.value = false;
    await loadTasks();
  } catch (error) {
    Message.error('保存失败: ' + error.message);
  }
};

// 导出任务
const handleExport = async () => {
  if (tasks.value.length === 0) {
    Message.warning('没有可导出的任务');
    return;
  }
  
  try {
    const savePath = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: `tasks_export_${new Date().toISOString().slice(0, 10)}.json`
    });
    
    if (savePath) {
      const exportData = tasks.value.map(t => ({
        name: t.name,
        description: t.description,
        schedule_type: t.schedule_type,
        schedule_config: t.schedule_config,
        script_id: t.script_id,
        wallet_ids: t.wallet_ids,
        profile_strategy: t.profile_strategy,
        parallel_mode: t.parallel_mode,
        max_parallel: t.max_parallel,
        retry_count: t.retry_count,
        retry_interval: t.retry_interval
      }));
      const content = new TextEncoder().encode(JSON.stringify(exportData, null, 2));
      await writeFile(savePath, content);
      Message.success('任务导出成功');
    }
  } catch (error) {
    console.error('Export error:', error);
    Message.error('导出失败');
  }
};

// 导入任务
const handleImport = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'JSON', extensions: ['json'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    });
    
    if (!selected) return;
    
    const content = await readFile(selected);
    const decoder = new TextDecoder();
    const imported = JSON.parse(decoder.decode(content));
    
    if (Array.isArray(imported) && imported.length > 0) {
      let successCount = 0;
      for (const task of imported) {
        try {
          await taskService.createTask({
            ...task,
            name: task.name + ' (导入)',
            enabled: false
          });
          successCount++;
        } catch (e) {
          console.error('导入任务失败:', e);
        }
      }
      
      await loadTasks();
      Message.success(`成功导入 ${successCount} 个任务`);
    } else {
      Message.warning('文件格式不正确');
    }
  } catch (error) {
    console.error('Import error:', error);
    Message.error('导入失败: ' + error.message);
  }
};

// 清空所有任务
const handleClearAll = () => {
  if (tasks.value.length === 0) {
    Message.info('任务列表为空');
    return;
  }
  
  Modal.warning({
    title: '确认清空',
    content: `确定要清空所有 ${tasks.value.length} 个任务吗？此操作不可恢复！`,
    okText: '确认清空',
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        for (const task of tasks.value) {
          await taskService.deleteTask(task.id);
        }
        await loadTasks();
        Message.success('已清空所有任务');
      } catch (error) {
        Message.error('清空失败: ' + error.message);
      }
    }
  });
};

// 查看任务详情
const handleViewDetail = (record) => {
  const scheduleConfig = JSON.parse(record.schedule_config || '{}');
  let scheduleInfo = '';
  
  switch (record.schedule_type) {
    case 'once':
      scheduleInfo = '单次执行';
      break;
    case 'interval':
      const interval = scheduleConfig.interval || 3600;
      scheduleInfo = `每 ${interval >= 3600 ? Math.floor(interval / 3600) + '小时' : Math.floor(interval / 60) + '分钟'}`;
      break;
    case 'cron':
      scheduleInfo = `Cron: ${scheduleConfig.cron || '-'}`;
      break;
  }
  
  const strategyLabels = { random: '随机分配', sequential: '顺序分配', specific: '指定环境' };
  
  Modal.info({
    title: '任务详情',
    content: `
      <div style="line-height: 1.8;">
        <p><strong>任务名称:</strong> ${record.name}</p>
        <p><strong>描述:</strong> ${record.description || '-'}</p>
        <p><strong>调度类型:</strong> ${getScheduleTypeLabel(record.schedule_type)}</p>
        <p><strong>调度配置:</strong> ${scheduleInfo}</p>
        <p><strong>环境策略:</strong> ${strategyLabels[record.profile_strategy] || '-'}</p>
        <p><strong>并行模式:</strong> ${record.parallel_mode ? '是 (最大' + record.max_parallel + '个)' : '否'}</p>
        <p><strong>重试次数:</strong> ${record.retry_count}次</p>
        <p><strong>重试间隔:</strong> ${record.retry_interval / 1000}秒</p>
        <p><strong>状态:</strong> ${record.enabled ? '启用' : '禁用'}</p>
        <p><strong>下次执行:</strong> ${record.next_run ? new Date(record.next_run).toLocaleString('zh-CN') : '-'}</p>
        <p><strong>最后执行:</strong> ${record.last_run ? new Date(record.last_run).toLocaleString('zh-CN') : '-'}</p>
      </div>
    `
  });
};

onMounted(() => {
  loadTasks();
  loadWallets();
  loadProfiles();
  loadScripts();
});
</script>

<template>
  <div class="task-manager">
    <div class="toolbar">
      <div class="left-tools">
        <a-button type="primary" @click="handleAdd">
          <template #icon><icon-plus /></template>
          新建任务
        </a-button>
        <a-button type="outline" status="success" @click="handleImport">
          <template #icon><icon-to-bottom /></template>
          导入任务
        </a-button>
        <a-button type="outline" @click="handleExport">
          <template #icon><icon-to-top /></template>
          导出任务
        </a-button>
        <a-button type="outline" status="danger" @click="handleClearAll" v-if="tasks.length > 0">
          <template #icon><icon-delete /></template>
          清空
        </a-button>
      </div>
    </div>

    <div class="table-wrapper">
      <a-table 
        :data="tasks" 
        :columns="columns" 
        :pagination="{ pageSize: 10 }"
        :bordered="false"
        :loading="loading"
        hoverable
        :empty-text="'暂无任务，请点击「新建任务」'"
      >
        <template #schedule_type="{ record }">
          <a-tag :color="getScheduleTypeColor(record.schedule_type)">
            {{ getScheduleTypeLabel(record.schedule_type) }}
          </a-tag>
        </template>
        <template #next_run="{ record }">
          <span :class="{ 'text-warning': record.next_run && new Date(record.next_run) < new Date() }">
            {{ formatNextRun(record.next_run) }}
          </span>
        </template>
        <template #status="{ record }">
          <a-switch 
            :model-value="record.enabled" 
            @change="() => handleToggle(record)"
            size="small"
          />
        </template>
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="mini" @click="handleRunNow(record)">
              <template #icon><icon-play-arrow /></template>
              执行
            </a-button>
            <a-button type="text" size="mini" @click="handleViewDetail(record)">
              <template #icon><icon-eye /></template>
              详情
            </a-button>
            <a-button type="text" size="mini" @click="handleEdit(record)">
              <template #icon><icon-edit /></template>
              编辑
            </a-button>
            <a-button type="text" status="danger" size="mini" @click="handleDelete(record)">
              <template #icon><icon-delete /></template>
              删除
            </a-button>
          </a-space>
        </template>
      </a-table>
    </div>

    <a-modal 
      v-model:visible="isModalVisible" 
      :title="isEditing ? '编辑任务' : '新建任务'" 
      @ok="handleSubmit"
      :width="700"
      :mask-closable="false"
    >
      <a-form :model="form" layout="vertical">
        <a-row :gutter="16">
          <a-col :span="16">
            <a-form-item label="任务名称" required>
              <a-input v-model="form.name" placeholder="例如：每日签到任务" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="启用任务">
              <a-switch v-model="form.enabled" />
            </a-form-item>
          </a-col>
        </a-row>
        
        <a-form-item label="任务描述">
          <a-textarea v-model="form.description" placeholder="可选：输入任务描述" :auto-size="{ minRows: 2, maxRows: 3 }" />
        </a-form-item>
        
        <a-divider orientation="left">调度配置</a-divider>
        
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item label="调度类型" required>
              <a-select v-model="form.schedule_type" placeholder="选择调度类型">
                <a-option v-for="opt in scheduleTypeOptions" :key="opt.value" :value="opt.value" :label="opt.label" />
              </a-select>
            </a-form-item>
          </a-col>
          <a-col :span="12" v-if="form.schedule_type === 'interval'">
            <a-form-item label="间隔时间 (秒)" required>
              <a-input-number v-model="form.interval_seconds" :min="60" :step="60" style="width: 100%" />
            </a-form-item>
          </a-col>
        </a-row>
        
        <a-form-item v-if="form.schedule_type === 'cron'" label="Cron表达式" required>
          <a-input v-model="form.cron_expression" placeholder="0 9 * * * (每天9点)" />
          <div class="form-help">
            格式: 分 时 日 月 周 | 示例: 0 9 * * * (每天9点), 0 */6 * * * (每6小时)
          </div>
        </a-form-item>
        
        <a-divider orientation="left">执行配置</a-divider>
        
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item label="选择脚本" required>
              <a-select v-model="form.script_id" placeholder="选择要执行的脚本">
                <a-option v-for="script in scripts" :key="script.id" :value="script.id" :label="script.name" />
              </a-select>
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item label="环境分配策略" required>
              <a-select v-model="form.profile_strategy" placeholder="选择环境分配策略">
                <a-option v-for="opt in profileStrategyOptions" :key="opt.value" :value="opt.value" :label="opt.label" />
              </a-select>
            </a-form-item>
          </a-col>
        </a-row>
        
        <a-form-item v-if="form.profile_strategy === 'specific'" label="指定环境" required>
          <a-select v-model="form.specific_profile_id" placeholder="选择指定环境">
            <a-option v-for="profile in profiles" :key="profile.id" :value="profile.id" :label="profile.name" />
          </a-select>
        </a-form-item>
        
        <a-form-item label="选择钱包" required>
          <a-select v-model="form.wallet_ids" multiple placeholder="选择要执行的钱包">
            <a-option v-for="wallet in wallets" :key="wallet.id" :value="wallet.id" :label="`${wallet.name} (${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)})`" />
          </a-select>
        </a-form-item>
        
        <a-divider orientation="left">高级选项</a-divider>
        
        <a-row :gutter="16">
          <a-col :span="8">
            <a-form-item label="并行执行">
              <a-switch v-model="form.parallel_mode" />
            </a-form-item>
          </a-col>
          <a-col :span="8" v-if="form.parallel_mode">
            <a-form-item label="最大并行数">
              <a-input-number v-model="form.max_parallel" :min="1" :max="10" style="width: 100%" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="重试次数">
              <a-input-number v-model="form.retry_count" :min="0" :max="10" style="width: 100%" />
            </a-form-item>
          </a-col>
        </a-row>
        
        <a-form-item label="重试间隔 (毫秒)">
          <a-input-number v-model="form.retry_interval" :min="1000" :step="1000" style="width: 200px" />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.task-manager {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.left-tools {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.table-wrapper {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  padding: 10px;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

.form-help {
  color: var(--color-text-3);
  font-size: 12px;
  margin-top: 4px;
}

.text-warning {
  color: rgb(var(--orange-6));
}

:deep(.arco-table-cell) {
  background: transparent !important;
}

:deep(.arco-divider-text) {
  font-size: 13px;
  color: var(--color-text-3);
}
</style>
