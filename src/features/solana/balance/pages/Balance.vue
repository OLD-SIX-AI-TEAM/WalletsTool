<script setup name="balance-solana">
import { Icon } from '@iconify/vue';
import { IconDelete } from '@arco-design/web-vue/es/icon';
import { computed, defineAsyncComponent, nextTick, onBeforeMount, onMounted, onBeforeUnmount, reactive, ref, triggerRef, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { PublicKey } from "@solana/web3.js";
import { Notification, Modal } from "@arco-design/web-vue";
import { utils as xlUtils, writeFile } from "xlsx";
import { getCurrentWindow } from '@tauri-apps/api/window'
import ChainIcon from '@/components/ChainIcon.vue'
import TitleBar from '@/components/TitleBar.vue'
import TableSkeleton from '@/components/TableSkeleton.vue'
import VirtualScrollerTable from '@/components/VirtualScrollerTable.vue'
import { debounce } from '@/utils/debounce.js'
import { WINDOW_CONFIG } from '@/utils/windowNames'
import { exportWithDialog, openDirectory } from '@/utils/exportWithDialog'

// 懒加载非关键组件
const ChainManagement = defineAsyncComponent(() => import('@/components/ChainManagement.vue'))
const RpcManagement = defineAsyncComponent(() => import('@/components/RpcManagement.vue'))
const TokenManagement = defineAsyncComponent(() => import('@/components/TokenManagement.vue'))
const CodeEditor = defineAsyncComponent(() => import('@/components/CodeEditor.vue'))
const ProxyConfigModal = defineAsyncComponent(() => import('@/components/ProxyConfigModal.vue'))
const WalletSystemImportModal = defineAsyncComponent(() => import('@/components/WalletSystemImportModal.vue'))

// 组件配置参数
const props = defineProps({})

// table列名 (Solana不显示Nonce)
const columns = [
  {
    title: '序号',
    align: 'center',
    width: 53,
    slotName: 'index'
  },
  {
    title: '钱包地址',
    align: 'center',
    dataIndex: 'address',
    width: 180,
    ellipsis: true,
    tooltip: true
  },
  {
    title: 'SOL余额',
    align: 'center',
    dataIndex: 'plat_balance',
    width: 120,
    ellipsis: true,
    tooltip: true
  },
  {
    title: '代币余额',
    align: 'center',
    dataIndex: 'coin_balance',
    width: 120,
    ellipsis: true,
    tooltip: true
  },
  {
    title: '状态',
    align: 'center',
    slotName: 'exec_status',
    width: 90,
    ellipsis: true,
    tooltip: true
  },
  {
    title: '错误信息',
    align: 'center',
    dataIndex: 'error_msg',
    width: 120,
    ellipsis: true,
    tooltip: true,
  },
  {
    title: '操作',
    align: 'center',
    slotName: 'optional',
    width: 80,
    ellipsis: true,
    tooltip: true
  }
]
const tableBool = ref(true)
const data = ref([])
// 选中的数据key
const selectedKeys = ref([]);

// 点击行实现选中和取消
function rowClick(record, event) {
  const index = selectedKeys.value.indexOf(record.address)
  index >= 0 ? selectedKeys.value.splice(index, 1) : selectedKeys.value.push(record.address)
}

// 仅查询目标代币
const onlyCoin = ref(true);
// 余额查询进度相关变量
const balanceProgress = ref(0);
const balanceTotal = ref(0);
const balanceCompleted = ref(0);
const showProgress = ref(false);
// 分页
const pagination = ref(false);
const scrollbar = ref(true);

// 窗口标题定义
const windowTitle = ref('Solana 余额查询');

// 窗口标题初始化
function initBalanceWindowTitle() {
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      const windowLabel = getCurrentWindow().label
      const saved = WINDOW_CONFIG.getCustomTitle(windowLabel)
      if (saved) {
        windowTitle.value = saved
        return
      }
    }
  } catch (e) {
    console.error('初始化Solana余额查询窗口标题失败:', e)
  }
}

initBalanceWindowTitle()

// chain默认值
const chainValue = ref('sol');
// 当前chain
const currentChain = ref({
  key: 'sol',
  name: 'Solana',
  scan_url: 'https://solscan.io/tx/',
  chain_id: 0,
  symbol: 'SOL'
});
// 链选项 (Solana specific)
const chainOptions = ref([{
  key: 'sol',
  name: 'Solana',
  scan_url: 'https://solscan.io/tx/',
  symbol: 'SOL'
}]);

// coin默认值
let coinValue = ref('');
// 币种选择器
const coinOptions = ref([]);
// 查询余额按钮loading
let balanceLoading = ref(false)
// 余额查询停止标志
let balanceStopFlag = ref(false)
// 详细配置
const form = reactive({
  thread_count: 3
})
// 录入 钱包地址 弹窗
let visible = ref(false)
let importText = ref('')
const systemImportVisible = ref(false)
const walletDbReady = ref(false) // 钱包数据库是否已初始化
// 导入loading状态
let importLoading = ref(false)
// 地址验证相关
const validationErrors = ref([])
const errorsExpanded = ref(false)
const addressErrorLines = ref([])
// 添加代币弹窗
let addCoinVisible = ref(false)
let coinAddress = ref('')
// 删除代币弹窗
let deleteTokenVisible = ref(false)
// 删除信息弹窗
let deleteItemVisible = ref(false)
// 当前币种名称
let currentCoin = ref({})
// 当前数据的key
let currentItemKey = ref('')
// 当前窗口ID
let currentWindowId = ref('')
// 链管理组件引用
const chainManageRef = ref(null);
// RPC管理组件引用
const rpcManageRef = ref(null);
// 代币管理组件引用
const tokenManageRef = ref(null);
// 代理配置组件引用
const proxyConfigRef = ref(null);
// 文件上传输入框引用
const uploadInputRef = ref(null);
const proxyConfigVisible = ref(false);
const proxyEnabled = ref(false);
const proxyStatus = ref('未配置');
const proxyCount = ref(0);
// 高级筛选相关变量
const advancedFilterVisible = ref(false);
const filterForm = reactive({
  platBalanceOperator: 'gt', 
  platBalanceValue: '',
  coinBalanceOperator: 'gt', 
  coinBalanceValue: '',
  errorMsg: ''
});

// 界面控制变量
const isSidePanelExpanded = ref(true);
const chainSelectorExpanded = ref(false);
const tokenSelectorExpanded = ref(false);
const chainSearchKeyword = ref('');
const tokenSearchKeyword = ref('');
const chainSearchInputRef = ref(null);
const tokenSearchInputRef = ref(null);

// 计算属性：过滤后的链列表
const filteredChainOptions = computed(() => {
  if (!chainSearchKeyword.value.trim()) {
    return chainOptions.value || [];
  }
  const keyword = chainSearchKeyword.value.toLowerCase();
  return (chainOptions.value || []).filter(
    (chain) =>
      (chain.name && chain.name.toLowerCase().includes(keyword)) ||
      (chain.key && chain.key.toLowerCase().includes(keyword))
  );
});

// 计算属性：过滤后的代币列表
const filteredCoinOptions = computed(() => {
  if (!tokenSearchKeyword.value.trim()) {
    return coinOptions.value || [];
  }
  const keyword = tokenSearchKeyword.value.toLowerCase();
  return (coinOptions.value || []).filter(
    (coin) =>
      (coin.label && coin.label.toLowerCase().includes(keyword)) ||
      (coin.symbol && coin.symbol.toLowerCase().includes(keyword)) ||
      (coin.key && coin.key.toLowerCase().includes(keyword))
  );
});

// 侧边栏控制
function expandSidePanel() { isSidePanelExpanded.value = true; }
function collapseSidePanel() { isSidePanelExpanded.value = false; }

// 选择器控制
function toggleChainSelector() {
  if (balanceLoading.value) {
    Notification.warning({ content: '执行过程中无法切换区块链', position: 'topLeft' });
    return;
  }
  chainSelectorExpanded.value = !chainSelectorExpanded.value;
  tokenSelectorExpanded.value = false;
  if (chainSelectorExpanded.value) {
    nextTick(() => {
      chainSearchInputRef.value?.focus();
    });
  }
}

function toggleTokenSelector() {
  if (balanceLoading.value) {
    Notification.warning({ content: '执行过程中无法切换代币', position: 'topLeft' });
    return;
  }
  tokenSelectorExpanded.value = !tokenSelectorExpanded.value;
  chainSelectorExpanded.value = false;
  if (tokenSelectorExpanded.value) {
    nextTick(() => {
      tokenSearchInputRef.value?.focus();
    });
  }
}

function handleChainSelect(chainKey) {
  // Solana currently mostly single chain, but keeping logic for future
  chainValue.value = chainKey;
  const chain = chainOptions.value.find((c) => c.key === chainKey);
  if (chain) currentChain.value = chain;
  chainSelectorExpanded.value = false;
  tokenSelectorExpanded.value = true;
  chainChange();
}

function handleTokenSelect(tokenKey) {
  coinChange(tokenKey);
  tokenSelectorExpanded.value = false;
}

