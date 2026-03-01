<script setup>
import { ref, onMounted, computed, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useEcosystemStore } from '@/stores/ecosystem'
import { useThemeStore } from '@/stores'
import { Notification } from '@arco-design/web-vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const router = useRouter()
const route = useRoute()
const ecoStore = useEcosystemStore()
const themeStore = useThemeStore()
const appWindow = getCurrentWindow()

const target = ref(route.query.target || 'transfer')

onMounted(async () => {
  const isTauri = typeof window !== 'undefined' && window.__TAURI_INTERNALS__
  if (!isTauri) return
  await nextTick()
  // 使用 requestAnimationFrame 确保页面已经渲染完成
  requestAnimationFrame(() => {
    appWindow.emit('page-loaded')
  })
})

// 页面映射配置
const pageMap = {
  evm: {
    transfer: '/eth/transfer',
    balance: '/eth/balance',
    monitor: '/eth/monitor'
  },
  solana: {
    transfer: '/sol/transfer',
    balance: '/sol/balance',
    // monitor: '/sol/monitor' // 暂不支持
  }
}

// 选择生态
const selectEcosystem = (eco) => {
  // 设置全局生态状态
  ecoStore.setEco(eco === 'evm' ? 'eth' : 'sol')
  
  // 获取目标路由
  const targetPath = pageMap[eco][target.value]
  
  if (targetPath) {
    router.replace(targetPath)
  } else {
    // 如果目标功能在该生态不支持（例如 Solana 监控），提示
    Notification.warning({
      title: '暂不支持',
      content: `${eco === 'solana' ? 'Solana' : 'EVM'} 生态暂不支持此功能`,
      position: 'top',
    })
    console.warn(`Feature ${target.value} not supported on ${eco}`)
  }
}

// 主题相关
const isDarkTheme = computed(() => themeStore.getEffectiveTheme() === 'dark')

// 关闭页面
const closeWindow = async () => {
  await appWindow.destroy()
}
</script>

