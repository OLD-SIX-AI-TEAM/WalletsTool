<script setup>
import { ref, onMounted, nextTick } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { 
  IconPlus, 
  IconPublic, 
  IconRight, 
  IconSettings, 
  IconEdit,
  IconRobot,
  IconDelete,
  IconToBottom,
  IconToTop
} from '@arco-design/web-vue/es/icon';
import { profileService, initBrowserAutomationTables } from '../services/browserAutomationService';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';

const profiles = ref([]);
const loading = ref(false);

const loadProfiles = async () => {
  loading.value = true;
  try {
    profiles.value = await profileService.getProfiles();
  } catch (error) {
    console.error('Failed to load profiles:', error);
    Message.error('加载环境配置失败: ' + error);
  } finally {
    loading.value = false;
  }
};

const activeProfile = ref(null);
const isEditing = ref(false);
const showBatchModal = ref(false);
const batchCount = ref(100);

const editingProfileId = ref(null);
const editNameInput = ref(null);
const editNameValue = ref('');

const startEditName = async (profile, event) => {
  event?.stopPropagation();
  editingProfileId.value = profile.id;
  editNameValue.value = profile.name;
  await nextTick();
  const inputEl = Array.isArray(editNameInput.value) ? editNameInput.value[0] : editNameInput.value;
  inputEl?.focus?.();
  inputEl?.select?.();
};

const saveEditName = async () => {
  const trimmedName = editNameValue.value.trim();
  if (!trimmedName) {
    editingProfileId.value = null;
    return;
  }
  const profile = profiles.value.find(p => p.id === editingProfileId.value);
  if (profile) {
    try {
      await profileService.updateProfile({
        id: profile.id,
        name: trimmedName
      });
      profile.name = trimmedName;
      Message.success('名称已更新');
    } catch (error) {
      Message.error('更新名称失败: ' + error);
    }
  }
  editingProfileId.value = null;
};

const cancelEditName = () => {
  editingProfileId.value = null;
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

const handleBatchGenerate = async () => {
  try {
    const newProfiles = await profileService.batchGenerate({
      count: batchCount.value,
      enable_all_spoofs: true
    });
    
    profiles.value.push(...newProfiles);
    Message.success(`成功生成 ${batchCount.value} 个配置`);
    showBatchModal.value = false;
  } catch (error) {
    Message.error('批量生成失败: ' + error);
  }
};

const handleNewProfile = async () => {
  try {
    const newProfile = await profileService.createProfile({
      name: `New Profile ${profiles.value.length + 1}`,
      user_agent: USER_AGENTS[0],
      viewport_width: 1920,
      viewport_height: 1080,
      proxy_type: 'direct',
      canvas_spoof: true,
      webgl_spoof: true,
      audio_spoof: true,
      timezone_spoof: true,
      geolocation_spoof: true,
      font_spoof: true,
      webrtc_spoof: true,
      navigator_override: true,
      webdriver_override: true
    });
    
    profiles.value.push(newProfile);
    handleEdit(newProfile);
    Message.success('新配置已创建');
  } catch (error) {
    Message.error('创建配置失败: ' + error);
  }
};

const handleEdit = (profile) => {
  if (editingProfileId.value !== null) {
    editingProfileId.value = null;
  }
  activeProfile.value = { ...profile };
  isEditing.value = true;
};

const handleSave = async () => {
  if (activeProfile.value) {
    try {
      const updated = await profileService.updateProfile(activeProfile.value);
      const index = profiles.value.findIndex(p => p.id === updated.id);
      if (index !== -1) {
        profiles.value[index] = updated;
      }
      isEditing.value = false;
      Message.success('配置已保存');
    } catch (error) {
      Message.error('保存失败: ' + error);
    }
  }
};

const handleDelete = () => {
  if (activeProfile.value) {
    Modal.warning({
      title: '确认删除',
      content: `确定要删除配置 "${activeProfile.value.name}" 吗？`,
      onOk: async () => {
        try {
          await profileService.deleteProfile(activeProfile.value.id);
          const index = profiles.value.findIndex(p => p.id === activeProfile.value.id);
          if (index !== -1) {
            profiles.value.splice(index, 1);
          }
          Message.success('配置已删除');
          isEditing.value = false;
          activeProfile.value = null;
        } catch (error) {
          Message.error('删除失败: ' + error);
        }
      }
    });
  }
};

const handleCancel = () => {
  isEditing.value = false;
  activeProfile.value = null;
};

// 导出配置
const handleExport = async () => {
  if (profiles.value.length === 0) {
    Message.warning('没有可导出的配置');
    return;
  }
  
  try {
    const savePath = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: `browser_profiles_${new Date().toISOString().slice(0, 10)}.json`
    });
    
    if (savePath) {
      const content = new TextEncoder().encode(JSON.stringify(profiles.value, null, 2));
      await writeFile(savePath, content);
      Message.success('配置导出成功');
    }
  } catch (error) {
    console.error('Export error:', error);
    Message.error('导出失败');
  }
};

