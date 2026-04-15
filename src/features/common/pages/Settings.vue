<script setup>
import { ref, computed, onMounted, nextTick } from 'vue'
import { Notification, Modal, Message } from '@arco-design/web-vue'
import { h } from 'vue'
import { useThemeStore } from '@/stores'
import { getVersion } from '@tauri-apps/api/app'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { relaunch } from '@tauri-apps/plugin-process'
import packageJson from '@/../package.json'

// 处理header拖拽
async function handleHeaderDrag(e) {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (!isTauri) return
  
  // 只响应左键
  if (e.button !== 0) return
  
  // 如果点击的是关闭按钮，不触发拖拽
  if (e.target.closest('.close-btn')) return
  
  try {
    const currentWindow = getCurrentWindow()
    await currentWindow.startDragging()
  } catch (error) {
    console.error('拖拽失败:', error)
  }
}

const themeStore = useThemeStore()
const appWindow = getCurrentWindow()

const isDarkTheme = computed(() => themeStore.getEffectiveTheme() === 'dark')

// 主题显示文本
const themeDisplayText = computed(() => {
  const theme = themeStore.currentTheme
  if (theme === 'auto') return '跟随系统'
  return theme === 'dark' ? '暗黑模式' : '明亮模式'
})

// 主题开关状态（true = dark, false = light）
const themeSwitchValue = computed({
  get: () => themeStore.getEffectiveTheme() === 'dark',
  set: (val) => {
    const newTheme = val ? 'dark' : 'light'
    if (newTheme !== themeStore.currentTheme) {
      themeStore.setTheme(newTheme)
    }
  }
})

const runtimeVersion = ref('')
const appVersion = computed(() => runtimeVersion.value || packageJson.version || '0.0.0')

const databaseLoading = ref(false)
const databaseStatus = ref(null)
const updateChecking = ref(false)
const updateInfo = ref(null)
const hasUpdate = ref(false)
const ignoredVersion = ref('')

const STORAGE_KEYS = {
  lastCheckAt: 'walletstool:update:lastCheckAt',
  ignoreVersion: 'walletstool:update:ignoreVersion',
}

// 窗口透明度设置
const windowOpacity = ref(1.0)
const opacityLoading = ref(false)

// 始终置顶设置
const alwaysOnTop = ref(false)
const alwaysOnTopLoading = ref(false)

// 从本地存储加载透明度设置
function loadOpacitySetting() {
  const saved = localStorage.getItem('mainWindowOpacity')
  if (saved) {
    windowOpacity.value = parseFloat(saved)
  }
}

// 加载置顶设置
function loadAlwaysOnTopSetting() {
  const saved = localStorage.getItem('mainWindowAlwaysOnTop')
  if (saved) {
    alwaysOnTop.value = saved === 'true'
  }
}

// 保存透明度设置到本地存储
function saveOpacitySetting(opacity) {
  localStorage.setItem('mainWindowOpacity', opacity.toString())
}

// 保存置顶设置到本地存储
function saveAlwaysOnTopSetting(value) {
  localStorage.setItem('mainWindowAlwaysOnTop', value.toString())
}

// 应用透明度到主窗口
async function applyWindowOpacity(opacity) {
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      // 发送事件通知主窗口更新透明度
      await emit('window-opacity-changed', { opacity })
    }
  } catch (error) {
    console.error('设置窗口透明度失败:', error)
  }
}

// 切换窗口置顶状态
async function toggleAlwaysOnTop() {
  try {
    alwaysOnTopLoading.value = true
    const newValue = !alwaysOnTop.value
    alwaysOnTop.value = newValue
    saveAlwaysOnTopSetting(newValue)

    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      console.log('[Settings] 发送置顶状态变化事件:', newValue)
      await emit('window-always-on-top-changed', { alwaysOnTop: newValue })
      console.log('[Settings] 置顶事件发送完成')
    }
  } catch (error) {
    console.error('切换置顶状态失败:', error)
  } finally {
    alwaysOnTopLoading.value = false
  }
}

