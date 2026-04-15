<template>
  <a-modal
    v-model:visible="modalVisible"
    title="代理配置"
    width="580px"
    :mask-closable="true"
    :closable="true"
    :keyboard="true"
    ok-text="保存"
    cancel-text="取消"
    @ok="handleSave"
    @cancel="handleCancel"
  >
    <div class="proxy-config-modal">
      <div class="config-header">
        <span class="label">启用代理服务</span>
        <a-switch v-model="proxyConfig.enabled" checked-text="开" unchecked-text="关" />
      </div>

      <div class="config-content">
        <div class="section-title">
          <span>代理列表</span>
          <span class="proxy-count" v-if="validProxiesCount > 0">
            (共 {{ validProxiesCount }} 个)
          </span>
        </div>
        <a-textarea
          v-model="proxyListText"
          placeholder="请输入代理地址，每行一个
格式示例：
http://127.0.0.1:7890
socks5://user:pass@127.0.0.1:1080"
          :auto-size="{ minRows: 8, maxRows: 12 }"
          class="proxy-textarea"
          @input="updateProxyCount"
          @change="updateProxyCount"
          @blur="updateProxyCount"
        />
        <div class="input-tip" v-if="totalLines > 0">
          已识别 {{ totalLines }} 行 <span v-if="filteredLinesCount > 0"> (过滤 {{ filteredLinesCount }} 行空白/注释)</span>
        </div>
      </div>

      <div class="test-section" v-if="proxyConfig.enabled && proxyConfig.proxies.length > 0">
        <div class="test-header">
          <div class="test-controls">
             <a-button 
              :type="testing ? 'secondary' : 'primary'" 
              status="success"
              size="small"
              @click="testing ? cancelTesting() : testAllProxies()"
            >
              <template #icon>
                <icon-stop v-if="testing" />
                <icon-play-arrow v-else />
              </template>
              {{ testing ? '停止测试' : '测试连接' }}
            </a-button>
            <div v-if="testing || testResults.length > 0" class="test-summary">
              <span class="summary-item success"><icon-check-circle /> {{ successCount }}</span>
              <span class="summary-item failure"><icon-close-circle /> {{ failureCount }}</span>
              <span class="summary-item time" v-if="!testing && testDuration > 0">耗时: {{ formatDuration(testDuration) }}</span>
            </div>
          </div>
          <div class="test-progress-bar" v-if="testing">
             <a-progress 
              :percent="testProgress" 
              size="small" 
              :show-text="false"
              :color="{ '0%': 'rgb(var(--primary-6))', '100%': 'rgb(var(--success-6))' }"
            />
          </div>
        </div>

        <div class="test-results-list" v-if="testResults.length > 0">
          <div 
            v-for="result in testResults" 
            :key="result.proxy"
            class="result-row"
          >
            <div class="result-proxy" :title="result.proxy">{{ result.proxy }}</div>
            <div class="result-status">
              <span v-if="result.success" class="status-success">{{ result.latency }} ms</span>
              <span v-else class="status-failure" :title="result.error">连接失败</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </a-modal>
</template>

<script setup>
import { ref, reactive, watch, computed, onMounted } from 'vue'
import { Message } from '@arco-design/web-vue'
import { IconPlayArrow, IconStop, IconCheckCircle, IconCloseCircle } from '@arco-design/web-vue/es/icon'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  windowId: { type: String, default: '' }
})

const emit = defineEmits(['update:modelValue', 'saved', 'config-change'])

const proxyConfig = reactive({ enabled: false, proxies: [] })
const proxyListText = ref('')
const testing = ref(false)
const testResults = ref([])
const testProgress = ref(0)
const testCompletedCount = ref(0)
const testTotalCount = ref(0)
const testSpeed = ref(0)
const estimatedTime = ref(0)
const testDuration = ref(0)
const testStartTime = ref(0)
const testCancelled = ref(false)

