<script setup>
import { defineAsyncComponent, ref, reactive, onMounted, computed } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { 
  IconPlus, 
  IconToBottom, 
  IconToTop, 
  IconDelete,
  IconSync,
  IconSearch,
  IconEye,
  IconEyeInvisible
} from '@arco-design/web-vue/es/icon';
import * as XLSX from 'xlsx';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';
import { walletService, initBrowserAutomationTables } from '../services/browserAutomationService';

const WalletSystemImportModal = defineAsyncComponent(() => import('@/components/WalletSystemImportModal.vue'))

const wallets = ref([]);
const searchQuery = ref('');
const loading = ref(false);

// 过滤后的钱包列表
const filteredWallets = computed(() => {
  if (!searchQuery.value) return wallets.value;
  const query = searchQuery.value.toLowerCase();
  return wallets.value.filter(w => 
    w.address.toLowerCase().includes(query) ||
    (w.name && w.name.toLowerCase().includes(query)) ||
    (w.label && w.label.toLowerCase().includes(query)) ||
    w.group_name.toLowerCase().includes(query)
  );
});

const columns = [
  { title: 'ID', dataIndex: 'id', width: 60 },
  { title: '名称', dataIndex: 'name', width: 120 },
  { title: '地址', dataIndex: 'address', ellipsis: true },
  { title: '备注', dataIndex: 'label', ellipsis: true },
  { title: '分组', dataIndex: 'group_name', width: 120 },
  { title: '代理', dataIndex: 'proxy', width: 150, ellipsis: true },
  { title: '链类型', dataIndex: 'chain_type', width: 80 },
  { title: '操作', slotName: 'action', width: 180, align: 'center' }
];

const isAddModalVisible = ref(false);
const isEditModalVisible = ref(false);
const systemImportVisible = ref(false);
const walletDbReady = ref(false);
const editingWallet = ref(null);
const showPrivateKey = ref(false);

const form = reactive({
  name: '',
  address: '',
  privateKey: '',
  label: '',
  group_name: 'Default',
  proxy: 'Direct',
  chain_type: 'evm'
});

// 加载钱包数据
const loadWallets = async () => {
  loading.value = true;
  try {
    wallets.value = await walletService.getWallets();
  } catch (error) {
    console.error('Failed to load wallets:', error);
    Message.error('加载钱包列表失败: ' + error);
  } finally {
    loading.value = false;
  }
};

const handleAdd = () => {
  form.name = '';
  form.address = '';
  form.privateKey = '';
  form.label = '';
  form.group_name = 'Default';
  form.proxy = 'Direct';
  form.chain_type = 'evm';
  showPrivateKey.value = false;
  isAddModalVisible.value = true;
};

const handleEdit = (record) => {
  editingWallet.value = record;
  form.name = record.name || '';
  form.address = record.address;
  form.privateKey = '';
  form.label = record.label || '';
  form.group_name = record.group_name;
  form.proxy = record.proxy;
  form.chain_type = record.chain_type;
  showPrivateKey.value = false;
  isEditModalVisible.value = true;
};

// 查看私钥
const handleViewPrivateKey = async (record) => {
  try {
    const privateKey = await walletService.getPrivateKey(record.id);
    Modal.info({
      title: `钱包 "${record.name}" 的私钥`,
      content: privateKey,
      okText: '复制',
      onOk: () => {
        navigator.clipboard.writeText(privateKey);
        Message.success('私钥已复制到剪贴板');
      }
    });
  } catch (error) {
    Message.error('获取私钥失败: ' + error);
  }
};