// 点击外部关闭选择器
function handleClickOutside(event) {
  const chainSelector = document.querySelector('.chain-selector-container');
  const tokenSelector = document.querySelector('.token-selector-container');
  const isInChainSelector = chainSelector && chainSelector.contains(event.target);
  const isInTokenSelector = tokenSelector && tokenSelector.contains(event.target);
  if (!isInChainSelector) chainSelectorExpanded.value = false;
  if (!isInTokenSelector) tokenSelectorExpanded.value = false;
}

// 计算属性：缓存筛选后的数据
const filteredData = computed(() => {
  if (!filterForm.platBalanceValue && !filterForm.coinBalanceValue && !filterForm.errorMsg) {
    return data.value;
  }
  
  return data.value.filter(item => {
    // SOL余额筛选
    if (filterForm.platBalanceValue && filterForm.platBalanceValue.trim() !== '') {
      const platBalanceValue = parseFloat(filterForm.platBalanceValue);
      const itemPlatBalance = parseFloat(item.plat_balance || 0);
      
      if (filterForm.platBalanceOperator === 'gt' && itemPlatBalance <= platBalanceValue) {
        return false;
      } else if (filterForm.platBalanceOperator === 'eq' && itemPlatBalance !== platBalanceValue) {
        return false;
      } else if (filterForm.platBalanceOperator === 'lt' && itemPlatBalance >= platBalanceValue) {
        return false;
      }
    }
    
    // 代币余额筛选
    if (filterForm.coinBalanceValue && filterForm.coinBalanceValue.trim() !== '') {
      const coinBalanceValue = parseFloat(filterForm.coinBalanceValue);
      const itemCoinBalance = parseFloat(item.coin_balance || 0);
      
      if (filterForm.coinBalanceOperator === 'gt' && itemCoinBalance <= coinBalanceValue) {
        return false;
      } else if (filterForm.coinBalanceOperator === 'eq' && itemCoinBalance !== coinBalanceValue) {
        return false;
      } else if (filterForm.coinBalanceOperator === 'lt' && itemCoinBalance >= coinBalanceValue) {
        return false;
      }
    }
    
    // 错误信息模糊匹配
    if (filterForm.errorMsg && filterForm.errorMsg.trim() !== '') {
      const errorMsg = item.error_msg || '';
      if (!errorMsg.toLowerCase().includes(filterForm.errorMsg.toLowerCase())) {
        return false;
      }
    }
    
    return true;
  });
});

const debouncedFilterUpdate = debounce(() => {
  // filteredData computed属性会自动响应filterForm的变化
}, 300);

// a-table行选择配置
const rowSelection = reactive({
  type: 'checkbox',
  showCheckedAll: true,
  selectedRowKeys: selectedKeys,
  onSelect: (rowKeys) => {
    selectedKeys.value = rowKeys
  },
  onSelectAll: (selected) => {
    if (selected) {
      selectedKeys.value = data.value.map(item => item.address)
    } else {
      selectedKeys.value = []
    }
  }
})

// 待处理的更新队列
const pendingUpdates = [];
let updateTimer = null;

// 批量处理更新
function processUpdates() {
  if (pendingUpdates.length === 0) return;

  // 批量应用更新
  const updates = pendingUpdates.splice(0, pendingUpdates.length);
  const dataRef = data.value;
  let hasChanges = false;
  
  // 使用Map优化查找
  const addressMap = new Map();
  dataRef.forEach((item, index) => {
    addressMap.set(item.address, index);
  });

  updates.forEach(({ item }) => {
    const targetIndex = addressMap.get(item.address);
    if (targetIndex !== undefined) {
      // 合并属性，避免覆盖整个对象
      Object.assign(dataRef[targetIndex], {
        plat_balance: item.plat_balance,
        coin_balance: item.coin_balance,
        exec_status: item.exec_status,
        error_msg: item.error_msg
      });
      hasChanges = true;
    }
  });

  if (hasChanges) {
    triggerRef(data);
    updateBalanceProgress();
  }
  
  updateTimer = null;
}

// 初始化Chain列表
onBeforeMount(async () => {
  try {
    const result = await invoke('get_chain_list')
    if (result && result.length > 0) {
      // 过滤出Solana生态的链
      const solanaChains = result.filter(item => item.ecosystem === 'solana')
      if (solanaChains.length > 0) {
        chainOptions.value = solanaChains
        // 默认选中第一个
        chainValue.value = solanaChains[0].key
        currentChain.value = solanaChains[0]
      }
    }
  } catch (e) {
    console.error('获取链列表失败:', e)
  }
  chainChange()
})

onMounted(async () => {
  document.addEventListener('click', handleClickOutside);
  
  // 获取窗口标题和ID
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
  if (isTauri) {
    try {
      const currentWindow = getCurrentWindow();
      
      // 获取窗口标题
      const title = await currentWindow.title();
      if (title) {
        windowTitle.value = title;
      }
      
      // 获取当前窗口ID
      currentWindowId.value = currentWindow.label;
      
      // 检查钱包管理是否已初始化（只需检查是否设置过密码，不需要当前解锁）
      try { walletDbReady.value = await invoke('is_wallet_manager_initialized'); } catch (e) { walletDbReady.value = false; }

      // 添加Tauri窗口关闭事件监听器
      await currentWindow.onCloseRequested(async (event) => {
        if (balanceLoading.value) {
          await stopBalanceQuery();
        }
      });
    } catch (error) {
      console.error('Error getting window info:', error);
    }
  } else {
    // 浏览器环境下设置默认标题和ID
    windowTitle.value = 'Solana 余额查询';
    currentWindowId.value = 'browser_window';
  }

  // 监听余额查询更新事件
  await listen('balance_item_update', (event) => {
    const { item, window_id } = event.payload
    // 检查是否是本窗口的事件
    if (window_id && window_id !== currentWindowId.value) {
      return; 
    }
    
    // 将更新加入队列
    pendingUpdates.push({ item });
    
    // 如果没有定时器，启动一个
    if (!updateTimer) {
      updateTimer = requestAnimationFrame(processUpdates);
    }
  })

  // 页面加载完成后发送事件
  nextTick(() => {
    setTimeout(() => {
      const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
      if (isTauri) {
        const currentWindow = getCurrentWindow();
        currentWindow.emit('page-loaded');
      }
    }, 50);
  });

  // 初始化代理状态
  await initProxyStatus();
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside);
});

function generateWindowId() {
  const timestamp = Date.now().toString(36);
  const randomPart = Math.random().toString(36).substring(2, 9);
  return `window_${timestamp}_${randomPart}`;
}


// RPC变化事件
async function chainChange() {
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
    if (isTauri) {
      const defaultTokens = [
        { key: 'sol', label: 'SOL', symbol: 'SOL', coin_type: 'base', decimals: 9 }
      ];
      try {
        const tokenList = await invoke('get_coin_list', { chainKey: chainValue.value || 'sol' });
        if(tokenList && tokenList.length > 0) {
           coinOptions.value = tokenList;
        } else {
           coinOptions.value = defaultTokens;
        }
      } catch(e) {
        coinOptions.value = defaultTokens;
      }
      
      coinOptions.value.sort((a, b) => {
        if (a.coin_type === 'base' && b.coin_type !== 'base') return -1;
        if (a.coin_type !== 'base' && b.coin_type === 'base') return 1;
        return a.label.localeCompare(b.label);
      });
      if (coinOptions.value.length > 0) { coinValue.value = coinOptions.value[0].key; currentCoin.value = coinOptions.value[0]; }
    } else {
        coinOptions.value = [{ key: 'sol', label: 'SOL', symbol: 'SOL', coin_type: 'base', decimals: 9 }];
        coinValue.value = 'sol';
        currentCoin.value = coinOptions.value[0];
    }
  } catch (error) { 
    console.error('加载代币列表失败:', error); 
    coinOptions.value = [{ key: 'sol', label: 'SOL', symbol: 'SOL', coin_type: 'base', decimals: 9 }];
    coinValue.value = 'sol'; 
    currentCoin.value = coinOptions.value[0];
  }
}

// coin变化事件
async function coinChange(value) {
  coinValue.value = value;
  currentCoin.value = coinOptions.value.filter(item => item.key === value)[0]
}

// 删除代币方法
function deleteToken() {
  if (currentCoin.value.coin_type === 'base') {
      Notification.warning({ content: '原生代币无法删除！', position: 'topLeft' });
      return;
  }
  deleteTokenVisible.value = true
}

function deleteTokenCancel() {
  deleteTokenVisible.value = false
}

async function deleteTokenConfirm() {
  deleteTokenVisible.value = false
  await invoke("remove_coin", { chain: chainValue.value || 'sol', key: currentCoin.value.key }).then(() => {
    Notification.success({ content: '删除成功！', position: 'topLeft' });
    chainChange()
  }).catch(() => {
    Notification.error({ content: '删除失败！', position: 'topLeft' });
  })
}

// 导入事件触发
function handleAddCoinClick() {
  addCoinVisible.value = true
}

function handleAddCoinCancel() {
  addCoinVisible.value = false
}

