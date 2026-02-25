import {defineStore} from 'pinia'
import {ref, watch} from "vue"

// 主题管理store
export const useThemeStore = defineStore('theme', () => {
    // 从localStorage获取初始主题，默认为明亮主题
    const currentTheme = ref(localStorage.getItem('theme') || 'light')
    
    // 应用主题到DOM
    function applyTheme(theme) {
        // 设置HTML根元素的data-theme属性
        document.documentElement.setAttribute('data-theme', theme)
        
        // 设置Arco Design的主题
        if (theme === 'dark') {
            document.body.setAttribute('arco-theme', 'dark')
            document.body.classList.remove('light-theme')
        } else {
            document.body.removeAttribute('arco-theme')
            document.body.classList.add('light-theme')
        }
    }
    
    // 切换主题
    function toggleTheme() {
        currentTheme.value = currentTheme.value === 'dark' ? 'light' : 'dark'
        applyTheme(currentTheme.value)
        localStorage.setItem('theme', currentTheme.value)
        
        // 通过Tauri事件系统广播主题变化到其他窗口
        if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
            try {
                import('@tauri-apps/api/event').then(({ emit }) => {
                    emit('theme-changed', { theme: currentTheme.value })
                })
            } catch (error) {
                console.error('Failed to emit theme change event:', error)
            }
        }
    }
    
    // 设置主题（用于接收其他窗口的主题变化）
    function setTheme(theme) {
        if (theme !== currentTheme.value) {
            currentTheme.value = theme
            applyTheme(theme)
            localStorage.setItem('theme', theme)
        }
    }
    
    // 初始化主题
    function initTheme() {
        applyTheme(currentTheme.value)
        
        // 监听来自其他窗口的主题变化事件
        if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
            try {
                import('@tauri-apps/api/event').then(({ listen }) => {
                    listen('theme-changed', (event) => {
                        setTheme(event.payload.theme)
                    })
                })
            } catch (error) {
                console.error('Failed to listen for theme change events:', error)
            }
        }
    }
    
    return {
        currentTheme,
        toggleTheme,
        setTheme,
        initTheme,
        applyTheme
    }
})