const successCount = computed(() => testResults.value.filter(r => r.success).length)
const failureCount = computed(() => testResults.value.filter(r => !r.success).length)
const totalLines = computed(() => proxyListText.value ? proxyListText.value.split('\n').length : 0)
const validProxiesCount = computed(() => proxyConfig.proxies.length)
const filteredLinesCount = computed(() => {
  if (!proxyListText.value) return 0
  return proxyListText.value.split('\n').filter(line => {
    const trimmed = line.trim()
    return !trimmed || trimmed.startsWith('#')
  }).length
})

const modalVisible = computed({
  get() { return Boolean(props.modelValue) },
  set(value) { emit('update:modelValue', value) }
})

watch(() => props.modelValue, (newVal) => {
  if (newVal) {
    loadProxyConfig()
  }
}, { immediate: true })

watch(() => props.windowId, () => {
  if (props.modelValue) {
    loadProxyConfig()
  }
}, { immediate: true })

watch(proxyListText, () => updateProxyCount())

async function loadProxyConfig() {
  if (!props.windowId) {
    console.warn('[ProxyConfigModal] windowId 为空，跳过加载')
    return
  }
  try {
    const config = await invoke('get_proxy_config_for_window', { windowId: props.windowId })
    const enabled = Boolean(config.enabled)
    const proxies = Array.isArray(config.proxies) ? config.proxies : []
    proxyConfig.enabled = enabled
    proxyConfig.proxies = proxies
    proxyListText.value = proxies.join('\n')
    console.log('[ProxyConfigModal] 加载配置成功:', { windowId: props.windowId, enabled, proxyCount: proxies.length })
  } catch (error) {
    console.error('[ProxyConfigModal] 加载配置失败:', error)
    proxyConfig.enabled = false
    proxyConfig.proxies = []
    proxyListText.value = ''
    Message.error('加载代理配置失败: ' + error)
  }
}

async function handleSave() {
  if (!props.windowId) {
    Message.error('窗口ID未初始化，无法保存代理配置')
    return
  }
  try {
    await invoke('save_proxy_config_for_window', {
      windowId: props.windowId,
      proxies: proxyConfig.proxies,
      enabled: proxyConfig.enabled
    })
    Message.success('代理配置保存成功')
    emit('saved', { enabled: proxyConfig.enabled, proxies: proxyConfig.proxies })
    emit('config-change', { enabled: proxyConfig.enabled, proxies: proxyConfig.proxies })
    emit('update:modelValue', false)
    console.log('[ProxyConfigModal] 保存配置:', { windowId: props.windowId, enabled: proxyConfig.enabled, proxyCount: proxyConfig.proxies.length })
  } catch (error) {
    console.error('[ProxyConfigModal] 保存配置失败:', error)
    Message.error('保存代理配置失败: ' + error)
  }
}

function handleCancel() {
  testResults.value = []
  testing.value = false
  emit('update:modelValue', false)
}

function updateProxyCount() {
  if (proxyListText.value) {
    proxyConfig.proxies = proxyListText.value.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('#'))
  } else {
    proxyConfig.proxies = []
  }
}