// Excel导入
const handleImport = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'Excel Files', extensions: ['xlsx', 'xls', 'csv'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    });

    if (!selected) return;

    const fileContent = await readFile(selected);
    const workbook = XLSX.read(fileContent, { type: 'buffer' });
    const firstSheet = workbook.Sheets[workbook.SheetNames[0]];
    const data = XLSX.utils.sheet_to_json(firstSheet);

    if (!data || data.length === 0) {
      Message.warning('文件为空或格式不正确');
      return;
    }

    // 解析导入的数据
    const importRequests = [];
    let skippedCount = 0;

    for (const row of data) {
      // 支持多种列名格式
      const name = row.name || row.Name || row.名称 || '';
      const address = row.address || row.Address || row.地址 || row['钱包地址'] || '';
      const privateKey = row.private_key || row.privateKey || row.PrivateKey || row.私钥 || '';
      const label = row.label || row.Label || row.备注 || '';
      const group = row.group || row.Group || row.分组 || 'Default';
      const proxy = row.proxy || row.Proxy || row.代理 || 'Direct';
      const chainType = row.chain_type || row.chainType || row.链类型 || 'evm';

      if (!address || typeof address !== 'string') continue;
      
      // 验证地址格式 (支持EVM和Solana地址)
      const trimmedAddress = address.toString().trim();
      if (!trimmedAddress.match(/^(0x[a-fA-F0-9]{40}|[1-9A-HJ-NP-Za-km-z]{32,44})$/)) {
        continue;
      }

      importRequests.push({
        name: name.toString().trim() || `Wallet ${importRequests.length + 1}`,
        address: trimmedAddress,
        private_key: privateKey.toString().trim(),
        label: label.toString().trim(),
        group_name: group.toString().trim(),
        proxy: proxy.toString().trim(),
        chain_type: chainType.toString().trim()
      });
    }

    if (importRequests.length === 0) {
      Message.warning('未找到有效的钱包数据');
      return;
    }

    // 批量导入
    const { wallets: importedWallets, errors } = await walletService.importWallets(importRequests);
    
    // 刷新列表
    await loadWallets();
    
    let msg = `成功导入 ${importedWallets.length} 个钱包`;
    if (errors.length > 0) msg += `，${errors.length} 个失败`;
    if (skippedCount > 0) msg += `，跳过 ${skippedCount} 个`;
    
    Message.success(msg);

    if (errors.length > 0) {
      console.error('Import errors:', errors);
    }

  } catch (error) {
    console.error('Import error:', error);
    Message.error('导入失败: ' + error);
  }
};

const openSystemSync = () => {
  systemImportVisible.value = true;
};

const handleSystemSync = async (imported) => {
  const list = (imported || [])
    .map((w) => ({
      name: (w?.name || '').trim() || `Wallet ${w?.address?.slice(-6) || 'Unknown'}`,
      address: String(w?.address || '').trim(),
      private_key: '',
      label: (w?.name || '').trim(),
      group_name: (w?.group_name || '').trim() || 'Default',
      proxy: 'Direct',
      chain_type: w?.chain_type || 'evm',
    }))
    .filter((w) => w.address);

  if (!list.length) {
    Message.info('没有新钱包需要同步');
    return;
  }

  try {
    const { wallets: importedWallets, errors } = await walletService.importWallets(list);
    await loadWallets();
    
    let msg = `同步成功，新增 ${importedWallets.length} 个钱包`;
    if (errors.length > 0) {
      msg += `，${errors.length} 个失败`;
      console.error('Sync errors:', errors);
    }
    Message.success(msg);
  } catch (error) {
    Message.error('同步失败: ' + error);
  }
};

// Excel导出
const handleExport = async () => {
  if (wallets.value.length === 0) {
    Message.warning('没有可导出的钱包');
    return;
  }

  try {
    const exportData = wallets.value.map(w => ({
      '名称': w.name,
      '地址': w.address,
      '备注': w.label || '',
      '分组': w.group_name,
      '代理': w.proxy,
      '链类型': w.chain_type
    }));

    const worksheet = XLSX.utils.json_to_sheet(exportData);
    const workbook = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(workbook, worksheet, 'Wallets');

    // 设置列宽
    worksheet['!cols'] = [
      { wch: 20 }, // 名称
      { wch: 50 }, // 地址
      { wch: 20 }, // 备注
      { wch: 15 }, // 分组
      { wch: 20 }, // 代理
      { wch: 10 }  // 链类型
    ];

    const excelBuffer = XLSX.write(workbook, { bookType: 'xlsx', type: 'array' });
    const uint8Array = new Uint8Array(excelBuffer);

    const savePath = await save({
      filters: [
        { name: 'Excel Files', extensions: ['xlsx'] }
      ],
      defaultPath: `wallets_export_${new Date().toISOString().slice(0, 10)}.xlsx`
    });

    if (savePath) {
      await writeFile(savePath, uint8Array);
      Message.success(`成功导出 ${wallets.value.length} 个钱包`);
    }
  } catch (error) {
    console.error('Export error:', error);
    Message.error('导出失败: ' + error);
  }
};

const handleDelete = (record) => {
  Modal.warning({
    title: '确认删除',
    content: `确定要删除钱包 "${record.name || record.address}" 吗？`,
    onOk: async () => {
      try {
        await walletService.deleteWallet(record.id);
        await loadWallets();
        Message.success('删除成功');
      } catch (error) {
        Message.error('删除失败: ' + error);
      }
    }
  });
};

