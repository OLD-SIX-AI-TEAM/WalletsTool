<script setup name="TransferStatusBar">
import { ref, computed } from 'vue';
import { Icon } from '@iconify/vue';
import ChainIcon from '@/components/ChainIcon.vue';

const props = defineProps({
  windowTitle: {
    type: String,
    default: '批量转账',
  },
  chainValue: {
    type: String,
    default: '',
  },
  currentChain: {
    type: Object,
    default: null,
  },
  chainOptions: {
    type: Array,
    default: () => [],
  },
  coinValue: {
    type: String,
    default: '',
  },
  coinOptions: {
    type: Array,
    default: () => [],
  },
  currentCoin: {
    type: Object,
    default: null,
  },
  proxyEnabled: {
    type: Boolean,
    default: false,
  },
  proxyCount: {
    type: Number,
    default: 0,
  },
  proxyStatusColor: {
    type: String,
    default: '#86909c',
  },
  isSidePanelExpanded: {
    type: Boolean,
    default: true,
  },
  chainFieldNames: {
    type: Object,
    default: () => ({ value: 'key', label: 'scan_url' }),
  },
  coinFieldNames: {
    type: Object,
    default: () => ({ value: 'key', label: 'label' }),
  },
});

const emit = defineEmits([
  'toggle-chain-selector',
  'toggle-token-selector',
  'select-chain',
  'select-token',
  'open-chain-explorer',
  'expand-side-panel',
  'collapse-side-panel',
  'open-proxy-config',
  'show-token-manage',
  'show-rpc-manage',
  'show-chain-manage',
]);

const chainSearchKeyword = ref('');
const tokenSearchKeyword = ref('');
const chainSearchInputRef = ref(null);
const tokenSearchInputRef = ref(null);
const chainSelectorExpanded = ref(false);
const tokenSelectorExpanded = ref(false);

const filteredChainOptions = computed(() => {
  if (!chainSearchKeyword.value.trim()) {
    return props.chainOptions;
  }
  const keyword = chainSearchKeyword.value.toLowerCase();
  return props.chainOptions.filter(
    (chain) =>
      chain.name.toLowerCase().includes(keyword) ||
      chain.key.toLowerCase().includes(keyword) ||
      (chain.scan_url && chain.scan_url.toLowerCase().includes(keyword))
  );
});

const filteredCoinOptions = computed(() => {
  if (!tokenSearchKeyword.value.trim()) {
    return props.coinOptions;
  }
  const keyword = tokenSearchKeyword.value.toLowerCase();
  return props.coinOptions.filter(
    (coin) =>
      coin.label.toLowerCase().includes(keyword) ||
      coin.symbol.toLowerCase().includes(keyword) ||
      (coin.key && coin.key.toLowerCase().includes(keyword))
  );
});

function toggleChainSelector() {
  chainSelectorExpanded.value = !chainSelectorExpanded.value;
  tokenSelectorExpanded.value = false;
}

function toggleTokenSelector() {
  if (!props.chainValue) {
    return;
  }
  tokenSelectorExpanded.value = !tokenSelectorExpanded.value;
  chainSelectorExpanded.value = false;
}

function handleChainSelect(chainKey) {
  emit('select-chain', chainKey);
  chainSelectorExpanded.value = false;
  tokenSelectorExpanded.value = true;
}

function handleTokenSelect(tokenKey) {
  emit('select-token', tokenKey);
  tokenSelectorExpanded.value = false;
}

function openChainExplorer() {
  emit('open-chain-explorer');
}

function openProxyConfig() {
  emit('open-proxy-config');
}

function toggleSidePanel() {
  if (props.isSidePanelExpanded) {
    emit('collapse-side-panel');
  } else {
    emit('expand-side-panel');
  }
}

function showTokenManage() {
  emit('show-token-manage');
}

function showRpcManage() {
  emit('show-rpc-manage');
}

function showChainManage() {
  emit('show-chain-manage');
}
</script>