// 添加代币核心方法
function addCoinFunc() {
  return new Promise(async (resolve, reject) => {
    try {
      let json = {
        "key": coinAddress.value.substring(0, 4).toLowerCase(), // 简化的key
        "coin": "Unknown",
        "type": "token",
        "contract_type": "spl",
        "contract_address": coinAddress.value,
        "abi": null 
      }
      
      await invoke('add_coin', {
        chain: chainValue.value || 'sol',
        objJson: JSON.stringify(json)
      })

      addCoinVisible.value = false
      coinAddress.value = ''
      resolve()
    } catch (err) {
      reject('添加代币失败：' + err)
    }
  })
}

const handleAddCoinBeforeOk = async () => {
  coinAddress.value = coinAddress.value.trim()
  if (!coinAddress.value) {
    Notification.warning({ content: '请输入SPL Token Mint地址！', position: 'topLeft' });
    return false
  }
  let flag = false
  await addCoinFunc().then(() => {
    Notification.success({ content: '添加代币成功！', position: 'topLeft' });
    flag = true
  }).catch(err => {
    Notification.error(err);
  })
  chainChange()
  return flag
}

function clearData() {
  if (balanceLoading.value) {
    Notification.warning({ content: '请停止或等待查询完成后再清空列表！', position: 'topLeft' });
    return;
  }
   if(data.value.length === 0){
    Notification.warning({ content: '当前列表无数据！', position: 'topLeft' });
    return;
  }
  Modal.confirm({
    title: '确认清空',
    content: '确定要清空所有列表数据吗？此操作不可撤销。',
    onOk: () => {
      data.value = [];
      Notification.success({ content: '清空列表成功！', position: 'topLeft' });
    }
  });
}

function handleManualImport() {
  visible.value = true
}

function openSystemImport() {
  systemImportVisible.value = true
}

function importAddressesToTable(addressList) {
  const raw = (addressList || []).map((a) => String(a || '').trim()).filter(Boolean)
  const originalCount = raw.length
  if (!originalCount) return { originalCount: 0, successCount: 0, internalDupCount: 0, existingDupCount: 0, invalidCount: 0 }

  const invalidList = raw.filter((a) => !validateAddress(a))
  const invalidCount = invalidList.length

  let importList = raw.filter((a) => validateAddress(a))
  const uniqueAddresses = new Set()
  importList = importList.filter((item) => {
    if (uniqueAddresses.has(item)) return false
    uniqueAddresses.add(item)
    return true
  })
  const internalDupCount = (raw.length - invalidCount) - importList.length

  const existingAddresses = new Set(data.value.map((item) => item.address))
  const beforeFilterCount = importList.length
  importList = importList.filter((item) => !existingAddresses.has(item))
  const existingDupCount = beforeFilterCount - importList.length

  const newItems = importList.map((item) => ({
    address: item,
    plat_balance: '',
    coin_balance: '',
    exec_status: '0',
    error_msg: ''
  }))

  if (newItems.length) {
    data.value.push(...newItems)
  }

  return {
    originalCount,
    successCount: importList.length,
    internalDupCount,
    existingDupCount,
    invalidCount,
  }
}

function handleSystemImportConfirm(wallets) {
  const addresses = (wallets || []).map((w) => w?.address).filter(Boolean)
  const stats = importAddressesToTable(addresses)
  const filteredCount = stats.internalDupCount + stats.existingDupCount + stats.invalidCount

  if (stats.originalCount === 0) {
    Notification.warning({ content: '未选择任何地址', position: 'topLeft' })
    return
  }

  if (filteredCount > 0) {
    const details = []
    if (stats.invalidCount > 0) details.push(`无效${stats.invalidCount}条`)
    if (stats.internalDupCount > 0) details.push(`内部重复${stats.internalDupCount}条`)
    if (stats.existingDupCount > 0) details.push(`与现有数据重复${stats.existingDupCount}条`)
    Notification.warning({
      title: '导入完成！',
      content: `原始地址${stats.originalCount}条，成功导入${stats.successCount}条，已过滤：${details.join('、')}`,
      position: 'topLeft',
    })
  } else {
    Notification.success({
      title: '导入成功！',
      content: `成功导入${stats.successCount}条地址`,
      position: 'topLeft',
    })
  }
}

function validateAddress(address) {
  try {
    if (!address || typeof address !== 'string') return false;
    const trimmedAddress = address.trim();
    new PublicKey(trimmedAddress);
    return true;
  } catch (error) {
    return false;
  }
}

function validateImportData() {
  const addresses = importText.value.split('\n').filter(line => line.trim() !== '');
  
  validationErrors.value = [];
  const errorLines = new Set();
  
  addresses.forEach((addr, index) => {
    const trimmedAddr = addr.trim();
    if (trimmedAddr && !validateAddress(trimmedAddr)) {
      validationErrors.value.push(`第${index + 1}行地址格式错误`);
      errorLines.add(index + 1);
    }
  });
  
  addressErrorLines.value = Array.from(errorLines);
}

function toggleErrorsExpanded() {
  errorsExpanded.value = !errorsExpanded.value;
}

const displayedErrors = computed(() => {
  if (errorsExpanded.value || validationErrors.value.length <= 3) {
    return validationErrors.value;
  }
  return validationErrors.value.slice(0, 3);
});

function handleCancel() {
  if (importLoading.value) {
    Notification.warning({ content: '正在导入数据，请稍候...', position: 'topLeft' });
    return false;
  }
  visible.value = false
  importText.value = ''
  validationErrors.value = []
  errorsExpanded.value = false
  addressErrorLines.value = []
}

function handleFileUpload() {
  uploadInputRef.value.click();
}

function handleFileChange(event) {
  const file = event.target.files[0];
  if (!file) return;

  const reader = new FileReader();
  reader.onload = async (e) => {
    try {
      const data = new Uint8Array(e.target.result);
      const workbook = xlUtils.read(data, { type: 'array' });
      const firstSheet = workbook.Sheets[workbook.SheetNames[0]];
      const jsonData = xlUtils.sheet_to_json(firstSheet, { header: 1 });

      const addresses = [];
      jsonData.forEach((row, index) => {
        if (row && row[0]) {
          const addr = String(row[0]).trim();
          if (validateAddress(addr)) {
            addresses.push(addr);
          }
        }
      });

      if (addresses.length > 0) {
        importText.value = addresses.join('\n');
        validateImportData();
        visible.value = true;
        Notification.success({
          content: `成功解析 ${addresses.length} 个地址`,
          position: 'topLeft',
        });
      } else {
        Notification.error({
          content: '未在文件中找到有效的地址数据',
          position: 'topLeft',
        });
      }
    } catch (error) {
      console.error('解析文件失败:', error);
      Notification.error({
        content: '解析文件失败，请确保文件格式正确',
        position: 'topLeft',
      });
    } finally {
      event.target.value = '';
    }
  };
  reader.readAsArrayBuffer(file);
}

const handleBeforeOk = async () => {
  validateImportData();
  if (validationErrors.value.length > 0) {
    return false;
  }
  importLoading.value = true;
  try {
    await new Promise(resolve => setTimeout(resolve, 100));
    let importList = importText.value.split('\n').filter(item => item.trim() !== '')
    const original_count = importList.length
    
    const uniqueAddresses = new Set()
    importList = importList.filter(item => {
      const trimmedAddr = item.trim()
      if (uniqueAddresses.has(trimmedAddr)) {
        return false 
      }
      uniqueAddresses.add(trimmedAddr)
      return true
    })
    
    const after_dedup_count = importList.length
    const internal_dup_count = original_count - after_dedup_count
    
    const beforeFilterCount = importList.length
    const existingAddresses = new Set(data.value.map(item => item.address));
    importList = importList.filter(item => !existingAddresses.has(item.trim()))
    
    const success_count = importList.length
    const existing_dup_count = beforeFilterCount - success_count
    const total_filtered_count = internal_dup_count + existing_dup_count
    
    const newItems = importList.map(item => ({
      address: item.trim(),
      plat_balance: '',
      coin_balance: '',
      exec_status: '0',
      error_msg: '',
    }));

    if (newItems.length > 0) {
      data.value.push(...newItems);
    }
    
    if (total_filtered_count > 0) {
      let message = `原始地址${original_count}条，成功导入${success_count}条`
      const details = []
      if (internal_dup_count > 0) details.push(`内部重复${internal_dup_count}条`)
      if (existing_dup_count > 0) details.push(`与现有数据重复${existing_dup_count}条`)
      if (details.length > 0) message += `，已过滤：${details.join('、')}`
      Notification.warning({ title: '导入完成！', content: message, position: 'topLeft' })
    } else {
      Notification.success({ title: '导入成功！', content: `成功导入${success_count}条地址` })
    }
    
    importText.value = ''
    validationErrors.value = []
    errorsExpanded.value = false
    addressErrorLines.value = []
    return true;
  } catch (error) {
    Notification.error('导入失败：' + error.message);
    return false;
  } finally {
    importLoading.value = false;
  }
}

