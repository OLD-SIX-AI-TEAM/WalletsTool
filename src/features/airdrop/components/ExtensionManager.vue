<script setup>
import { ref, onMounted, computed } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import {
  IconPlus,
  IconFolder,
  IconDelete,
  IconRefresh,
  IconCheck,
  IconClose,
  IconSettings,
  IconExport,
  IconToBottom
} from '@arco-design/web-vue/es/icon';
import { extensionService } from '../services/extensionService';
import { initBrowserAutomationTables } from '../services/browserAutomationService';

const extensions = ref([]);
const loading = ref(false);
const scanning = ref(false);
const scanResults = ref([]);

const showScanModal = ref(false);
const scanFolderPath = ref('');

const loadExtensions = async () => {
  loading.value = true;
  try {
    extensions.value = await extensionService.getExtensions();
  } catch (error) {
    console.error('Failed to load extensions:', error);
    Message.error('加载插件列表失败: ' + error);
  } finally {
    loading.value = false;
  }
};

const handleToggle = async (extension) => {
  try {
    const updated = await extensionService.toggleExtension(extension.id, !extension.enabled);
    const index = extensions.value.findIndex(e => e.id === extension.id);
    if (index !== -1) {
      extensions.value[index] = updated;
    }
    Message.success(updated.enabled ? '插件已启用' : '插件已禁用');
  } catch (error) {
    Message.error('切换状态失败: ' + error);
  }
};

const handleDelete = (extension) => {
  Modal.warning({
    title: '确认删除',
    content: `确定要删除插件 "${extension.name}" 吗？`,
    onOk: async () => {
      try {
        await extensionService.deleteExtension(extension.id);
        const index = extensions.value.findIndex(e => e.id === extension.id);
        if (index !== -1) {
          extensions.value.splice(index, 1);
        }
        Message.success('插件已删除');
      } catch (error) {
        Message.error('删除失败: ' + error);
      }
    }
  });
};

const handleSelectFolder = async () => {
  try {
    const folderPath = await extensionService.selectExtensionFolder();
    if (folderPath) {
      scanFolderPath.value = folderPath;
      await handleScanFolder();
    }
  } catch (error) {
    Message.error('选择目录失败: ' + error);
  }
};

const handleScanFolder = async () => {
  if (!scanFolderPath.value) {
    Message.warning('请先选择插件目录');
    return;
  }

  console.log('[ExtensionManager] 开始扫描目录:', scanFolderPath.value);
  scanning.value = true;
  try {
    scanResults.value = await extensionService.scanExtensionFolder(scanFolderPath.value);
    console.log('[ExtensionManager] 扫描结果:', scanResults.value);
    
    // 显示详细结果
    const validCount = scanResults.value.filter(r => r.hasManifest).length;
    const totalCount = scanResults.value.length;
    console.log(`[ExtensionManager] 找到 ${totalCount} 个子目录，${validCount} 个有效插件`);
    
    if (scanResults.value.length === 0) {
      Message.info('未找到任何子目录');
    } else if (validCount === 0) {
      Message.info(`扫描了 ${totalCount} 个子目录，但未找到有效的浏览器插件（缺少 manifest.json）`);
    }
  } catch (error) {
    console.error('[ExtensionManager] 扫描目录失败:', error);
    Message.error('扫描目录失败: ' + error);
  } finally {
    scanning.value = false;
  }
};

const handleImportExtension = async (result) => {
  try {
    const extension = await extensionService.importExtensionFromFolder(
      result.path,
      result.manifestInfo?.name || result.name
    );
    extensions.value.push(extension);
    Message.success(`插件 "${extension.name}" 导入成功`);
  } catch (error) {
    Message.error('导入插件失败: ' + error);
  }
};

