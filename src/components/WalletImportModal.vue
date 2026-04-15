<script setup name="wallet-import-modal">
import { ref, computed, nextTick } from 'vue';
import { ethers } from 'ethers';
import { Keypair, PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';
import { Notification } from '@arco-design/web-vue';
import CodeEditor from './CodeEditor.vue';

// Props
const props = defineProps({
  title: {
    type: String,
    default: '钱包信息录入'
  },
  ecosystem: {
    type: String,
    default: 'evm'
  }
});

// 内部管理的visible状态
const visible = ref(false);

// Emits
const emit = defineEmits(['confirm', 'cancel']);

// 响应式数据
const privateKeyText = ref('');
const addressText = ref('');
const validationErrors = ref([]);
const errorsExpanded = ref(false);
const privateKeyErrorLines = ref([]);
const addressErrorLines = ref([]);
const importLoading = ref(false);

// 编辑器引用
const privateKeyEditorRef = ref(null);
const addressEditorRef = ref(null);
const isScrollSyncing = ref(false);
const isLineSelectionSyncing = ref(false);

// 计算属性：动态提示文本
const privateKeyPlaceholder = computed(() => {
  return props.ecosystem === 'solana' 
    ? '请输入私钥，一行一个\n格式：Base58编码字符串\n示例：5Mokk...' 
    : '请输入私钥，一行一个\n格式：0x开头的64位十六进制字符串\n示例：0x1234567890abcdef...';
});

const addressPlaceholder = computed(() => {
  return props.ecosystem === 'solana'
    ? '请输入接收地址，一行一个\n格式：Base58编码地址\n示例：Gwgh...'
    : '请输入接收地址，一行一个\n格式：0x开头的40位十六进制地址\n示例：0x742d35Cc6634C0532925a3b8D4...';
});

// 验证私钥格式
function validatePrivateKey(privateKey) {
  try {
    const cleanKey = privateKey.trim();
    
    if (props.ecosystem === 'solana') {
      // Solana 私钥验证 (Base58)
      try {
        const secretKey = bs58.decode(cleanKey);
        if (secretKey.length !== 64) return false;
        Keypair.fromSecretKey(secretKey);
        return true;
      } catch {
        return false;
      }
    } else {
      // EVM 私钥验证
      const keyPart = cleanKey.startsWith('0x') ? cleanKey.slice(2) : cleanKey;
      // 检查是否为64位十六进制字符串
      if (!/^[0-9a-fA-F]{64}$/.test(keyPart)) {
        return false;
      }
      // 尝试创建钱包实例验证私钥有效性
      new ethers.Wallet(cleanKey.startsWith('0x') ? cleanKey : '0x' + cleanKey);
      return true;
    }
  } catch (error) {
    return false;
  }
}

// 验证地址格式
function validateAddress(address) {
  try {
    // 检查地址是否为空或undefined
    if (!address || typeof address !== 'string') {
      return false;
    }

    const trimmedAddress = address.trim();

    if (props.ecosystem === 'solana') {
      // Solana 地址验证
      try {
        const pubKey = new PublicKey(trimmedAddress);
        return PublicKey.isOnCurve(pubKey.toBytes());
      } catch {
        return false;
      }
    } else {
      // EVM 地址验证
      // 检查是否以0x开头且长度为42
      if (!trimmedAddress.startsWith('0x') || trimmedAddress.length !== 42) {
        return false;
      }

      // 检查除0x外的部分是否为有效的十六进制字符
      const hexPart = trimmedAddress.slice(2);
      if (!/^[0-9a-fA-F]{40}$/.test(hexPart)) {
        return false;
      }

      // 使用ethers.js进行最终验证
      return ethers.isAddress(trimmedAddress);
    }
  } catch (error) {
    return false;
  }
}

// 验证导入数据
function validateImportData() {
  const privateKeys = privateKeyText.value.split('\n').filter(line => line.trim() !== '');
  const addresses = addressText.value.split('\n').filter(line => line.trim() !== '');
  
  validationErrors.value = [];
  const errorLines = new Set();
  
  // 检查行数是否匹配（只有当两个字段都有内容时才检查）
  if (privateKeys.length > 0 && addresses.length > 0 && privateKeys.length !== addresses.length) {
    validationErrors.value.push(`私钥行数(${privateKeys.length})与地址行数(${addresses.length})不匹配`);
  }
  
  // 验证私钥格式
  privateKeys.forEach((key, index) => {
    const trimmedKey = key.trim();
    if (trimmedKey && !validatePrivateKey(trimmedKey)) {
      validationErrors.value.push(`第${index + 1}行私钥格式错误`);
      errorLines.add(index + 1);
    }
  });
  
  // 验证地址格式
  addresses.forEach((addr, index) => {
    const trimmedAddr = addr.trim();
    if (trimmedAddr && !validateAddress(trimmedAddr)) {
      validationErrors.value.push(`第${index + 1}行地址格式错误`);
      errorLines.add(index + 1);
    }
  });
  
  // 同步错误行号到两个编辑器
  privateKeyErrorLines.value = Array.from(errorLines);
  addressErrorLines.value = Array.from(errorLines);
}

// 切换错误信息展开状态
function toggleErrorsExpanded() {
  errorsExpanded.value = !errorsExpanded.value;
}

// 处理私钥编辑器滚动事件
function handlePrivateKeyScroll(scrollInfo) {
  if (isScrollSyncing.value) return;
  
  isScrollSyncing.value = true;
  if (addressEditorRef.value && addressEditorRef.value.syncScroll) {
    addressEditorRef.value.syncScroll(scrollInfo);
  }
  nextTick(() => {
    isScrollSyncing.value = false;
  });
}

// 处理地址编辑器滚动事件
function handleAddressScroll(scrollInfo) {
  if (isScrollSyncing.value) return;
  
  isScrollSyncing.value = true;
  if (privateKeyEditorRef.value && privateKeyEditorRef.value.syncScroll) {
    privateKeyEditorRef.value.syncScroll(scrollInfo);
  }
  nextTick(() => {
    isScrollSyncing.value = false;
  });
}

// 处理私钥编辑器行号选择事件
function handlePrivateKeyLineSelect(lineSelectionInfo) {
  if (isLineSelectionSyncing.value) return;
  
  isLineSelectionSyncing.value = true;
  if (addressEditorRef.value && addressEditorRef.value.syncLineSelection) {
    addressEditorRef.value.syncLineSelection(lineSelectionInfo);
  }
  nextTick(() => {
    isLineSelectionSyncing.value = false;
  });
}

// 处理地址编辑器行号选择事件
function handleAddressLineSelect(lineSelectionInfo) {
  if (isLineSelectionSyncing.value) return;
  
  isLineSelectionSyncing.value = true;
  if (privateKeyEditorRef.value && privateKeyEditorRef.value.syncLineSelection) {
    privateKeyEditorRef.value.syncLineSelection(lineSelectionInfo);
  }
  nextTick(() => {
    isLineSelectionSyncing.value = false;
  });
}

// 处理弹窗取消
function handleCancel() {
  visible.value = false;
  emit('cancel');
  // 重置数据
  privateKeyText.value = '';
  addressText.value = '';
  validationErrors.value = [];
  errorsExpanded.value = false;
  
  // 提示用户清除剪贴板历史
  // setTimeout(() => {
  //   Notification.warning({
  //     title: '安全提示',
  //     content: '为防止私钥泄露，建议您手动清除 Win+V 剪贴板历史记录',
  //     duration: 5000,
  //     position: 'topLeft'
  //   });
  // }, 300);
}

// 处理弹窗确认前的验证
const handleBeforeOk = async () => {
  // 验证数据
  validateImportData();
  
  if (validationErrors.value.length > 0) {
    return false;
  }
  
   importLoading.value = true;

   try {
     const privateKeys = privateKeyText.value.split('\n').filter(line => line.trim() !== '');
     const addresses = addressText.value.split('\n').filter(line => line.trim() !== '');

     // 添加一个短暂的延迟，让用户看到loading效果
     await new Promise(resolve => setTimeout(resolve, 800));

     // 清空剪贴板
     await clearClipboard();

     // 发送确认事件，传递数据
    emit('confirm', {
      privateKeys: privateKeys.map(key => key.trim()),
      addresses: addresses.map(addr => addr.trim())
    });
    
     // 关闭弹窗
     visible.value = false;

     // 提示用户清除剪贴板历史
    //  setTimeout(() => {
    //    Notification.warning({
    //      title: '安全提示',
    //      content: '为防止私钥泄露，建议您手动清除 Win+V 剪贴板历史记录',
    //      duration: 5000,
    //      position: 'topLeft'
    //    });
    //  }, 300);
     
     // 重置数据
    privateKeyText.value = '';
    addressText.value = '';
    validationErrors.value = [];
    errorsExpanded.value = false;
    
    return true;
  } catch (error) {
    console.error('处理导入数据失败:', error);
    return false;
  } finally {
      importLoading.value = false;
  }
};

// 清空剪贴板功能
async function clearClipboard() {
  try {
    await navigator.clipboard.writeText('');
  } catch (error) {
    console.error('清空剪贴板失败:', error);
  }
}

// 显示弹窗的方法
function show() {
  // 重置数据
  privateKeyText.value = '';
  addressText.value = '';
  validationErrors.value = [];
  errorsExpanded.value = false;

  // 显示弹窗
  visible.value = true;
}

// 暴露方法给父组件
defineExpose({
  show
});
</script>

<template>
  <a-modal 
    class="import-modal" 
    :visible="visible" 
    :width="1250" 
    :title="title" 
    @cancel="handleCancel"
    :on-before-ok="handleBeforeOk" 
    :confirm-loading="importLoading"
    :ok-text="importLoading ? '正在处理中...' : '确认导入并清空剪贴板'"
    :cancel-button-props="{ disabled: importLoading }"
    :mask-closable="!importLoading"
    :closable="!importLoading"
  >
    <a-alert type="warning" closable style="margin-bottom: 10px">
      当前方式存在剪切板泄露私钥数据风险，为了防止电脑中其他恶意程序监听剪切板导致私钥泄露不推荐使用这个方式！！
    </a-alert>
    <div
      :style="{ display: 'flex', gap: '10px', marginTop: '10px', width: '1200px', height: '500px', position: 'relative' }">
      <!-- Loading遮罩层 -->
      <div v-if="importLoading" class="loading-overlay">
        <a-spin size="large">
          <template #tip>
            <div class="loading-text">正在处理钱包信息，请稍候...</div>
          </template>
        </a-spin>
      </div>
      
      <!-- 左侧：私钥输入 -->
      <div style="width: 740px; height: 100%; display: flex; flex-direction: column;">
        <div class="input-label" style="margin-bottom: 8px; font-weight: 500;">发送方私钥</div>
        <CodeEditor 
          ref="privateKeyEditorRef"
          v-model="privateKeyText" 
          :error-lines="privateKeyErrorLines"
          :disabled="importLoading"
          :placeholder="privateKeyPlaceholder" 
          @input="validateImportData"
          @scroll="handlePrivateKeyScroll"
          @line-select="handlePrivateKeyLineSelect"
          style="flex: 1; height: 100%;" />
      </div>

      <!-- 右侧：接收地址输入 -->
      <div style="width: 450px; height: 100%; display: flex; flex-direction: column;">
        <div class="input-label" style="margin-bottom: 8px; font-weight: 500;">接收地址</div>
        <CodeEditor 
          ref="addressEditorRef"
          v-model="addressText" 
          :error-lines="addressErrorLines"
          :disabled="importLoading"
          :placeholder="addressPlaceholder" 
          @input="validateImportData"
          @scroll="handleAddressScroll"
          @line-select="handleAddressLineSelect"
          style="flex: 1; height: 100%;" />
      </div>
    </div>

    <!-- 验证错误提示 -->
    <div v-if="validationErrors.length > 0" style="margin-top: 15px;">
      <a-alert style="padding: 5px 15px;" type="error" :title="`发现 ${validationErrors.length} 个问题`" :show-icon="true">
        <ul style="margin: 8px 0 0 0; padding-left: 20px;">
          <li v-for="(error, index) in displayedErrors" :key="error"
            style="margin-bottom: 4px; color: #f53f3f; font-size: 12px;">{{ error }}</li>
        </ul>
        <div v-if="validationErrors.length > 3" style="margin-top: 10px; text-align: center;">
          <a-button type="text" size="small" @click="toggleErrorsExpanded" style="color: #165dff;font-size: 12px;">
            {{ errorsExpanded ? '⬆️收起' : '⬇️展开全部' }}
          </a-button>
        </div>
      </a-alert>
    </div>
  </a-modal>
</template>

<style scoped>
.import-modal {
  /* 弹窗样式可以根据需要添加 */
}

.input-label {
  font-weight: 500;
  color: var(--text-color, #1d2129);
  margin-bottom: 8px;
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--loading-bg, #f7f8fa);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  border-radius: 6px;
  backdrop-filter: blur(2px);
}

.loading-text {
  margin-top: 12px;
  font-size: 14px;
  color: var(--text-color, #1d2129);
  font-weight: 500;
}
</style>