// 导入配置
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
      for (const p of imported) {
        try {
          await profileService.createProfile({
            name: p.name + ' (导入)',
            description: p.description,
            user_agent: p.user_agent,
            viewport_width: p.viewport_width,
            viewport_height: p.viewport_height,
            device_scale_factor: p.device_scale_factor,
            locale: p.locale,
            timezone_id: p.timezone_id,
            proxy_type: p.proxy_type,
            proxy_host: p.proxy_host,
            proxy_port: p.proxy_port,
            proxy_username: p.proxy_username,
            proxy_password: p.proxy_password,
            canvas_spoof: p.canvas_spoof,
            webgl_spoof: p.webgl_spoof,
            audio_spoof: p.audio_spoof,
            timezone_spoof: p.timezone_spoof,
            geolocation_spoof: p.geolocation_spoof,
            font_spoof: p.font_spoof,
            webrtc_spoof: p.webrtc_spoof,
            navigator_override: p.navigator_override,
            webdriver_override: p.webdriver_override,
            custom_headers: p.custom_headers,
            headless: p.headless,
            extensions: p.extensions
          });
          successCount++;
        } catch (e) {
          console.error('Failed to import profile:', e);
        }
      }
      
      await loadProfiles();
      Message.success(`成功导入 ${successCount} 个配置`);
    } else {
      Message.warning('文件格式不正确');
    }
  } catch (error) {
    console.error('Import error:', error);
    Message.error('导入失败: ' + error.message);
  }
};

const USER_AGENTS = [
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15"
];

const PROXY_TYPES = [
  { label: 'Direct', value: 'direct' },
  { label: 'HTTP', value: 'http' },
  { label: 'SOCKS5', value: 'socks5' }
];

onMounted(async () => {
  // 初始化表结构（包含迁移）
  try {
    await initBrowserAutomationTables();
  } catch (e) {
    console.log('Tables may already exist:', e);
  }
  await loadProfiles();
});
</script>