function deleteItem(item) {
  if (balanceLoading.value) {
    Notification.warning({ content: '请停止或等待查询完成后再删除数据！', position: 'topLeft' });
    return
  }
  deleteItemModalShow(item.address)
}

function deleteItemModalShow(address) {
  deleteItemVisible.value = true
  currentItemKey.value = address
}

function deleteItemCancel() {
  deleteItemVisible.value = false
}

async function deleteItemConfirm() {
  deleteItemVisible.value = false
  data.value = data.value.filter(obj => currentItemKey.value !== obj.address)
  Notification.success({ content: '删除成功！', position: 'topLeft' });
}

function updateBalanceProgress() {
  balanceCompleted.value = data.value.filter(item =>
    item.exec_status === '2' || item.exec_status === '3'
  ).length
  balanceProgress.value = balanceTotal.value > 0 ? Number((balanceCompleted.value / balanceTotal.value).toFixed(2)) : 0
}

const debouncedQueryBalance = debounce(queryBalance, 500);
const debouncedStopBalanceQuery = debounce(stopBalanceQuery, 300);
const debouncedDeleteSelected = debounce(deleteSelected, 400);
const debouncedExportAllToExcel = debounce(exportAllToExcel, 600);
const debouncedExportSelectToExcel = debounce(exportSelectToExcel, 600);
const debouncedClearData = debounce(clearData, 600);
const debouncedDeleteItemConfirm = debounce(deleteItemConfirm, 400);

// 清空剪贴板功能
async function clearClipboard() {
  try {
    await navigator.clipboard.writeText('');
    Notification.success({ content: '剪贴板已清空', position: 'topLeft' });
  } catch (error) {
    console.error('清空剪贴板失败:', error);
    Notification.error({ content: '清空剪贴板失败', position: 'topLeft' });
  }
}

async function queryBalance() {
  if (data.value.length === 0) {
    Notification.warning({ content: '请先导入地址！', position: 'topLeft' });
    return
  }
  if (!coinValue.value) {
    Notification.warning({ content: '请先选择代币！', position: 'topLeft' });
    return
  }

  executeBalanceQuery(data.value, true);
}

async function executeBalanceQuery(queryData) {
  if (currentCoin.value.coin_type === 'base' || currentCoin.value.coin_type === 'token') {
    balanceLoading.value = true
    balanceStopFlag.value = false

    balanceTotal.value = data.value.length
    balanceCompleted.value = 0
    balanceProgress.value = 0

    data.value.forEach(item => {
      item.plat_balance = ''
      item.coin_balance = ''
      item.error_msg = ''
      item.exec_status = '0'
    })

    showProgress.value = true

    await queryBalanceInBatches()
  } else {
    Notification.warning({ content: '查询 coin 类型错误！', position: 'topLeft' });
  }
}

async function queryBalanceInBatches() {
  const BATCH_SIZE = 50; 
  const totalItems = data.value.length;
  const totalBatches = Math.ceil(totalItems / BATCH_SIZE);
  
  try {
    for (let batchIndex = 0; batchIndex < totalBatches; batchIndex++) {
      if (balanceStopFlag.value) {
        balanceLoading.value = false;
        showProgress.value = false;
        return;
      }
      
      const startIndex = batchIndex * BATCH_SIZE;
      const endIndex = Math.min(startIndex + BATCH_SIZE, totalItems);
      const batchData = data.value.slice(startIndex, endIndex);
      
      await queryBalanceBatch(batchData, startIndex);
      updateBalanceProgress();
      
      if (batchIndex < totalBatches - 1) {
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    }
    
    balanceProgress.value = 1;
    setTimeout(() => { showProgress.value = false; }, 3000);
    
    const successCount = data.value.filter(item => item.exec_status === '2').length;
    const failCount = data.value.filter(item => item.exec_status === '3').length;
    
    if (successCount === totalItems) {
      Notification.success({ content: '查询成功！', position: 'topLeft' });
    } else if (successCount > 0) {
      Notification.warning({ content: `查询完成！成功 ${successCount} 条，失败 ${failCount} 条`, position: 'topLeft' });
    } else {
      Notification.error({ content: '查询失败：所有记录都查询失败', position: 'topLeft' });
    }
    
  } catch (error) {
    console.error('分批查询失败:', error);
    data.value.forEach(item => { item.exec_status = '3'; item.error_msg = '查询失败！'; });
    showProgress.value = false;
    Notification.error('查询失败：' + error.message);
  } finally {
    balanceLoading.value = false;
  }
}

async function queryBalanceBatch(batchData, startIndex) {
  try {
    const params = {
      items: batchData.map((item, index) => ({
        key: String(startIndex + index), 
        address: item.address,
        private_key: null,
        plat_balance: null,
        coin_balance: null,
        exec_status: '0',
        error_msg: null,
        retry_flag: false
      })),
      window_id: currentWindowId.value,
      query_id: `query_${Date.now()}_${startIndex}`,
      chain: chainValue.value || 'sol'
    };

    if (balanceStopFlag.value) return;

    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
    if (isTauri) {
      await invoke('sol_query_balances_with_updates', {
        params,
        window: getCurrentWindow() 
      });
    } else {
      await new Promise(r => setTimeout(r, 500));
      batchData.forEach((item, index) => {
        const dataIndex = startIndex + index;
        if (data.value[dataIndex]) {
          data.value[dataIndex].plat_balance = '1.5';
          data.value[dataIndex].coin_balance = '0';
          data.value[dataIndex].exec_status = '2';
        }
      });
    }

  } catch (error) {
    console.error('批次查询失败:', error);
    batchData.forEach((item, index) => {
      const dataIndex = startIndex + index;
      if (data.value[dataIndex]) {
        data.value[dataIndex].exec_status = '3';
        data.value[dataIndex].error_msg = '查询失败！';
      }
    });
  }
}

async function stopBalanceQuery() {
  balanceLoading.value = false;
  balanceStopFlag.value = true;
  showProgress.value = false;
  pendingUpdates.length = 0;
  if (updateTimer) {
    cancelAnimationFrame(updateTimer);
    updateTimer = null;
  }
}

function selectSucceeded() {
  selectedKeys.value = data.value.filter(item => item.exec_status === '2').map(item => item.address)
}

function selectFailed() {
  selectedKeys.value = data.value.filter(item => item.exec_status === '3').map(item => item.address)
}

function InvertSelection() {
  selectedKeys.value = data.value.filter(item => selectedKeys.value.indexOf(item.address) < 0).map(item => item.address)
}

function showAdvancedFilter() {
  advancedFilterVisible.value = true;
}

function applyAdvancedFilter() {
  const filteredItems = filteredData.value.map(item => item.address);
  selectedKeys.value = filteredItems;
  advancedFilterVisible.value = false;
  Notification.success({ content: `已筛选并选中 ${filteredItems.length} 条数据`, position: 'topLeft' });
}

function deleteSelected() {
  if (balanceLoading.value) {
    Notification.warning({ content: '请停止或等待查询完成后再删除数据！', position: 'topLeft' });
    return
  }
  data.value = data.value.filter(item => !selectedKeys.value.includes(item.address))
  Notification.success({ content: '删除成功', position: 'topLeft' })
}

function exportAllToExcel() {
  exportExcel(data.value)
}

function exportSelectToExcel() {
  const select_data = data.value.filter(item => selectedKeys.value.includes(item.address))
  exportExcel(select_data)
}

function exportExcel(target_data) {
  if (target_data.length === 0) {
    Notification.warning({ content: '无法导出空列表！', position: 'topLeft' });
    return;
  }
  const timestamp = new Date().toISOString().slice(0, 19).replace(/[:-]/g, '');
  const export_data = [['地址', 'SOL余额', '代币余额', '查询状态', '错误信息']];
  target_data.forEach(item => {
    export_data.push([item.address, item.plat_balance, item.coin_balance, item.exec_status, item.error_msg]);
  });

  exportWithDialog(export_data, `sol_balance_data_${timestamp}.xlsx`).then((path) => {
    if (path) {
      openDirectory(path);
      Notification.success({
        content: '导出成功！',
        duration: 4000,
        position: 'topLeft',
      });
    }
  });
}

// 链管理相关方法 (Empty for Solana single chain, but needed for template)
function showChainManage() {
  if (chainManageRef.value) {
    chainManageRef.value.show();
  }
}

function showRpcManage() {
  if (!chainValue.value) {
    Notification.warning({ content: '请先选择区块链！', position: 'topLeft' });
    return;
  }
  rpcManageRef.value?.show();
}

function showTokenManage() {
  tokenManageRef.value?.show();
}

function openBlockchainScan() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
  if (isTauri) {
    import('@tauri-apps/plugin-shell').then(({ open }) => {
      open('https://solscan.io');
    });
  } else {
    window.open('https://solscan.io', '_blank');
  }
}

async function handleChainUpdated() {
  // Reload if needed
}

function handleRpcUpdated() {
  // Reload
}

function handleTokenUpdated() {
  chainChange();
}

async function openProxyConfig() {
  proxyConfigVisible.value = true;
}