// 处理透明度滑块变化
async function handleOpacityChange(value) {
  windowOpacity.value = value
  saveOpacitySetting(value)
  await applyWindowOpacity(value)
}

// 获取当前主窗口透明度
async function fetchCurrentOpacity() {
  try {
    opacityLoading.value = true
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      const savedOpacity = localStorage.getItem('mainWindowOpacity')
      if (savedOpacity) {
        windowOpacity.value = parseFloat(savedOpacity)
      } else {
        const opacity = await invoke('get_main_window_opacity')
        windowOpacity.value = opacity
        saveOpacitySetting(opacity)
      }
    }
  } catch (error) {
    console.error('获取窗口透明度失败:', error)
  } finally {
    opacityLoading.value = false
  }
}

async function closeWindow() {
  await appWindow.destroy()
}

async function checkDatabaseStatus() {
  try {
    databaseLoading.value = true
    let status
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      status = await invoke('check_database_schema')
    } else {
      status = {
        db_exists: true,
        chains_table_exists: true,
        tokens_table_exists: true,
        rpc_table_exists: true,
        abi_column_exists: true,
        contract_type_column_exists: true,
        needs_migration: false
      }
    }

    let statusText = ''
    let notificationType = 'success'

    if (typeof status === 'object' && status !== null) {
      if (!status.db_exists) {
        statusText = '数据库文件不存在'
        notificationType = 'error'
      } else if (!(status.chains_table_exists && status.tokens_table_exists && status.rpc_table_exists)) {
        const missingTables = []
        if (!status.chains_table_exists) missingTables.push('链表(chains)')
        if (!status.tokens_table_exists) missingTables.push('代币表(tokens)')
        if (!status.rpc_table_exists) missingTables.push('RPC表(rpc_providers)')
        statusText = `数据库缺少必要表：${missingTables.join(', ')}`
        notificationType = 'error'
      } else if (status.needs_migration) {
        statusText = '数据库需要迁移更新'
        notificationType = 'warning'
      } else {
        statusText = '数据库结构完整，运行正常'
      }
    } else {
      statusText = typeof status === 'string' ? status : JSON.stringify(status)
    }

    databaseStatus.value = statusText

    if (notificationType === 'success') {
      Notification.success({ title: '数据库状态', content: statusText, position: 'top' })
    } else if (notificationType === 'warning') {
      Notification.warning({ title: '数据库状态', content: statusText, position: 'top' })
    } else {
      Notification.error({ title: '数据库状态', content: statusText, position: 'top' })
    }
  } catch (error) {
    console.error('检查数据库状态失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    databaseStatus.value = '检查失败: ' + errorText
    Notification.error({ title: '检查失败', content: errorText, position: 'top' })
  } finally {
    databaseLoading.value = false
  }
}

async function reloadDatabase() {
  Modal.confirm({
    title: '确认恢复出厂设置',
    content: '此操作将重置数据库到初始状态，所有自定义配置将丢失。确定要继续吗？',
    okText: '确定',
    cancelText: '取消',
    width: 380,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        databaseLoading.value = true
        let result
        const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
        if (isTauri) {
          result = await invoke('reload_database')
        } else {
          result = '恢复出厂设置成功'
        }

        const resultText = typeof result === 'string' ? result : JSON.stringify(result)
        Notification.success({ title: '恢复成功', content: resultText, position: 'top' })

        setTimeout(async () => {
          await checkDatabaseStatus()
        }, 500)
      } catch (error) {
        console.error('恢复出厂设置失败:', error)
        const errorText = typeof error === 'string' ? error : error.message || '未知错误'
        Notification.error({ title: '恢复失败', content: errorText, position: 'top' })
      } finally {
        databaseLoading.value = false
      }
    }
  })
}

