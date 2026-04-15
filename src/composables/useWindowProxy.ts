import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

export interface ProxyConfig {
  enabled: boolean
  proxies: string[]
}

export function useWindowProxy(ecosystem: string) {
  const currentWindowId = ref('')
  const proxyConfigVisible = ref(false)
  const proxyEnabled = ref(false)
  const proxyStatus = ref('未配置')
  const proxyCount = ref(0)

  const getScopedWindowId = (label: string) => `${label}_${ecosystem}`

  const proxyStatusColor = computed(() => {
    switch (proxyStatus.value) {
      case '已配置': return '#00b42a'
      case '连接中': return '#ff7d00'
      case '已连接': return '#00b42a'
      case '连接失败': return '#f53f3f'
      default: return '#86909c'
    }
  })

  function openProxyConfig() {
    proxyConfigVisible.value = true
  }

  function handleProxyConfigChange(config: ProxyConfig) {
    proxyEnabled.value = config.enabled
    proxyCount.value = config.proxies ? config.proxies.length : 0
    proxyStatus.value = config.enabled && proxyCount.value > 0 ? '已配置' : '未配置'
  }

  async function initProxyStatus() {
    try {
      const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__
      if (!isTauri) return

      const win = await getCurrentWindow()
      const label = win.label
      const scopedWindowId = getScopedWindowId(label)
      currentWindowId.value = scopedWindowId

      // 后端全局窗口标签：SolanaProvider / EthereumProvider 的 get_random_proxy_client 依赖此值查找代理
      await invoke('set_proxy_window_id', { windowId: scopedWindowId })

      const config: ProxyConfig = await invoke('get_proxy_config_for_window', { windowId: scopedWindowId })

      handleProxyConfigChange(config)
      console.log('[useWindowProxy] 初始化完成:', { windowId: scopedWindowId, enabled: config.enabled, proxyCount: config.proxies?.length || 0 })
    } catch (error) {
      console.error('[useWindowProxy] 初始化代理状态失败:', error)
    }
  }

  async function cleanupProxyConfig() {
    try {
      const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__
      if (!isTauri || !currentWindowId.value) return
      await invoke('clear_proxy_config_for_window', { windowId: currentWindowId.value })
      console.log('[useWindowProxy] 清理完成:', currentWindowId.value)
    } catch (e) {
      console.error('[useWindowProxy] 清理代理配置失败:', e)
    }
  }

  async function copyProxyConfigToWindow(newWindowId: string, newWindowLabel: string, sourceWindowLabel: string) {
    try {
      const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__
      if (!isTauri) return

      const sourceId = getScopedWindowId(sourceWindowLabel)
      let configToCopy: ProxyConfig = { enabled: false, proxies: [] }
      try {
        configToCopy = await invoke('get_proxy_config_for_window', { windowId: sourceId })
      } catch (e) {
        console.error('[useWindowProxy] 读取源窗口代理配置失败:', e)
      }

      await invoke('save_proxy_config_for_window', {
        windowId: newWindowId,
        proxies: configToCopy.proxies || [],
        enabled: configToCopy.enabled || false
      })

      console.log('[useWindowProxy] 复制代理配置完成:', { from: sourceId, to: newWindowId })
    } catch (error) {
      console.error('[useWindowProxy] 复制代理配置失败:', error)
    }
  }

  return {
    currentWindowId,
    proxyConfigVisible,
    proxyEnabled,
    proxyStatus,
    proxyCount,
    proxyStatusColor,
    getScopedWindowId,
    openProxyConfig,
    handleProxyConfigChange,
    initProxyStatus,
    cleanupProxyConfig,
    copyProxyConfigToWindow
  }
}
