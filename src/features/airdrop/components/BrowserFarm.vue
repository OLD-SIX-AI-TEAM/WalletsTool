<script setup>
import { ref, onMounted, nextTick, computed } from 'vue';
import { Message, Modal } from '@arco-design/web-vue';
import { 
  IconPlus, 
  IconPublic, 
  IconRight, 
  IconSettings, 
  IconEdit,
  IconRobot,
  IconDelete,
  IconToBottom,
  IconToTop,
  IconImport,
  IconClose,
  IconRefresh
} from '@arco-design/web-vue/es/icon';
import { profileService, initBrowserAutomationTables } from '../services/browserAutomationService';
import { 
  generateBatchProfiles, 
  generateFingerprint,
  PLATFORM_CONFIGS,
  REGION_CONFIGS 
} from '../services/fingerprintGenerator';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';

const profiles = ref([]);
const loading = ref(false);

const loadProfiles = async () => {
  loading.value = true;
  try {
    profiles.value = await profileService.getProfiles();
  } catch (error) {
    console.error('Failed to load profiles:', error);
    Message.error('加载环境配置失败: ' + error);
  } finally {
    loading.value = false;
  }
};

const activeProfile = ref(null);
const isEditing = ref(false);
const showBatchModal = ref(false);

// 批量导入代理相关
const showProxyImportModal = ref(false);
const proxyImportText = ref('');
const proxyImportType = ref('http');
const proxyAssignMode = ref('random'); // random: 随机分配, sequential: 顺序分配

const editingProfileId = ref(null);
const editNameInput = ref(null);
const editNameValue = ref('');

const startEditName = async (profile, event) => {
  event?.stopPropagation();
  editingProfileId.value = profile.id;
  editNameValue.value = profile.name;
  await nextTick();
  const inputEl = Array.isArray(editNameInput.value) ? editNameInput.value[0] : editNameInput.value;
  if (inputEl) {
    inputEl.focus();
    // 使用 setTimeout 确保在 DOM 完全渲染后再选中文本
    setTimeout(() => {
      inputEl.select();
    }, 0);
  }
};

const saveEditName = async () => {
  const trimmedName = editNameValue.value.trim();
  if (!trimmedName) {
    editingProfileId.value = null;
    return;
  }
  const profile = profiles.value.find(p => p.id === editingProfileId.value);
  if (profile) {
    try {
      await profileService.updateProfile({
        id: profile.id,
        name: trimmedName
      });
      profile.name = trimmedName;
      Message.success('名称已更新');
    } catch (error) {
      Message.error('更新名称失败: ' + error);
    }
  }
  editingProfileId.value = null;
};

const cancelEditName = () => {
  editingProfileId.value = null;
};

const handleNameKeydown = (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    saveEditName();
  } else if (event.key === 'Escape') {
    event.preventDefault();
    cancelEditName();
  }
};

// 批量生成配置选项
const batchGenerateOptions = ref({
  count: 10,
  baseName: 'Auto-Profile',
  platformFilter: 'all', // all, desktop, mobile
  regionFilter: 'all',   // all, asia, europe, americas
  ensureUniqueness: true,
});

// 获取现有指纹哈希用于去重
const getExistingFingerprintHashes = () => {
  return profiles.value
    .filter(p => p.fingerprint_hash && p.fingerprint_hash.trim() !== '')
    .map(p => String(p.fingerprint_hash));
};

const handleBatchGenerate = async () => {
  try {
    const existingHashes = batchGenerateOptions.value.ensureUniqueness 
      ? getExistingFingerprintHashes() 
      : [];
    
    // 使用新的指纹生成器生成配置
    const generatedProfiles = generateBatchProfiles(
      batchGenerateOptions.value.count,
      batchGenerateOptions.value.baseName,
      existingHashes
    );
    
    // 创建到数据库
    const newProfiles = [];
    for (const profileData of generatedProfiles) {
      try {
        const profile = await profileService.createProfile(profileData);
        newProfiles.push(profile);
      } catch (e) {
        console.error('创建配置失败:', e);
      }
    }
    
    profiles.value.push(...newProfiles);
    Message.success(`成功生成 ${newProfiles.length} 个唯一配置`);
    showBatchModal.value = false;
  } catch (error) {
    Message.error('批量生成失败: ' + error);
  }
};

