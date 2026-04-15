<template>
  <a-modal
    :visible="visible"
    :title="title"
    :width="1000"
    :mask-closable="false"
    :footer="false"
    :body-style="{ padding: '15px' }"
    unmountOnClose
    @update:visible="handleVisibleUpdate"
  >
    <div class="system-import-layout">
      <div class="left">
        <a-spin :loading="groupsLoading" style="width: 100%">
          <a-tree
            :data="groups"
            :field-names="{ key: 'id', title: 'name', children: 'children' }"
            block-node
            v-model:selected-keys="selectedGroupKeys"
            v-model:expanded-keys="expandedKeys"
            @select="onGroupSelect"
            @node-click="onNodeClick"
          />
        </a-spin>
      </div>
      <div class="right">
        <div class="filters">
          <a-select v-model="filterForm.chainType" style="width: 120px" :disabled="chainLocked">
            <a-option value="all">全部链</a-option>
            <a-option value="evm">EVM</a-option>
            <a-option value="solana">Solana</a-option>
          </a-select>
          <a-select v-model="filterForm.walletType" style="width: 140px" :disabled="walletTypeLocked">
            <a-option value="all">全部来源</a-option>
            <a-option value="full_wallet">完整钱包</a-option>
            <a-option value="address_only">仅地址</a-option>
          </a-select>
          <a-input-search
            v-model="filterForm.searchKeyword"
            allow-clear
            placeholder="搜索名称/地址/备注"
            style="flex: 1"
          />
        </div>

        <div class="list">
          <a-tabs v-if="isTransferPair" v-model:active-key="transferPairTab" type="rounded" size="medium" class="transfer-pair-tabs">
            <a-tab-pane key="from" title="出账钱包（私钥）">
              <VirtualScrollerTable
                :columns="columns"
                :data="filteredFromItems"
                :row-selection="rowSelection"
                :selected-keys="selectedFromKeys"
                row-key="key"
                height="100%"
                page-type="system_import"
                :loading="walletsLoading"
                :empty-data="filteredFromItems.length === 0"
                @update:selected-keys="(keys) => handleSelectionChange(keys, 'from')"
              >
                <template #index="{ rowIndex }">
                  {{ rowIndex + 1 }}
                </template>
                <template #chain_type="{ record }">
                  <span>{{ record.chain_type === 'evm' ? 'EVM' : 'SOL' }}</span>
                </template>
                <template #wallet_type="{ record }">
                  <span>{{ record.has_private_key ? '完整' : '仅地址' }}</span>
                </template>
              </VirtualScrollerTable>
            </a-tab-pane>
            <a-tab-pane key="to" title="收款地址">
              <VirtualScrollerTable
                :columns="columns"
                :data="filteredToItems"
                :row-selection="rowSelection"
                :selected-keys="selectedToKeys"
                row-key="key"
                height="100%"
                page-type="system_import"
                :loading="walletsLoading"
                :empty-data="filteredToItems.length === 0"
                @update:selected-keys="(keys) => handleSelectionChange(keys, 'to')"
              >
                <template #index="{ rowIndex }">
                  {{ rowIndex + 1 }}
                </template>
                <template #chain_type="{ record }">
                  <span>{{ record.chain_type === 'evm' ? 'EVM' : 'SOL' }}</span>
                </template>
                <template #wallet_type="{ record }">
                  <span>{{ record.has_private_key ? '完整' : '仅地址' }}</span>
                </template>
              </VirtualScrollerTable>
            </a-tab-pane>
          </a-tabs>
          <VirtualScrollerTable
            v-else
            :columns="columns"
            :data="filteredItems"
            :row-selection="rowSelection"
            :selected-keys="selectedItemKeys"
            row-key="key"
            height="100%"
            page-type="system_import"
            :loading="walletsLoading"
            :empty-data="filteredItems.length === 0"
            @update:selected-keys="handleSelectionChange"
          >
            <template #index="{ rowIndex }">
              {{ rowIndex + 1 }}
            </template>
            <template #chain_type="{ record }">
              <span>{{ record.chain_type === 'evm' ? 'EVM' : 'SOL' }}</span>
            </template>
            <template #wallet_type="{ record }">
              <span>{{ record.has_private_key ? '完整' : '仅地址' }}</span>
            </template>
          </VirtualScrollerTable>
        </div>

        <div class="footer-bar">
          <div class="left-info">
            <template v-if="isTransferPair">
              出账: {{ selectedFromKeys.length }} 个，收款: {{ selectedToKeys.length }} 个
            </template>
            <template v-else>
              已选择: {{ selectedItemKeys.length }} 个
              <span v-if="maxSelection">（最多 {{ maxSelection }} 个）</span>
            </template>
          </div>
          <div class="right-actions">
            <a-button @click="handleCancel" :disabled="confirmLoading">取消</a-button>
            <a-button v-if="isTransferPair && transferPairTab === 'to'" @click="handleTransferPairBack" :disabled="confirmLoading">上一步</a-button>
            <a-button @click="handleClearSelection" :disabled="isTransferPair ? (transferPairTab === 'from' ? selectedFromKeys.length === 0 : selectedToKeys.length === 0) : selectedItemKeys.length === 0">清空</a-button>
            <a-button v-if="isTransferPair" type="primary" @click="handleTransferPairPrimary" :loading="confirmLoading">
              {{ transferPairTab === 'from' ? '下一步' : '确认导入' }}
            </a-button>
            <a-button v-else type="primary" @click="handleConfirm" :loading="confirmLoading">确认</a-button>
          </div>
        </div>
      </div>
    </div>
  </a-modal>

  <a-modal
    v-model:visible="passwordModalVisible"
    title="输入钱包管理主密码"
    :mask-closable="false"
    :closable="false"
    :confirm-loading="passwordVerifying"
    unmountOnClose
    @cancel="handlePasswordCancel"
    @before-ok="handlePasswordOk"
  >
    <a-form layout="vertical">
      <a-form-item label="密码">
        <a-input-password ref="passwordInputRef" v-model="passwordInput" placeholder="仅用于本次解密，不会保存" @keyup.enter="() => !passwordVerifying && handlePasswordOk()" />
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script setup>
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Message, Modal, Notification } from '@arco-design/web-vue'
import VirtualScrollerTable from '@/components/VirtualScrollerTable.vue'
import { openSealedSecret } from '@/utils/secretCrypto'