function handleProxyConfigChange(config) {
  proxyEnabled.value = config.enabled;
  proxyCount.value = config.proxies ? config.proxies.length : 0;
  if (config.enabled && proxyCount.value > 0) {
    proxyStatus.value = '已配置';
  } else {
    proxyStatus.value = '未配置';
  }
  const currentWindow = getCurrentWindow();
  const storageKey = `proxy_config_${currentWindow.label}`;
  localStorage.setItem(storageKey, JSON.stringify({
    enabled: config.enabled,
    proxies: config.proxies || []
  }));
}

const proxyStatusColor = computed(() => {
  switch (proxyStatus.value) {
    case '已配置': return '#00b42a';
    case '连接中': return '#ff7d00';
    case '已连接': return '#00b42a';
    case '连接失败': return '#f53f3f';
    default: return '#86909c';
  }
});

async function initProxyStatus() {
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;
    if (isTauri) {
      let windowId = currentWindowId.value;
      if (!windowId || windowId.trim() === '') {
        windowId = generateWindowId();
        currentWindowId.value = windowId;
      }
      await invoke('set_proxy_window_id', { windowId });
      const currentWindow = await getCurrentWindow();
      const proxyStorageKey = `proxy_config_${currentWindow.label}`;
      const storedConfig = localStorage.getItem(proxyStorageKey);
      let config;
      if (storedConfig) {
        try { config = JSON.parse(storedConfig); } catch (e) { config = await invoke('get_proxy_config_for_window', { windowId }); }
      } else {
        config = await invoke('get_proxy_config_for_window', { windowId });
      }
      handleProxyConfigChange(config);
    }
  } catch (error) {
    console.error('初始化代理状态失败:', error);
  }
}

async function handleBeforeClose() {
  if (balanceLoading.value) {
    balanceStopFlag.value = true;
  }
}
</script>

<template>
  <!-- 标题栏组件 -->
  <TitleBar :title="windowTitle" ecosystem="Solana" @before-close="handleBeforeClose" />

  <div class="container balance" @click="handleClickOutside">
    <!-- 隐藏的文件输入框 -->
    <input
      type="file"
      ref="uploadInputRef"
      accept=".xlsx,.xls,.csv"
      style="display: none"
      @change="handleFileChange"
    />

    <div class="main-content" :style="{ gap: isSidePanelExpanded ? '10px' : '0px' }">
      <!-- 左侧主面板 -->
      <div class="left-panel">
        <div class="table-section">
          <!-- 骨架屏 -->
          <TableSkeleton v-if="balanceLoading && data.length === 0" :rows="8" />
          
          <!-- 正常表格 -->
          <VirtualScrollerTable 
            v-else-if="tableBool" 
            :columns="columns" 
            :data="filteredData"
            :row-selection="rowSelection" 
            :selected-keys="selectedKeys"
            @row-click="rowClick"
            @update:selected-keys="selectedKeys = $event"
            @open-manual-import="handleManualImport"
            @open-file-upload="handleFileUpload"
            @open-system-import="openSystemImport"
            row-key="address"
            height="100%"
            page-type="balance"
            :empty-data="filteredData.length === 0"
            :show-system-import="walletDbReady"
            class="table-with-side-actions"
          >

            <template #exec_status="{ record }">
              <a-tag v-if="record.exec_status === '0'" color="#86909c">等待查询
              </a-tag>
              <a-tag v-if="record.exec_status === '1'" color="#ff7d00">查询中
              </a-tag>
              <a-tag v-if="record.exec_status === '2'" color="#00b42a">查询成功
              </a-tag>
              <a-tag v-if="record.exec_status === '3'" color="#f53f3f">查询失败
              </a-tag>
            </template>
            <!-- Solana暂无最后交易时间 -->
            <template #optional="{ record }">
              <a-button type="text" size="small" @click.stop="deleteItem(record)" status="danger">
                <template #icon>
                  <IconDelete />
                </template>
              </a-button>
            </template>
          </VirtualScrollerTable>

          <!-- 余额查询进度条 - 悬浮在表格上方 -->
          <Transition name="progress-slide" appear>
            <div v-if="showProgress" class="floating-progress-bar">
              <div class="progress-content">
                <div class="progress-header">
                  <span class="progress-title">查询进度</span>
                  <span class="progress-count">{{ balanceCompleted }}/{{ balanceTotal }}</span>
                </div>
                <a-progress 
                  :percent="balanceProgress" 
                  :stroke-width="6" 
                  :animation="true" 
                  :color="{
                    '0%': '#37ecba',
                    '100%': '#009efd',
                  }" 
                  class="progress-bar"
                />
              </div>
            </div>
          </Transition>

        </div>

          <!-- 悬浮操作栏 -->
          <div class="action-buttons-section">
            <div class="floating-action-bar">
              <div class="floating-action-content">
                <div v-if="!balanceLoading" class="btn-wrapper">
                  <a-button type="success" class="floating-btn success-btn" @click="debouncedQueryBalance">
                    <template #icon><Icon icon="mdi:play" /></template>
                    <span class="btn-text">查询余额</span>
                  </a-button>
                </div>
                <div v-else class="btn-wrapper btn-stop-wrapper">
                  <a-button type="success" class="floating-btn success-btn stopping" @click="debouncedStopBalanceQuery">
                    <template #icon><Icon icon="mdi:stop" /></template>
                    <span class="btn-text btn-text-stop">
                      <span class="btn-text-normal">查询中...</span>
                      <span class="btn-text-hover">停止查询</span>
                    </span>
                  </a-button>
                </div>
              </div>
            </div>
          </div>

        <!-- 底部配置区 -->
        <div class="config-section">
          <div class="config-container">
             <div class="config-item">
               <span class="config-label">线程数</span>
               <a-input-number :max="999" :min="1" mode="button" v-model="form.thread_count" style="width: 120px; margin-left: 8px;" />
             </div>
             
             <div class="config-divider"></div>
             
             <div class="config-item">
               <span class="config-label">仅查询目标代币</span>
               <a-switch v-model="onlyCoin" style="margin-left: 8px;" />
             </div>
          </div>
        </div>
      </div>

      <!-- 右侧工具栏 -->
      <div class="right-panel" :style="{ width: isSidePanelExpanded ? '50px' : '0', overflow: isSidePanelExpanded ? 'visible' : 'hidden'}">
        <div class="side-actions-panel-fixed">
          <div class="side-actions-content-fixed">
            <a-tooltip content="钱包录入" position="left"><a-button type="primary" size="mini" @click="handleManualImport"><template #icon><Icon icon="mdi:wallet" style="color: #165dff; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="导入文件" position="left"><a-button type="primary" size="mini" @click="handleFileUpload"><template #icon><Icon icon="mdi:upload" style="color: #00b42a; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip v-if="walletDbReady" content="从系统导入" position="left"><a-button type="primary" size="mini" status="warning" @click="openSystemImport"><template #icon><Icon icon="mdi:database-import" style="color: #ff7d00; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="清空表格" position="left"><a-button type="primary" status="danger" size="mini" @click="debouncedClearData"><template #icon><Icon icon="mdi:delete-sweep" style="color: #f53f3f; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="导出数据" position="left">
              <a-dropdown>
                <a-button size="mini">
                  <template #icon><Icon icon="mdi:export" style="color: #722ed1; font-size: 20px" /></template>
                </a-button>
                <template #content>
                  <a-doption @click="debouncedExportAllToExcel" class="dropdown-option">
                    <Icon icon="mdi:database-export" style="margin-right: 8px; margin-bottom: -2px" />导出全部数据
                  </a-doption>
                  <a-doption @click="debouncedExportSelectToExcel" class="dropdown-option">
                    <Icon icon="mdi:database-export" style="margin-right: 8px; margin-bottom: -2px" />导出选中数据
                  </a-doption>
                </template>
              </a-dropdown>
            </a-tooltip>
            
            <div class="side-actions-divider"></div>
            
            <a-tooltip content="选中成功的数据" position="left"><a-button type="outline" status="success" size="mini" @click="selectSucceeded"><template #icon><Icon icon="mdi:check-circle" style="color: #00b42a; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="选中失败的数据" position="left"><a-button type="outline" status="danger" size="mini" @click="selectFailed"><template #icon><Icon icon="mdi:close-circle" style="color: #f53f3f; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="反选" position="left"><a-button type="outline" size="mini" @click="InvertSelection"><template #icon><Icon icon="mdi:swap-horizontal" style="color: #165dff; font-size: 20px" /></template></a-button></a-tooltip>
            <a-tooltip content="高级筛选" position="left"><a-button type="primary" size="mini" @click="showAdvancedFilter"><template #icon><Icon icon="mdi:filter" style="color: #165dff; font-size: 20px" /></template></a-button></a-tooltip>
