<template>
  <!-- RPC管理弹窗 -->
  <a-modal v-model:visible="visible" title="RPC管理" :width="1000" @cancel="closeManage" :footer="false">
    <div class="rpc-header">
      <div class="header-left">
        <h3 class="modal-title">{{ chainName }} - RPC节点列表</h3>
        <span class="rpc-count">共 {{ rpcManageData.length }} 个节点</span>
      </div>
      <div class="header-right">
        <div class="header-actions">
          <a-tooltip content="启用所有RPC节点">
            <a-button type="outline" status="success" @click="batchEnableAllRpc">
              <template #icon><icon-check-circle /></template>
              全部启用
            </a-button>
          </a-tooltip>
          <a-tooltip content="禁用所有RPC节点">
            <a-button type="outline" status="warning" @click="batchDisableAllRpc">
              <template #icon><icon-close-circle /></template>
              全部禁用
            </a-button>
          </a-tooltip>
          <a-button type="outline" @click="batchTestAllRpc" :loading="batchTesting">
            <template #icon><icon-thunderbolt /></template>
            {{ batchTesting ? '测试中...' : '批量测试' }}
          </a-button>
          <a-button type="primary" @click="showAddRpc">
            <template #icon><icon-plus /></template>
            添加节点
          </a-button>
        </div>
      </div>
    </div>

    <a-table 
      :data="rpcManageData" 
      :loading="rpcTableLoading" 
      :pagination="false" 
      :scroll="{ y: 450 }"
      :bordered="{ cell: true }"
      size="medium"
    >
      <template #columns>
        <a-table-column title="序号" :width="70" align="center">
          <template #cell="{ rowIndex }">{{ rowIndex + 1 }}</template>
        </a-table-column>
        <a-table-column title="RPC地址" data-index="rpc_url" :ellipsis="true" :tooltip="true" />
        <a-table-column title="优先级" data-index="priority" :width="80" align="center" />
        <a-table-column title="状态" :width="80" align="center">
          <template #cell="{ record }">
            <a-switch 
              v-model="record.is_active" 
              size="small"
              :before-change="() => handleSwitchBeforeChange(record)"
            />
          </template>
        </a-table-column>
        <a-table-column title="响应时间" data-index="response_time" :width="100" align="center">
          <template #cell="{ record }">
            <span :class="getResponseTimeClass(record.response_time)">
              {{ (record.response_time !== null && record.response_time !== undefined) ? `${record.response_time} ms` : '-' }}
            </span>
          </template>
        </a-table-column>
        <a-table-column title="成功率" data-index="success_rate" :width="90" align="center">
          <template #cell="{ record }">
            <span :class="getSuccessRateClass(record.success_rate)">
              {{ (record.success_rate !== null && record.success_rate !== undefined) ? `${record.success_rate.toFixed(0)}%` : '-' }}
            </span>
          </template>
        </a-table-column>
        <a-table-column title="操作" :width="140" align="center">
          <template #cell="{ record }">
            <a-space>
              <a-tooltip content="测试连接">
                <a-button type="text" @click="testRpcConnection(record)" :loading="isRecordTesting(record)">
                  <template #icon><icon-play-arrow :style="{ fontSize: '18px' }" /></template>
                </a-button>
              </a-tooltip>
              <a-tooltip content="编辑">
                <a-button type="text" @click="showEditRpc(record)">
                  <template #icon><icon-edit :style="{ fontSize: '18px' }" /></template>
                </a-button>
              </a-tooltip>
              <a-popconfirm content="确定要删除这个RPC节点吗？" @ok="deleteRpcFromManage(record.id)">
                <a-tooltip content="删除">
                  <a-button type="text" status="danger">
                    <template #icon><icon-delete :style="{ fontSize: '18px' }" /></template>
                  </a-button>
                </a-tooltip>
              </a-popconfirm>
            </a-space>
          </template>
        </a-table-column>
      </template>
    </a-table>
  </a-modal>

  <!-- 添加/编辑RPC弹窗 -->
  <a-modal 
    v-model:visible="rpcFormVisible" 
    :title="isRpcEditMode ? '编辑RPC' : '添加RPC'" 
    :width="600"
    @cancel="() => rpcFormVisible = false" 
    :on-before-ok="submitRpcForm"
  >
    <div v-if="!isRpcEditMode" class="mode-tabs">
      <a-tabs v-model:active-key="rpcInputMode" type="rounded" size="medium">
        <a-tab-pane key="single" title="单个添加"></a-tab-pane>
        <a-tab-pane key="batch" title="批量添加"></a-tab-pane>
      </a-tabs>
    </div>

    <a-form :model="rpcForm" layout="vertical" class="rpc-form">
      <!-- 单个添加模式 -->
      <div v-if="rpcInputMode === 'single'">
        <a-form-item label="RPC地址" required>
          <a-input v-model="rpcForm.rpc_url" placeholder="例如：https://mainnet.infura.io/v3/your-key" allow-clear />
          <template #help>必须以 http:// 或 https:// 开头</template>
        </a-form-item>

        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item label="优先级" required>
              <a-input-number v-model="rpcForm.priority" :min="1" :max="999" placeholder="100" />
              <template #help>数值越大优先级越高 (1-999)</template>
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item label="状态">
              <a-switch v-model="rpcForm.is_active" checked-text="启用" unchecked-text="禁用" />
            </a-form-item>
          </a-col>
        </a-row>
      </div>
      
      <!-- 批量添加模式 -->
      <div v-if="rpcInputMode === 'batch'">
        <a-form-item label="批量RPC地址" required>
          <a-textarea
            v-model="batchRpcText"
            class="batch-rpc-textarea"
            placeholder="每行一个RPC地址，系统将自动校验和去重"
            :auto-size="{ minRows: 8, maxRows: 12 }"
            @paste="handleBatchRpcPaste"
          />
        </a-form-item>
        
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item label="默认优先级">
              <a-input-number v-model="batchDefaultPriority" :min="1" :max="999" placeholder="100" />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item label="默认状态">
              <a-switch v-model="batchDefaultActive" checked-text="启用" unchecked-text="禁用" />
            </a-form-item>
          </a-col>
        </a-row>
        
        <!-- 统计信息 -->
        <div v-if="batchRpcStats.total > 0" class="batch-stats">
          <div class="stat-item total">总计: {{ batchRpcStats.total }}</div>
          <div class="stat-item valid">有效: {{ batchRpcStats.valid }}</div>
          <div class="stat-item invalid" v-if="batchRpcStats.invalid > 0">无效: {{ batchRpcStats.invalid }}</div>
          <div class="stat-item duplicate" v-if="batchRpcStats.duplicate > 0">重复: {{ batchRpcStats.duplicate }}</div>
        </div>

        <!-- 校验结果显示 -->
        <div v-if="batchRpcValidation.length > 0 && (batchRpcStats.invalid > 0 || batchRpcStats.duplicate > 0)" class="validation-results">
          <div v-for="(item, index) in batchRpcValidation" :key="index" class="validation-item" :class="{ 'error': !item.valid }">
             <span v-if="!item.valid" class="validation-msg">{{ item.url }} - {{ item.message }}</span>
          </div>
        </div>
      </div>
    </a-form>
  </a-modal>