const handleImportAll = async () => {
  const validExtensions = scanResults.value.filter(r => r.hasManifest);
  if (validExtensions.length === 0) {
    Message.warning('没有可导入的插件');
    return;
  }

  let successCount = 0;
  for (const result of validExtensions) {
    try {
      const extension = await extensionService.importExtensionFromFolder(
        result.path,
        result.manifestInfo?.name || result.name
      );
      if (!extensions.value.find(e => e.path === extension.path)) {
        extensions.value.push(extension);
      }
      successCount++;
    } catch (error) {
      console.error('Failed to import:', result.name, error);
    }
  }

  Message.success(`成功导入 ${successCount} 个插件`);
  showScanModal.value = false;
  scanResults.value = [];
};

const enabledCount = computed(() => extensions.value.filter(e => e.enabled).length);

onMounted(async () => {
  try {
    await initBrowserAutomationTables();
  } catch (e) {
    console.log('Tables may already exist:', e);
  }
  await loadExtensions();
});
</script>

<template>
  <div class="extension-manager">
    <div class="extension-list">
      <div class="list-header">
        <h3 class="header-title">浏览器插件管理</h3>
        <p class="header-subtitle">{{ extensions.length }} 个插件 · {{ enabledCount }} 个已启用</p>
        <div class="header-actions-row">
          <a-button type="primary" size="small" @click="showScanModal = true" long>
            <template #icon><icon-plus /></template>
            添加插件
          </a-button>
        </div>
        <div class="header-actions-row secondary">
          <a-button type="outline" size="small" @click="loadExtensions">
            <template #icon><icon-refresh /></template>
            刷新
          </a-button>
        </div>
      </div>
      
      <div class="list-content" v-loading="loading">
        <div 
          v-for="extension in extensions" 
          :key="extension.id"
          class="extension-item"
          :class="{ disabled: !extension.enabled }"
        >
          <div class="extension-icon">
            <icon-settings />
          </div>
          <div class="extension-info">
            <div class="extension-name">
              {{ extension.name }}
              <span v-if="extension.version" class="version">v{{ extension.version }}</span>
            </div>
            <div class="extension-desc" v-if="extension.description">
              {{ extension.description }}
            </div>
            <div class="extension-path" :title="extension.path">
              {{ extension.path }}
            </div>
          </div>
          <div class="extension-actions">
            <a-switch 
              :model-value="extension.enabled" 
              @change="handleToggle(extension)"
              size="small"
            />
            <a-button 
              type="text" 
              size="small" 
              status="danger"
              @click="handleDelete(extension)"
            >
              <template #icon><icon-delete /></template>
            </a-button>
          </div>
        </div>
        <div v-if="extensions.length === 0" class="empty-extensions">
          暂无插件，点击"添加插件"导入浏览器扩展
        </div>
      </div>
    </div>

    <div class="extension-tips">
      <div class="tips-header">
        <icon-settings style="margin-right: 8px;" />
        使用说明
      </div>
      <div class="tips-content">
        <div class="tip-item">
          <span class="tip-num">1</span>
          <span>启用插件后，在脚本录制时可以选择加载</span>
        </div>
        <div class="tip-item">
          <span class="tip-num">2</span>
          <span>支持导入 Chrome 扩展目录（包含 manifest.json）</span>
        </div>
        <div class="tip-item">
          <span class="tip-num">3</span>
          <span>插件将在浏览器启动时自动加载</span>
        </div>
      </div>
    </div>

    <a-modal 
      v-model:visible="showScanModal" 
      title="添加浏览器插件"
      :width="600"
      :footer="false"
    >
      <div class="scan-modal-content">
        <div class="scan-input-row">
          <a-input 
            v-model="scanFolderPath" 
            placeholder="选择或输入插件目录路径"
            style="flex: 1"
          />
          <a-button type="primary" @click="handleSelectFolder">
            <template #icon><icon-folder /></template>
            选择目录
          </a-button>
        </div>

        <a-button 
          type="outline" 
          long 
          @click="handleScanFolder"
          :loading="scanning"
          style="margin-top: 12px"
        >
          <template #icon><icon-refresh /></template>
          扫描目录
        </a-button>

        <div v-if="scanResults.length > 0" class="scan-results">
          <div class="results-header">
            <span>扫描结果 ({{ scanResults.filter(r => r.hasManifest).length }} 个有效插件)</span>
            <a-button 
              type="primary" 
              size="small"
              @click="handleImportAll"
              v-if="scanResults.some(r => r.hasManifest)"
            >
              <template #icon><icon-to-bottom /></template>
              全部导入
            </a-button>
          </div>
          
          <div class="results-list">
            <div 
              v-for="result in scanResults" 
              :key="result.path"
              class="result-item"
              :class="{ valid: result.hasManifest, invalid: !result.hasManifest }"
            >
              <div class="result-icon">
                <icon-check v-if="result.hasManifest" style="color: var(--color-success-6)" />
                <icon-close v-else style="color: var(--color-danger-6)" />
              </div>
              <div class="result-info">
                <div class="result-name">
                  {{ result.manifestInfo?.name || result.name }}
                  <span v-if="result.manifestInfo?.version" class="version">
                    v{{ result.manifestInfo.version }}
                  </span>
                </div>
                <div class="result-path">{{ result.path }}</div>
              </div>
              <a-button 
                v-if="result.hasManifest"
                type="primary"
                size="small"
                @click="handleImportExtension(result)"
              >
                导入
              </a-button>
            </div>
          </div>
        </div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.extension-manager {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.extension-list {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
  overflow: hidden;
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
  padding-top: 8px;
  border-top: 1px solid var(--color-border);
  margin-top: 4px;
}

.list-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

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

.extension-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  background: var(--color-fill-1);
  margin-bottom: 8px;
  transition: all 0.2s;
}