<template>
  <div class="status-bar">
    <div class="status-bar-left">
      <div class="status-group">
        <div class="chain-selector-container" style="position: relative">
          <div
            class="status-item status-chain"
            :class="{ 'status-chain-active': chainSelectorExpanded }"
            @click="toggleChainSelector"
            title="点击切换区块链"
          >
            <ChainIcon
              v-if="currentChain?.key"
              :chain-key="currentChain?.key"
              :pic-data="currentChain?.pic_data"
              :alt="currentChain?.name"
              style="width: 14px; height: 14px"
            />
            <span class="status-label">{{ currentChain?.name || '选择区块链' }}</span>
            <Icon
              icon="mdi:chevron-up"
              style="font-size: 12px; margin-left: 4px; transition: transform 0.2s"
              :style="{
                transform: chainSelectorExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
              }"
            />
            <a-tag
              v-if="currentChain?.scan_url"
              size="small"
              class="status-explorer-tag"
              @click.stop="openChainExplorer"
              title="打开区块链浏览器"
            >
              <Icon icon="mdi:open-in-new" />
            </a-tag>
          </div>

          <Transition name="selector-slide">
            <div
              v-if="chainSelectorExpanded"
              class="selector-dropdown selector-dropdown-up"
            >
              <div class="selector-search">
                <a-input
                  ref="chainSearchInputRef"
                  v-model="chainSearchKeyword"
                  placeholder="搜索区块链..."
                  size="small"
                  allow-clear
                >
                  <template #prefix>
                    <Icon
                      icon="mdi:magnify"
                      style="font-size: 14px; color: var(--text-color-quaternary, #c9cdd4)"
                    />
                  </template>
                </a-input>
              </div>
              <div class="selector-list">
                <div
                  v-for="chain in filteredChainOptions"
                  :key="chain.key"
                  class="selector-item"
                  :class="{ 'selector-item-selected': chainValue === chain.key }"
                  @click="handleChainSelect(chain.key)"
                >
                  <ChainIcon
                    :chain-key="chain.key"
                    :pic-data="chain.pic_data"
                    :alt="chain.name"
                    style="width: 18px; height: 18px; flex-shrink: 0"
                  />
                  <span class="selector-item-name">{{ chain.name }}</span>
                  <span class="selector-item-url">{{ chain.scan_url }}</span>
                  <Icon
                    v-if="chainValue === chain.key"
                    icon="mdi:check"
                    style="font-size: 14px; color: var(--primary-6, #165dff); margin-left: auto"
                  />
                </div>
              </div>
            </div>
          </Transition>
        </div>

        <div class="status-divider"></div>

        <div class="token-selector-container" style="position: relative">
          <div
            class="status-item status-token"
            :class="{ 'status-token-active': tokenSelectorExpanded }"
            @click="toggleTokenSelector"
            title="点击切换代币"
          >
            <Icon icon="mdi:coins" style="font-size: 14px" />
            <span class="status-label">{{ currentCoin?.label || '选择代币' }}</span>
            <Icon
              icon="mdi:chevron-up"
              style="font-size: 12px; margin-left: 4px; transition: transform 0.2s"
              :style="{
                transform: tokenSelectorExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
              }"
            />
          </div>

          <Transition name="selector-slide">
            <div
              v-if="tokenSelectorExpanded"
              class="selector-dropdown selector-dropdown-up"
            >
              <div class="selector-search">
                <a-input
                  ref="tokenSearchInputRef"
                  v-model="tokenSearchKeyword"
                  placeholder="搜索代币..."
                  size="small"
                  allow-clear
                >
                  <template #prefix>
                    <Icon
                      icon="mdi:magnify"
                      style="font-size: 14px; color: var(--text-color-quaternary, #c9cdd4)"
                    />
                  </template>
                </a-input>
              </div>
              <div class="selector-list">
                <div
                  v-for="token in filteredCoinOptions"
                  :key="token.key"
                  class="selector-item"
                  :class="{ 'selector-item-selected': coinValue === token.key }"
                  @click="handleTokenSelect(token.key)"
                >
                  <Icon
                    :icon="token.coin_type === 'base' ? 'mdi:circle-slice-8' : 'mdi:coin'"
                    :style="{
                      fontSize: '18px',
                      color:
                        token.coin_type === 'base'
                          ? 'var(--primary-6, #165dff)'
                          : 'var(--success-6, #0fa962)',
                      flexShrink: '0',
                    }"
                  />
                  <span class="selector-item-name">{{ token.label }}</span>
                  <span class="selector-item-symbol">({{ token.symbol }})</span>
                  <Icon
                    v-if="coinValue === token.key"
                    icon="mdi:check"
                    style="font-size: 14px; color: var(--primary-6, #165dff); margin-left: auto"
                  />
                </div>
              </div>
            </div>
          </Transition>
        </div>
      </div>
      <div class="status-divider-vertical"></div>
      <div
        class="status-group status-gas-group"
        v-show="chainValue && chainValue !== 'sol'"
      >
        <Icon icon="mdi:gas-station" style="font-size: 14px; color: var(--text-color-tertiary, #c9cdd4)" />
        <span class="status-gas-label">Gas:</span>
        <span class="status-gas-value"
          >{{ currentChain?.gas_price ?? '--' }}
          <span class="status-gas-unit">Gwei</span></span
        >
      </div>
    </div>
    <div class="status-bar-right">
      <div
        class="status-proxy-indicator"
        :class="{ 'proxy-active': proxyEnabled }"
        :style="{ color: proxyEnabled ? proxyStatusColor : 'var(--text-color-quaternary, #c9cdd4)' }"
        title="代理状态"
        @click="openProxyConfig"
      >
        <Icon icon="mdi:shield-network" style="font-size: 14px" />
        <span class="proxy-status-text">{{ proxyEnabled ? '已启用代理' : '未启动代理' }}</span>
        <span v-if="proxyEnabled" class="proxy-count-text">({{ proxyCount }}个)</span>
      </div>
      <div class="status-divider-vertical"></div>
      <div
        class="status-menu-btn"
        :class="{ 'menu-btn-expanded': isSidePanelExpanded }"
        @click="toggleSidePanel"
        :title="isSidePanelExpanded ? '关闭功能菜单' : '打开功能菜单'"
      >
        <Icon icon="mdi:menu" style="font-size: 15px" />
      </div>
      <a-dropdown>
        <div class="status-settings-btn" title="设置">
          <Icon icon="mdi:cog" style="font-size: 15px" />
        </div>
        <template #content>
          <a-doption @click="showTokenManage" :disabled="!chainValue">
            <template #icon><Icon icon="mdi:coin" /></template>
            代币管理
          </a-doption>
          <a-doption @click="showRpcManage" :disabled="!chainValue">
            <template #icon><Icon icon="mdi:link" /></template>
            RPC管理
          </a-doption>
          <a-doption @click="showChainManage">
            <template #icon><Icon icon="mdi:web" /></template>
            区块链管理
          </a-doption>
          <a-doption @click="openProxyConfig">
            <template #icon><Icon icon="mdi:shield-network" /></template>
            代理配置
            <a-tag :color="proxyEnabled ? proxyStatusColor : '#86909c'" size="small" style="margin-left: 4px">
              {{ proxyEnabled ? proxyCount + '个' : '未启用' }}
            </a-tag>
          </a-doption>
        </template>
      </a-dropdown>
    </div>
  </div>