</template>

<script setup>
import { ref, reactive, watch, computed } from 'vue'
import { IconPlus, IconDelete, IconThunderbolt, IconEdit, IconPlayArrow, IconCheckCircle, IconCloseCircle, IconStop, IconPlayCircle } from '@arco-design/web-vue/es/icon'
import { Notification } from '@arco-design/web-vue'
import { invoke } from '@tauri-apps/api/core'

// Props
const props = defineProps({
  chainValue: {
    type: String,
    default: ''
  },
  chainOptions: {
    type: Array,
    default: () => []
  }
})

// Emits
const emit = defineEmits(['rpc-updated'])

// 响应式数据
const visible = ref(false)
const rpcFormVisible = ref(false)
const isRpcEditMode = ref(false)
const rpcManageData = ref([])
const currentEditRpc = ref(null)
const rpcTableLoading = ref(false)
const batchTesting = ref(false)

// 批量添加RPC相关
const rpcInputMode = ref('single')
const batchRpcText = ref('')
const batchDefaultPriority = ref(100)
const batchDefaultActive = ref(true)
const batchRpcValidation = ref([])
const batchRpcStats = reactive({
  total: 0,
  valid: 0,
  invalid: 0,
  duplicate: 0
})

// RPC信息表单
const rpcForm = reactive({
  rpc_url: '',
  priority: 100,
  is_active: true
})