<template>
  <div class="entry-container" :class="{ 'light-theme': !isDarkTheme }">
    <!-- 背景装饰 -->
    <div class="bg-decoration">
      <div class="bg-circle bg-circle-1"></div>
      <div class="bg-circle bg-circle-2"></div>
      <div class="bg-gradient"></div>
    </div>

    <div class="content-wrapper">
      <button class="close-btn" @click="closeWindow" title="关闭窗口">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
      <h1 class="page-title">选择生态网络</h1>
      <p class="page-subtitle">请选择您要操作的区块链网络</p>

      <div class="ecosystem-grid">
        <!-- EVM Card -->
        <div class="eco-card evm-card" @click="selectEcosystem('evm')">
          <div class="card-inner">
            <div class="icon-wrapper">
              <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 115 182" class="eco-icon">
                <path fill="#F0CDC2" stroke="currentColor" stroke-width="2" d="M57.505 181v-45.16L1.641 103.171z"></path>
                <path fill="#C9B3F5" stroke="currentColor" stroke-width="2" d="M57.69 181v-45.16l55.865-32.669z"></path>
                <path fill="#88AAF1" stroke="currentColor" stroke-width="2" d="M57.506 124.615V66.979L1 92.28z"></path>
                <path fill="#C9B3F5" stroke="currentColor" stroke-width="2" d="M57.69 124.615V66.979l56.506 25.302z"></path>
                <path fill="#F0CDC2" stroke="currentColor" stroke-width="2" d="M1 92.281 57.505 1v65.979z"></path>
                <path fill="#B8FAF6" stroke="currentColor" stroke-width="2" d="M114.196 92.281 57.691 1v65.979z"></path>
              </svg>
            </div>
            <h2>EVM 生态</h2>
            <p>Ethereum, BSC, Polygon, Arbitrum, Optimism 等兼容网络</p>
            <div class="arrow-btn">
              <span>进入</span>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            </div>
          </div>
        </div>

        <!-- Solana Card -->
        <div class="eco-card solana-card" @click="selectEcosystem('solana')">
          <div class="card-inner">
            <div class="icon-wrapper">
              <svg width="64" height="64" viewBox="0 0 397.7 311.7" xmlns="http://www.w3.org/2000/svg" class="eco-icon">
                <path d="M64.6 237.9c2.4-2.4 5.7-3.8 9.2-3.8h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1l62.7-62.7z" fill="currentColor" />
                <path d="M64.6 3.8C67.1 1.4 70.4 0 73.8 0h317.4c5.8 0 8.7 7 4.6 11.1l-62.7 62.7c-2.4 2.4-5.7 3.8-9.2 3.8H6.5c-5.8 0-8.7-7-4.6-11.1L64.6 3.8z" fill="currentColor" />
                <path d="M333.1 120.1c-2.4-2.4-5.7-3.8-9.2-3.8H6.5c-5.8 0-8.7 7-4.6 11.1l62.7 62.7c2.4 2.4 5.7 3.8 9.2 3.8h317.4c5.8 0 8.7-7 4.6-11.1l-62.7-62.7z" fill="currentColor" />
              </svg>
            </div>
            <h2>Solana 生态</h2>
            <p>Solana 主网及测试网高性能网络</p>
            <div class="arrow-btn">
              <span>进入</span>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.entry-container {
  position: relative;
  min-height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--bg-color, #1a1b1e);
  color: var(--text-color, #fff);
  overflow: hidden;
}

/* 复用 Home 背景逻辑 */
.bg-decoration {
  position: absolute;
  top: 0; left: 0; width: 100%; height: 100%;
  z-index: 0;
  pointer-events: none;
}
.bg-gradient {
  position: absolute; width: 100%; height: 100%;
  background: linear-gradient(135deg, #0a0e1a 0%, #0d1117 25%, #111827 50%, #0f172a 75%, #0a0f1c 100%);
}

/* 深蓝渐变光晕装饰 */
.bg-circle {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.4;
}

.bg-circle-1 {
  width: 600px;
  height: 600px;
  background: radial-gradient(circle, rgba(59, 91, 219, 0.3) 0%, transparent 70%);
  top: -200px;
  right: -100px;
}

.bg-circle-2 {
  width: 500px;
  height: 500px;
  background: radial-gradient(circle, rgba(30, 58, 138, 0.25) 0%, transparent 70%);
  bottom: -150px;
  left: -100px;
}

.content-wrapper {
  position: relative;
  z-index: 10;
  text-align: center;
  width: 100%;
  max-width: 900px;
  padding: 20px;
  animation: fadeIn 0.6s ease-out;
}

.close-btn {
  position: absolute;
  top: 0;
  right: 0;
  width: 40px;
  height: 40px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s ease;
  color: rgba(255, 255, 255, 0.7);
}

.close-btn:hover {
  background: rgba(255, 100, 100, 0.3);
  color: #fff;
  transform: scale(1.1);
}

.close-btn svg {
  width: 20px;
  height: 20px;
  pointer-events: none;
}

.light-theme .close-btn {
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .close-btn:hover {
  background: rgba(255, 100, 100, 0.2);
  color: #e74c3c;
}

.page-title {
  font-size: 36px;
  font-weight: 700;
  margin-bottom: 8px;
  line-height: 1.4;
  padding-bottom: 4px;
  background: linear-gradient(90deg, #e8eaf6, #5b8aff);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.page-subtitle {
  font-size: 16px;
  color: #9aa3c2;
  margin-bottom: 50px;
}

.ecosystem-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 30px;
}

.eco-card {
  background: rgba(22, 27, 34, 0.6);
  border: 1px solid rgba(88, 108, 199, 0.15);
  border-radius: 20px;
  padding: 40px 30px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
  backdrop-filter: blur(10px);
}

.eco-card:hover {
  transform: translateY(-8px);
  background: rgba(30, 41, 59, 0.8);
  border-color: rgba(91, 138, 255, 0.4);
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4), 0 0 30px rgba(91, 138, 255, 0.1);
}

.icon-wrapper {
  width: 100px;
  height: 100px;
  margin: 0 auto 24px;
  background: rgba(15, 23, 42, 0.5);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.3s ease;
  border: 1px solid rgba(88, 108, 199, 0.2);
}

.eco-card:hover .icon-wrapper {
  transform: scale(1.1) rotate(5deg);
}

.eco-icon {
  width: 50px;
  height: 50px;
  color: #e8eaf6;
}

.evm-card .icon-wrapper {
  background: linear-gradient(135deg, rgba(59, 91, 219, 0.25), rgba(91, 138, 255, 0.15));
  color: #5b8aff;
}

.solana-card .icon-wrapper {
  background: linear-gradient(135deg, rgba(20, 241, 149, 0.15), rgba(154, 91, 255, 0.15));
  color: #14F195;
}

h2 {
  font-size: 24px;
  margin-bottom: 12px;
  color: #e8eaf6;
}

p {
  color: #9aa3c2;
  margin-bottom: 30px;
  min-height: 48px;
}

.arrow-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 24px;
  background: rgba(88, 108, 199, 0.15);
  border-radius: 30px;
  font-weight: 500;
  transition: all 0.3s ease;
  color: #9aa3c2;
}

.eco-card:hover .arrow-btn {
  background: linear-gradient(135deg, #3b5bdb, #5b8aff);
  color: #fff;
  padding-right: 20px;
}

.arrow-btn svg {
  width: 18px;
  height: 18px;
  transition: transform 0.3s ease;
}

.eco-card:hover .arrow-btn svg {
  transform: translateX(4px);
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 明亮主题样式 */
.light-theme {
  background: #f5f7fa;
  color: #2c3e50;
}

.light-theme .bg-gradient {
  background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 50%, #e0eafc 100%);
  opacity: 0.8;
}

.light-theme .bg-circle {
  background: rgba(103, 126, 234, 0.08);
}

.light-theme .page-title {
  background: linear-gradient(90deg, #586cc7, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.light-theme .page-subtitle {
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .eco-card {
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.05);
}

.light-theme .eco-card:hover {
  background: rgba(255, 255, 255, 0.95);
  border-color: rgba(88, 108, 199, 0.3);
  box-shadow: 0 16px 32px rgba(88, 108, 199, 0.15);
}

.light-theme h2 {
  color: #2c3e50;
}

.light-theme p {
  color: rgba(0, 0, 0, 0.6);
}

.light-theme .arrow-btn {
  background: rgba(0, 0, 0, 0.05);
  color: #555;
}

.light-theme .eco-card:hover .arrow-btn {
  background: #586cc7;
  color: #fff;
}

.light-theme .evm-card .icon-wrapper {
  background: linear-gradient(135deg, rgba(88, 108, 199, 0.1), rgba(118, 75, 162, 0.1));
  color: #586cc7;
}

.light-theme .solana-card .icon-wrapper {
  background: linear-gradient(135deg, rgba(0, 255, 163, 0.15), rgba(220, 31, 255, 0.15));
  color: #10b981; /* 稍深一点的绿色以在浅色背景下可见 */
}
</style>