const handleSubmitAdd = async () => {
  if (!form.name) {
    Message.error('请输入钱包名称');
    return;
  }

  if (!form.address) {
    Message.error('请输入钱包地址');
    return;
  }

  // 验证地址格式
  const trimmedAddress = form.address.trim();
  if (!trimmedAddress.match(/^(0x[a-fA-F0-9]{40}|[1-9A-HJ-NP-Za-km-z]{32,44})$/)) {
    Message.error('请输入有效的钱包地址 (EVM或Solana格式)');
    return;
  }

  try {
    await walletService.createWallet({
      name: form.name.trim(),
      address: trimmedAddress,
      private_key: form.privateKey.trim(),
      label: form.label.trim(),
      group_name: form.group_name.trim(),
      proxy: form.proxy.trim(),
      chain_type: form.chain_type.trim()
    });
    
    await loadWallets();
    isAddModalVisible.value = false;
    Message.success('添加成功');
  } catch (error) {
    Message.error('添加失败: ' + error);
  }
};

const handleSubmitEdit = async () => {
  if (!form.name) {
    Message.error('请输入钱包名称');
    return;
  }

  if (!form.address) {
    Message.error('请输入钱包地址');
    return;
  }

  const trimmedAddress = form.address.trim();
  if (!trimmedAddress.match(/^(0x[a-fA-F0-9]{40}|[1-9A-HJ-NP-Za-km-z]{32,44})$/)) {
    Message.error('请输入有效的钱包地址');
    return;
  }

  try {
    const updateRequest = {
      id: editingWallet.value.id,
      name: form.name.trim(),
      address: trimmedAddress,
      label: form.label.trim(),
      group_name: form.group_name.trim(),
      proxy: form.proxy.trim(),
      chain_type: form.chain_type.trim()
    };

    // 如果输入了新私钥，则更新
    if (form.privateKey.trim()) {
      updateRequest.private_key = form.privateKey.trim();
    }

    await walletService.updateWallet(updateRequest);
    await loadWallets();
    
    isEditModalVisible.value = false;
    editingWallet.value = null;
    Message.success('更新成功');
  } catch (error) {
    Message.error('更新失败: ' + error);
  }
};

// 清空所有钱包
const handleClearAll = () => {
  if (wallets.value.length === 0) {
    Message.info('钱包列表为空');
    return;
  }
  
  Modal.warning({
    title: '确认清空',
    content: `确定要清空所有 ${wallets.value.length} 个钱包吗？此操作不可恢复！`,
    okText: '确认清空',
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        for (const wallet of wallets.value) {
          await walletService.deleteWallet(wallet.id);
        }
        await loadWallets();
        Message.success('已清空所有钱包');
      } catch (error) {
        Message.error('清空失败: ' + error);
      }
    }
  });
};

onMounted(async () => {
  // 初始化表结构
  try {
    await initBrowserAutomationTables();
  } catch (e) {
    console.log('Tables may already exist:', e);
  }
  // 检查钱包管理是否已初始化（只需检查是否设置过密码，不需要当前解锁）
  try {
    walletDbReady.value = await invoke('is_wallet_manager_initialized');
  } catch (e) {
    walletDbReady.value = false;
  }
  // 加载钱包数据
  await loadWallets();
});
</script>