// 重新生成指纹
const handleRegenerateFingerprint = async () => {
  if (!activeProfile.value) return;
  
  try {
    const existingHashes = getExistingFingerprintHashes();
    const newFingerprint = generateFingerprint();
    
    // 检查是否重复
    if (existingHashes.includes(newFingerprint.fingerprintHash)) {
      Message.warning('生成的指纹与现有配置重复，请重试');
      return;
    }
    
    // 更新当前编辑的配置
    activeProfile.value.user_agent = newFingerprint.userAgent;
    activeProfile.value.viewport_width = newFingerprint.screenSize.width;
    activeProfile.value.viewport_height = newFingerprint.screenSize.height;
    activeProfile.value.device_scale_factor = newFingerprint.devicePixelRatio;
    activeProfile.value.locale = newFingerprint.locale;
    activeProfile.value.timezone_id = newFingerprint.timezone;
    activeProfile.value.hardware_concurrency = newFingerprint.hardwareConcurrency;
    activeProfile.value.device_memory = newFingerprint.deviceMemory;
    activeProfile.value.color_depth = newFingerprint.colorDepth;
    activeProfile.value.languages = JSON.stringify(newFingerprint.languages);
    activeProfile.value.vendor = newFingerprint.vendor;
    activeProfile.value.gpu_vendor = newFingerprint.gpuVendor;
    activeProfile.value.gpu_renderer = newFingerprint.gpuRenderer;
    activeProfile.value.color_scheme = newFingerprint.colorScheme;
    activeProfile.value.max_touch_points = newFingerprint.touchConfig.maxTouchPoints;
    activeProfile.value.has_touch = newFingerprint.touchConfig.hasTouch;
    activeProfile.value.screen_orientation_angle = newFingerprint.screenOrientation.angle;
    activeProfile.value.screen_orientation_type = newFingerprint.screenOrientation.type;
    activeProfile.value.font_family = newFingerprint.fontFamily;
    activeProfile.value.client_hints_platform = newFingerprint.clientHintsPlatform;
    activeProfile.value.client_hints_platform_version = newFingerprint.clientHintsPlatformVersion;
    activeProfile.value.client_hints_architecture = newFingerprint.clientHintsArchitecture;
    activeProfile.value.client_hints_bitness = newFingerprint.clientHintsBitness;
    activeProfile.value.client_hints_model = newFingerprint.clientHintsModel;
    activeProfile.value.client_hints_wow64 = newFingerprint.clientHintsWow64;
    activeProfile.value.fingerprint_hash = newFingerprint.fingerprintHash;
    activeProfile.value.platform_name = newFingerprint.platformName;
    
    Message.success('指纹已重新生成');
  } catch (error) {
    Message.error('重新生成指纹失败: ' + error);
  }
};

const handleNewProfile = async () => {
  try {
    const newProfile = await profileService.createProfile({
      name: `New Profile ${profiles.value.length + 1}`,
      user_agent: USER_AGENTS[0],
      viewport_width: 1920,
      viewport_height: 1080,
      proxy_type: 'direct',
      canvas_spoof: true,
      webgl_spoof: true,
      audio_spoof: true,
      timezone_spoof: true,
      geolocation_spoof: true,
      font_spoof: true,
      webrtc_spoof: true,
      navigator_override: true,
      webdriver_override: true
    });
    
    profiles.value.push(newProfile);
    handleEdit(newProfile);
    Message.success('新配置已创建');
  } catch (error) {
    Message.error('创建配置失败: ' + error);
  }
};

const handleEdit = async (profile) => {
  if (editingProfileId.value !== null) {
    editingProfileId.value = null;
  }
  activeProfile.value = { ...profile };
  isEditing.value = true;
  
  // 如果没有指纹，自动生成
  if (!activeProfile.value.fingerprint_hash || activeProfile.value.fingerprint_hash.trim() === '') {
    console.log(`[handleEdit] 环境 "${profile.name}" 没有指纹，自动生成...`);
    await autoGenerateFingerprint();
  }
};

// 自动为当前环境生成指纹
const autoGenerateFingerprint = async () => {
  if (!activeProfile.value) return;
  
  try {
    const existingHashes = getExistingFingerprintHashes();
    let newFingerprint;
    let attempts = 0;
    const maxAttempts = 10;
    
    // 尝试生成唯一的指纹
    do {
      newFingerprint = generateFingerprint();
      attempts++;
    } while (existingHashes.includes(newFingerprint.fingerprintHash) && attempts < maxAttempts);
    
    if (attempts >= maxAttempts) {
      console.warn('[autoGenerateFingerprint] 无法生成唯一指纹，使用最后一个生成的');
    }
    
    // 更新当前编辑的配置
    activeProfile.value.user_agent = newFingerprint.userAgent;
    activeProfile.value.viewport_width = newFingerprint.screenSize.width;
    activeProfile.value.viewport_height = newFingerprint.screenSize.height;
    activeProfile.value.device_scale_factor = newFingerprint.devicePixelRatio;
    activeProfile.value.locale = newFingerprint.locale;
    activeProfile.value.timezone_id = newFingerprint.timezone;
    activeProfile.value.hardware_concurrency = newFingerprint.hardwareConcurrency;
    activeProfile.value.device_memory = newFingerprint.deviceMemory;
    activeProfile.value.color_depth = newFingerprint.colorDepth;
    activeProfile.value.languages = JSON.stringify(newFingerprint.languages);
    activeProfile.value.vendor = newFingerprint.vendor;
    activeProfile.value.gpu_vendor = newFingerprint.gpuVendor;
    activeProfile.value.gpu_renderer = newFingerprint.gpuRenderer;
    activeProfile.value.color_scheme = newFingerprint.colorScheme;
    activeProfile.value.max_touch_points = newFingerprint.touchConfig.maxTouchPoints;
    activeProfile.value.has_touch = newFingerprint.touchConfig.hasTouch;
    activeProfile.value.screen_orientation_angle = newFingerprint.screenOrientation.angle;
    activeProfile.value.screen_orientation_type = newFingerprint.screenOrientation.type;
    activeProfile.value.font_family = newFingerprint.fontFamily;
    activeProfile.value.client_hints_platform = newFingerprint.clientHintsPlatform;
    activeProfile.value.client_hints_platform_version = newFingerprint.clientHintsPlatformVersion;
    activeProfile.value.client_hints_architecture = newFingerprint.clientHintsArchitecture;
    activeProfile.value.client_hints_bitness = newFingerprint.clientHintsBitness;
    activeProfile.value.client_hints_model = newFingerprint.clientHintsModel;
    activeProfile.value.client_hints_wow64 = newFingerprint.clientHintsWow64;
    activeProfile.value.fingerprint_hash = newFingerprint.fingerprintHash;
    activeProfile.value.platform_name = newFingerprint.platformName;
    
    // 自动保存到数据库
    const updated = await profileService.updateProfile(activeProfile.value);
    const index = profiles.value.findIndex(p => p.id === updated.id);
    if (index !== -1) {
      profiles.value[index] = updated;
    }
    
    console.log(`[autoGenerateFingerprint] 已为 "${activeProfile.value.name}" 生成并保存指纹: ${newFingerprint.fingerprintHash}`);
  } catch (error) {
    console.error('[autoGenerateFingerprint] 自动生成指纹失败:', error);
  }
};

