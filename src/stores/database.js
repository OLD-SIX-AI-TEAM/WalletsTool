import { defineStore } from 'pinia'

export const useDatabaseStore = defineStore('database', {
  state: () => ({
    publicReady: false,
    secureState: 'not_initialized', // not_initialized | locked | unlocked
    loading: false,
    error: null,
  }),

  actions: {
    async checkStatus() {
      this.loading = true
      this.error = null
      try {
        if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
          // Web development mode - simulate unlocked state
          this.publicReady = true
          this.secureState = 'unlocked'
          return
        }

        const { invoke } = await import('@tauri-apps/api/core')
        const status = await invoke('get_dual_database_status')
        this.publicReady = status.public_ready
        this.secureState = status.secure_state
      } catch (error) {
        console.error('Failed to check database status:', error)
        this.error = error?.message || '检查数据库状态失败'
      } finally {
        this.loading = false
      }
    },

    async initPublicDatabase() {
      this.loading = true
      this.error = null
      try {
        if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
          this.publicReady = true
          return
        }

        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('init_public_db')
        this.publicReady = true
      } catch (error) {
        console.error('Failed to init public database:', error)
        this.error = error?.message || '初始化公开数据库失败'
        throw error
      } finally {
        this.loading = false
      }
    },

    async initSecureDatabase(password) {
      this.loading = true
      this.error = null
      try {
        if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
          this.secureState = 'unlocked'
          return
        }

        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('init_secure_db', { encryptedPassword: password })
        this.secureState = 'unlocked'
      } catch (error) {
        console.error('Failed to init secure database:', error)
        this.error = error?.message || '初始化安全数据库失败'
        throw error
      } finally {
        this.loading = false
      }
    },

    async unlockSecureDatabase(password) {
      this.loading = true
      this.error = null
      try {
        if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
          this.secureState = 'unlocked'
          return
        }

        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('unlock_secure_db', { encryptedPassword: password })
        this.secureState = 'unlocked'
      } catch (error) {
        console.error('Failed to unlock secure database:', error)
        this.error = error?.message || '解锁安全数据库失败'
        throw error
      } finally {
        this.loading = false
      }
    },

    async lockSecureDatabase() {
      this.loading = true
      this.error = null
      try {
        if (typeof window === 'undefined' || !window.__TAURI_INTERNALS__) {
          this.secureState = 'locked'
          return
        }

        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('lock_secure_db')
        this.secureState = 'locked'
      } catch (error) {
        console.error('Failed to lock secure database:', error)
        this.error = error?.message || '锁定安全数据库失败'
        throw error
      } finally {
        this.loading = false
      }
    },

    clearError() {
      this.error = null
    },
  },

  getters: {
    isReady: (state) => state.publicReady && state.secureState === 'unlocked',
    needsPasswordSetup: (state) => state.publicReady && state.secureState === 'not_initialized',
    needsUnlock: (state) => state.publicReady && state.secureState === 'locked',
  },
})