<template>
  <div class="browser-farm">
    <div class="profile-list">
      <div class="list-header">
        <h3 class="header-title">环境配置列表</h3>
        <p class="header-subtitle">{{ profiles.length }} 个配置</p>
        <div class="header-actions-row">
          <a-button type="primary" size="small" @click="handleNewProfile" long>
            <template #icon><icon-plus /></template>
            新建配置
          </a-button>
        </div>
        <div class="header-actions-row secondary">
          <a-button type="outline" size="small" @click="showBatchModal = true">
            <template #icon><icon-robot /></template>
            批量生成
          </a-button>
          <div class="action-group">
            <a-button type="secondary" size="small" @click="handleImport" title="导入">
              <template #icon><icon-to-bottom /></template>
            </a-button>
            <a-button type="secondary" size="small" @click="handleExport" title="导出">
              <template #icon><icon-to-top /></template>
            </a-button>
          </div>
        </div>
      </div>
      
      <div class="list-content" v-loading="loading">
        <div 
          v-for="profile in profiles" 
          :key="profile.id"
          class="profile-item"
          :class="{ active: activeProfile && activeProfile.id === profile.id }"
          @click="handleEdit(profile)"
        >
          <div class="profile-icon">
            <icon-public />
          </div>
          <div class="profile-info">
            <template v-if="editingProfileId === profile.id">
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
              <div class="profile-name editable" @click="(e) => startEditName(profile, e)" title="点击编辑名称">
                {{ profile.name }}
              </div>
            </template>
            <div class="profile-desc">{{ profile.viewport_width }}x{{ profile.viewport_height }} | {{ profile.proxy_type }}</div>
          </div>
          <icon-right class="arrow" />
        </div>
        <div v-if="profiles.length === 0" class="empty-profiles">
          暂无配置，点击"新建"或"批量生成"创建
        </div>
      </div>
    </div>

    <div class="profile-editor" v-if="isEditing && activeProfile">
      <div class="editor-header">
        <h3>编辑配置: {{ activeProfile.name }}</h3>
        <div class="actions">
          <a-button status="danger" @click="handleDelete">删除</a-button>
          <a-button style="margin-left: 10px;" @click="handleCancel">取消</a-button>
          <a-button style="margin-left: 10px;" type="primary" @click="handleSave">保存</a-button>
        </div>
      </div>

      <div class="editor-form">
        <a-form :model="activeProfile" layout="vertical">
          <a-form-item label="配置名称">
            <a-input v-model="activeProfile.name" />
          </a-form-item>
          
          <a-form-item label="描述">
            <a-textarea v-model="activeProfile.description" :auto-size="{ minRows: 2, maxRows: 4 }" placeholder="配置描述..." />
          </a-form-item>

          <a-form-item label="User Agent">
            <a-textarea v-model="activeProfile.user_agent" :auto-size="{ minRows: 2, maxRows: 4 }" />
          </a-form-item>

          <a-row :gutter="16">
            <a-col :span="8">
              <a-form-item label="视口宽度">
                <a-input-number v-model="activeProfile.viewport_width" :min="320" :max="3840" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="视口高度">
                <a-input-number v-model="activeProfile.viewport_height" :min="240" :max="2160" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="设备缩放">
                <a-input-number v-model="activeProfile.device_scale_factor" :min="0.5" :max="3" :step="0.1" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-row :gutter="16">
            <a-col :span="12">
              <a-form-item label="语言">
                <a-input v-model="activeProfile.locale" placeholder="en-US" />
              </a-form-item>
            </a-col>
            <a-col :span="12">
              <a-form-item label="时区">
                <a-input v-model="activeProfile.timezone_id" placeholder="America/New_York" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-divider orientation="left">代理配置</a-divider>
          
          <a-row :gutter="16">
            <a-col :span="12">
              <a-form-item label="代理类型">
                <a-select v-model="activeProfile.proxy_type">
                  <a-option v-for="type in PROXY_TYPES" :key="type.value" :value="type.value">{{ type.label }}</a-option>
                </a-select>
              </a-form-item>
            </a-col>
            <a-col :span="12">
              <a-form-item label="代理主机">
                <a-input v-model="activeProfile.proxy_host" placeholder="proxy.example.com" />
              </a-form-item>
            </a-col>
          </a-row>
          
          <a-row :gutter="16">
            <a-col :span="8">
              <a-form-item label="代理端口">
                <a-input-number v-model="activeProfile.proxy_port" :min="1" :max="65535" placeholder="8080" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="代理用户名">
                <a-input v-model="activeProfile.proxy_username" placeholder="可选" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="代理密码">
                <a-input-password v-model="activeProfile.proxy_password" placeholder="可选" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-divider orientation="left">指纹保护 (Anti-Detect)</a-divider>

          <div class="fingerprint-switches">
            <div class="switch-item">
              <a-switch v-model="activeProfile.canvas_spoof" />
              <span>Canvas 指纹混淆</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webgl_spoof" />
              <span>WebGL 渲染伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.audio_spoof" />
              <span>Audio Context 噪音</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.timezone_spoof" />
              <span>时区伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.geolocation_spoof" />
              <span>地理位置伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.font_spoof" />
              <span>字体伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webrtc_spoof" />
              <span>WebRTC 防泄漏</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.navigator_override" />
              <span>Navigator 覆盖</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webdriver_override" />
              <span>WebDriver 覆盖</span>
            </div>
          </div>

          <a-divider orientation="left">高级选项</a-divider>

          <div class="advanced-options">
            <div class="switch-item">
              <a-switch v-model="activeProfile.headless" />
              <span>无头模式</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.is_default" />
              <span>设为默认</span>
            </div>
          </div>
        </a-form>
      </div>
    </div>
    
    <div class="empty-state" v-else>
      <icon-settings style="font-size: 48px; color: var(--color-text-4)" />
      <p>请选择左侧配置进行编辑</p>
    </div>

    <a-modal v-model:visible="showBatchModal" title="批量生成环境配置" @ok="handleBatchGenerate">
      <a-form :model="{ batchCount }">
        <a-form-item label="生成数量">
          <a-input-number v-model="batchCount" :min="1" :max="1000" />
        </a-form-item>
        <div style="color: var(--color-text-3); font-size: 12px;">
          <p>将随机生成以下配置项：</p>
          <ul>
            <li>User Agent (Chrome/Firefox/Safari)</li>
            <li>分辨率 (1920x1080 等常用分辨率)</li>
            <li>语言和时区</li>
            <li>所有指纹保护选项（默认开启）</li>
          </ul>
        </div>
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.browser-farm {
  height: 100%;
  display: flex;
  gap: 10px;
}

