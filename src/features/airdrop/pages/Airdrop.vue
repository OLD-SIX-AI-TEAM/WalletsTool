<script setup name="Airdrop">
import { ref } from 'vue';
import WalletManager from '../components/WalletManager.vue';
import BrowserFarm from '../components/BrowserFarm.vue';
import ScriptEditor from '../components/ScriptEditor.vue';
import ExecutionPanel from '../components/ExecutionPanel.vue';
import TaskManager from '../components/TaskManager.vue';
import TaskMonitor from '../components/TaskMonitor.vue';

const activeTab = ref('1');
</script>

<template>
  <div class="airdrop-page">
    <div class="content-container">
      <a-tabs v-model:active-key="activeTab" type="card-gutter" size="large" animation>
        <a-tab-pane key="1" title="Wallet Manager">
          <div class="tab-content">
            <WalletManager />
          </div>
        </a-tab-pane>
        <a-tab-pane key="2" title="Browser Farm">
          <div class="tab-content">
            <BrowserFarm />
          </div>
        </a-tab-pane>
        <a-tab-pane key="3" title="Script Editor">
          <div class="tab-content">
            <ScriptEditor />
          </div>
        </a-tab-pane>
        <a-tab-pane key="4" title="Task Manager">
          <div class="tab-content">
            <TaskManager />
          </div>
        </a-tab-pane>
        <a-tab-pane key="5" title="Task Monitor">
          <div class="tab-content">
            <TaskMonitor />
          </div>
        </a-tab-pane>
        <a-tab-pane key="6" title="Execution Panel">
          <div class="tab-content">
            <ExecutionPanel />
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>
  </div>
</template>

<style scoped>
.airdrop-page {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg-1);
  color: var(--color-text-1);
}

.content-container {
  flex: 1;
  padding: 16px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

:deep(.arco-tabs) {
  height: 100%;
  display: flex;
  flex-direction: column;
}

:deep(.arco-tabs-content) {
  flex: 1;
  padding-top: 16px;
}

:deep(.arco-tabs-pane) {
  height: 100%;
}

.tab-content {
  height: 100%;
  background-color: var(--color-bg-2);
  border-radius: 4px;
  border: 1px solid var(--color-border-2);
  overflow: hidden;
  position: relative;
}

/* Customizing Tabs to match the dark/tech theme */
:deep(.arco-tabs-nav-type-card-gutter .arco-tabs-tab) {
  background-color: var(--color-bg-2);
  border: 1px solid var(--color-border-2);
}

:deep(.arco-tabs-nav-type-card-gutter .arco-tabs-tab-active) {
  background-color: var(--color-bg-1);
  color: rgb(var(--primary-6));
  font-weight: bold;
  border-bottom-color: var(--color-bg-1);
}
</style>