async function testAllProxies() {
  if (proxyConfig.proxies.length === 0) {
    Message.warning('请先添加代理地址')
    return
  }
  testing.value = true
  testCancelled.value = false
  testResults.value = []
  testProgress.value = 0
  testCompletedCount.value = 0
  testTotalCount.value = proxyConfig.proxies.length
  testSpeed.value = 0
  estimatedTime.value = 0
  testStartTime.value = Date.now()

  const batchSize = 50
  const proxies = [...proxyConfig.proxies]
  const progressTimer = setInterval(() => updateTestProgress(), 100)

  try {
    for (let i = 0; i < proxies.length; i += batchSize) {
      if (testCancelled.value) break
      const batch = proxies.slice(i, i + batchSize)
      const batchPromises = batch.map(async (proxy) => {
        if (testCancelled.value) return null
        try {
          const result = await invoke('test_proxy_connection', { proxyUrl: proxy })
          const testResult = { proxy, success: result.success, latency: result.latency, error: result.error }
          testResults.value.push(testResult)
          testCompletedCount.value++
          return testResult
        } catch (error) {
          const testResult = { proxy, success: false, error: error.toString() }
          testResults.value.push(testResult)
          testCompletedCount.value++
          return testResult
        }
      })
      await Promise.all(batchPromises)
    }
    testDuration.value = Date.now() - testStartTime.value
    if (testCancelled.value) {
      Message.warning(`测试已取消，已完成 ${testCompletedCount.value}/${testTotalCount.value} 个代理的测试`)
    } else {
      Message.success(`测试完成: ${successCount.value} 成功, ${failureCount.value} 失败`)
    }
  } catch (error) {
    console.error('代理测试失败:', error)
    Message.error('代理测试失败: ' + error)
  } finally {
    testing.value = false
    testCancelled.value = false
    clearInterval(progressTimer)
    updateTestProgress()
  }
}

function cancelTesting() {
  testCancelled.value = true
}

function updateTestProgress() {
  if (testTotalCount.value > 0) {
    testProgress.value = Math.round((testCompletedCount.value / testTotalCount.value) * 100)
  }
  const elapsedTime = (Date.now() - testStartTime.value) / 1000
  if (elapsedTime > 0 && testCompletedCount.value > 0) {
    testSpeed.value = testCompletedCount.value / elapsedTime
    const remainingCount = testTotalCount.value - testCompletedCount.value
    estimatedTime.value = testSpeed.value > 0 && remainingCount > 0 ? remainingCount / testSpeed.value : 0
  }
}

function formatDuration(milliseconds) {
  if (!milliseconds || milliseconds < 0) return '0秒'
  const seconds = Math.floor(milliseconds / 1000)
  const minutes = Math.floor(seconds / 60)
  return minutes > 0 ? `${minutes}分${seconds % 60}秒` : `${seconds}秒`
}

</script>

<style scoped>
.proxy-config-modal { padding: 8px 0; }
.config-header { display: flex; justify-content: space-between; align-items: center; padding: 0 4px 16px; border-bottom: 1px solid var(--color-border); margin-bottom: 16px; }
.label { font-size: 14px; font-weight: 500; color: var(--color-text-1); }
.section-title { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; font-weight: 500; color: var(--color-text-1); }
.proxy-count { font-weight: normal; font-size: 12px; color: var(--color-text-3); }
.proxy-textarea { font-family: 'Consolas', monospace; font-size: 13px; line-height: 1.5; background-color: var(--color-fill-1); }
.input-tip { margin-top: 6px; font-size: 12px; color: var(--color-text-4); text-align: right; }
.test-section { margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--color-border); }
.test-header { display: flex; flex-direction: column; gap: 12px; margin-bottom: 12px; }
.test-controls { display: flex; justify-content: space-between; align-items: center; }
.test-summary { display: flex; gap: 12px; font-size: 13px; align-items: center; }
.summary-item { display: flex; align-items: center; gap: 4px; }
.summary-item.success { color: rgb(var(--green-6)); }
.summary-item.failure { color: rgb(var(--red-6)); }
.summary-item.time { color: var(--color-text-3); font-size: 12px; }
.test-results-list { max-height: 180px; overflow-y: auto; border: 1px solid var(--color-border); border-radius: 4px; padding: 4px 0; }
.result-row { display: flex; justify-content: space-between; padding: 6px 12px; font-size: 12px; border-bottom: 1px solid var(--color-fill-2); }
.result-row:last-child { border-bottom: none; }
.result-proxy { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--color-text-2); margin-right: 12px; font-family: 'Consolas', monospace; }
.status-success { color: rgb(var(--green-6)); font-weight: 500; }
.status-failure { color: rgb(var(--red-6)); }
</style>