const handleSave = async () => {
  if (activeProfile.value) {
    try {
      const updated = await profileService.updateProfile(activeProfile.value);
      const index = profiles.value.findIndex(p => p.id === updated.id);
      if (index !== -1) {
        profiles.value[index] = updated;
      }
      isEditing.value = false;
      Message.success('配置已保存');
    } catch (error) {
      Message.error('保存失败: ' + error);
    }
  }
};

const handleDelete = () => {
  if (activeProfile.value) {
    Modal.warning({
      title: '确认删除',
      content: `确定要删除配置 "${activeProfile.value.name}" 吗？`,
      onOk: async () => {
        try {
          await profileService.deleteProfile(activeProfile.value.id);
          const index = profiles.value.findIndex(p => p.id === activeProfile.value.id);
          if (index !== -1) {
            profiles.value.splice(index, 1);
          }
          Message.success('配置已删除');
          isEditing.value = false;
          activeProfile.value = null;
        } catch (error) {
          Message.error('删除失败: ' + error);
        }
      }
    });
  }
};

// 批量删除所有环境
const handleDeleteAll = () => {
  if (profiles.value.length === 0) {
    Message.warning('没有可删除的环境配置');
    return;
  }

  Modal.error({
    title: '危险操作：删除所有环境',
    content: `确定要删除所有 ${profiles.value.length} 个环境配置吗？此操作不可恢复！`,
    okText: '确认删除全部',
    cancelText: '取消',
    onOk: async () => {
      try {
        const total = profiles.value.length;
        let deleted = 0;
        const errors = [];

        // 逐个删除所有配置
        for (const profile of [...profiles.value]) {
          try {
            await profileService.deleteProfile(profile.id);
            deleted++;
          } catch (e) {
            errors.push(`删除 "${profile.name}" 失败: ${e}`);
          }
        }

        // 清空本地列表
        profiles.value = [];
        isEditing.value = false;
        activeProfile.value = null;

        if (errors.length > 0) {
          console.error('批量删除错误:', errors);
        }

        Message.success(`已成功删除 ${deleted}/${total} 个环境配置`);

        // 自动创建一个默认配置
        await createDefaultProfile();
      } catch (error) {
        console.error('批量删除失败:', error);
        Message.error('批量删除失败: ' + error);
      }
    }
  });
};

const handleCancel = () => {
  isEditing.value = false;
  activeProfile.value = null;
};

// 解析代理字符串
const parseProxyLine = (line) => {
  const trimmed = line.trim();
  if (!trimmed) return null;
  
  // 支持格式：
  // 1. host:port
  // 2. host:port:username:password
  // 3. username:password@host:port
  // 4. protocol://host:port
  // 5. protocol://username:password@host:port
  
  let protocol = proxyImportType.value;
  let host = '';
  let port = '';
  let username = '';
  let password = '';
  
  // 检查是否包含协议前缀
  let proxyStr = trimmed;
  const protocolMatch = trimmed.match(/^(http|https|socks4|socks5):\/\//i);
  if (protocolMatch) {
    protocol = protocolMatch[1].toLowerCase();
    proxyStr = trimmed.substring(protocolMatch[0].length);
  }
  
  // 检查是否包含认证信息 @符号格式
  const authMatch = proxyStr.match(/^(.*)@(.*)$/);
  if (authMatch) {
    const auth = authMatch[1];
    const address = authMatch[2];
    const authParts = auth.split(':');
    if (authParts.length >= 2) {
      username = authParts[0];
      password = authParts.slice(1).join(':');
    }
    proxyStr = address;
  }
  
  // 解析地址和端口
  const parts = proxyStr.split(':');
  if (parts.length >= 2) {
    host = parts[0];
    port = parts[1];
    
    // 检查是否还有认证信息（host:port:username:password 格式）
    if (parts.length >= 4 && !username) {
      username = parts[2];
      password = parts.slice(3).join(':');
    }
  } else {
    return null;
  }
  
  // 验证端口
  const portNum = parseInt(port, 10);
  if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
    return null;
  }
  
  return {
    protocol,
    host,
    port: portNum,
    username: username || null,
    password: password || null
  };
};