// 计算属性
const chainName = computed(() => {
  const chain = props.chainOptions.find(c => c.key === props.chainValue)
  return chain?.name || '当前区块链'
})

// 监听 visible 变化
watch(visible, (newVal) => {
  if (newVal && props.chainValue) {
    loadRpcData()
  }
})

// 监听链变化
watch(() => props.chainValue, (newVal) => {
  if (newVal && visible.value) {
    loadRpcData()
  }
})

// 辅助函数：样式类
function getResponseTimeClass(time) {
  // 如果 time 是 null, undefined 或 0 (0可能表示异常或未测试，但如果是真0ms也不太可能)
  // 严格判断 null 或 undefined
  if (time === null || time === undefined) return 'text-gray'
  if (time <= 200) return 'text-success'
  if (time <= 500) return 'text-warning'
  return 'text-danger'
}

function getSuccessRateClass(rate) {
  if (rate === undefined || rate === null) return 'text-gray'
  if (rate >= 98) return 'text-success'
  if (rate >= 90) return 'text-warning'
  return 'text-danger'
}

function isRecordTesting(record) {
  const index = rpcManageData.value.findIndex(item => item.id === record.id)
  if (index === -1) return false
  return rpcManageData.value[index].testing
}

// 加载RPC数据
async function loadRpcData() {
  if (!props.chainValue) return
  
  try {
    rpcTableLoading.value = true
    const result = await invoke('get_rpc_providers', { chainKey: props.chainValue })
    // 排序：先按启用状态排序（启用在前），再按RPC地址排序
    const sortedData = sortRpcData(result || [])
    // 初始化 UI 状态字段
    rpcManageData.value = sortedData.map(item => ({
      ...item,
      testing: false
    }))
  } catch (error) {
    console.error('加载RPC数据失败:', error)
    Notification.error({ content: '加载RPC数据失败: ' + (error.message || error), position: 'topLeft' })
  } finally {
    rpcTableLoading.value = false
  }
}

// 显示添加RPC弹窗
function showAddRpc() {
  isRpcEditMode.value = false
  currentEditRpc.value = null
  resetRpcForm()
  rpcFormVisible.value = true
}

// 显示编辑RPC弹窗
function showEditRpc(record) {
  isRpcEditMode.value = true
  currentEditRpc.value = record
  
  // 填充表单数据
  Object.assign(rpcForm, {
    rpc_url: record.rpc_url || '',
    priority: record.priority || 100,
    is_active: record.is_active !== undefined ? record.is_active : true
  })
  
  // 强制切回单条模式
  rpcInputMode.value = 'single'
  rpcFormVisible.value = true
}

// 重置RPC表单
function resetRpcForm() {
  Object.assign(rpcForm, {
    rpc_url: '',
    priority: 100,
    is_active: true
  })
  
  // 重置批量模式相关数据
  rpcInputMode.value = 'single'
  batchRpcText.value = ''
  batchDefaultPriority.value = 100
  batchDefaultActive.value = true
  batchRpcValidation.value = []
  resetBatchRpcStats()
}