async function exportDatabaseToInitSql() {
  try {
    databaseLoading.value = true
    let result
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      result = await invoke('export_database_to_init_sql')
    } else {
      result = '数据库导出成功（浏览器环境模拟）'
    }

    const resultText = typeof result === 'string' ? result : JSON.stringify(result)
    Notification.success({ title: '导出成功', content: resultText, position: 'top' })
  } catch (error) {
    console.error('导出数据库失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    Notification.error({ title: '导出失败', content: errorText, position: 'top' })
  } finally {
    databaseLoading.value = false
  }
}

async function refreshPageData() {
  try {
    databaseStatus.value = null
    Notification.success({ title: '刷新成功', content: '所有状态已重置', position: 'top' })
    setTimeout(async () => {
      await checkDatabaseStatus()
    }, 300)
  } catch (error) {
    console.error('刷新页面数据失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    Notification.error({ title: '刷新失败', content: errorText, position: 'top' })
  }
}

// 获取被忽略的版本
function getIgnoredVersion() {
  return localStorage.getItem(STORAGE_KEYS.ignoreVersion) || ''
}

// 忽略当前版本
function ignoreCurrentVersion() {
  if (updateInfo.value?.latest_version) {
    localStorage.setItem(STORAGE_KEYS.ignoreVersion, updateInfo.value.latest_version)
    hasUpdate.value = false
  }
}

// 页面加载时检查是否有已保存的更新状态
onMounted(async () => {
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) runtimeVersion.value = await getVersion()
    
    // 加载透明度设置
    loadOpacitySetting()
    await fetchCurrentOpacity()
    
    // 加载置顶设置
    loadAlwaysOnTopSetting()
    
    // 检查是否有已保存的更新状态
    const savedIgnored = getIgnoredVersion()
    if (savedIgnored) {
      ignoredVersion.value = savedIgnored
    }
    
    await nextTick()
    requestAnimationFrame(() => {
      appWindow.emit('page-loaded')
    })
  } catch (error) {
    console.error('Failed to get app version:', error)
  }
})

