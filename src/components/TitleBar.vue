<template>
  <!-- 自定义标题栏 -->
  <div class="title-bar">
    <div class="title-bar-left">
      <img src="/app-icon.png" alt="Logo" class="title-bar-logo" />
      
      <template v-if="isEditing">
        <input
          ref="editInputRef"
          v-model="editTitle"
          class="title-bar-input"
          :placeholder="defaultTitle"
          @blur="saveTitle"
          @keydown="handleKeydown"
          @click.stop
        />
      </template>
      
      <template v-else>
        <div
          class="title-wrapper"
          @click="startEditing"
          @mouseenter="isHovered = true"
          @mouseleave="isHovered = false"
        >
          <div
            class="title-bar-text"
            :class="{ 'title-editable': true, 'title-hovered': isHovered && !isCustom }"
            title="点击编辑窗口名称"
          >
            {{ displayTitle }}
          </div>
          
          <span
            v-if="isCustom && isHovered"
            class="reset-button"
            @click.stop="resetToDefault"
            title="恢复默认名称"
          >
            ↺
          </span>
          
          <span v-if="isCustom" class="custom-badge" title="已自定义名称">★</span>
        </div>
        
        <!-- 生态标识 -->
        <div v-if="ecosystem" class="ecosystem-badge" :class="ecosystem.toLowerCase()">
          {{ ecosystem }}
        </div>
      </template>
    </div>
    
    <div class="title-bar-controls">
      <!-- 主题切换开关 -->
      <div class="titlebar-center">
        <div class="theme-toggle-container">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="theme-icon">
            <circle cx="12" cy="12" r="5" />
            <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
          </svg>
          <a-switch v-model="isDarkTheme" @change="toggleTheme" size="small" class="theme-switch" />
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="theme-icon">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        </div>
      </div>
      <button class="title-bar-control" @click="minimizeWindow" title="最小化">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M2 6h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
      <button class="title-bar-control" @click="maximizeWindow" :title="isMaximized ? '还原' : '最大化'">
        <svg v-if="!isMaximized" width="12" height="12" viewBox="0 0 12 12">
          <rect x="2" y="2" width="8" height="8" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" />
        </svg>
        <svg v-else width="12" height="12" viewBox="0 0 12 12">
          <rect x="3" y="1" width="6" height="6" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" />
          <rect x="1" y="3" width="6" height="6" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" />
        </svg>
      </button>
<button
          class="title-bar-control close"
          :class="{ 'disabled': disableClose }"
          @click="closeWindow"
          :title="disableClose ? '操作进行中，无法关闭' : '关闭'"
          :style="{ cursor: disableClose ? 'not-allowed' : 'pointer', opacity: disableClose ? 0.5 : 1 }"
        >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref, nextTick, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Notification } from '@arco-design/web-vue'
import { useThemeStore } from '@/stores'
import { WINDOW_CONFIG } from '@/utils/windowNames'

const isMaximized = ref(false)
const isEditing = ref(false)
const isHovered = ref(false)
const editTitle = ref('')
const editInputRef = ref(null)
const customTitle = ref(null)

const props = defineProps({
  title: {
    type: String,
    default: 'WalletsTool'
  },
  windowLabel: {
    type: String,
    default: ''
  },
  customClose: {
    type: Boolean,
    default: false
  },
  disableClose: {
    type: Boolean,
    default: false
  },
  ecosystem: {
    type: String,
    default: ''
  }
})

const emit = defineEmits(['before-close', 'title-changed'])

const themeStore = useThemeStore()
const currentTheme = computed(() => themeStore.currentTheme)
const isDarkTheme = computed({
  get: () => themeStore.getEffectiveTheme() === 'dark',
  set: (value) => {
  }
})

const defaultTitle = computed(() => props.title || 'WalletsTool')

const displayTitle = computed(() => {
  if (customTitle.value) {
    return customTitle.value
  }
  return defaultTitle.value
})

const isCustom = computed(() => {
  return customTitle.value !== null
})

function toggleTheme() {
  themeStore.toggleTheme()
}

function getWindowLabel() {
  if (props.windowLabel) {
    return props.windowLabel
  }
  try {
    const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
    if (isTauri) {
      return getCurrentWindow().label
    }
  } catch (e) {
    console.error('获取窗口标签失败:', e)
  }
  return 'main'
}

async function loadCustomTitle() {
  const label = getWindowLabel()
  const saved = WINDOW_CONFIG.getCustomTitle(label)
  if (saved) {
    customTitle.value = saved
    try {
      const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
      if (isTauri) {
        const currentWindow = getCurrentWindow()
        await currentWindow.setTitle(saved)
      }
    } catch (e) {
      console.error('设置窗口标题失败:', e)
    }
  }
}

async function startEditing() {
  isEditing.value = true
  editTitle.value = customTitle.value || defaultTitle.value
  
  await nextTick()
  
  if (editInputRef.value) {
    editInputRef.value.focus()
    editInputRef.value.select()
  }
}