// 重置批量RPC统计
function resetBatchRpcStats() {
  Object.assign(batchRpcStats, {
    total: 0,
    valid: 0,
    invalid: 0,
    duplicate: 0
  })
}

// URL标准化函数
function normalizeUrl(url) {
  try {
    const urlObj = new URL(url.trim())
    // 移除末尾斜杠，转换为小写
    return urlObj.href.toLowerCase().replace(/\/$/, '')
  } catch {
    return url.trim().toLowerCase().replace(/\/$/, '')
  }
}

// 实时校验批量RPC输入
function validateBatchRpcInput() {
  if (!batchRpcText.value.trim()) {
    batchRpcValidation.value = []
    resetBatchRpcStats()
    return
  }
  
  // 解析输入的RPC地址
  const inputUrls = batchRpcText.value
    .split('\n')
    .map(url => url.trim())
    .filter(url => url.length > 0)
  
  // 获取现有RPC地址并标准化用于去重比较
  const existingNormalizedUrls = new Set(
    rpcManageData.value.map(rpc => normalizeUrl(rpc.rpc_url))
  )
  
  // 校验和统计
  const validation = []
  const seenNormalizedUrls = new Set()
  
  resetBatchRpcStats()
  batchRpcStats.total = inputUrls.length
  
  inputUrls.forEach(url => {
    const result = validateRpcUrl(url)
    const normalizedUrl = normalizeUrl(url)
    
    if (!result.valid) {
      validation.push({ url, valid: false, message: result.message })
      batchRpcStats.invalid++
    } else if (existingNormalizedUrls.has(normalizedUrl)) {
      validation.push({ url, valid: false, message: '重复地址（已存在）' })
      batchRpcStats.duplicate++
    } else if (seenNormalizedUrls.has(normalizedUrl)) {
      validation.push({ url, valid: false, message: '重复地址（输入重复）' })
      batchRpcStats.duplicate++
    } else {
      validation.push({ url, valid: true, message: result.message })
      seenNormalizedUrls.add(normalizedUrl)
      batchRpcStats.valid++
    }
  })
  
  batchRpcValidation.value = validation
}

// 监听批量RPC文本变化
watch(batchRpcText, () => {
  validateBatchRpcInput()
}, { immediate: true })

// 处理批量RPC粘贴事件，自动识别URL并换行
function handleBatchRpcPaste(event) {
  event.preventDefault()
  const clipboardData = event.clipboardData || window.clipboardData
  const pastedText = clipboardData.getData('text')

  // 由于URL可能直接连接在一起，需要特殊处理
  // 使用前瞻来正确分割URL：匹配URL直到遇到下一个协议开头或文本结束
  const urlPattern = /(?:https?|wss?):\/\/(?:[a-zA-Z0-9][-a-zA-Z0-9]*\.)+[a-zA-Z0-9][-a-zA-Z0-9]*(?=\s*(?:https?|wss?):\/\/|$)/g
  const urls = pastedText.match(urlPattern)

  if (urls && urls.length > 0) {
    // 获取当前文本，如果非空则添加换行
    const currentText = batchRpcText.value.trim()
    const separator = currentText ? '\n' : ''
    batchRpcText.value = currentText + separator + urls.join('\n')
  } else {
    // 如果没有识别到URL，直接插入粘贴的文本
    const textarea = event.target
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const currentText = batchRpcText.value
    const newText = currentText.slice(0, start) + pastedText + currentText.slice(end)
    batchRpcText.value = newText
  }
}

// RPC数据排序函数
function sortRpcData(data) {
  return data.sort((a, b) => {
    // 首先按is_active降序排序（true在前，false在后）
    if (a.is_active !== b.is_active) {
      return b.is_active - a.is_active
    }
    // 在相同启用状态内，按rpc_url升序排序
    return a.rpc_url.localeCompare(b.rpc_url)
  })
}