.extension-item:hover {
  background: var(--color-fill-2);
}

.extension-item.disabled {
  opacity: 0.6;
}

.extension-icon {
  width: 36px;
  height: 36px;
  background: rgba(var(--primary-6), 0.1);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgb(var(--primary-6));
  flex-shrink: 0;
}

.extension-info {
  flex: 1;
  min-width: 0;
}

.extension-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
  display: flex;
  align-items: center;
  gap: 8px;
}

.extension-name .version {
  font-size: 11px;
  color: var(--color-text-3);
  background: var(--color-fill-3);
  padding: 2px 6px;
  border-radius: 4px;
}

.extension-desc {
  font-size: 12px;
  color: var(--color-text-3);
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extension-path {
  font-size: 11px;
  color: var(--color-text-4);
  margin-top: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extension-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.empty-extensions {
  text-align: center;
  padding: 40px 20px;
  color: var(--color-text-3);
  font-size: 13px;
}

.extension-tips {
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  padding: 16px;
}

.tips-header {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
  margin-bottom: 12px;
  display: flex;
  align-items: center;
}

.tips-content {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tip-item {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--color-text-2);
}

.tip-num {
  width: 20px;
  height: 20px;
  background: rgba(var(--primary-6), 0.1);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  color: rgb(var(--primary-6));
  flex-shrink: 0;
}

.scan-modal-content {
  padding: 8px 0;
}

.scan-input-row {
  display: flex;
  gap: 12px;
}

.scan-results {
  margin-top: 16px;
  border-top: 1px solid var(--color-border);
  padding-top: 16px;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--color-text-2);
}

.results-list {
  max-height: 300px;
  overflow-y: auto;
}

.result-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  margin-bottom: 8px;
}

.result-item.valid {
  background: rgba(var(--success-1), 0.3);
}

.result-item.invalid {
  background: var(--color-fill-1);
  opacity: 0.6;
}

.result-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.result-info {
  flex: 1;
  min-width: 0;
}

.result-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-1);
  display: flex;
  align-items: center;
  gap: 8px;
}

.result-name .version {
  font-size: 11px;
  color: var(--color-text-3);
  background: var(--color-fill-3);
  padding: 2px 6px;
  border-radius: 4px;
}

.result-path {
  font-size: 11px;
  color: var(--color-text-4);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