// 清除所有代理
const handleClearAllProxies = async () => {
  if (profiles.value.length === 0) {
    Message.warning('没有可用的环境配置');
    return;
  }

  const profilesWithProxy = profiles.value.filter(p => p.proxy_type !== 'direct');
  if (profilesWithProxy.length === 0) {
    Message.info('当前没有配置使用代理');
    return;
  }

  Modal.warning({
    title: '确认清除代理',
    content: `确定要清除所有环境配置中的代理设置吗？共有 ${profilesWithProxy.length} 个配置使用了代理。`,
    onOk: async () => {
      try {
        let clearedCount = 0;
        const errors = [];

        for (const profile of profilesWithProxy) {
          try {
            const updated = await profileService.updateProfile({
              id: profile.id,
              proxy_type: 'direct',
              proxy_host: null,
              proxy_port: null,
              proxy_username: null,
              proxy_password: null
            });

            // 更新本地数据
            const index = profiles.value.findIndex(p => p.id === profile.id);
            if (index !== -1) {
              profiles.value[index] = updated;
            }
            clearedCount++;
          } catch (error) {
            errors.push(`清除 "${profile.name}" 的代理失败: ${error}`);
          }
        }

        if (errors.length > 0) {
          console.error('清除代理错误:', errors);
        }

        Message.success(`成功清除 ${clearedCount} 个配置的代理设置`);

        // 如果正在编辑的配置被清除了代理，更新编辑状态
        if (activeProfile.value && activeProfile.value.proxy_type !== 'direct') {
          const updatedActive = profiles.value.find(p => p.id === activeProfile.value.id);
          if (updatedActive) {
            activeProfile.value = { ...updatedActive };
          }
        }
      } catch (error) {
        console.error('清除代理失败:', error);
        Message.error('清除代理失败: ' + error);
      }
    }
  });
};

// 批量导入代理
const handleProxyImport = async () => {
  if (!proxyImportText.value.trim()) {
    Message.warning('请输入代理列表');
    return;
  }
  
  if (profiles.value.length === 0) {
    Message.warning('没有可用的环境配置，请先创建或导入环境配置');
    return;
  }
  
  const lines = proxyImportText.value.split('\n');
  const proxies = [];
  const errors = [];
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line) continue;
    
    const proxy = parseProxyLine(line);
    if (proxy) {
      proxies.push(proxy);
    } else {
      errors.push(`第 ${i + 1} 行格式错误: ${line.substring(0, 50)}${line.length > 50 ? '...' : ''}`);
    }
  }
  
  if (proxies.length === 0) {
    Message.error('没有解析到有效的代理配置');
    return;
  }
  
  if (errors.length > 0) {
    console.warn('代理解析错误:', errors);
  }
  
  // 分配代理到环境配置
  const updatedProfiles = [];
  const assignErrors = [];
  
  // 创建环境配置的副本用于分配
  let targetProfiles = [...profiles.value];
  
  // 如果代理数量多于环境配置，只使用前 N 个代理
  const effectiveProxies = proxies.slice(0, targetProfiles.length);
  
  // 根据分配模式处理
  if (proxyAssignMode.value === 'random') {
    // 随机打乱环境配置顺序
    targetProfiles = targetProfiles.sort(() => Math.random() - 0.5);
  }
  // sequential 模式保持原有顺序
  // same 模式也保持原有顺序，但所有环境使用同一个代理

  // 确定循环次数：same 模式遍历所有环境，其他模式遍历有效代理数量
  const loopCount = proxyAssignMode.value === 'same'
    ? targetProfiles.length
    : effectiveProxies.length;

  for (let i = 0; i < loopCount; i++) {
    const profile = targetProfiles[i];
    // same 模式下所有环境使用第一个代理
    const proxy = proxyAssignMode.value === 'same'
      ? proxies[0]
      : effectiveProxies[i];
    
    try {
      const updated = await profileService.updateProfile({
        id: profile.id,
        proxy_type: proxy.protocol,
        proxy_host: proxy.host,
        proxy_port: proxy.port,
        proxy_username: proxy.username,
        proxy_password: proxy.password
      });
      
      updatedProfiles.push(updated);
      
      // 更新本地数据
      const index = profiles.value.findIndex(p => p.id === profile.id);
      if (index !== -1) {
        profiles.value[index] = updated;
      }
    } catch (error) {
      assignErrors.push(`分配给 "${profile.name}" 失败: ${error}`);
    }
  }
  
  // 显示结果
  let message = `成功为 ${updatedProfiles.length} 个环境配置分配代理`;
  // same 模式下只使用第一个代理，不显示代理未分配提示
  if (proxyAssignMode.value !== 'same' && proxies.length > targetProfiles.length) {
    message += `，${proxies.length - targetProfiles.length} 个代理未分配（环境配置不足）`;
  }
  if (assignErrors.length > 0) {
    message += `，${assignErrors.length} 个分配失败`;
    console.error('代理分配错误:', assignErrors);
  }
  if (errors.length > 0) {
    message += `，${errors.length} 行解析失败`;
  }
  
  Message.success(message);
  showProxyImportModal.value = false;
  proxyImportText.value = '';
};