async function checkForUpdate() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  console.log('[checkForUpdate] 开始检查更新, isTauri:', isTauri, 'appVersion:', appVersion.value)

  if (!isTauri) {
    Notification.warning({ title: '提示', content: '浏览器环境下无法检查更新', position: 'top' })
    return
  }

  try {
    updateChecking.value = true
    console.log('[checkForUpdate] 调用 invoke check_update...')

    let result = null
    let usedFallback = false

    try {
      // 首先尝试 Tauri Updater
      result = await invoke('check_update', {
        currentVersion: appVersion.value
      })
      console.log('[checkForUpdate] Tauri Updater 检查结果:', result)
    } catch (updaterError) {
      // Tauri Updater 失败，尝试 GitHub Release API（带代理回退）
      console.log('[checkForUpdate] Tauri Updater 失败，尝试 GitHub Release API:', updaterError)
      usedFallback = true

      try {
        const githubResult = await invoke('check_github_release_update', {
          owner: 'WalletsTool',
          repo: 'WalletsTool',
          currentVersion: appVersion.value
        })

        if (githubResult) {
          result = {
            has_update: true,
            current_version: githubResult.current_version,
            latest_version: githubResult.latest_version,
            release_notes: githubResult.body,
            download_url: githubResult.html_url,
            installer_url: githubResult.installer_url,
            published_at: githubResult.published_at
          }
        } else {
          result = {
            has_update: false,
            current_version: appVersion.value,
            latest_version: appVersion.value,
            release_notes: null,
            download_url: null,
            published_at: null
          }
        }
        console.log('[checkForUpdate] GitHub Release API 检查结果:', result)
      } catch (githubError) {
        console.error('[checkForUpdate] GitHub Release API 也失败:', githubError)
        throw new Error('所有更新检查方式均失败，请检查网络连接')
      }
    }

    console.log('[checkForUpdate] has_update:', result.has_update, 'current_version:', result.current_version)

    updateInfo.value = result

    if (result.has_update) {
      const ignored = getIgnoredVersion()
      if (ignored && ignored === result.latest_version) {
        // 版本已被忽略，显示提示
        hasUpdate.value = false
        Notification.info({
          title: '检查更新',
          content: `v${result.latest_version} 已是最新版本（已忽略）`,
          position: 'top'
        })
      } else {
        hasUpdate.value = true
        console.log('[checkForUpdate] 发现新版本，显示确认对话框')
        Modal.confirm({
          title: '发现新版本',
          content: () => h('div', { style: 'max-height: 300px; overflow-y: auto;' }, [
            usedFallback ? h('div', {
              style: 'margin-bottom: 12px; padding: 8px 12px; background: rgba(255, 165, 0, 0.1); border-radius: 4px; font-size: 12px; color: #ff8c00;'
            }, result.installer_url ? '⚠️ 通过备用通道检测到更新，将自动下载安装' : '⚠️ 通过备用通道检测到更新，建议前往 GitHub 下载最新版本') : null,
            h('div', { style: 'margin-bottom: 12px;' }, [
              h('span', { style: 'color: #666;' }, '当前版本: '),
              h('span', { style: 'font-weight: 600; color: #586cc7;' }, result.current_version)
            ]),
            h('div', { style: 'margin-bottom: 12px;' }, [
              h('span', { style: 'color: #666;' }, '最新版本: '),
              h('span', { style: 'font-weight: 600; color: #52c41a;' }, result.latest_version)
            ]),
            result.published_at ? h('div', { style: 'margin-bottom: 12px; font-size: 12px; color: #999;' },
              `发布时间: ${result.published_at}`) : null,
            h('div', { style: 'margin-top: 16px;' }, [
              h('div', { style: 'font-weight: 600; margin-bottom: 8px;' }, '更新内容:'),
              h('div', {
                style: 'background: rgba(88, 108, 199, 0.05); padding: 12px; border-radius: 8px; font-size: 13px; line-height: 1.6; white-space: pre-wrap;'
              }, result.release_notes || '暂无更新说明')
            ])
          ]),
          okText: usedFallback && !result.installer_url ? '前往下载' : '下载并安装',
          cancelText: '稍后提醒',
          width: 380,
          onOk: async () => {
            if (usedFallback && !result.installer_url) {
              // 备用通道未找到安装包，打开浏览器下载页面
              const { open } = await import('@tauri-apps/plugin-shell')
              await open(result.download_url)
            } else if (usedFallback && result.installer_url) {
              // 备用通道找到安装包，直接下载安装
              await downloadAndInstallFromUrl(result.installer_url)
            } else {
              // 正常使用 Tauri Updater 下载安装
              await downloadAndInstallUpdate()
            }
          },
          onCancel: () => {
            // 用户选择稍后提醒，不忽略版本，保持hasUpdate为true
          }
        })
      }
    } else {
      hasUpdate.value = false
      console.log('[checkForUpdate] 当前已是最新版本，显示通知')
      Notification.success({ title: '检查更新', content: `当前版本 v${result.current_version} 已是最新版本`, position: 'top' })
      console.log('[checkForUpdate] 通知已发送')
    }
  } catch (error) {
    console.error('[checkForUpdate] 检查更新失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    Notification.error({ title: '检查更新失败', content: errorText, position: 'top' })
  } finally {
    updateChecking.value = false
    console.log('[checkForUpdate] 检查完成')
  }
}

async function downloadAndInstallUpdate() {
  try {
    updateChecking.value = true

    Notification.info({
      title: '正在下载更新',
      content: '请稍候，下载完成后将自动安装并重启',
      position: 'top',
      duration: 0
    })

    const result = await invoke('download_and_install_update')

    Notification.success({ title: '更新完成', content: result, position: 'top' })

    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) await relaunch()
  } catch (error) {
    console.error('下载更新失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    Notification.error({ title: '下载更新失败', content: errorText, position: 'top' })
  } finally {
    updateChecking.value = false
  }
}