// 提交RPC表单
async function submitRpcForm() {
  try {
    if (rpcInputMode.value === 'batch' && !isRpcEditMode.value) {
      return await submitBatchRpcs()
    } else {
      return await submitSingleRpc()
    }
  } catch (error) {
    console.error('保存RPC信息失败:', error)
    Notification.error({ content: '保存RPC信息失败: ' + error.message, position: 'topLeft' })
    return false
  }
}

// 提交单个RPC
async function submitSingleRpc() {
  // 验证必填字段
  if (!rpcForm.rpc_url) {
    Notification.warning({ content: '请输入RPC地址', position: 'topLeft' })
    return false
  }

  // 验证RPC地址格式
  if (!rpcForm.rpc_url.startsWith('http://') && !rpcForm.rpc_url.startsWith('https://')) {
    Notification.warning({ content: 'RPC地址必须以http://或https://开头', position: 'topLeft' })
    return false
  }

  const rpcData = {
    chain_key: props.chainValue,
    rpc_url: rpcForm.rpc_url,
    priority: rpcForm.priority,
    is_active: rpcForm.is_active
  }

  if (isRpcEditMode.value) {
    await invoke('update_rpc_provider', {
      id: currentEditRpc.value.id,
      request: {
        rpc_url: rpcData.rpc_url,
        priority: rpcData.priority,
        is_active: rpcData.is_active
      }
    })
    Notification.success({ content: 'RPC信息更新成功', position: 'topLeft' })
  } else {
    await invoke('add_rpc_provider', {
      chainKey: rpcData.chain_key,
      rpcUrl: rpcData.rpc_url,
      priority: rpcData.priority
    })
    Notification.success({ content: 'RPC添加成功', position: 'topLeft' })
  }

  rpcFormVisible.value = false
  await loadRpcData()
  return true
}

// 校验RPC地址
function validateRpcUrl(url) {
  const trimmedUrl = url.trim()
  if (!trimmedUrl) {
    return { valid: false, message: '地址为空' }
  }
  
  // 检查是否为有效的URL格式
  try {
    new URL(trimmedUrl)
  } catch {
    return { valid: false, message: '无效的URL格式' }
  }
  
  // 检查是否以http或https开头
  if (!trimmedUrl.startsWith('http://') && !trimmedUrl.startsWith('https://')) {
    return { valid: false, message: '必须以http://或https://开头' }
  }
  
  // 建议使用https
  if (trimmedUrl.startsWith('http://')) {
    return { valid: true, message: '有效（建议使用https）' }
  }
  
  return { valid: true, message: '有效' }
}