<a-tooltip content="删除选中" position="left"><a-button type="outline" status="danger" size="mini" @click="debouncedDeleteSelected"><template #icon><Icon icon="mdi:trash-can" style="color: #f53f3f; font-size: 20px" /></template></a-button></a-tooltip>
            
            <div class="side-actions-divider"></div>
            
            <a-tooltip content="清空剪贴板" position="left"><a-button type="outline" status="warning" size="mini" @click="clearClipboard"><template #icon><Icon icon="mdi:clipboard-remove" style="color: #ff7d00; font-size: 20px" /></template></a-button></a-tooltip>
          </div>
        </div>
      </div>
    </div>

    <!-- 底部状态栏 -->
    <div class="status-bar">
      <div class="status-bar-left">
        <div class="status-group">
          <!-- 链选择器 -->
          <div class="chain-selector-container" style="position: relative">
            <div
              class="status-item status-chain"
              :class="{
                'status-chain-active': chainSelectorExpanded,
                'status-item-disabled': balanceLoading,
              }"
              @click="toggleChainSelector"
              :title="balanceLoading ? '执行过程中无法切换区块链' : '点击切换区块链'"
            >
              <ChainIcon v-if="currentChain?.key" :chain-key="currentChain?.key" :alt="currentChain?.name" style="width: 14px; height: 14px" />
              <span class="status-label">{{ currentChain?.name || '选择区块链' }}</span>
              <Icon icon="mdi:chevron-up" style="font-size: 12px; margin-left: 4px; transition: transform 0.2s" :style="{ transform: chainSelectorExpanded ? 'rotate(180deg)' : 'rotate(0deg)' }" />
              <a-tag v-if="currentChain?.scan_url" size="small" class="status-explorer-tag" @click.stop="openBlockchainScan" title="打开区块链浏览器"><Icon icon="mdi:open-in-new" /></a-tag>
            </div>
            <Transition name="selector-slide">
              <div v-if="chainSelectorExpanded" class="selector-dropdown selector-dropdown-up">
                <div class="selector-search">
                  <a-input ref="chainSearchInputRef" v-model="chainSearchKeyword" placeholder="搜索区块链..." size="small" allow-clear><template #prefix><Icon icon="mdi:magnify" style="font-size: 14px; color: var(--text-color-quaternary, #c9cdd4)" /></template></a-input>
                </div>
                <div class="selector-list">
                  <div v-for="chain in filteredChainOptions" :key="chain.key" class="selector-item" :class="{ 'selector-item-selected': chainValue === chain.key }" @click.stop="handleChainSelect(chain.key)">
                    <ChainIcon :chain-key="chain.key" :alt="chain.name" style="width: 18px; height: 18px; flex-shrink: 0" />
                    <span class="selector-item-name">{{ chain.name }}</span>
                    <span class="selector-item-url">{{ chain.scan_url }}</span>
                    <Icon v-if="chainValue === chain.key" icon="mdi:check" style="font-size: 14px; color: var(--primary-6, #165dff); margin-left: auto" />
                  </div>
                </div>
              </div>
            </Transition>
          </div>
          
          <div class="status-divider"></div>
          
          <!-- 代币选择器 -->
          <div class="token-selector-container" style="position: relative">
            <div
              class="status-item status-token"
              :class="{
                'status-token-active': tokenSelectorExpanded,
                'status-item-disabled': balanceLoading,
              }"
              @click="toggleTokenSelector"
              :title="balanceLoading ? '执行过程中无法切换代币' : '点击切换代币'"
            >
              <Icon icon="mdi:coins" style="font-size: 14px" />
              <span class="status-label">{{ currentCoin?.label || '选择代币' }}</span>
              <Icon icon="mdi:chevron-up" style="font-size: 12px; margin-left: 4px; transition: transform 0.2s" :style="{ transform: tokenSelectorExpanded ? 'rotate(180deg)' : 'rotate(0deg)' }" />
            </div>
            <Transition name="selector-slide">
              <div v-if="tokenSelectorExpanded" class="selector-dropdown selector-dropdown-up">
                <div class="selector-search">
                  <a-input ref="tokenSearchInputRef" v-model="tokenSearchKeyword" placeholder="搜索代币..." size="small" allow-clear><template #prefix><Icon icon="mdi:magnify" style="font-size: 14px; color: var(--text-color-quaternary, #c9cdd4)" /></template></a-input>
                </div>
                <div class="selector-list">
                  <div v-for="token in filteredCoinOptions" :key="token.key" class="selector-item" :class="{ 'selector-item-selected': coinValue === token.key }" @click="handleTokenSelect(token.key)">
                    <Icon :icon="token.coin_type === 'base' ? 'mdi:circle-slice-8' : 'mdi:coin'" :style="{ fontSize: '18px', color: token.coin_type === 'base' ? 'var(--primary-6, #165dff)' : 'var(--success-6, #0fa962)', flexShrink: '0' }" />
                    <span class="selector-item-name">{{ token.label }}</span>
                    <span class="selector-item-symbol">({{ token.symbol }})</span>
                    <Icon v-if="coinValue === token.key" icon="mdi:check" style="font-size: 14px; color: var(--primary-6, #165dff); margin-left: auto" />
                  </div>
                </div>
              </div>
            </Transition>
          </div>
        </div>
      </div>
      
      <div class="status-bar-right">
        <!-- 代理状态 -->
        <div class="status-proxy-indicator" :class="{ 'proxy-active': proxyEnabled }" :style="{ color: proxyEnabled ? proxyStatusColor : 'var(--text-color-quaternary, #c9cdd4)' }" title="代理状态" @click="openProxyConfig">
          <Icon icon="mdi:shield-network" style="font-size: 14px" />
          <span class="proxy-status-text">{{ proxyEnabled ? '已启用代理' : '未启动代理' }}</span>
          <span v-if="proxyEnabled" class="proxy-count-text">({{ proxyCount }}个)</span>
        </div>
        
        <div class="status-divider-vertical"></div>
        
        <!-- 菜单按钮 -->
        <div class="status-menu-btn" :class="{ 'menu-btn-expanded': isSidePanelExpanded }" @click="isSidePanelExpanded ? collapseSidePanel() : expandSidePanel()" :title="isSidePanelExpanded ? '关闭功能菜单' : '打开功能菜单'">
          <Icon icon="mdi:menu" style="font-size: 15px" />
        </div>
        
        <!-- 设置按钮 -->
        <a-dropdown position="tr">
          <div class="status-settings-btn" title="设置"><Icon icon="mdi:cog" style="font-size: 15px" /></div>
          <template #content>
            <a-doption @click="toggleChainSelector"><template #icon><Icon icon="mdi:swap-horizontal" /></template>重新选择区块链</a-doption>
            <a-doption @click="showTokenManage" :disabled="!chainValue"><template #icon><Icon icon="mdi:coin" /></template>代币管理</a-doption>
            <a-doption @click="showRpcManage" :disabled="!chainValue"><template #icon><Icon icon="mdi:link" /></template>RPC管理</a-doption>
            <a-doption @click="showChainManage"><template #icon><Icon icon="mdi:web" /></template>区块链管理</a-doption>
            <a-doption @click="openProxyConfig"><template #icon><Icon icon="mdi:shield-network" /></template>代理配置<a-tag :color="proxyEnabled ? proxyStatusColor : '#86909c'" size="small" style="margin-left: 4px">{{ proxyEnabled ? proxyCount + '个' : '未启用' }}</a-tag></a-doption>
          </template>
        </a-dropdown>
      </div>
    </div>
  </div>
  
  <!-- 录入弹窗 -->
  <a-modal v-model:visible="visible" :width="700" title="录入钱包地址" @cancel="handleCancel" :on-before-ok="handleBeforeOk"
    :confirm-loading="importLoading"
    :cancel-button-props="{ disabled: importLoading }"
    :mask-closable="!importLoading"
    :closable="!importLoading">
    <div style="margin-top: 10px; height: 400px; position: relative;">
      <!-- Loading 遮罩层 -->
      <div v-if="importLoading" class="loading-overlay">
        <a-spin size="large" />
        <div class="loading-text">正在导入数据，请稍候...</div>
      </div>
      
      <CodeEditor 
        v-model="importText" 
        :error-lines="addressErrorLines"
        :disabled="importLoading"
        placeholder="格式：一行一个地址&#10;示例：8Y5eJ4... (Solana Address)"
        @input="validateImportData"
        style="height: 100%;"
      />
    </div>
    
    <!-- 验证错误提示 -->
    <div v-if="validationErrors.length > 0" style="margin-top: 15px;">
      <a-alert style="padding: 5px 15px;" type="error" :title="`发现 ${validationErrors.length} 个问题`" :show-icon="true">
        <ul style="margin: 8px 0 0 0; padding-left: 20px;">
          <li v-for="(error, index) in displayedErrors" :key="error"
            style="margin-bottom: 4px; color: #f53f3f; font-size: 12px;">{{ error }}</li>
        </ul>
        <div v-if="validationErrors.length > 3" style="margin-top: 10px; text-align: center;">
          <a-button type="text" size="small" @click="toggleErrorsExpanded" style="color: #165dff;font-size: 12px;">
            {{ errorsExpanded ? '⬆️收起' : '⬇️展开全部' }}
          </a-button>
        </div>
      </a-alert>
    </div>
  </a-modal>

  <WalletSystemImportModal v-model:visible="systemImportVisible" ecosystem="solana" import-mode="address_only" :title="'从系统导入查询地址'" @confirm="handleSystemImportConfirm" @cancel="systemImportVisible = false" />
  
  <!-- 添加代币弹窗 -->
  <a-modal v-model:visible="addCoinVisible" :width="700" title="添加代币" @cancel="handleAddCoinCancel"
    :on-before-ok="handleAddCoinBeforeOk" unmountOnClose>
    <a-input v-model="coinAddress" placeholder="请输入代币合约地址 (Mint Address)" allow-clear />
  </a-modal>
  
  <!-- 删除代币确认框 -->
  <a-modal v-model:visible="deleteTokenVisible" title="删除确认">
    <div>确认删除【 {{ currentCoin.label }} 】代币？</div>
    <template #footer>
      <a-button @click="deleteTokenCancel">取消</a-button>
      <a-button type="primary" status="danger" @click="deleteTokenConfirm" style="margin-left: 10px">确定
      </a-button>
    </template>
  </a-modal>
  
  <!-- 删除数据确认框 -->
  <a-modal v-model:visible="deleteItemVisible" title="删除确认">
    <div>确认删除地址为【
      {{ currentItemKey.substring(0, 15) + '......' + currentItemKey.substring(currentItemKey.length - 15) }}
      】的数据？
    </div>
    <template #footer>
      <a-button @click="deleteItemCancel">取消</a-button>
      <a-button type="primary" status="danger" @click="debouncedDeleteItemConfirm" style="margin-left: 10px">确定
      </a-button>
    </template>
  </a-modal>

  <!-- 高级筛选弹窗 -->
  <a-modal v-model:visible="advancedFilterVisible" title="高级筛选" width="500px">
    <a-form :model="filterForm" layout="vertical">
      <!-- 平台币余额筛选 -->
      <a-form-item label="SOL余额筛选">
        <div style="display: flex; gap: 10px; align-items: center;">
          <a-select v-model="filterForm.platBalanceOperator" style="width: 100px;">
            <a-option value="gt">大于</a-option>
            <a-option value="eq">等于</a-option>
            <a-option value="lt">小于</a-option>
          </a-select>
          <a-input v-model="filterForm.platBalanceValue" placeholder="请输入SOL余额值" style="flex: 1;" @input="debouncedFilterUpdate" />
        </div>
      </a-form-item>
      
      <!-- 代币余额筛选 -->
      <a-form-item label="代币余额筛选">
        <div style="display: flex; gap: 10px; align-items: center;">
          <a-select v-model="filterForm.coinBalanceOperator" style="width: 100px;">
            <a-option value="gt">大于</a-option>
            <a-option value="eq">等于</a-option>
            <a-option value="lt">小于</a-option>
          </a-select>
          <a-input v-model="filterForm.coinBalanceValue" placeholder="请输入代币余额值" style="flex: 1;" @input="debouncedFilterUpdate" />
        </div>
      </a-form-item>
      
      <!-- 错误信息模糊匹配 -->
      <a-form-item label="错误信息模糊匹配">
        <a-input v-model="filterForm.errorMsg" placeholder="请输入要匹配的错误信息" @input="debouncedFilterUpdate" />
      </a-form-item>
    </a-form>
    
    <template #footer>
      <a-button @click="advancedFilterVisible = false">取消</a-button>
      <a-button type="primary" @click="applyAdvancedFilter" style="margin-left: 10px;">应用筛选</a-button>
    </template>
  </a-modal>

  <!-- 链管理组件 -->
  <ChainManagement ref="chainManageRef" @chain-updated="handleChainUpdated" ecosystem-filter="solana" />
  <!-- RPC管理组件 -->
  <RpcManagement ref="rpcManageRef" :chain-value="chainValue" :chain-options="chainOptions" @rpc-updated="handleRpcUpdated" />
  <!-- 代币管理组件 -->
  <TokenManagement ref="tokenManageRef" :chain-value="chainValue" :chain-options="chainOptions" @token-updated="handleTokenUpdated" />
  
  <!-- 代理配置弹窗 -->
  <ProxyConfigModal 
    v-if="proxyConfigVisible" 
    :visible="proxyConfigVisible" 
    @update:visible="proxyConfigVisible = $event"
    @config-change="handleProxyConfigChange"
    ref="proxyConfigRef"
  />