<template>
  <div class="wallet-manager">
    <!-- Toolbar -->
    <div class="toolbar">
      <div class="left-tools">
        <a-button type="primary" @click="handleAdd">
          <template #icon><icon-plus /></template>
          添加钱包
        </a-button>
        <a-button type="primary" status="warning" @click="openSystemSync" v-if="walletDbReady">
          <template #icon><icon-sync /></template>
          从系统同步
        </a-button>
        <a-button type="outline" status="success" @click="handleImport">
          <template #icon><icon-to-bottom /></template>
          导入 (Excel)
        </a-button>
        <a-button type="outline" @click="handleExport">
          <template #icon><icon-to-top /></template>
          导出配置
        </a-button>
        <a-button type="outline" status="danger" @click="handleClearAll" v-if="wallets.length > 0">
          <template #icon><icon-delete /></template>
          清空
        </a-button>
      </div>
      <div class="right-tools">
        <a-input-search 
          v-model="searchQuery" 
          placeholder="搜索名称/地址/备注/分组..." 
          style="width: 280px" 
          allow-clear 
        />
      </div>
    </div>

    <!-- Stats -->
    <div class="stats-bar" v-if="wallets.length > 0">
      <span class="stat-item">总计: <strong>{{ wallets.length }}</strong> 个钱包</span>
      <span class="stat-item" v-if="searchQuery && filteredWallets.length !== wallets.length">
        筛选结果: <strong>{{ filteredWallets.length }}</strong> 个
      </span>
    </div>

    <!-- Table -->
    <div class="table-wrapper">
      <a-table 
        :data="filteredWallets" 
        :columns="columns" 
        :pagination="{ pageSize: 15 }"
        :bordered="false"
        :loading="loading"
        hoverable
        :empty-text="searchQuery ? '未找到匹配的钱包' : '暂无钱包，请点击「添加钱包」或「导入」'"
      >
        <template #action="{ record }">
          <a-space>
            <a-button type="text" size="mini" @click="handleViewPrivateKey(record)">
              <template #icon><icon-eye /></template>
              私钥
            </a-button>
            <a-button type="text" size="mini" @click="handleEdit(record)">
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

    <!-- Add Wallet Modal -->
    <a-modal v-model:visible="isAddModalVisible" title="添加钱包" @ok="handleSubmitAdd">
      <a-form :model="form" layout="vertical">
        <a-form-item label="钱包名称" required>
          <a-input v-model="form.name" placeholder="我的钱包" />
        </a-form-item>
        <a-form-item label="钱包地址" required>
          <a-input v-model="form.address" placeholder="0x... (EVM) 或 Solana地址" />
        </a-form-item>
        <a-form-item label="私钥" required>
          <a-input-password v-model="form.privateKey" placeholder="输入私钥（将加密存储）" />
        </a-form-item>
        <a-form-item label="备注">
          <a-input v-model="form.label" placeholder="备注信息" />
        </a-form-item>
        <a-row :gutter="16">
          <a-col :span="8">
            <a-form-item label="分组">
              <a-input v-model="form.group_name" placeholder="Default" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="代理 (Proxy)">
              <a-input v-model="form.proxy" placeholder="Direct" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="链类型">
              <a-select v-model="form.chain_type">
                <a-option value="evm">EVM</a-option>
                <a-option value="solana">Solana</a-option>
              </a-select>
            </a-form-item>
          </a-col>
        </a-row>
      </a-form>
    </a-modal>

    <!-- Edit Wallet Modal -->
    <a-modal v-model:visible="isEditModalVisible" title="编辑钱包" @ok="handleSubmitEdit">
      <a-form :model="form" layout="vertical">
        <a-form-item label="钱包名称" required>
          <a-input v-model="form.name" placeholder="我的钱包" />
        </a-form-item>
        <a-form-item label="钱包地址" required>
          <a-input v-model="form.address" placeholder="0x..." />
        </a-form-item>
        <a-form-item label="私钥 (留空保持不变)">
          <a-input-password v-model="form.privateKey" placeholder="输入新私钥以更新（将加密存储）" />
        </a-form-item>
        <a-form-item label="备注">
          <a-input v-model="form.label" placeholder="备注信息" />
        </a-form-item>
        <a-row :gutter="16">
          <a-col :span="8">
            <a-form-item label="分组">
              <a-input v-model="form.group_name" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="代理 (Proxy)">
              <a-input v-model="form.proxy" />
            </a-form-item>
          </a-col>
          <a-col :span="8">
            <a-form-item label="链类型">
              <a-select v-model="form.chain_type">
                <a-option value="evm">EVM</a-option>
                <a-option value="solana">Solana</a-option>
              </a-select>
            </a-form-item>
          </a-col>
        </a-row>
      </a-form>
    </a-modal>

    <WalletSystemImportModal
      v-model:visible="systemImportVisible"
      ecosystem="all"
      import-mode="address_only"
      :title="'从系统同步钱包'"
      @confirm="handleSystemSync"
      @cancel="systemImportVisible = false"
    />
  </div>
</template>

<style scoped>
.wallet-manager {
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

.stats-bar {
  display: flex;
  gap: 10px;
  padding: 8px 12px;
  background: var(--color-fill-2);
  border-radius: 6px;
  font-size: 13px;
  color: var(--color-text-2);
}

.stats-bar strong {
  color: rgb(var(--primary-6));
}

.table-wrapper {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  padding: 10px;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

/* Custom Table Styles override */
:deep(.arco-table-cell) {
  background: transparent !important;
}

/* 暗黑主题样式覆盖 */
:root[data-theme="dark"] .wallet-manager {
  background: transparent;
}

:root[data-theme="dark"] .toolbar {
  background: linear-gradient(135deg, #1c2128 0%, #161b22 100%) !important;
  border-color: rgba(88, 108, 199, 0.2) !important;
}

:root[data-theme="dark"] .stats-bar {
  background: rgba(88, 108, 199, 0.15) !important;
  color: var(--color-text-2, #c9d1d9);
}

:root[data-theme="dark"] .table-wrapper {
  background: linear-gradient(135deg, #161b22 0%, #0d1117 100%) !important;
  border-color: rgba(88, 108, 199, 0.2) !important;
}

:root[data-theme="dark"] .stats-bar strong {
  color: #5b8aff !important;
}
</style>