const props = defineProps({
  visible: { type: Boolean, default: false },
  ecosystem: { type: String, default: 'evm' },
  importMode: { type: String, default: 'address_only' },
  title: { type: String, default: '从系统钱包导入' },
  multiple: { type: Boolean, default: true },
  maxSelection: { type: Number, default: undefined },
})

const emit = defineEmits(['confirm', 'cancel', 'update:visible'])

const groupsLoading = ref(false)
const walletsLoading = ref(false)
const confirmLoading = ref(false)

const groups = ref([])
const expandedKeys = ref([])
const selectedGroupKeys = ref(['ALL'])

const wallets = ref([])

const rowSelection = { type: 'checkbox' }

const selectedItemKeys = ref([])
const selectedFromKeys = ref([])
const selectedToKeys = ref([])
const transferPairTab = ref('from')

const filterForm = reactive({
  chainType: 'all',
  walletType: 'all',
  searchKeyword: '',
})

const chainLocked = computed(() => props.ecosystem === 'evm' || props.ecosystem === 'solana')
const isTransferPair = computed(() => props.importMode === 'transfer_pair')
const isAddressOnly = computed(() => props.importMode === 'address_only')
const walletTypeLocked = computed(() => isTransferPair.value && transferPairTab.value === 'from')

watch(
  () => props.ecosystem,
  (v) => {
    if (v === 'evm' || v === 'solana') {
      filterForm.chainType = v
    }
  },
  { immediate: true }
)

watch(
  () => [isTransferPair.value, transferPairTab.value],
  () => {
    if (!isTransferPair.value) return
    if (transferPairTab.value === 'from') {
      filterForm.walletType = 'full_wallet'
    }
  },
  { immediate: true }
)