// 导出配置
const handleExport = async () => {
  if (profiles.value.length === 0) {
    Message.warning('没有可导出的配置');
    return;
  }
  
  try {
    const savePath = await save({
      filters: [{ name: 'JSON', extensions: ['json'] }],
      defaultPath: `browser_profiles_${new Date().toISOString().slice(0, 10)}.json`
    });
    
    if (savePath) {
      const content = new TextEncoder().encode(JSON.stringify(profiles.value, null, 2));
      await writeFile(savePath, content);
      Message.success('配置导出成功');
    }
  } catch (error) {
    console.error('Export error:', error);
    Message.error('导出失败');
  }
};

// 导入配置
const handleImport = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        { name: 'JSON', extensions: ['json'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    });
    
    if (!selected) return;
    
    const content = await readFile(selected);
    const decoder = new TextDecoder();
    const imported = JSON.parse(decoder.decode(content));
    
    if (Array.isArray(imported) && imported.length > 0) {
      let successCount = 0;
      for (const p of imported) {
        try {
          await profileService.createProfile({
            name: p.name + ' (导入)',
            description: p.description,
            user_agent: p.user_agent,
            viewport_width: p.viewport_width,
            viewport_height: p.viewport_height,
            device_scale_factor: p.device_scale_factor,
            locale: p.locale,
            timezone_id: p.timezone_id,
            proxy_type: p.proxy_type,
            proxy_host: p.proxy_host,
            proxy_port: p.proxy_port,
            proxy_username: p.proxy_username,
            proxy_password: p.proxy_password,
            canvas_spoof: p.canvas_spoof,
            webgl_spoof: p.webgl_spoof,
            audio_spoof: p.audio_spoof,
            timezone_spoof: p.timezone_spoof,
            geolocation_spoof: p.geolocation_spoof,
            font_spoof: p.font_spoof,
            webrtc_spoof: p.webrtc_spoof,
            navigator_override: p.navigator_override,
            webdriver_override: p.webdriver_override,
            custom_headers: p.custom_headers,
            headless: p.headless,
            extensions: p.extensions
          });
          successCount++;
        } catch (e) {
          console.error('Failed to import profile:', e);
        }
      }
      
      await loadProfiles();
      Message.success(`成功导入 ${successCount} 个配置`);
    } else {
      Message.warning('文件格式不正确');
    }
  } catch (error) {
    console.error('Import error:', error);
    Message.error('导入失败: ' + error.message);
  }
};

const USER_AGENTS = [
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15"
];

const PROXY_TYPES = [
  { label: 'Direct', value: 'direct' },
  { label: 'HTTP', value: 'http' },
  { label: 'HTTPS', value: 'https' },
  { label: 'SOCKS5', value: 'socks5' }
];

// 代理分配模式选项
const PROXY_ASSIGN_MODES = [
  { label: '随机分配', value: 'random' },
  { label: '顺序分配', value: 'sequential' },
  { label: '使用相同配置', value: 'same' }
];

// 创建默认配置
const createDefaultProfile = async () => {
  try {
    const newProfile = await profileService.createProfile({
      name: `配置 ${profiles.value.length + 1}`,
      user_agent: USER_AGENTS[0],
      viewport_width: 1920,
      viewport_height: 1080,
      proxy_type: 'direct',
      canvas_spoof: true,
      webgl_spoof: true,
      audio_spoof: true,
      timezone_spoof: true,
      geolocation_spoof: true,
      font_spoof: true,
      webrtc_spoof: true,
      navigator_override: true,
      webdriver_override: true
    });
    
    profiles.value.push(newProfile);
    activeProfile.value = { ...newProfile };
    isEditing.value = true;
    Message.success('已创建新配置');
    return newProfile;
  } catch (error) {
    Message.error('创建配置失败: ' + error);
    return null;
  }
};

onMounted(async () => {
  // 初始化表结构（包含迁移）
  try {
    await initBrowserAutomationTables();
  } catch (e) {
    console.log('Tables may already exist:', e);
  }
  await loadProfiles();
  
  // 如果没有配置，自动创建一个
  if (profiles.value.length === 0) {
    await createDefaultProfile();
  } else {
    // 否则选中第一个配置
    handleEdit(profiles.value[0]);
  }
});
</script>