</template>

<style scoped lang="less">
/* Loading 遮罩层样式 */
.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--loading-bg, #f7f8fa);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  border-radius: 6px;
}

.loading-text {
  margin-top: 16px;
  color: #4e5969;
  font-size: 14px;
  font-weight: 500;
}

/* 布局样式 */
.container {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden; /* Prevent scrollbar during transitions */
  padding: 50px 10px 50px 10px; /* 为顶部TitleBar和底部StatusBar留出空间 */
  min-width: 1000px;
  background: var(--bg-color, rgb(42, 42, 43));
}

.container::-webkit-scrollbar {
  display: none;
}

.container {
  -ms-overflow-style: none;
  scrollbar-width: none;
}

/* 下拉菜单项样式 */
:deep(.arco-dropdown-option-content) {
  display: flex;
  align-items: center;
}

/* 主内容区 */
.main-content {
  flex: 1;
  display: flex;
  overflow: visible;
  position: relative;
}

/* 左侧面板 */
.left-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: visible;
  min-width: 0;
}

/* 表格区域 */
.table-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.action-buttons-section {
  flex-shrink: 0;
  position: relative;
  overflow: visible;
  height: 10px;
}

.config-section { flex-shrink: 0; background: var(--card-bg, var(--color-bg-1, #ffffff)); border: 1px solid var(--color-border, #e5e6eb); border-radius: 12px; padding: 16px; box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04); }


.config-container {
  display: flex;
  align-items: center;
  gap: 20px;
  width: 100%;
}

.config-item {
  display: flex;
  align-items: center;
}

.config-divider {
  width: 1px;
  height: 20px;
  background: var(--color-border, #e5e6eb);
}

.config-label {
  font-size: 14px;
  color: var(--text-color-secondary, #4e5969);
}

/* 右侧工具栏面板 */
.right-panel {
  width: 50px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  transition: width 0.3s ease;
  overflow: visible;
}

.side-actions-panel-fixed {
  width: 50px;
  background: rgba(22, 27, 34, 0.9);
  border: 1px solid rgba(88, 108, 199, 0.2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px;
  pointer-events: none;
  box-shadow: 3px 0px 6px 0px rgba(0, 0, 0, 0.2), -1px 0 4px rgba(0, 0, 0, 0.1);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  height: 100%;
}

.side-actions-content-fixed {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  opacity: 1;
  pointer-events: auto;
  flex: 1;
  justify-content: center;
}

.side-actions-divider {
  width: 30px;
  height: 1px;
  background: linear-gradient(to right, transparent, rgba(88, 108, 199, 0.3), transparent);
  margin: 8px 0;
}

/* 侧边栏按钮样式 */
.side-actions-content-fixed :deep(.arco-btn) {
  width: 36px;
  height: 36px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  border: 1px solid rgba(88, 108, 199, 0.25);
  background: rgba(13, 17, 23, 0.6);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.side-actions-content-fixed :deep(.arco-btn:hover) {
  background: rgba(88, 108, 199, 0.25);
  border-color: rgba(91, 138, 255, 0.5);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(91, 138, 255, 0.2);
}

.side-actions-content-fixed :deep(.arco-btn[type='primary']) {
  background: linear-gradient(135deg, #3b5bdb 0%, #5b8aff 100%);
  border-color: #5b8aff;
  box-shadow: 0 2px 6px rgba(91, 138, 255, 0.3);
}

.side-actions-content-fixed :deep(.arco-btn[type='primary']:hover) {
  background: linear-gradient(135deg, #5b8aff 0%, #3b5bdb 100%);
  box-shadow: 0 4px 12px rgba(91, 138, 255, 0.4);
}

.side-actions-content-fixed :deep(.arco-btn[status='success']) {
  background: linear-gradient(135deg, #0d8a4f 0%, #14b866 100%);
  border-color: #14b866;
  box-shadow: 0 2px 6px rgba(20, 184, 102, 0.3);
}

.side-actions-content-fixed :deep(.arco-btn[status='success']:hover) {
  background: linear-gradient(135deg, #14b866 0%, #0d8a4f 100%);
  box-shadow: 0 4px 12px rgba(20, 184, 102, 0.4);
}

.side-actions-content-fixed :deep(.arco-btn[status='danger']) {
  background: linear-gradient(135deg, #dc2626 0%, #ef4444 100%);
  border-color: #ef4444;
  box-shadow: 0 2px 6px rgba(239, 68, 68, 0.3);
}

.side-actions-content-fixed :deep(.arco-btn[status='danger']:hover) {
  background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.4);
}

/* 表格与侧边栏联动 */
.table-with-side-actions {
  margin-right: 0;
  margin-top: 0;
  height: 100%;
}

/* 悬浮操作栏 */
.floating-action-bar { position: relative; z-index: 10; width: 100%; display: flex; justify-content: center; pointer-events: none; margin-top: -24px;}

.floating-action-content {
  display: flex;
  gap: 40px;
  align-items: center;
  pointer-events: auto;
  background: var(--bg-color, #ffffff);
  padding: 6px 12px;
  border-radius: 50px;
  /* 只有内容部分有背景和阴影，看起来像悬浮 */
  box-shadow: none; 
  background: transparent;
}

.btn-wrapper {
  min-width: 140px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.floating-btn {
  min-width: 140px;
  height: 44px;
  font-size: 15px;
  font-weight: 600;
  border-radius: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s ease;
  border: none;
  cursor: pointer;
}

.floating-btn.success-btn {
  background: linear-gradient(135deg, #00b42a 0%, #009624 100%);
  color: #ffffff;
  box-shadow: 0 4px 12px rgba(0, 180, 42, 0.3);
}

.floating-btn.success-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(0, 180, 42, 0.4);
}

.btn-stop-wrapper .floating-btn.stopping {
  background: linear-gradient(135deg, #ff7d00 0%, #e67000 100%) !important;
  box-shadow: 0 4px 12px rgba(255, 125, 0, 0.3) !important;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-stop-wrapper .floating-btn.stopping:hover {
  background: linear-gradient(135deg, #ff4d4f 0%, #e64547 100%) !important;
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(255, 77, 79, 0.3);
}

.btn-text-stop {
  position: relative;
  display: inline-block;
  min-width: 60px;
  text-align: center;
}

.btn-text-normal, .btn-text-hover {
  display: block;
  transition: all 0.15s ease;
}

.btn-text-hover {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  text-align: center;
  opacity: 0;
  transform: translateY(-5px);
}

.btn-stop-wrapper .floating-btn.stopping:hover .btn-text-normal {
  opacity: 0;
  transform: translateY(5px);
}

.btn-stop-wrapper .floating-btn.stopping:hover .btn-text-hover {
  opacity: 1;
  transform: translateY(0);
}

/* 悬浮进度条 */
.floating-progress-bar {
  position: fixed;
  top: 50px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10000;
  width: 90%;
  max-width: 600px;
  background: var(--card-bg, #ffffff);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), 0 2px 8px rgba(0, 0, 0, 0.08);
  border: 1px solid var(--border-color, #e5e6eb);
  backdrop-filter: blur(8px);
}

.progress-content {
  padding: 5px 20px;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.progress-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color, #1d2129);
}

.progress-count {
  font-size: 13px;
  color: var(--text-color-secondary, #86909c);
  font-weight: 500;
}

.progress-bar {
  width: 100%;
}

.progress-slide-enter-active {
  transition: all 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.progress-slide-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.6, 1);
}

.progress-slide-enter-from {
  opacity: 0;
  transform: translateX(-50%) translateY(-100%);
}

.progress-slide-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-100%);
}

.progress-slide-enter-to,
.progress-slide-leave-from {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}

/* 底部状态栏 */
.status-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 40px;
  background: linear-gradient(135deg, #161b22 0%, #0d1117 100%);
  border-top: 1px solid rgba(88, 108, 199, 0.2);
  box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.2);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  z-index: 1000;
  font-size: 12px;
}

.status-bar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-gas-group {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 10px;
  background: var(--color-fill-1, #f2f3f5);
  border-radius: 12px;
  margin-left: 4px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-color-secondary, #6b778c);
}

.status-label {
  font-weight: 500;
  color: var(--text-color, #1d2129);
}

.status-explorer-tag {
  margin-left: 8px;
  cursor: pointer;
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-fill-1, #f2f3f5);
  border: 1px solid var(--color-border-2, #e5e6eb);
  color: var(--text-color-tertiary, #8c8f94);
  transition: all 0.2s ease;
}

.status-explorer-tag:hover {
  background: var(--primary-1, #e8f1ff);
  border-color: var(--primary-3, #94bfff);
  color: var(--primary-6, #165dff);
}

.status-chain {
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-chain:hover {
  background: linear-gradient(135deg, var(--primary-1, #e8f1ff), var(--color-fill-2, #f2f3f5));
}

.status-chain:hover .status-label {
  color: var(--primary-6, #165dff);
}

.status-chain:hover .status-explorer-tag {
  background: var(--primary-1, #e8f1ff);
  border-color: var(--primary-3, #94bfff);
  color: var(--primary-6, #165dff);
}

.status-token {
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-token:hover {
  background: linear-gradient(135deg, var(--success-1, #e6fffb), var(--color-fill-2, #f2f3f5));
}

.status-token:hover .status-label {
  color: var(--success-6, #0fa962);
}

.status-gas-label {
  color: var(--text-color-tertiary, #8c8f94);
  font-size: 11px;
}

.status-gas-value {
  font-weight: 600;
  color: var(--primary-6, #165dff);
  font-size: 13px;
}

.status-gas-unit {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-color-tertiary, #8c8f94);
}

.status-divider {
  width: 1px;
  height: 18px;
  background: linear-gradient(to bottom, transparent, var(--color-border, #e5e6eb) 30%, var(--color-border, #e5e6eb) 70%, transparent);
  margin: 0 2px;
}

.status-divider-vertical {
  width: 1px;
  height: 24px;
  background: linear-gradient(to bottom, transparent, var(--color-border-2, #d9d9d9) 30%, var(--color-border-2, #d9d9d9) 70%, transparent);
  margin: 0 8px;
}

.status-menu-btn {
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-color-secondary, #6b778c);
}

.status-menu-btn:hover {
  background: var(--color-fill-2, #f2f3f5);
  color: var(--primary-6, #165dff);
}

.status-menu-btn.menu-btn-expanded {
  color: var(--primary-6, #165dff);
  background: var(--primary-1, #e8f1ff);
}

.status-settings-btn {
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-color-secondary, #6b778c);
}

.status-settings-btn:hover {
  background: var(--color-fill-2, #f2f3f5);
  color: var(--primary-6, #165dff);
  transform: rotate(90deg);
}

.status-proxy-indicator {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 12px;
  background: var(--color-fill-1, #f2f3f5);
  transition: all 0.2s ease;
  cursor: pointer;
}

.status-proxy-indicator:hover {
  background: var(--color-fill-2, #e5e6eb);
}

.status-proxy-indicator.proxy-active {
  background: var(--success-1, #e6fffb);
}

.status-proxy-indicator.proxy-active:hover {
  background: var(--success-2, #b7f0e6);
}

.proxy-status-text {
  font-size: 12px;
  font-weight: 500;
}

.proxy-count-text {
  font-size: 11px;
  color: var(--text-color-tertiary, #8c8f94);
}

.status-chain-active {
  background: linear-gradient(135deg, var(--primary-1, #e8f1ff), var(--color-fill-2, #f2f3f5)) !important;
}

.status-chain-active .status-label {
  color: var(--primary-6, #165dff) !important;
}

.status-token-active {
  background: linear-gradient(135deg, var(--success-1, #e6fffb), var(--color-fill-2, #f2f3f5)) !important;
}

.status-token-active .status-label {
  color: var(--success-6, #0fa962) !important;
}

.status-item-disabled {
  cursor: not-allowed !important;
  opacity: 0.6;
}

.status-item-disabled:hover {
  background: transparent !important;
}

.status-item-disabled:hover .status-label {
  color: var(--text-color, #1d2129) !important;
}

.status-item-disabled .status-explorer-tag {
  cursor: not-allowed !important;
  pointer-events: none;
}

/* 下拉选择器 */
.selector-dropdown {
  position: absolute;
  bottom: 100%;
  left: 0;
  background: var(--card-bg, #ffffff);
  border: 1px solid var(--color-border, #e5e6eb);
  border-radius: 12px;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.15), 0 -2px 8px rgba(0, 0, 0, 0.1);
  z-index: 10000;
  margin-bottom: 8px;
  min-width: 360px;
  max-height: 320px;
  display: flex;
  flex-direction: column;
  overflow: visible;
}

.selector-dropdown-up {
  border-radius: 12px 12px 4px 4px;
}

.selector-search {
  padding: 12px 12px 8px 12px;
  border-bottom: 1px solid var(--color-border-2, #f0f0f0);
  background: var(--color-fill-1, #f7f8fa);
}

.selector-list {
  flex: 1;
  overflow-y: auto;
  max-height: 240px;
  padding: 8px;
}

.selector-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  margin-bottom: 2px;
}

.selector-item:hover {
  background: var(--color-fill-2, #f2f3f5);
}

.selector-item-selected {
  background: var(--primary-1, #e8f1ff);
}

.selector-item-selected:hover {
  background: var(--primary-2, #d4e4ff);
}

.selector-item-name {
  font-weight: 500;
  color: var(--text-color, #1d2129);
  flex: 1;
  overflow: visible;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.selector-item-url {
  font-size: 11px;
  color: var(--text-color-tertiary, #8c8f94);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.selector-item-symbol {
  font-size: 11px;
  color: var(--text-color-secondary, #6b778c);
  margin-left: 4px;
}

.selector-slide-enter-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.selector-slide-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.selector-slide-enter-from, .selector-slide-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

.selector-slide-enter-to, .selector-slide-leave-from {
  opacity: 1;
  transform: translateY(0);
}
</style>