.profile-list {
  width: 280px;
  background: var(--color-bg-2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
}

.list-header {
  padding: 15px;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.header-title {
  margin: 0;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.header-subtitle {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-3);
}

.header-actions-row {
  display: flex;
  gap: 8px;
}

.header-actions-row.secondary {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 8px;
  border-top: 1px solid var(--color-border);
  margin-top: 4px;
}

.action-group {
  display: flex;
  gap: 4px;
}

.action-group .arco-btn {
  padding: 0 6px;
  color: var(--color-text-3);
}

.action-group .arco-btn:hover {
  color: rgb(var(--primary-6));
  background: var(--color-fill-2);
}

.list-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

/* 自定义滚动条样式 */
.list-content::-webkit-scrollbar {
  width: 6px;
}

.list-content::-webkit-scrollbar-track {
  background: transparent;
}

.list-content::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 3px;
}

.list-content::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox 滚动条 */
.list-content {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) transparent;
}

.profile-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-2);
}

.profile-item:hover {
  background: var(--color-fill-2);
}

.profile-item.active {
  background: rgba(var(--primary-6), 0.1);
}

.profile-icon {
  width: 32px;
  height: 32px;
  background: rgba(var(--primary-6), 0.1);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgb(var(--primary-6));
}

.profile-info {
  flex: 1;
}

.profile-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.profile-name.editable {
  cursor: text;
  padding: 2px 4px;
  margin: -2px -4px;
  border-radius: 4px;
  transition: all 0.2s;
}

.profile-name.editable:hover {
  background: var(--color-fill-2);
}

.name-edit-input {
  width: 100%;
  background: var(--color-bg-1);
  border: 1px solid rgb(var(--primary-6));
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
  outline: none;
  line-height: 1.2;
}

.name-edit-input:focus {
  border-color: rgb(var(--primary-6));
  box-shadow: 0 0 0 2px rgba(var(--primary-6), 0.2);
}

.profile-desc {
  font-size: 12px;
  color: var(--color-text-3);
}

.arrow {
  color: var(--color-text-4);
  font-size: 12px;
}

.empty-profiles {
  text-align: center;
  padding: 30px 20px;
  color: var(--color-text-3);
  font-size: 12px;
}

.profile-editor {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
  overflow: hidden;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  padding-bottom: 15px;
  border-bottom: 1px solid var(--color-border);
}

.editor-header h3 {
  margin: 0;
  color: var(--color-text-1);
}

.editor-form {
  flex: 1;
  overflow-y: auto;
  padding-right: 10px;
}

/* 编辑表单自定义滚动条 */
.editor-form::-webkit-scrollbar {
  width: 6px;
}

.editor-form::-webkit-scrollbar-track {
  background: transparent;
}

.editor-form::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 3px;
}

.editor-form::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox 滚动条 */
.editor-form {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) transparent;
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
}

/* 指纹保护开关布局 */
.fingerprint-switches {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  margin-bottom: 10px;
}

.switch-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: var(--color-fill-2);
  border-radius: 6px;
  transition: background 0.2s;
}

.switch-item:hover {
  background: var(--color-fill-3);
}

.switch-item span {
  font-size: 13px;
  color: var(--color-text-2);
  white-space: nowrap;
}

/* 高级选项布局 */
.advanced-options {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}
</style>