// 提交批量RPC
async function submitBatchRpcs() {
  if (!batchRpcText.value.trim()) {
    Notification.warning({ content: '请输入RPC地址', position: 'topLeft' })
    return false
  }

  // 解析输入的RPC地址
  const inputUrls = batchRpcText.value
    .split('\n')
    .map(url => url.trim())
    .filter(url => url.length > 0)

  if (inputUrls.length === 0) {
    Notification.warning({ content: '请输入有效的RPC地址', position: 'topLeft' })
    return false
  }
  
  // 获取现有RPC地址并标准化用于去重比较
  const existingNormalizedUrls = new Set(
    rpcManageData.value.map(rpc => normalizeUrl(rpc.rpc_url))
  )
  
  // 校验和统计
  const validation = []
  const validUrls = []
  const seenNormalizedUrls = new Set()
  
  resetBatchRpcStats()
  batchRpcStats.total = inputUrls.length
  
  inputUrls.forEach(url => {
    const result = validateRpcUrl(url)
    const normalizedUrl = normalizeUrl(url)
    
    if (!result.valid) {
      validation.push({ url, valid: false, message: result.message })
      batchRpcStats.invalid++
    } else if (existingNormalizedUrls.has(normalizedUrl)) {
      validation.push({ url, valid: false, message: '重复地址（已存在）' })
      batchRpcStats.duplicate++
    } else if (seenNormalizedUrls.has(normalizedUrl)) {
      validation.push({ url, valid: false, message: '重复地址（输入重复）' })
      batchRpcStats.duplicate++
    } else {
      validation.push({ url, valid: true, message: result.message })
      validUrls.push(url)
      seenNormalizedUrls.add(normalizedUrl)
      existingNormalizedUrls.add(normalizedUrl) // 防止后续重复
      batchRpcStats.valid++
    }
  })
  
  batchRpcValidation.value = validation
  
  // 如果没有有效的URL，不关闭弹窗，让用户修改
  if (validUrls.length === 0) {
    Notification.warning({ content: '没有有效的RPC地址可以添加', position: 'topLeft' })
    return false
  }
  
  // 批量添加有效的RPC
  let successCount = 0
  let failCount = 0
  
  for (const url of validUrls) {
    try {
      await invoke('add_rpc_provider', {
        chainKey: props.chainValue,
        rpcUrl: url,
        priority: batchDefaultPriority.value
      })
      successCount++
    } catch (error) {
      console.error(`添加RPC失败: ${url}`, error)
      failCount++
    }
  }
  
  // 显示结果
  if (successCount > 0) {
    Notification.success({ content: `成功添加 ${successCount} 个RPC地址${failCount > 0 ? `，失败 ${failCount} 个` : ''}`, position: 'topLeft' })
  }

  if (failCount > 0 && successCount === 0) {
    Notification.error({ content: `添加失败，请检查网络连接或RPC地址`, position: 'topLeft' })
    return false
  }
  
  // 刷新数据并关闭弹窗
  await loadRpcData()
  rpcFormVisible.value = false
  return true
}

// 测试RPC连接
async function testRpcConnection(record) {
  if (!record || !record.rpc_url) return

  // 查找最新的记录索引，确保响应式更新
  const index = rpcManageData.value.findIndex(item => item.id === record.id)
  if (index === -1) return

  try {
    // 设置测试状态 - 使用 Vue 响应式更新方式
    rpcManageData.value[index] = { ...rpcManageData.value[index], testing: true }

    console.log('开始测试RPC连接:', record.rpc_url)

    let result;
    // 判断是否为Solana链，优先使用ecosystem字段判断
    const currentChain = props.chainOptions.find(c => c.key === props.chainValue);
    const isSolana = currentChain?.ecosystem === 'solana' || (props.chainValue && props.chainValue.toLowerCase().includes('sol'));

    if (isSolana) {
      result = await invoke('test_solana_rpc_connection', { rpcUrl: record.rpc_url });
    } else {
      result = await invoke('test_rpc_connection', { rpcUrl: record.rpc_url, chainKey: props.chainValue });
    }

    console.log('RPC测试结果:', result)

    if (result.success) {
      Notification.success({ content: `测试成功: ${result.response_time_ms}ms`, position: 'topLeft' })
      // 更新响应时间 - 使用 Vue 响应式更新方式
      rpcManageData.value[index] = {
        ...rpcManageData.value[index],
        response_time: result.response_time_ms,
        success_rate: 100,
        testing: false
      }
    } else {
      Notification.error({ content: `测试失败`, position: 'topLeft' })
      rpcManageData.value[index] = {
        ...rpcManageData.value[index],
        response_time: result.response_time_ms,
        success_rate: 0,
        testing: false
      }
    }
  } catch (error) {
    console.error('RPC测试失败:', error)
    Notification.error({ content: '测试出错: ' + (error.message || error), position: 'topLeft' })
    rpcManageData.value[index] = {
      ...rpcManageData.value[index],
      response_time: null,
      success_rate: 0,
      testing: false
    }
  }
}