async function downloadAndInstallFromUrl(url) {
  try {
    updateChecking.value = true

    Notification.info({
      title: '正在下载更新',
      content: '请稍候，下载完成后将自动安装',
      position: 'top',
      duration: 0
    })

    const result = await invoke('download_and_install_from_url', { url })

    Notification.success({ title: '更新完成', content: result, position: 'top' })
  } catch (error) {
    console.error('下载安装更新失败:', error)
    const errorText = typeof error === 'string' ? error : error.message || '未知错误'
    Notification.error({ title: '下载更新失败', content: errorText, position: 'top' })
  } finally {
    updateChecking.value = false
  }
}
</script>

<template>
  <div class="settings-container" :class="{ 'light-theme': !isDarkTheme }">
    <div class="settings-header" @mousedown="handleHeaderDrag">
      <h2>设置</h2>
      <button class="close-btn" @click="closeWindow" title="关闭">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="settings-content">
      <div class="version-info">
        <span class="version-label">版本</span>
        <span class="version-value">v{{ appVersion }}</span>
      </div>

      <div class="settings-section">
        <div class="section-title">外观</div>
        <div class="setting-item theme-item">
          <div class="item-left">
            <div class="item-icon theme-icon">
              <svg v-if="isDarkTheme" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
              </svg>
              <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="5"/>
                <line x1="12" y1="1" x2="12" y2="3"/>
                <line x1="12" y1="21" x2="12" y2="23"/>
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
                <line x1="1" y1="12" x2="3" y2="12"/>
                <line x1="21" y1="12" x2="23" y2="12"/>
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">主题模式</div>
              <div class="item-desc">{{ themeDisplayText }}</div>
            </div>
          </div>
          <div class="item-right">
            <div class="theme-toggle" :class="{ active: themeSwitchValue }" @click="themeSwitchValue = !themeSwitchValue">
              <div class="toggle-thumb"></div>
            </div>
          </div>
        </div>

        <div class="setting-item opacity-item" :class="{ disabled: opacityLoading }">
          <div class="item-left">
            <div class="item-icon opacity-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 2a10 10 0 0 1 0 20 10 10 0 0 1 0-20z" fill="currentColor" fill-opacity="0.3"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">主窗口透明度</div>
              <div class="item-desc">调节主窗口透明程度: {{ Math.round(windowOpacity * 100) }}% </div>
            </div>
          </div>
          <div class="item-right opacity-control">
            <input
              type="range"
              min="0.1"
              max="1"
              step="0.05"
              :value="windowOpacity"
              @input="handleOpacityChange(parseFloat($event.target.value))"
              class="opacity-slider"
            />
          </div>
        </div>

        <div class="setting-item" :class="{ disabled: alwaysOnTopLoading }" @click="toggleAlwaysOnTop">
          <div class="item-left">
            <div class="item-icon pin-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2v8"/>
                <path d="M4.93 10.93l2.83 2.83"/>
                <path d="M2 18l3 3 5-5"/>
                <path d="M19.07 10.93l-2.83 2.83"/>
                <path d="M22 18l-3 3-5-5"/>
                <path d="M12 14v7"/>
                <circle cx="12" cy="18" r="2"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">始终置顶显示</div>
              <div class="item-desc">窗口始终显示在最前面</div>
            </div>
          </div>
          <div class="item-right">
            <div class="theme-toggle" :class="{ active: alwaysOnTop }">
              <div class="toggle-thumb"></div>
            </div>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">数据库</div>
        <div class="setting-item" @click="checkDatabaseStatus" :class="{ disabled: databaseLoading }">
          <div class="item-left">
            <div class="item-icon db-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <ellipse cx="12" cy="5" rx="9" ry="3"/>
                <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/>
                <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">检查数据库状态</div>
              <div class="item-desc">验证数据库结构完整性</div>
            </div>
          </div>
          <div class="item-right">
            <svg class="arrow-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6"/>
            </svg>
          </div>
        </div>

        <div class="setting-item" @click="refreshPageData" :class="{ disabled: databaseLoading }">
          <div class="item-left">
            <div class="item-icon refresh-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M23 4v6h-6"/>
                <path d="M1 20v-6h6"/>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">刷新页面数据</div>
              <div class="item-desc">重置所有状态并重新检查</div>
            </div>
          </div>
          <div class="item-right">
            <svg class="arrow-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6"/>
            </svg>
          </div>
        </div>

        <div class="setting-item" @click="exportDatabaseToInitSql" :class="{ disabled: databaseLoading }">
          <div class="item-left">
            <div class="item-icon export-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">导出数据库</div>
              <div class="item-desc">导出当前数据到 init.sql</div>
            </div>
          </div>
          <div class="item-right">
            <svg class="arrow-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6"/>
            </svg>
          </div>
        </div>

        <div class="setting-item danger" @click="reloadDatabase" :class="{ disabled: databaseLoading }">
          <div class="item-left">
            <div class="item-icon reset-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
                <path d="M3 3v5h5"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">恢复出厂设置</div>
              <div class="item-desc">重置数据库到初始状态</div>
            </div>
          </div>
          <div class="item-right">
            <svg class="arrow-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6"/>
            </svg>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <div class="section-title">更新</div>
        <div class="setting-item" @click="checkForUpdate" :class="{ disabled: updateChecking }">
          <div class="item-left">
            <div class="item-icon update-icon" :class="{ 'loading-spin': updateChecking }">
              <svg v-if="!updateChecking" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>
              </svg>
              <svg v-else class="loading-spinner" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"/>
              </svg>
            </div>
            <div class="item-info">
              <div class="item-title">
                {{ updateChecking ? '正在检查更新...' : '检查更新' }}
                <span v-if="hasUpdate && !updateChecking" class="update-badge">NEW</span>
              </div>
              <div class="item-desc">
                {{ updateChecking ? '正在从多个镜像源查询最新版本' : (hasUpdate ? `发现新版本 v${updateInfo?.latest_version}` : '检查是否有新版本可用') }}
              </div>
            </div>
          </div>
          <div class="item-right">
            <span v-if="hasUpdate && !updateChecking" class="update-dot"></span>
            <svg v-if="!updateChecking" class="arrow-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6"/>
            </svg>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  width: 100%;
  height: 100vh;
  background: var(--bg-color, #1a1b1e);
  color: var(--text-color, #fff);
  display: flex;
  flex-direction: column;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  user-select: none;
  cursor: grab;
}

.settings-header:active {
  cursor: grabbing;
}

.settings-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.close-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  color: rgba(255, 255, 255, 0.7);
}