<template>
  <div class="browser-farm">
    <div class="profile-list">
      <div class="list-header">
        <div class="list-header-top">
          <div>
            <h3 class="header-title">环境配置列表</h3>
            <p class="header-subtitle">{{ profiles.length }} 个配置</p>
          </div>
          <a-space>
            <a-button type="primary" size="small" @click="handleNewProfile">
              <template #icon><icon-plus /></template>
            </a-button>
            <a-button status="danger" size="small" @click="handleDeleteAll" :disabled="profiles.length === 0" title="删除所有环境">
              <template #icon><icon-delete /></template>
            </a-button>
          </a-space>
        </div>
      </div>
      
      <div class="list-content" v-loading="loading">
        <div 
          v-for="profile in profiles" 
          :key="profile.id"
          class="profile-item"
          :class="{ active: activeProfile && activeProfile.id === profile.id }"
          @click="handleEdit(profile)"
        >
          <div class="profile-icon">
            <icon-public />
          </div>
          <div class="profile-info">
            <template v-if="editingProfileId === profile.id">
              <input
                ref="editNameInput"
                v-model="editNameValue"
                class="name-edit-input"
                @blur="saveEditName"
                @keydown="handleNameKeydown"
                @click.stop
              />
            </template>
            <template v-else>
              <div class="profile-name editable" @click="(e) => startEditName(profile, e)" title="点击编辑名称">
                {{ profile.name }}
              </div>
            </template>
            <div class="profile-desc">{{ profile.viewport_width }}x{{ profile.viewport_height }} | {{ profile.proxy_type }}</div>
          </div>
          <icon-right class="arrow" />
        </div>
        <div v-if="profiles.length === 0" class="empty-profiles">
          暂无配置，点击"新建"或"批量生成"创建
        </div>
      </div>
    </div>

    <div class="profile-editor" v-if="isEditing && activeProfile">
      <!-- 操作工具栏区域 -->
      <div class="editor-toolbar-area">
        <div class="toolbar-section">
          <span class="toolbar-section-label">代理管理</span>
          <div class="toolbar-section-actions">
            <a-button type="primary" size="small" @click="showProxyImportModal = true">
              <template #icon><icon-import /></template>
              导入代理
            </a-button>
            <a-button type="secondary" size="small" @click="handleClearAllProxies">
              <template #icon><icon-close /></template>
              清除代理
            </a-button>
          </div>
        </div>
        <div class="toolbar-section-divider"></div>
        <div class="toolbar-section">
          <span class="toolbar-section-label">配置管理</span>
          <div class="toolbar-section-actions">
            <a-button type="outline" size="small" @click="showBatchModal = true">
              <template #icon><icon-robot /></template>
              批量生成
            </a-button>
            <a-button type="secondary" size="small" @click="handleImport">
              <template #icon><icon-to-bottom /></template>
              导入配置
            </a-button>
            <a-button type="secondary" size="small" @click="handleExport">
              <template #icon><icon-to-top /></template>
              导出配置
            </a-button>
          </div>
        </div>
      </div>

      <!-- 配置编辑区域 -->
      <div class="editor-header">
        <div class="editor-title-row">
          <h3>编辑配置: {{ activeProfile.name }}</h3>
          <div class="editor-actions-main">
            <a-button status="danger" size="small" @click="handleDelete">
              <template #icon><icon-delete /></template>
              删除
            </a-button>
            <a-button size="small" @click="handleCancel">取消</a-button>
            <a-button type="primary" size="small" @click="handleSave">保存</a-button>
          </div>
        </div>
        
        <!-- 指纹信息展示 -->
        <div class="fingerprint-info-bar" v-if="activeProfile.fingerprint_hash && activeProfile.fingerprint_hash.trim() !== ''">
          <div class="fingerprint-tags">
            <a-tag size="small" color="arcoblue">{{ activeProfile.platform_name || 'Unknown Platform' }}</a-tag>
            <a-tag size="small" color="green">{{ activeProfile.locale }}</a-tag>
            <a-tag size="small" color="orange">{{ activeProfile.timezone_id }}</a-tag>
            <a-tag size="small" color="purple" v-if="activeProfile.gpu_vendor">{{ activeProfile.gpu_vendor }}</a-tag>
          </div>
          <a-button type="outline" size="mini" @click="handleRegenerateFingerprint">
            <template #icon><icon-refresh /></template>
            重新生成指纹
          </a-button>
        </div>
        <div class="fingerprint-info-bar" v-else>
          <a-tag size="small" color="gray">未生成指纹</a-tag>
          <a-button type="outline" size="mini" @click="handleRegenerateFingerprint">
            <template #icon><icon-refresh /></template>
            生成指纹
          </a-button>
        </div>
      </div>

      <div class="editor-form">
        <a-form :model="activeProfile" layout="vertical">
          <a-form-item label="配置名称">
            <a-input v-model="activeProfile.name" />
          </a-form-item>
          
          <a-form-item label="描述">
            <a-textarea v-model="activeProfile.description" :auto-size="{ minRows: 2, maxRows: 4 }" placeholder="配置描述..." />
          </a-form-item>

          <a-form-item label="User Agent">
            <a-textarea v-model="activeProfile.user_agent" :auto-size="{ minRows: 2, maxRows: 4 }" />
          </a-form-item>

          <a-row :gutter="16">
            <a-col :span="8">
              <a-form-item label="视口宽度">
                <a-input-number v-model="activeProfile.viewport_width" :min="320" :max="3840" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="视口高度">
                <a-input-number v-model="activeProfile.viewport_height" :min="240" :max="2160" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="设备缩放">
                <a-input-number v-model="activeProfile.device_scale_factor" :min="0.5" :max="3" :step="0.1" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-row :gutter="16">
            <a-col :span="12">
              <a-form-item label="语言">
                <a-input v-model="activeProfile.locale" placeholder="en-US" />
              </a-form-item>
            </a-col>
            <a-col :span="12">
              <a-form-item label="时区">
                <a-input v-model="activeProfile.timezone_id" placeholder="America/New_York" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-divider orientation="left">代理配置</a-divider>
          
          <a-row :gutter="16">
            <a-col :span="12">
              <a-form-item label="代理类型">
                <a-select v-model="activeProfile.proxy_type">
                  <a-option v-for="type in PROXY_TYPES" :key="type.value" :value="type.value">{{ type.label }}</a-option>
                </a-select>
              </a-form-item>
            </a-col>
            <a-col :span="12">
              <a-form-item label="代理主机">
                <a-input v-model="activeProfile.proxy_host" placeholder="proxy.example.com" />
              </a-form-item>
            </a-col>
          </a-row>
          
          <a-row :gutter="16">
            <a-col :span="8">
              <a-form-item label="代理端口">
                <a-input-number v-model="activeProfile.proxy_port" :min="1" :max="65535" placeholder="8080" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="代理用户名">
                <a-input v-model="activeProfile.proxy_username" placeholder="可选" />
              </a-form-item>
            </a-col>
            <a-col :span="8">
              <a-form-item label="代理密码">
                <a-input-password v-model="activeProfile.proxy_password" placeholder="可选" />
              </a-form-item>
            </a-col>
          </a-row>

          <a-divider orientation="left">指纹保护 (Anti-Detect)</a-divider>

          <div class="fingerprint-switches">
            <div class="switch-item">
              <a-switch v-model="activeProfile.canvas_spoof" />
              <span>Canvas 指纹混淆</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webgl_spoof" />
              <span>WebGL 渲染伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.audio_spoof" />
              <span>Audio Context 噪音</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.timezone_spoof" />
              <span>时区伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.geolocation_spoof" />
              <span>地理位置伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.font_spoof" />
              <span>字体伪装</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webrtc_spoof" />
              <span>WebRTC 防泄漏</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.navigator_override" />
              <span>Navigator 覆盖</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.webdriver_override" />
              <span>WebDriver 覆盖</span>
            </div>
          </div>

          <a-divider orientation="left">高级选项</a-divider>

          <div class="advanced-options">
            <div class="switch-item">
              <a-switch v-model="activeProfile.headless" />
              <span>无头模式</span>
            </div>
            <div class="switch-item">
              <a-switch v-model="activeProfile.is_default" />
              <span>设为默认</span>
            </div>
          </div>
        </a-form>
      </div>
    </div>
    
    <div class="empty-state" v-else>
      <icon-settings style="font-size: 48px; color: var(--color-text-4)" />
      <p>请选择左侧配置进行编辑</p>
    </div>

    <a-modal v-model:visible="showBatchModal" title="批量生成环境配置" @ok="handleBatchGenerate" width="600px">
      <a-form layout="vertical">
        <a-row :gutter="16">
          <a-col :span="12">
            <a-form-item label="生成数量">
              <a-input-number v-model="batchGenerateOptions.count" :min="1" :max="1000" style="width: 100%" />
            </a-form-item>
          </a-col>
          <a-col :span="12">
            <a-form-item label="配置名称前缀">
              <a-input v-model="batchGenerateOptions.baseName" placeholder="Auto-Profile" />
            </a-form-item>
          </a-col>
        </a-row>
        
        <a-form-item>
          <a-checkbox v-model="batchGenerateOptions.ensureUniqueness">
            确保指纹唯一性（自动去重）
          </a-checkbox>
        </a-form-item>
        
        <a-divider orientation="left">生成内容预览</a-divider>
        
        <div style="color: var(--color-text-3); font-size: 12px;">
          <p>将智能生成以下配置项（确保所有信息相互对应）：</p>
          <a-row :gutter="8">
            <a-col :span="12">
              <ul style="margin: 0; padding-left: 16px;">
                <li>User Agent（与平台匹配）</li>
                <li>平台信息（Windows/macOS/Linux/iOS/Android）</li>
                <li>GPU 信息（与平台匹配）</li>
                <li>分辨率（与设备类型匹配）</li>
                <li>触摸配置（与设备类型匹配）</li>
              </ul>
            </a-col>
            <a-col :span="12">
              <ul style="margin: 0; padding-left: 16px;">
                <li>语言列表（与区域匹配）</li>
                <li>时区（与区域匹配）</li>
                <li>字体（与平台匹配）</li>
                <li>硬件信息（CPU核心/内存）</li>
                <li>Client Hints 信息</li>
              </ul>
            </a-col>
          </a-row>
          <p style="margin-top: 8px; color: rgb(var(--primary-6));">
            <icon-refresh style="margin-right: 4px;" />
            所有指纹保护选项默认开启，生成的指纹具有强抗性
          </p>
        </div>
      </a-form>
    </a-modal>

    <!-- 批量导入代理对话框 -->
    <a-modal 
      v-model:visible="showProxyImportModal" 
      title="批量导入代理" 
      @ok="handleProxyImport"
      :okButtonProps="{ disabled: !proxyImportText.trim() }"
      width="600px"
    >
      <a-form layout="vertical">
        <a-form-item label="默认代理类型">
          <a-select v-model="proxyImportType">
            <a-option v-for="type in PROXY_TYPES.filter(t => t.value !== 'direct')" :key="type.value" :value="type.value">{{ type.label }}</a-option>
          </a-select>
        </a-form-item>
        
        <a-form-item label="分配模式">
          <a-radio-group v-model="proxyAssignMode">
            <a-radio v-for="mode in PROXY_ASSIGN_MODES" :key="mode.value" :value="mode.value">{{ mode.label }}</a-radio>
          </a-radio-group>
          <div style="color: var(--color-text-3); font-size: 12px; margin-top: 4px;">
            <span v-if="proxyAssignMode === 'random'">代理将随机分配给现有环境配置</span>
            <span v-else-if="proxyAssignMode === 'sequential'">代理将按顺序分配给现有环境配置</span>
            <span v-else>所有环境配置将使用相同的代理（只取第一个代理配置）</span>
          </div>
        </a-form-item>
        
        <a-form-item label="代理列表">
          <a-textarea 
            v-model="proxyImportText" 
            :auto-size="{ minRows: 8, maxRows: 15 }" 
            placeholder="每行一个代理，支持以下格式：&#10;host:port&#10;host:port:username:password&#10;username:password@host:port&#10;http://host:port&#10;http://username:password@host:port&#10;&#10;示例：&#10;192.168.1.1:8080&#10;192.168.1.2:8080:user:pass&#10;user:pass@192.168.1.3:8080"
          />
        </a-form-item>
        
        <div style="color: var(--color-text-3); font-size: 12px;">
          <p>说明：</p>
          <ul>
            <li>支持 HTTP、HTTPS、SOCKS5 代理</li>
            <li>如果代理数量多于环境配置，多余的代理将被忽略</li>
            <li>代理将分配到现有的环境配置中</li>
          </ul>
        </div>
      </a-form>
    </a-modal>
  </div>
