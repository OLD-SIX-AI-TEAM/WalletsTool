<script setup>
import { ref, watch, nextTick } from 'vue';
import { Message } from '@arco-design/web-vue';
import { openSealedSecret } from '@/utils/secretCrypto';

const props = defineProps({
  visible: {
    type: Boolean,
    default: false,
  },
  title: {
    type: String,
    default: '安全复制',
  },
  sealedPrivateKey: {
    type: String,
    default: '',
  },
  password: {
    type: String,
    default: '',
  },
  transportToken: {
    type: String,
    default: '',
  },
  transportAesKey: {
    type: Object,
    default: null,
  },
});

const emit = defineEmits(['update:visible']);
const copying = ref(false);
const copied = ref(false);
const suffixChars = ref([]);

const hexToBytes = (hex) => {
  const s = String(hex || '').trim();
  if (!s || s.length % 2 !== 0) throw new Error('hex 格式错误');
  const out = new Uint8Array(s.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(s.slice(i * 2, i * 2 + 2), 16);
  return out;
};
const base64ToBytes = (b64) => {
  const binary = atob(b64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
};

const openTransportSecret = async (sealed) => {
  const s = String(sealed ?? '').trim();
  if (!s.startsWith('t1:')) throw new Error('必须使用加密格式传输');
  const parts = s.slice(3).split(':');
  if (parts.length !== 3) throw new Error('密文格式错误');
  const [token, ivHex, cipherB64] = parts;
  if (props.transportToken && token !== props.transportToken) throw new Error('传输令牌不匹配');
  if (!props.transportAesKey) throw new Error('缺少传输密钥');
  const iv = hexToBytes(ivHex);
  const cipher = base64ToBytes(cipherB64);
  const plainBytes = new Uint8Array(
    await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, props.transportAesKey, cipher)
  );
  return new TextDecoder().decode(plainBytes);
};

const openAnySecret = async () => {
  if (!props.sealedPrivateKey) return '';
  if (props.sealedPrivateKey.startsWith('t1:')) return openTransportSecret(props.sealedPrivateKey);
  if (!props.password) return '';
  return openSealedSecret(props.sealedPrivateKey, props.password);
};

const doCopyIncomplete = async () => {
  if (!props.sealedPrivateKey || copying.value) return;
  copying.value = true;
  try {
    const plain = await openAnySecret();
    if (!plain) return;
    const incompleteKey = plain.slice(0, -6);
    await navigator.clipboard.writeText(incompleteKey);
    copied.value = true;
    Message.success(`已复制私钥前${incompleteKey.length}位，请手动补充后6位`);
  } catch (error) {
    Message.error({ content: '复制失败', position: 'top', offset: 500 });
  } finally {
    copying.value = false;
  }
};

const doCopyFull = async () => {
  if (!props.sealedPrivateKey || copying.value) return;
  copying.value = true;
  try {
    const plain = await openAnySecret();
    if (!plain) return;
    await navigator.clipboard.writeText(plain);
    Message.warning({
      content: '已复制完整私钥，请注意剪贴板泄露风险！',
      position: 'top',
      offset: 500,
    });
  } catch (error) {
    Message.error({ content: '复制失败', position: 'top', offset: 500 });
  } finally {
    copying.value = false;
  }
};

const handleClose = () => {
  copied.value = false;
  suffixChars.value = [];
  emit('update:visible', false);
};

const decryptAndShow = async () => {
  if (!props.sealedPrivateKey) return;
  try {
    const plain = await openAnySecret();
    if (!plain) return;
    const suffix = plain.slice(-6);
    suffixChars.value = suffix.split('');
    setTimeout(() => doCopyIncomplete(), 100);
  } catch (e) {
    console.error('解密失败:', e);
  }
};

watch(() => props.visible, (val) => {
  if (val) {
    nextTick(() => decryptAndShow());
  }
});
</script>

<template>
  <a-modal
    :visible="visible"
    :title="title"
    :footer="false"
    :maskClosable="true"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="secret-reveal-content">
      <div class="warning-text">
        为保障你的资产安全，复制后请手动补充私钥末尾6位字符，确保私钥可用
      </div>
      <div class="suffix-boxes">
        <div
          v-for="(char, index) in suffixChars"
          :key="index"
          class="suffix-box"
        >
          {{ char }}
        </div>
      </div>
      <div
        v-if="sealedPrivateKey"
        class="copy-full-link"
        :class="{ copied: copied }"
        @click="doCopyFull"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
        </svg>
        <span>复制完整私钥（存在剪贴板泄露风险）</span>
      </div>
      <a-button
        type="primary"
        long
        style="width: 100%; margin-top: 16px;"
        @click="handleClose"
      >
        确定
      </a-button>
    </div>
  </a-modal>
</template>

<style scoped>
.secret-reveal-content {
  text-align: center;
  padding: 10px 0;
}

.warning-text {
  color: gray;
  margin-bottom: 16px;
  font-size: 16px;
}

.suffix-boxes {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-bottom: 20px;
}

.suffix-box {
  width: 36px;
  height: 44px;
  border: 2px solid #165dff;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  font-weight: bold;
  color: #165dff;
  background: #f7f8fa;
  flex-shrink: 0;
}

.copy-full-link {
  color: #86909c;
  font-size: 13px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.copy-full-link:hover {
  color: #7c7c7c;
}

.copy-full-link.copied {
  color: #7c7c7c;
}
</style>