.close-btn:hover {
  background: rgba(255, 100, 100, 0.3);
  color: #fff;
}

.close-btn svg {
  width: 14px;
  height: 14px;
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px 0;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.settings-content::-webkit-scrollbar {
  display: none;
}

.version-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  margin-bottom: 8px;
}

.version-label {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
}

.version-value {
  font-size: 13px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.8);
  background: rgba(88, 108, 199, 0.2);
  padding: 2px 8px;
  border-radius: 4px;
}

.settings-section {
  margin-bottom: 8px;
}

.section-title {
  font-size: 12px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.4);
  padding: 8px 20px 4px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  cursor: pointer;
  transition: background 0.2s ease;
}

.setting-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.setting-item.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.setting-item.danger .item-title {
  color: #ff6b6b;
}

.item-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.item-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.item-icon svg {
  width: 16px;
  height: 16px;
}

.theme-icon {
  background: linear-gradient(135deg, #586cc7, #764ba2);
  color: #fff;
}

.db-icon {
  background: linear-gradient(135deg, #52c41a, #389e0d);
  color: #fff;
}

.refresh-icon {
  background: linear-gradient(135deg, #faad14, #d48806);
  color: #fff;
}

.export-icon {
  background: linear-gradient(135deg, #13c2c2, #08979c);
  color: #fff;
}

.reset-icon {
  background: linear-gradient(135deg, #ff6b6b, #e74c3c);
  color: #fff;
}

.update-icon {
  background: linear-gradient(135deg, #722ed1, #531dab);
  color: #fff;
}

.update-icon.loading-spin {
  background: linear-gradient(135deg, #586cc7, #722ed1);
}

.loading-spinner {
  animation: spinner-rotate 1s linear infinite;
}

@keyframes spinner-rotate {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.update-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px 6px;
  margin-left: 8px;
  background: linear-gradient(135deg, #ef4444, #dc2626);
  color: #fff;
  font-size: 10px;
  font-weight: 600;
  border-radius: 4px;
  animation: badgePulse 2s ease-in-out infinite;
}

.update-dot {
  width: 8px;
  height: 8px;
  background: #ef4444;
  border-radius: 50%;
  margin-right: 8px;
  box-shadow: 0 0 6px rgba(239, 68, 68, 0.8);
  animation: badgePulse 2s ease-in-out infinite;
}

@keyframes badgePulse {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.1);
    opacity: 0.8;
  }
}

.opacity-icon {
  background: linear-gradient(135deg, #13c2c2, #08979c);
  color: #fff;
}

.pin-icon {
  background: linear-gradient(135deg, #f59e0b, #d97706);
  color: #fff;
}

.opacity-item {
  cursor: default;
}

.opacity-item:hover {
  background: transparent;
}

.theme-item {
  cursor: default;
}

.theme-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.opacity-control {
  display: flex;
  align-items: center;
}

.opacity-slider {
  width: 120px;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
  outline: none;
  cursor: pointer;
}

.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #586cc7;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.opacity-slider::-webkit-slider-thumb:hover {
  transform: scale(1.1);
  background: #6b7fd4;
}

.opacity-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #586cc7;
  cursor: pointer;
  border: none;
  transition: all 0.2s ease;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.opacity-slider::-moz-range-thumb:hover {
  transform: scale(1.1);
  background: #6b7fd4;
}

.item-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.item-title {
  font-size: 14px;
  font-weight: 500;
  color: #fff;
}

.item-desc {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
}

.item-right {
  display: flex;
  align-items: center;
}

.arrow-icon {
  width: 16px;
  height: 16px;
  color: rgba(255, 255, 255, 0.3);
}

.theme-toggle {
  width: 44px;
  height: 24px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.15);
  position: relative;
  transition: background 0.3s ease;
}

.theme-toggle.active {
  background: #586cc7;
}

.toggle-thumb {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #fff;
  position: absolute;
  top: 2px;
  left: 2px;
  transition: transform 0.3s ease;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.theme-toggle.active .toggle-thumb {
  transform: translateX(20px);
}

.light-theme {
  background: #f5f7fa;
  color: #2c3e50;
}

.light-theme .settings-header {
  border-bottom-color: rgba(0, 0, 0, 0.08);
}

.light-theme .close-btn {
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .close-btn:hover {
  background: rgba(255, 100, 100, 0.2);
  color: #e74c3c;
}

.light-theme .version-label {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .version-value {
  color: #586cc7;
  background: rgba(88, 108, 199, 0.1);
}

.light-theme .section-title {
  color: rgba(0, 0, 0, 0.4);
}

.light-theme .setting-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.light-theme .item-title {
  color: #2c3e50;
}

.light-theme .item-desc {
  color: rgba(0, 0, 0, 0.5);
}

.light-theme .arrow-icon {
  color: rgba(0, 0, 0, 0.25);
}

.light-theme .theme-toggle {
  background: rgba(0, 0, 0, 0.1);
}

.light-theme .theme-toggle.active {
  background: #586cc7;
}

.light-theme .theme-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.light-theme .opacity-slider {
  background: rgba(0, 0, 0, 0.1);
}

.light-theme .opacity-slider::-webkit-slider-thumb {
  background: #586cc7;
}

.light-theme .opacity-slider::-webkit-slider-thumb:hover {
  background: #6b7fd4;
}

.light-theme .opacity-slider::-moz-range-thumb {
  background: #586cc7;
}

.light-theme .opacity-slider::-moz-range-thumb:hover {
  background: #6b7fd4;
}


</style>