watch(
  () => filterForm.walletType,
  () => {
    if (!props.visible) return
    if (isTransferPair.value) {
      if (transferPairTab.value === 'from') selectedFromKeys.value = []
      else selectedToKeys.value = []
      return
    }
    selectedItemKeys.value = []
  }
)


const columns = [
  { title: '序号', align: 'center', width: 53, slotName: 'index' },
  { title: '名称', align: 'center', dataIndex: 'name', width: 100, ellipsis: true, tooltip: true },
  { title: '地址', align: 'center', dataIndex: 'address', width: 360, ellipsis: true, tooltip: true },
  { title: '链', align: 'center', width: 70, slotName: 'chain_type' },
  { title: '类型', align: 'center', width: 80, slotName: 'wallet_type' },
  { title: '备注', align: 'center', dataIndex: 'remark', ellipsis: true, tooltip: true },
]

const parseDbId = (val) => {
  if (typeof val === 'number' && Number.isFinite(val)) return val
  if (typeof val === 'string' && /^\d+$/.test(val)) return Number(val)
  return null
}

const buildTree = (list) => {
  const map = {}
  const roots = []
  ;(list || []).forEach((item) => {
    map[item.id] = { ...item }
  })
  ;(list || []).forEach((item) => {
    if (item.parent_id && map[item.parent_id]) {
      if (!map[item.parent_id].children) {
        map[item.parent_id].children = []
      }
      map[item.parent_id].children.push(map[item.id])
    } else {
      roots.push(map[item.id])
    }
  })
  return roots
}

async function loadGroups() {
  groupsLoading.value = true
  try {
    const res = await invoke('get_groups')
    const userRoots = buildTree(res)
    const systemGroups = [
      { id: 'SYSTEM_EVM', name: 'EVM 生态', children: [], isSystem: true },
      { id: 'SYSTEM_SOLANA', name: 'Solana 生态', children: [], isSystem: true },
    ]
    const otherRoots = []
    userRoots.forEach((node) => {
      if (node.chain_type === 'evm') systemGroups[0].children.push(node)
      else if (node.chain_type === 'solana') systemGroups[1].children.push(node)
      else otherRoots.push(node)
    })
    groups.value = [{ id: 'ALL', name: '全部', children: [...systemGroups, ...otherRoots], isSystem: true }]
    expandedKeys.value = ['ALL', 'SYSTEM_EVM', 'SYSTEM_SOLANA']
  } catch (e) {
    Message.error('加载分组失败: ' + (e?.message || e))
    groups.value = [{ id: 'ALL', name: '全部', children: [], isSystem: true }]
    expandedKeys.value = ['ALL']
  } finally {
    groupsLoading.value = false
  }
}

async function loadWallets() {
  walletsLoading.value = true
  try {
    const key = selectedGroupKeys.value?.[0]
    if (!key) {
      wallets.value = []
      return
    }

    if (key === 'ALL') {
      const chainType = filterForm.chainType === 'all' ? null : filterForm.chainType
      wallets.value = await invoke('get_wallets', { group_id: null, chain_type: chainType })
      return
    }

    if (key === 'SYSTEM_EVM' || key === 'SYSTEM_SOLANA') {
      const chainType = key === 'SYSTEM_EVM' ? 'evm' : 'solana'
      wallets.value = await invoke('get_wallets', { group_id: null, chain_type: chainType })
      return
    }

    const gid = parseDbId(key)
    if (gid === null) {
      wallets.value = []
      return
    }
    const chainType = filterForm.chainType === 'all' ? null : filterForm.chainType
    wallets.value = await invoke('get_wallets', { group_id: gid, chain_type: chainType })
  } catch (e) {
    Message.error('加载钱包失败: ' + (e?.message || e))
    wallets.value = []
  } finally {
    walletsLoading.value = false
  }
}