</template>

<style scoped>
:deep(.arco-dropdown) { overflow: visible !important; max-height: none !important; }
:deep(.arco-dropdown-list) { overflow: visible !important; max-height: none !important; }
:deep(.arco-dropdown ::-webkit-scrollbar) { display: none !important; width: 0 !important; height: 0 !important; }
:deep(.arco-trigger-content) { overflow: visible !important; max-height: none !important; }
:deep(.arco-trigger-popup) { overflow: visible !important; max-height: none !important; }
:deep(.arco-dropdown-wrapper) { overflow: visible !important; max-height: none !important; }
.status-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 40px;
  background: linear-gradient(
    to bottom,
    var(--color-bg-2, #ffffff),
    var(--color-bg-1, #f7f8fa)
  );
  border-top: 1px solid var(--color-border, #e5e6eb);
  box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.04);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  z-index: 1000;
  font-size: 12px;
}

.status-bar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-gas-group {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 10px;
  background: var(--color-fill-1, #f2f3f5);
  border-radius: 12px;
  margin-left: 4px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-color-secondary, #6b778c);
}

.status-label {
  font-weight: 500;
  color: var(--text-color, #1d2129);
}

.status-explorer-tag {
  margin-left: 8px;
  cursor: pointer;
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-fill-1, #f2f3f5);
  border: 1px solid var(--color-border-2, #e5e6eb);
  color: var(--text-color-tertiary, #8c8f94);
  transition: all 0.2s ease;
}

.status-explorer-tag:hover {
  background: var(--primary-1, #e8f1ff);
  border-color: var(--primary-3, #94bfff);
  color: var(--primary-6, #165dff);
}

.status-chain {
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-chain:hover {
  background: linear-gradient(
    135deg,
    var(--primary-1, #e8f1ff),
    var(--color-fill-2, #f2f3f5)
  );
}

.status-chain:hover .status-label {
  color: var(--primary-6, #165dff);
}

.status-chain:hover .status-explorer-tag {
  background: var(--primary-1, #e8f1ff);
  border-color: var(--primary-3, #94bfff);
  color: var(--primary-6, #165dff);
}

.status-token {
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-token:hover {
  background: linear-gradient(
    135deg,
    var(--success-1, #e6fffb),
    var(--color-fill-2, #f2f3f5)
  );
}

.status-token:hover .status-label {
  color: var(--success-6, #0fa962);
}

.status-gas-label {
  color: var(--text-color-tertiary, #8c8f94);
  font-size: 11px;
}

.status-gas-value {
  font-weight: 600;
  color: var(--primary-6, #165dff);
  font-size: 13px;
}

.status-gas-unit {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-color-tertiary, #8c8f94);
}

.status-divider {
  width: 1px;
  height: 18px;
  background: linear-gradient(
    to bottom,
    transparent,
    var(--color-border, #e5e6eb) 30%,
    var(--color-border, #e5e6eb) 70%,
    transparent
  );
  margin: 0 2px;
}

.status-divider-vertical {
  width: 1px;
  height: 24px;
  background: linear-gradient(
    to bottom,
    transparent,
    var(--color-border-2, #d9d9d9) 30%,
    var(--color-border-2, #d9d9d9) 70%,
    transparent
  );
  margin: 0 8px;
}

.status-menu-btn {
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-color-secondary, #6b778c);
}

.status-menu-btn:hover {
  background: var(--color-fill-2, #f2f3f5);
  color: var(--primary-6, #165dff);
}

.status-menu-btn.menu-btn-expanded {
  color: var(--primary-6, #165dff);
  background: var(--primary-1, #e8f1ff);
}

.status-settings-btn {
  cursor: pointer;
  padding: 6px;
  border-radius: 6px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-color-secondary, #6b778c);
}

.status-settings-btn:hover {
  background: var(--color-fill-2, #f2f3f5);
  color: var(--primary-6, #165dff);
  transform: rotate(90deg);
}

.status-proxy-indicator {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 12px;
  background: var(--color-fill-1, #f2f3f5);
  transition: all 0.2s ease;
  cursor: pointer;
}

.status-proxy-indicator:hover {
  background: var(--color-fill-2, #e5e6eb);
}

.status-proxy-indicator.proxy-active {
  background: var(--success-1, #e6fffb);
}

.status-proxy-indicator.proxy-active:hover {
  background: var(--success-2, #b7f0e6);
}

.proxy-status-text {
  font-size: 12px;
  font-weight: 500;
}

.proxy-count-text {
  font-size: 11px;
  color: var(--text-color-tertiary, #8c8f94);
}

.selector-dropdown {
  position: absolute;
  bottom: 100%;
  left: 0;
  background: var(--card-bg, #ffffff);
  border: 1px solid var(--color-border, #e5e6eb);
  border-radius: 12px;
  box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.15), 0 -2px 8px rgba(0, 0, 0, 0.1);
  z-index: 10000;
  margin-bottom: 8px;
  min-width: 360px;
  max-height: 320px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.selector-dropdown-up {
  border-radius: 12px 12px 4px 4px;
}

.selector-search {
  padding: 12px 12px 8px 12px;
  border-bottom: 1px solid var(--color-border-2, #f0f0f0);
  background: var(--color-fill-1, #f7f8fa);
}

.selector-list {
  flex: 1;
  overflow-y: auto;
  max-height: 240px;
  padding: 8px;
}

.selector-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  margin-bottom: 2px;
}

.selector-item:hover {
  background: var(--color-fill-2, #f2f3f5);
}

.selector-item-selected {
  background: var(--primary-1, #e8f1ff);
}

.selector-item-selected:hover {
  background: var(--primary-2, #d4e4ff);
}

.selector-item-name {
  font-weight: 500;
  color: var(--text-color, #1d2129);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.selector-item-url {
  font-size: 11px;
  color: var(--text-color-tertiary, #8c8f94);
}

.selector-item-symbol {
  font-size: 11px;
  color: var(--text-color-secondary, #6b778c);
  margin-left: 4px;
}

.selector-slide-enter-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.selector-slide-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.selector-slide-enter-from,
.selector-slide-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

.selector-slide-enter-to,
.selector-slide-leave-from {
  opacity: 1;
  transform: translateY(0);
}

.status-chain-active {
  background: linear-gradient(
    135deg,
    var(--primary-1, #e8f1ff),
    var(--color-fill-2, #f2f3f5)
  ) !important;
}

.status-chain-active .status-label {
  color: var(--primary-6, #165dff) !important;
}

.status-token-active {
  background: linear-gradient(
    135deg,
    var(--success-1, #e6fffb),
    var(--color-fill-2, #f2f3f5)
  ) !important;
}

.status-token-active .status-label {
  color: var(--success-6, #0fa962) !important;
}
</style>