async function saveTitle() {
  const trimmedTitle = editTitle.value.trim()
  
  if (!trimmedTitle) {
    cancelEdit()
    return
  }
  
  if (trimmedTitle.length > 50) {
    Notification.warning({
      content: '窗口名称不能超过50个字符',
      position: 'top'
    })
    editInputRef.value?.focus()
    return
  }
  
  const label = getWindowLabel()
  const newTitle = trimmedTitle
  
  if (WINDOW_CONFIG.saveCustomTitle(label, newTitle)) {
    customTitle.value = newTitle
    
    try {
      const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
      if (isTauri) {
        const currentWindow = getCurrentWindow()
        await currentWindow.setTitle(newTitle)
      }
    } catch (e) {
      console.error('设置窗口标题失败:', e)
    }
    
    emit('title-changed', newTitle)
  }
  
  isEditing.value = false
}

async function resetToDefault() {
  const label = getWindowLabel()
  
  if (WINDOW_CONFIG.removeCustomTitle(label)) {
    customTitle.value = null
    
    try {
      const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
      if (isTauri) {
        const currentWindow = getCurrentWindow()
        await currentWindow.setTitle(defaultTitle.value)
      }
    } catch (e) {
      console.error('恢复默认窗口标题失败:', e)
    }
    
    emit('title-changed', defaultTitle.value)
  }
}

function cancelEdit() {
  editTitle.value = ''
  isEditing.value = false
}

function handleKeydown(event) {
  if (event.key === 'Enter') {
    event.preventDefault()
    saveTitle()
  } else if (event.key === 'Escape') {
    event.preventDefault()
    cancelEdit()
  }
}

async function minimizeWindow() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (isTauri) {
    try {
      const currentWindow = getCurrentWindow()
      await currentWindow.minimize()
    } catch (error) {
      console.error('Error minimizing window:', error)
    }
  }
}

async function maximizeWindow() {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (isTauri) {
    try {
      const currentWindow = getCurrentWindow()
      const currentMaximized = await currentWindow.isMaximized()
      if (currentMaximized) {
        await currentWindow.unmaximize()
        isMaximized.value = false
      } else {
        await currentWindow.maximize()
        isMaximized.value = true
      }
    } catch (error) {
      console.error('Error toggling maximize window:', error)
    }
  }
}

async function closeWindow() {
  if (props.disableClose) {
    return
  }

  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (isTauri) {
    try {
      console.log('TitleBar窗口关闭事件触发，正在通知父组件执行清理操作...')

      emit('before-close')

      if (props.customClose) {
        return
      }

      const currentWindow = getCurrentWindow()
      await currentWindow.destroy()
    } catch (error) {
      console.error('Error closing window:', error)
    }
  }
}

let unlistenResize = null

onMounted(async () => {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (isTauri) {
    try {
      const currentWindow = getCurrentWindow()
      isMaximized.value = await currentWindow.isMaximized()
      
      // 监听窗口大小变化，同步最大化状态
      unlistenResize = await currentWindow.onResized(async () => {
        isMaximized.value = await currentWindow.isMaximized()
      })
      
      await loadCustomTitle()
    } catch (error) {
      console.error('Error getting window state:', error)
    }
  }
})

watch(() => props.title, (newTitle) => {
  if (!customTitle.value && newTitle) {
  }
})

onUnmounted(() => {
  if (unlistenResize) {
    unlistenResize()
  }
})
</script>

