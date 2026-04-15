import {defineStore} from 'pinia'
import {ref, watch} from "vue"

// 主题管理store
export const useThemeStore = defineStore('theme', () => {
    // 从localStorage获取初始主题，默认为'auto'（跟随系统）
    const currentTheme = ref(localStorage.getItem('theme') || 'auto')
    
    // 媒体查询监听器引用
    let mediaQueryListener = null
    
    // 获取系统主题偏好
    function getSystemTheme() {
        if (typeof window === 'undefined') return 'light'
        return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    
    // 获取实际应用的主题（auto时返回系统主题）
    function getEffectiveTheme() {
        if (currentTheme.value === 'auto') {
            return getSystemTheme()
        }
        return currentTheme.value
    }
    
    // 应用主题到DOM
    function applyTheme(theme) {
        const effectiveTheme = theme === 'auto' ? getSystemTheme() : theme
        
        // 设置HTML根元素的data-theme属性
        document.documentElement.setAttribute('data-theme', effectiveTheme)
        
        // 设置Arco Design的主题
        if (effectiveTheme === 'dark') {
            document.body.setAttribute('arco-theme', 'dark')
            document.body.classList.remove('light-theme')
        } else {
            document.body.removeAttribute('arco-theme')
            document.body.classList.add('light-theme')
        }
    }
    
    // 监听系统主题变化
    function startListeningToSystemTheme() {
        if (typeof window === 'undefined') return
        
        const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
        
        // 移除旧的监听器
        stopListeningToSystemTheme()
        
        // 添加新的监听器
        mediaQueryListener = (e) => {
            if (currentTheme.value === 'auto') {
                applyTheme('auto')
            }
        }
        
        mediaQuery.addEventListener('change', mediaQueryListener)
    }
    
    // 停止监听系统主题变化
    function stopListeningToSystemTheme() {
        if (typeof window === 'undefined' || !mediaQueryListener) return
        
        const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
        mediaQuery.removeEventListener('change', mediaQueryListener)
        mediaQueryListener = null
    }
    
    // 切换主题（light <-> dark，不使用auto）
    function toggleTheme() {
        const currentEffective = getEffectiveTheme()
        const newTheme = currentEffective === 'dark' ? 'light' : 'dark'
        currentTheme.value = newTheme
        applyTheme(newTheme)
        localStorage.setItem('theme', newTheme)
        
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

            // 如果是auto模式，开始监听系统主题变化
            if (theme === 'auto') {
                startListeningToSystemTheme()
            } else {
                stopListeningToSystemTheme()
            }

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
    }
    
    // 初始化主题
    function initTheme() {
        applyTheme(currentTheme.value)
        
        // 如果是auto模式，开始监听系统主题变化
        if (currentTheme.value === 'auto') {
            startListeningToSystemTheme()
        }
        
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
        applyTheme,
        getEffectiveTheme
    }
})