const items = computed(() => {
  return (wallets.value || []).map((w) => ({
    key: `wallet:${w.id}`,
    id: w.id,
    name: w.name || '',
    address: w.address,
    chain_type: (w.chain_type || '').toLowerCase(),
    wallet_type: w.wallet_type || '',
    has_private_key: !!w.has_private_key,
    has_mnemonic: !!w.has_mnemonic,
    group_id: w.group_id ?? null,
    group_name: w.group_name || '',
    remark: w.remark || '',
  }))
})

const filteredItems = computed(() => {
  const keyword = (filterForm.searchKeyword || '').trim().toLowerCase()
  return items.value
    .filter((it) => {
      if (filterForm.chainType !== 'all' && it.chain_type !== filterForm.chainType) return false
      if (filterForm.walletType === 'full_wallet' && !it.has_private_key) return false
      if (filterForm.walletType === 'address_only' && it.has_private_key) return false
      if (!keyword) return true
      const hay = `${it.name || ''} ${it.address || ''} ${it.remark || ''}`.toLowerCase()
      return hay.includes(keyword)
    })
})

const filteredFromItems = computed(() => {
  if (!isTransferPair.value) return []
  const keyword = (filterForm.searchKeyword || '').trim().toLowerCase()
  return items.value.filter((it) => {
    if (filterForm.chainType !== 'all' && it.chain_type !== filterForm.chainType) return false
    if (!it.has_private_key) return false
    if (!keyword) return true
    const hay = `${it.name || ''} ${it.address || ''} ${it.remark || ''}`.toLowerCase()
    return hay.includes(keyword)
  })
})

const filteredToItems = computed(() => {
  if (!isTransferPair.value) return []
  const keyword = (filterForm.searchKeyword || '').trim().toLowerCase()
  return items.value.filter((it) => {
    if (filterForm.chainType !== 'all' && it.chain_type !== filterForm.chainType) return false
    if (filterForm.walletType === 'full_wallet' && !it.has_private_key) return false
    if (filterForm.walletType === 'address_only' && it.has_private_key) return false
    if (!keyword) return true
    const hay = `${it.name || ''} ${it.address || ''} ${it.remark || ''}`.toLowerCase()
    return hay.includes(keyword)
  })
})

function handleSelectionChange(keys, target = 'single') {
  if (!props.multiple && keys.length > 1) {
    const last = [keys[keys.length - 1]]
    if (target === 'from') selectedFromKeys.value = last
    else if (target === 'to') selectedToKeys.value = last
    else selectedItemKeys.value = last
    return
  }

  if (props.maxSelection && keys.length > props.maxSelection) {
    Message.warning(`最多只能选择 ${props.maxSelection} 个`)
    const sliced = keys.slice(0, props.maxSelection)
    if (target === 'from') selectedFromKeys.value = sliced
    else if (target === 'to') selectedToKeys.value = sliced
    else selectedItemKeys.value = sliced
    return
  }

  if (target === 'from') selectedFromKeys.value = keys
  else if (target === 'to') selectedToKeys.value = keys
  else selectedItemKeys.value = keys
}

function handleClearSelection() {
  if (isTransferPair.value) {
    if (transferPairTab.value === 'from') selectedFromKeys.value = []
    else selectedToKeys.value = []
    return
  }
  selectedItemKeys.value = []
}

function handleVisibleUpdate(v) {
  emit('update:visible', v)
  if (!v) emit('cancel')
}

function handleCancel() {
  emit('update:visible', false)
  emit('cancel')
}

function onGroupSelect(keys) {
  if (keys?.length) {
    selectedGroupKeys.value = keys
  }
  refreshLists()
}

function onNodeClick(node) {
  selectedGroupKeys.value = [node.id]
  refreshLists()
}

async function refreshLists() {
  selectedItemKeys.value = []
  selectedFromKeys.value = []
  selectedToKeys.value = []
  transferPairTab.value = 'from'
  await loadWallets()
}

watch(
  () => props.visible,
  async (v) => {
    if (!v) return
    if (!groups.value.length) await loadGroups()
    if (!selectedGroupKeys.value?.length) selectedGroupKeys.value = ['ALL']
    await refreshLists()
  }
)