<style scoped>
.title-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 40px;
  background: linear-gradient(to bottom, var(--color-bg-2, #ffffff), var(--color-bg-1, #f7f8fa));
  color: white;
  font-size: 14px;
  -webkit-app-region: drag;
  user-select: none;
  backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(30, 58, 138, 0.3);
  padding: 0 10px;
  font-weight: 500;
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 1000;
}

.title-bar-left {
  display: flex;
  align-items: center;
  margin-left: 10px;
}

.title-bar-logo {
  width: 24px;
  height: 24px;
  margin-right: 8px;
  border-radius: 4px;
  object-fit: contain;
}

.title-bar-text {
  font-weight: 500;
  font-size: 14px;
}

.title-wrapper {
  display: flex;
  align-items: center;
  gap: 4px;
}

.title-bar-input {
  display: inline-block;
  background: var(--color-bg-1, #1a1a1a);
  border: 1px solid rgba(255, 255, 255, 0.4);
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 14px;
  font-weight: 500;
  color: inherit;
  outline: none;
  line-height: 1.2;
  width: auto;
  min-width: 120px;
  max-width: 300px;
  -webkit-app-region: no-drag;
  transition: all 0.2s ease;
  box-sizing: border-box;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  position: relative;
  z-index: 10;
}

.title-bar-input:focus {
  border-color: rgba(66, 153, 225, 0.8);
  background: var(--color-bg-1, #1a1a1a);
  box-shadow: 0 0 0 2px rgba(66, 153, 225, 0.2);
}

.title-bar-input::placeholder {
  color: rgba(255, 255, 255, 0.5);
}

.custom-badge {
  font-size: 10px;
  color: #fbbf24;
  opacity: 0.8;
  transition: opacity 0.2s ease;
}

.custom-badge:hover {
  opacity: 1;
}

.reset-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  background: rgba(96, 165, 250, 0.8);
  border-radius: 4px;
  color: white;
  font-size: 11px;
  cursor: pointer;
  -webkit-app-region: no-drag;
  transition: all 0.2s ease;
  margin-left: 4px;
}

.reset-button:hover {
  background: rgba(59, 130, 246, 0.9);
  transform: scale(1.1);
}

.ecosystem-badge {
  margin-left: 12px;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  -webkit-app-region: no-drag;
  cursor: default;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}

.ecosystem-badge.solana {
  background: linear-gradient(135deg, #9945FF 0%, #14F195 100%);
  color: white;
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.ecosystem-badge.evm {
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  color: white;
  border: 1px solid rgba(255, 255, 255, 0.2);
}

.title-editable {
  cursor: text;
  padding: 2px 6px;
  border-radius: 4px;
  transition: all 0.2s ease;
  border: 1px solid transparent;
}

.title-editable.title-hovered {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.2);
}

.title-editable.title-hovered:hover {
  background: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.3);
}

.title-bar-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 100%;
  -webkit-app-region: no-drag;
}

.title-bar-control {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  border-radius: 6px;
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  font-size: 16px;
  transition: all 0.2s ease;
  -webkit-app-region: no-drag;
}

.title-bar-control:hover {
  background: rgba(255, 255, 255, 0.2);
  color: rgba(255, 255, 255, 0.9);
}

.title-bar-control.close:hover {
  background-color: rgba(96, 96, 96, 0.9) !important;
  color: white !important;
}

.title-bar-control.theme-toggle {
  width: 40px;
  margin-right: 5px;
}

.title-bar-control.theme-toggle:hover {
  background-color: rgba(255, 255, 255, 0.15);
  border-radius: 4px;
}

.title-bar-control.theme-toggle:hover .theme-icon {
  transform: scale(1.1);
}

.theme-icon {
  font-size: 16px;
  transition: transform 0.3s ease;
  width: 14px;
  height: 14px;
  opacity: 0.8;
}

.title-bar-control.close:hover {
  background: rgba(96, 96, 96, 0.9);
  color: white;
}

.title-bar-control:first-of-type:hover {
  background: rgba(206, 184, 136, 0.8);
  color: white;
}

.title-bar-control:nth-of-type(2):hover {
  background: rgba(73, 152, 220, 0.8);
  color: white;
}

.titlebar-center {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  z-index: 1002;
}

.theme-toggle-container {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  padding: 6px 12px;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  -webkit-app-region: no-drag;
}

.theme-icon {
  color: rgba(255, 255, 255, 0.7);
  transition: color 0.2s ease;
}

.theme-toggle-container:hover .theme-icon {
  color: rgba(255, 255, 255, 0.9);
}

.theme-switch {
  margin: 0 4px;
}

.title-bar-control .iconify {
  opacity: 0.9;
  transition: opacity 0.2s ease;
}

.title-bar-control:hover .iconify {
  opacity: 1;
}

/* 明亮主题样式 */
:root[data-theme="light"] .title-bar {
  background: linear-gradient(135deg, #f8fafc 0%, #e2e8f0 100%);
  color: #1a202c;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
}

:root[data-theme="light"] .title-bar-control {
  color: #4a5568;
}

:root[data-theme="light"] .title-bar-control:hover {
  background-color: rgba(0, 0, 0, 0.1);
}

:root[data-theme="light"] .title-bar-control.close:hover {
  background-color: #e53e3e;
  color: white;
}

:root[data-theme="light"] .title-switch :deep(.arco-switch) {
  background-color: rgba(0, 0, 0, 0.1);
  border: 1px solid rgba(0, 0, 0, 0.2);
}

:root[data-theme="light"] .title-switch :deep(.arco-switch-checked) {
  background-color: rgba(0, 0, 0, 0.2);
}

:root[data-theme="light"] .title-switch :deep(.arco-switch-dot) {
  background-color: #4a5568;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

:root[data-theme="light"] .theme-toggle-container {
  background: rgba(0, 0, 0, 0.05);
  border: 1px solid rgba(0, 0, 0, 0.1);
}

:root[data-theme="light"] .theme-icon {
  color: rgba(0, 0, 0, 0.7);
}

:root[data-theme="light"] .theme-toggle-container:hover .theme-icon {
  color: rgba(0, 0, 0, 0.9);
}

:root[data-theme="light"] .title-bar-input {
  background: rgba(0, 0, 0, 0.05);
  border-color: rgba(0, 0, 0, 0.2);
  color: #1a202c;
}

:root[data-theme="light"] .title-bar-input:focus {
  border-color: rgba(66, 153, 225, 0.8);
  background: rgba(0, 0, 0, 0.08);
}

:root[data-theme="light"] .title-bar-input::placeholder {
  color: rgba(0, 0, 0, 0.4);
}

:root[data-theme="light"] .title-editable.title-hovered {
  background: rgba(0, 0, 0, 0.08);
  border-color: rgba(0, 0, 0, 0.15);
}

:root[data-theme="light"] .title-editable.title-hovered:hover {
  background: rgba(0, 0, 0, 0.12);
  border-color: rgba(0, 0, 0, 0.25);
}
</style>