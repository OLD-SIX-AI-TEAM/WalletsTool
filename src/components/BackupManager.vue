<script setup>
import { ref, reactive, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Message, Modal } from '@arco-design/web-vue';
import { IconCloud, IconDownload, IconUpload, IconLock, IconHistory } from '@arco-design/web-vue/es/icon';
import { save, open } from '@tauri-apps/plugin-dialog';

const props = defineProps({
  visible: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['update:visible', 'backup-restored']);

// State
const activeTab = ref('create');
const isLoading = ref(false);
// Create backup form - 使用单独的 ref 变量，与 WalletManager 保持一致
const backupPassword = ref('');
const confirmPassword = ref('');
const backupDescription = ref('');
const backupType = ref('full');

// Restore backup form
const restorePassword = ref('');
const mergeStrategy = ref('skip_existing');

const backupTypeOptions = [
  { label: '完整备份（钱包+分组+仅地址）', value: 'full' },
  { label: '仅钱包数据', value: 'wallets_only' },
  { label: '仅分组结构', value: 'groups_only' }
];

const mergeStrategyOptions = [
  { label: '跳过已存在的数据', value: 'skip_existing' },
  { label: '完全替换现有数据', value: 'replace_all' },
  { label: '覆盖冲突的数据', value: 'overwrite_existing' }
];

// 密码强度验证
const validatePasswordStrength = (password) => {
  const errors = [];
  
  if (password.length < 12) {
    errors.push('密码至少需要12位');
  }
  if (!/[A-Z]/.test(password)) {
    errors.push('需要包含大写字母');
  }
  if (!/[a-z]/.test(password)) {
    errors.push('需要包含小写字母');
  }
  if (!/[0-9]/.test(password)) {
    errors.push('需要包含数字');
  }
  if (!/[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(password)) {
    errors.push('需要包含特殊字符');
  }
  
  return errors;
};

// 获取密码强度信息
const getPasswordStrengthInfo = (password) => {
  const length = password.length;
  const hasUpper = /[A-Z]/.test(password);
  const hasLower = /[a-z]/.test(password);
  const hasNumber = /[0-9]/.test(password);
  const hasSpecial = /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(password);
  
  let strength = 0;
  if (length >= 12) strength++;
  if (hasUpper) strength++;
  if (hasLower) strength++;
  if (hasNumber) strength++;
  if (hasSpecial) strength++;
  
  const labels = ['非常弱', '弱', '一般', '中等', '良好', '强'];
  const colors = ['red', 'orange', 'yellow', 'green', 'blue', 'purple'];
  
  return {
    score: strength,
    label: labels[Math.min(strength, 5)],
    color: colors[Math.min(strength, 5)]
  };
};

// Computed
const canCreateBackup = computed(() => {
  const password = backupPassword.value;
  const confirmPwd = confirmPassword.value;
  const hasPassword = !!password && password.length > 0;
  const isMatch = password === confirmPwd;
  const errors = validatePasswordStrength(password);
  const isStrongEnough = errors.length === 0;

  return hasPassword && isMatch && isStrongEnough;
});

const backupPasswordErrors = computed(() => {
  if (!backupPassword.value) return '';
  return validatePasswordStrength(backupPassword.value).join('；');
});

// Methods
const handleClose = () => {
  emit('update:visible', false);
  resetForms();
};

const resetForms = () => {
  backupPassword.value = '';
  confirmPassword.value = '';
  backupDescription.value = '';
  backupType.value = 'full';
  restorePassword.value = '';
  mergeStrategy.value = 'skip_existing';
};

// 创建备份并保存到文件
const handleCreateBackup = async () => {
  if (!canCreateBackup.value) {
    Message.error('密码强度不足，请使用至少12位的强密码（包含大小写字母、数字和特殊字符）');
    return;
  }

  isLoading.value = true;
  try {
    // 1. 创建加密备份
    const backup = await invoke('create_encrypted_backup', {
      request: {
        backup_password: backupPassword.value,
        description: backupDescription.value || null,
        backup_type: backupType.value
      }
    });

    // 2. 选择保存位置
    const filePath = await save({
      defaultPath: `wallet_backup_${formatDate(new Date())}.json`,
      filters: [
        { name: '加密备份文件', extensions: ['json'] },
        { name: '所有文件', extensions: ['*'] }
      ]
    });

    if (filePath) {
      // 3. 保存到文件
      await invoke('save_backup_to_file', {
        backup,
        file_path: filePath
      });

      Message.success('备份创建成功！');
      handleClose();
    }
  } catch (error) {
    Message.error('备份失败: ' + error);
  } finally {
    isLoading.value = false;
  }
};

// 当前正在恢复的备份
const currentRestoreBackup = ref(null);
const showRestorePasswordModal = ref(false);
const restorePasswordInput = ref('');
const restorePasswordInputRef = ref(null);

// 从文件恢复备份
const handleRestoreFromFile = async () => {
  try {
    // 1. 选择备份文件
    const selected = await open({
      multiple: false,
      filters: [
        { name: '加密备份文件', extensions: ['json'] },
        { name: '所有文件', extensions: ['*'] }
      ]
    });

    if (!selected) return;

    const filePath = Array.isArray(selected) ? selected[0] : selected;

    // 2. 读取备份文件
    const backup = await invoke('load_backup_from_file', { file_path: filePath });

    // 3. 保存备份并显示确认信息
    currentRestoreBackup.value = backup;
    
    Modal.confirm({
      title: '恢复备份确认',
      content: `备份信息：\n创建时间：${new Date(backup.metadata.created_at).toLocaleString()}\n钱包数量：${backup.metadata.wallet_count}\n分组数量：${backup.metadata.group_count}\n仅地址数量：${backup.metadata.watch_address_count}${backup.metadata.description ? '\n描述：' + backup.metadata.description : ''}\n\n注意：恢复操作将修改现有数据，请确保已备份当前数据！`,
      okText: '确认恢复',
      cancelText: '取消',
      onOk: () => {
        // 显示密码输入弹窗
        restorePasswordInput.value = '';
        showRestorePasswordModal.value = true;
        nextTick(() => {
          restorePasswordInputRef.value?.focus();
        });
      }
    });
  } catch (error) {
    Message.error('加载备份失败: ' + error);
  }
};

// 执行恢复
const executeRestore = async () => {
  if (!restorePasswordInput.value) {
    Message.error('请输入备份密码');
    return;
  }

  if (!currentRestoreBackup.value) {
    Message.error('备份数据不存在');
    return;
  }

  isLoading.value = true;
  try {
    const result = await invoke('restore_encrypted_backup', {
      request: {
        backup_data: currentRestoreBackup.value,
        backup_password: restorePasswordInput.value,
        merge_strategy: mergeStrategy.value
      }
    });

    if (result.success) {
      Message.success(`恢复成功！共恢复 ${result.wallets_restored} 个钱包，${result.groups_restored} 个分组`);
      showRestorePasswordModal.value = false;
      currentRestoreBackup.value = null;
      emit('backup-restored');
      handleClose();
    } else {
      Message.warning(`恢复完成，但有 ${result.errors.length} 个错误`);
      console.error('恢复错误:', result.errors);
    }
  } catch (error) {
    Message.error('恢复失败: ' + error);
  } finally {
    isLoading.value = false;
  }
};

const formatDate = (date) => {
  const d = new Date(date);
  return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}_${String(d.getHours()).padStart(2, '0')}${String(d.getMinutes()).padStart(2, '0')}`;
};


</script>

<template>
  <a-modal
    :visible="visible"
    title="云端备份管理"
    width="700px"
    :footer="false"
    @cancel="handleClose"
  >
    <a-tabs v-model:active-key="activeTab">
      <!-- 创建备份 -->
      <a-tab-pane key="create" title="创建备份">
        <a-form layout="vertical">
          <!-- <a-alert type="info" style="margin-bottom: 16px;">
            备份文件将使用 AES-256-GCM 加密，请妥善保管备份密码！
          </a-alert> -->

          <a-form-item label="备份类型">
            <a-radio-group v-model="backupType" type="button" default-value="full">
              <a-radio v-for="opt in backupTypeOptions" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </a-radio>
            </a-radio-group>
          </a-form-item>

          <a-form-item label="备份密码" required :help="backupPasswordErrors || (backupPassword ? '密码强度：' + getPasswordStrengthInfo(backupPassword).label : '')">
            <a-input-password
              v-model="backupPassword"
              placeholder="设置备份密码"
              allow-clear
            >
              <template #prefix><icon-lock /></template>
            </a-input-password>
          </a-form-item>

          <a-form-item label="确认密码" required>
            <a-input-password
              v-model="confirmPassword"
              placeholder="再次输入备份密码"
              allow-clear
              :error-info="confirmPassword.length > 0 && backupPassword !== confirmPassword ? '两次输入的密码不一致' : ''"
            >
              <template #prefix><icon-lock /></template>
            </a-input-password>
          </a-form-item>

          <a-form-item label="备份描述（可选）">
            <a-textarea
              v-model="backupDescription"
              placeholder="添加备份描述，方便日后识别"
              :auto-size="{ minRows: 2, maxRows: 4 }"
            />
          </a-form-item>

          <a-form-item>
            <a-button
              type="primary"
              size="large"
              :loading="isLoading"
              :disabled="!canCreateBackup"
              @click="handleCreateBackup"
              long
            >
              <template #icon><icon-cloud /></template>
              创建加密备份
            </a-button>
          </a-form-item>
        </a-form>
      </a-tab-pane>

      <!-- 恢复备份 -->
      <a-tab-pane key="restore" title="恢复备份">
        <a-form layout="vertical">
          <a-alert type="warning" style="margin-bottom: 16px;">
            恢复备份将修改当前钱包数据，建议先创建当前数据的备份！
          </a-alert>

          <a-form-item label="合并策略">
            <a-select v-model="mergeStrategy">
              <a-option
                v-for="opt in mergeStrategyOptions"
                :key="opt.value"
                :value="opt.value"
                :label="opt.label"
              />
            </a-select>
          </a-form-item>

          <a-form-item>
            <a-button
              type="primary"
              size="large"
              :loading="isLoading"
              @click="handleRestoreFromFile"
              long
            >
              <template #icon><icon-upload /></template>
              选择备份文件恢复
            </a-button>
          </a-form-item>
        </a-form>
      </a-tab-pane>

      <!-- 使用说明 -->
      <a-tab-pane key="help" title="使用说明">
        <div class="help-content">
          <h4>🔐 加密备份说明</h4>
          <ul>
            <li>备份使用 <strong>AES-256-GCM</strong> 加密算法，安全性极高</li>
            <li>备份密码使用 <strong>PBKDF2</strong> 进行 60 万次迭代派生密钥</li>
            <li>备份文件包含钱包私钥、助记词等敏感信息，请妥善保管</li>
          </ul>

          <h4>☁️ 云端存储建议</h4>
          <ul>
            <li>将备份文件保存到云盘（如 iCloud、Google Drive、OneDrive 等）</li>
            <li>建议定期创建备份，特别是在添加重要钱包后</li>
            <li>可以将备份文件复制到多个云盘，增加安全性</li>
          </ul>

          <h4>⚠️ 重要提醒</h4>
          <ul>
            <li><strong style="color: #ff4d4f;">备份密码无法找回</strong>，请务必牢记</li>
            <li>建议将备份密码与备份文件分开存储</li>
            <li>恢复备份前建议先备份当前数据</li>
          </ul>
        </div>
      </a-tab-pane>
    </a-tabs>

    <!-- 恢复密码输入弹窗 -->
    <a-modal
      v-model:visible="showRestorePasswordModal"
      title="输入备份密码"
      :footer="false"
      :mask-closable="false"
    >
      <a-form layout="vertical">
        <a-alert type="warning" style="margin-bottom: 16px;">
          请输入创建备份时设置的密码。密码错误将无法恢复数据。
        </a-alert>
        <a-form-item label="备份密码" required>
          <a-input-password
            ref="restorePasswordInputRef"
            v-model="restorePasswordInput"
            placeholder="请输入备份密码"
            allow-clear
            @keyup.enter="executeRestore"
          >
            <template #prefix><icon-lock /></template>
          </a-input-password>
        </a-form-item>
        <a-form-item>
          <a-space style="width: 100%; justify-content: flex-end;">
            <a-button style="margin-right: 10px;" @click="showRestorePasswordModal = false">取消</a-button>
            <a-button type="primary" :loading="isLoading" @click="executeRestore">
              开始恢复
            </a-button>
          </a-space>
        </a-form-item>
      </a-form></a-modal>
  </a-modal>
</template>

<style scoped>
.help-content {
  padding: 8px;
}

.help-content h4 {
  margin: 16px 0 8px 0;
  color: var(--color-text-1);
}

.help-content h4:first-child {
  margin-top: 0;
}

.help-content ul {
  margin: 0;
  padding-left: 20px;
}

.help-content li {
  margin: 8px 0;
  color: var(--color-text-2);
  line-height: 1.6;
}

/* 隐藏浏览器密码管理器图标 */
:deep(input[type="password"]::-webkit-credentials-auto-fill-button),
:deep(input[type="password"]::-webkit-textfield-decoration-container),
:deep(input::-webkit-contacts-auto-fill-button),
:deep(input::-webkit-password-toggle) {
  visibility: hidden;
  display: none !important;
  pointer-events: none;
}

/* Firefox */
:deep(input[type="password"]) {
  -moz-appearance: textfield;
}

/* Edge/IE */
:deep(input[type="password"]::-ms-reveal) {
  display: none;
}
</style>