watch(
  () => [filterForm.chainType, selectedGroupKeys.value?.[0]],
  async () => {
    if (!props.visible) return
    await refreshLists()
  }
)

const passwordModalVisible = ref(false)
const passwordInput = ref('')
const passwordVerifying = ref(false)
const passwordInputRef = ref(null)
let pendingPasswordResolve = null

async function requestPassword() {
  passwordInput.value = ''
  passwordModalVisible.value = true
  return new Promise((resolve) => {
    pendingPasswordResolve = resolve
  })
}

watch(passwordModalVisible, async (v) => {
  if (!v) return
  await nextTick()
  setTimeout(() => {
    const el = passwordInputRef.value
    if (el?.focus) {
      el.focus()
      return
    }
    const inputEl = el?.$el?.querySelector?.('input')
    if (inputEl?.focus) inputEl.focus()
  }, 0)
})

function handlePasswordCancel() {
  passwordModalVisible.value = false
  if (pendingPasswordResolve) pendingPasswordResolve(null)
  pendingPasswordResolve = null
}

async function handlePasswordOk() {
  const pwd = String(passwordInput.value || '').trim()
  if (!pwd) {
    Message.error('请输入密码')
    return false
  }

  passwordVerifying.value = true
  try {
    const ok = await invoke('verify_password', { request: { password: pwd } })
    if (!ok) {
      Message.error('主密码错误')
      return false
    }
    passwordModalVisible.value = false
    if (pendingPasswordResolve) pendingPasswordResolve(pwd)
    pendingPasswordResolve = null
    return true
  } catch (e) {
    Message.error('验证主密码失败: ' + (e?.message || e))
    return false
  } finally {
    passwordVerifying.value = false
  }
}

async function buildConfirmPayload(password) {
  const selected = new Map(items.value.map((it) => [it.key, it]))
  const picked = selectedItemKeys.value.map((k) => selected.get(k)).filter(Boolean)
  if (!picked.length) return []

  if (props.importMode !== 'full') {
    return picked.map((it) => ({
      id: it.id,
      name: it.name,
      address: it.address,
      chain_type: it.chain_type,
      wallet_type: it.has_private_key ? 'full_wallet' : 'address_only',
      group_id: it.group_id ?? undefined,
      group_name: it.group_name || undefined,
      remark: it.remark || undefined,
    }))
  }

  const invalid = picked.filter((it) => !it.has_private_key)
  const valid = picked.filter((it) => it.has_private_key)

  if (!valid.length) return []

  const out = []
  for (const it of valid) {
    const secrets = await invoke('get_wallet_secrets', { id: it.id, password, transport_token: null })
    const sealedPk = secrets?.sealed_private_key
    if (!sealedPk) continue
    const privateKey = await openSealedSecret(sealedPk, password)
    out.push({
      id: it.id,
      name: it.name,
      address: it.address,
      private_key: privateKey,
      chain_type: it.chain_type,
      wallet_type: 'full_wallet',
      group_id: it.group_id ?? undefined,
      group_name: it.group_name || undefined,
      remark: it.remark || undefined,
    })
  }

  if (invalid.length) {
    Notification.warning({
      title: '部分钱包无私钥',
      content: `已自动忽略 ${invalid.length} 个仅地址钱包`,
      position: 'topLeft',
    })
  }

  return out
}

async function handleConfirm() {
  if (!selectedItemKeys.value.length) {
    Notification.warning({ title: '未选择任何钱包', position: 'topLeft' })
    return
  }

  if (props.importMode === 'full') {
    const selected = new Map(items.value.map((it) => [it.key, it]))
    const picked = selectedItemKeys.value.map((k) => selected.get(k)).filter(Boolean)
    const hasInvalid = picked.some((it) => !it.has_private_key)
    if (hasInvalid) {
      const ok = await new Promise((resolve) => {
        Modal.confirm({
          title: '部分选择无法用于转账',
          content: '包含仅地址或无私钥钱包，无法用于转账。是否仅导入可用的完整钱包？',
          onOk: () => resolve(true),
          onCancel: () => resolve(false),
        })
      })
      if (!ok) return
    }
  }

  confirmLoading.value = true
  let password = null
  try {
    if (props.importMode === 'full') {
      password = await requestPassword()
      if (!password) return
    }
    const payload = await buildConfirmPayload(password)
    if (!payload.length) {
      Notification.warning({ title: '无可导入数据', position: 'topLeft' })
      return
    }
    emit('confirm', payload)
    emit('update:visible', false)
  } catch (e) {
    Message.error('导入失败: ' + (e?.message || e))
  } finally {
    confirmLoading.value = false
    passwordInput.value = ''
    password = null
  }
}