</template>

<style scoped>
.browser-farm {
  height: 100%;
  display: flex;
  gap: 10px;
}

.profile-list {
  width: 280px;
  background: var(--color-bg-2);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
}

.list-header {
  padding: 15px;
  border-bottom: 1px solid var(--color-border);
}

.list-header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.header-title {
  margin: 0 0 4px 0;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.header-subtitle {
  margin: 0;
  font-size: 12px;
  color: var(--color-text-3);
}

.list-content {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

/* 自定义滚动条样式 */
.list-content::-webkit-scrollbar {
  width: 6px;
}

.list-content::-webkit-scrollbar-track {
  background: transparent;
}

.list-content::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 3px;
}

.list-content::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox 滚动条 */
.list-content {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) transparent;
}

.profile-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-2);
}

.profile-item:hover {
  background: var(--color-fill-2);
}

.profile-item.active {
  background: rgba(var(--primary-6), 0.1);
}

.profile-icon {
  width: 32px;
  height: 32px;
  background: rgba(var(--primary-6), 0.1);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgb(var(--primary-6));
}

.profile-info {
  flex: 1;
}

.profile-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
}

.profile-name.editable {
  cursor: text;
  padding: 2px 4px;
  margin: -2px -4px;
  border-radius: 4px;
  transition: all 0.2s;
}