// 批量测试所有RPC
async function batchTestAllRpc() {
  if (rpcManageData.value.length === 0) {
    Notification.warning({ content: '没有可测试的RPC节点', position: 'topLeft' })
    return
  }

  try {
    batchTesting.value = true
    console.log('开始批量测试RPC节点，总数:', rpcManageData.value.length)

    // 并发测试所有RPC
    const testPromises = rpcManageData.value.map(async (item, index) => {
      // 跳过已经在测试中的
      if (item.testing) return

      try {
        // 使用 Vue 的响应式更新方式
        rpcManageData.value[index] = { ...rpcManageData.value[index], testing: true }
        console.log('测试RPC:', item.rpc_url)

        let result;
        const currentChain = props.chainOptions.find(c => c.key === props.chainValue);
        const isSolana = currentChain?.ecosystem === 'solana' || (props.chainValue && props.chainValue.toLowerCase().includes('sol'));

        if (isSolana) {
          result = await invoke('test_solana_rpc_connection', { rpcUrl: item.rpc_url });
        } else {
          result = await invoke('test_rpc_connection', { rpcUrl: item.rpc_url, chainKey: props.chainValue });
        }

        console.log(`[RPC测试结果] ${item.rpc_url}:`, result);

        if (result.success) {
          // 使用 Vue 的响应式更新方式替换整个对象
          rpcManageData.value[index] = {
            ...rpcManageData.value[index],
            response_time: result.response_time_ms,
            success_rate: 100,
            testing: false
          }
        } else {
          rpcManageData.value[index] = {
            ...rpcManageData.value[index],
            response_time: result.response_time_ms,
            success_rate: 0,
            testing: false
          }
        }
      } catch (error) {
        console.error('RPC测试失败:', item.rpc_url, error)
        rpcManageData.value[index] = {
          ...rpcManageData.value[index],
          response_time: null,
          testing: false
        }
      }
    })

    await Promise.all(testPromises)
    Notification.success({ content: '批量测试完成', position: 'topLeft' })
  } catch (error) {
    console.error('批量测试失败:', error)
    Notification.error({ content: '批量测试失败: ' + error.message, position: 'topLeft' })
  } finally {
    batchTesting.value = false
  }
}

// Switch 改变前的钩子，实际逻辑在 toggleRpcStatus 中处理
// 这里返回 Promise 来控制 switch 状态
async function handleSwitchBeforeChange(record) {
  try {
    // 乐观更新：先切换
    const newValue = !record.is_active
    
    // 调用后端 API
    // 注意：Tauri invoke 参数映射默认可能是驼峰转下划线，也可能需要完全匹配
    // 为了稳妥，这里使用后端定义的参数名 rpc_url
    await invoke('update_rpc_provider', {
      id: record.id,
      request: {
        rpc_url: record.rpc_url,
        priority: record.priority,
        is_active: newValue
      }
    })
    
    // 成功后，通知并保持新状态
    Notification.success({ content: `RPC已${newValue ? '启用' : '禁用'}`, position: 'topLeft' })
    
    // 注意：这里取消了 sortRpcData 的调用，避免用户点击后列表跳变
    // 如果需要更新顺序，建议在页面加载时处理，或者提供专门的排序按钮
    
    return true // 允许切换
  } catch (error) {
    console.error('切换RPC状态失败:', error)
    Notification.error({ content: '切换RPC状态失败: ' + error.message, position: 'topLeft' })
    return false // 阻止切换
  }
}

// 批量启用所有RPC
async function batchEnableAllRpc() {
  if (rpcManageData.value.length === 0) return
  
  try {
    // 并发更新
    const promises = rpcManageData.value
      .filter(item => !item.is_active)
      .map(item => invoke('update_rpc_provider', {
        id: item.id,
        request: {
          rpc_url: item.rpc_url,
          priority: item.priority,
          is_active: true
        }
      }))
      
    if (promises.length === 0) {
      Notification.info({ content: '所有节点已处于启用状态', position: 'topLeft' })
      return
    }
    
    await Promise.all(promises)
    Notification.success({ content: `已启用 ${promises.length} 个节点`, position: 'topLeft' })
    await loadRpcData()
  } catch (error) {
    console.error('批量启用失败:', error)
    Notification.error({ content: '批量启用失败', position: 'topLeft' })
  }
}