function handleTransferPairBack() {
  transferPairTab.value = 'from'
}

async function handleTransferPairPrimary() {
  if (transferPairTab.value === 'from') {
    if (!selectedFromKeys.value.length) {
      Notification.warning({ title: '未选择出账钱包', position: 'topLeft' })
      return
    }
    transferPairTab.value = 'to'
    return
  }
  await handleConfirmTransferPair()
}

async function buildTransferPairPayload(password) {
  const selected = new Map(items.value.map((it) => [it.key, it]))
  const pickedFrom = selectedFromKeys.value.map((k) => selected.get(k)).filter(Boolean)
  const pickedTo = selectedToKeys.value.map((k) => selected.get(k)).filter(Boolean)
  const validFrom = pickedFrom.filter((it) => it.has_private_key)
  const toAddresses = pickedTo.map((it) => String(it.address || '').trim()).filter(Boolean)

  if (!validFrom.length || !toAddresses.length) {
    return null
  }

  const fromWallets = []
  for (const it of validFrom) {
    const secrets = await invoke('get_wallet_secrets', { id: it.id, password, transport_token: null })
    const sealedPk = secrets?.sealed_private_key
    if (!sealedPk) continue
    const privateKey = await openSealedSecret(sealedPk, password)
    fromWallets.push({
      id: it.id,
      name: it.name,
      address: it.address,
      private_key: privateKey,
      chain_type: it.chain_type,
      wallet_type: 'full_wallet',
      group_id: it.group_id ?? undefined,
      group_name: it.group_name || undefined,
      remark: it.remark || undefined,
    })
  }

  if (!fromWallets.length) return null

  return { from_wallets: fromWallets, to_addresses: toAddresses }
}

async function handleConfirmTransferPair() {
  if (!selectedFromKeys.value.length) {
    Notification.warning({ title: '未选择出账钱包', position: 'topLeft' })
    transferPairTab.value = 'from'
    return
  }
  if (!selectedToKeys.value.length) {
    Notification.warning({ title: '未选择收款地址', position: 'topLeft' })
    transferPairTab.value = 'to'
    return
  }

  confirmLoading.value = true
  let password = null
  try {
    password = await requestPassword()
    if (!password) return
    const payload = await buildTransferPairPayload(password)
    if (!payload) {
      Notification.warning({ title: '无可导入数据', position: 'topLeft' })
      return
    }
    emit('confirm', payload)
    emit('update:visible', false)
  } catch (e) {
    Message.error('导入失败: ' + (e?.message || e))
  } finally {
    confirmLoading.value = false
    passwordInput.value = ''
    password = null
  }
}
</script>

<style scoped>
.system-import-layout {
  display: flex;
  gap: 12px;
  height: 560px;
}

.left {
  width: 200px;
  border-right: 1px solid var(--color-border);
  padding-right: 10px;
  overflow: auto;
}

.right {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.filters {
  display: flex;
  gap: 10px;
  margin-bottom: 10px;
}

.list {
  flex: 1;
  min-height: 0;
}

.footer-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 10px;
}

.left-info {
  color: var(--color-text-2);
}

.right-actions {
  display: flex;
  gap: 8px;
}

.transfer-pair-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.transfer-pair-tabs :deep(.arco-tabs-content) {
  flex: 1;
  min-height: 0;
}

.transfer-pair-tabs :deep(.arco-tabs-content-list) {
  height: 100%;
}

.transfer-pair-tabs :deep(.arco-tabs-pane) {
  height: 100%;
}
</style>