.profile-name.editable:hover {
  background: var(--color-fill-2);
}

.name-edit-input {
  width: 100%;
  background: var(--color-bg-1);
  border: 1px solid rgb(var(--primary-6));
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-1);
  outline: none;
  line-height: 1.2;
}

.name-edit-input:focus {
  border-color: rgb(var(--primary-6));
  box-shadow: 0 0 0 2px rgba(var(--primary-6), 0.2);
}

.profile-desc {
  font-size: 12px;
  color: var(--color-text-3);
}

.arrow {
  color: var(--color-text-4);
  font-size: 12px;
}

.empty-profiles {
  text-align: center;
  padding: 30px 20px;
  color: var(--color-text-3);
  font-size: 12px;
}

.profile-editor {
  flex: 1;
  background: var(--color-bg-2);
  border-radius: 8px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--color-border);
  overflow: hidden;
}

/* 操作工具栏区域 */
.editor-toolbar-area {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 12px 16px;
  background: var(--color-fill-2);
  border-radius: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
  border: 1px solid var(--color-border);
}

/* 配置编辑区域容器 */
.editor-content-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.toolbar-section {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-section-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--color-text-2);
  white-space: nowrap;
}

.toolbar-section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toolbar-section-divider {
  width: 1px;
  height: 24px;
  background: var(--color-border);
}

/* 配置编辑区域标题栏 */
.editor-header {
  display: flex;
  flex-direction: column;
  padding: 16px;
  background: var(--color-bg-2);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  margin-bottom: 12px;
  gap: 12px;
}

.editor-title-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}

.editor-header h3 {
  margin: 0;
  color: var(--color-text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-shrink: 0;
  font-size: 16px;
}

.editor-actions-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* 指纹信息栏 */
.fingerprint-info-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--color-fill-2);
  border-radius: 6px;
  flex-wrap: wrap;
}

.fingerprint-tags {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.editor-form {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  background: var(--color-bg-2);
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

/* 编辑表单自定义滚动条 */
.editor-form::-webkit-scrollbar {
  width: 6px;
}

.editor-form::-webkit-scrollbar-track {
  background: transparent;
}

.editor-form::-webkit-scrollbar-thumb {
  background: var(--color-text-4);
  border-radius: 3px;
}

.editor-form::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-3);
}

/* Firefox 滚动条 */
.editor-form {
  scrollbar-width: thin;
  scrollbar-color: var(--color-text-4) transparent;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-4);
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 2px dashed var(--color-border);
}

/* 指纹保护开关布局 */
.fingerprint-switches {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
  margin-bottom: 10px;
}

.switch-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: var(--color-fill-2);
  border-radius: 6px;
  transition: background 0.2s;
}

.switch-item:hover {
  background: var(--color-fill-3);
}

.switch-item span {
  font-size: 13px;
  color: var(--color-text-2);
  white-space: nowrap;
}

/* 高级选项布局 */
.advanced-options {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}
</style>