// 批量禁用所有RPC
async function batchDisableAllRpc() {
  if (rpcManageData.value.length === 0) return
  
  try {
    // 并发更新
    const promises = rpcManageData.value
      .filter(item => item.is_active)
      .map(item => invoke('update_rpc_provider', {
        id: item.id,
        request: {
          rpc_url: item.rpc_url,
          priority: item.priority,
          is_active: false
        }
      }))
      
    if (promises.length === 0) {
      Notification.info({ content: '所有节点已处于禁用状态', position: 'topLeft' })
      return
    }
    
    await Promise.all(promises)
    Notification.success({ content: `已禁用 ${promises.length} 个节点`, position: 'topLeft' })
    await loadRpcData()
  } catch (error) {
    console.error('批量禁用失败:', error)
    Notification.error({ content: '批量禁用失败', position: 'topLeft' })
  }
}

// 删除RPC
async function deleteRpcFromManage(id) {
  try {
    await invoke('delete_rpc_provider', { id })
    Notification.success({ content: 'RPC删除成功', position: 'topLeft' })
    await loadRpcData()
  } catch (error) {
    console.error('删除RPC失败:', error)
    Notification.error({ content: '删除RPC失败: ' + error.message, position: 'topLeft' })
  }
}

// 关闭管理弹窗
function closeManage() {
  visible.value = false
}

// 显示弹窗
function show() {
  visible.value = true
}

// 隐藏弹窗
function hide() {
  visible.value = false
}

// 暴露方法给父组件
defineExpose({
  show,
  hide
})
</script>

<style scoped>
.rpc-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.header-right{
  flex: 1;
}

.header-actions{
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
}

.modal-title {
  margin: 0;
  font-size: 18px;
  font-weight: 500;
  color: var(--color-text-1);
}

.rpc-count {
  font-size: 12px;
  color: var(--color-text-3);
}

.mode-tabs {
  margin-bottom: 20px;
}

.rpc-form {
  margin-top: 10px;
}

.batch-stats {
  display: flex;
  gap: 16px;
  padding: 10px 16px;
  background-color: var(--color-fill-2);
  border-radius: 4px;
  margin-top: 10px;
  font-size: 13px;
}

.stat-item {
  display: flex;
  align-items: center;
  font-weight: 500;
}

.stat-item.total { color: var(--color-text-2); }
.stat-item.valid { color: rgb(var(--green-6)); }
.stat-item.invalid { color: rgb(var(--red-6)); }
.stat-item.duplicate { color: rgb(var(--orange-6)); }

.validation-results {
  margin-top: 12px;
  max-height: 120px;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  padding: 8px;
}

.validation-item {
  font-size: 12px;
  margin-bottom: 4px;
  line-height: 1.5;
}

.validation-item.error {
  color: rgb(var(--red-6));
}

.text-success { color: rgb(var(--green-6)); }
.text-warning { color: rgb(var(--orange-6)); }
.text-danger { color: rgb(var(--red-6)); }
.text-gray { color: var(--color-text-4); }

/* 调整表格内开关和按钮的垂直对齐 */
:deep(.arco-table-cell) {
  vertical-align: middle;
}

/* 修复按钮 loading 状态导致行高抖动的问题 */
:deep(.arco-btn-text) {
  min-height: 32px;
  min-width: 32px;
  padding: 0 4px;
}

:deep(.arco-btn-text.arco-btn-loading) {
  min-height: 32px;
  min-width: 32px;
  padding: 0 4px;
}

:deep(.arco-btn-text .arco-btn-icon) {
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>