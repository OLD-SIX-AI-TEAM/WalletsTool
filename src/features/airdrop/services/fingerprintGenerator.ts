/**
 * 增强版浏览器指纹生成器
 * 生成独一无二且强抗性的浏览器指纹
 * 
 * 设计原则：所有指纹信息必须相互对应，确保一致性
 * - User-Agent 决定 platform、oscpu、vendor
 * - platform 决定 GPU 类型（macOS用Apple GPU，Windows/Linux用NVIDIA/Intel/AMD）
 * - locale 决定 languages、timezone 的合理组合
 * - screenSize 决定 devicePixelRatio、touchConfig（移动端vs桌面端）
 */

// ==================== 平台配置定义 ====================

interface PlatformConfig {
  name: string;
  platform: string;
  oscpu: string;
  userAgentPatterns: string[];
  gpuVendor: 'Apple' | 'NVIDIA' | 'Intel' | 'AMD';
  clientHintsPlatform: string;
  clientHintsPlatformVersion: string;
  clientHintsArchitecture: string;
  clientHintsBitness: string;
  fontFamilies: string[];
  // 该平台的典型分辨率
  typicalScreenSizes: { width: number; height: number }[];
  // 该平台支持的触摸配置
  supportedTouchConfigs: { maxTouchPoints: number; hasTouch: boolean }[];
}

const PLATFORM_CONFIGS: PlatformConfig[] = [
  {
    name: 'Windows Chrome',
    platform: 'Win32',
    oscpu: 'Windows NT 10.0; Win64; x64',
    userAgentPatterns: [
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Safari/537.36',
    ],
    gpuVendor: 'NVIDIA',
    clientHintsPlatform: 'Windows',
    clientHintsPlatformVersion: '15.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      'Segoe UI, Tahoma, Geneva, Verdana, sans-serif',
      'Arial, Helvetica, sans-serif',
      'Microsoft YaHei, SimHei, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
      { width: 1440, height: 900 },
      { width: 1536, height: 864 },
      { width: 1680, height: 1050 },
      { width: 1280, height: 720 },
      { width: 1600, height: 900 },
      { width: 1920, height: 1200 },
      { width: 2048, height: 1152 },
      { width: 2560, height: 1600 },
      { width: 3440, height: 1440 },
      { width: 3840, height: 2160 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
      { maxTouchPoints: 5, hasTouch: true },
    ],
  },
  {
    name: 'Windows Edge',
    platform: 'Win32',
    oscpu: 'Windows NT 10.0; Win64; x64',
    userAgentPatterns: [
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Safari/537.36 Edg/{version}.0.0.0',
    ],
    gpuVendor: 'NVIDIA',
    clientHintsPlatform: 'Windows',
    clientHintsPlatformVersion: '15.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      'Segoe UI, Tahoma, Geneva, Verdana, sans-serif',
      'Arial, Helvetica, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
      { width: 1440, height: 900 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
      { maxTouchPoints: 5, hasTouch: true },
    ],
  },
  {
    name: 'Windows Firefox',
    platform: 'Win32',
    oscpu: 'Windows NT 10.0; Win64; x64',
    userAgentPatterns: [
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:{version}) Gecko/20100101 Firefox/{version}',
    ],
    gpuVendor: 'NVIDIA',
    clientHintsPlatform: 'Windows',
    clientHintsPlatformVersion: '15.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      'Segoe UI, Tahoma, Geneva, Verdana, sans-serif',
      'Arial, Helvetica, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
      { width: 2560, height: 1440 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'macOS Chrome',
    platform: 'MacIntel',
    oscpu: 'Intel Mac OS X 10_15_7',
    userAgentPatterns: [
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Safari/537.36',
    ],
    gpuVendor: 'Apple',
    clientHintsPlatform: 'macOS',
    clientHintsPlatformVersion: '14.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
      'Helvetica Neue, Helvetica, Arial, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 2880, height: 1800 },
      { width: 3024, height: 1964 },
      { width: 3456, height: 2234 },
      { width: 1440, height: 900 },
      { width: 1680, height: 1050 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'macOS Safari',
    platform: 'MacIntel',
    oscpu: 'Intel Mac OS X 10_15_7',
    userAgentPatterns: [
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/{safariVersion} Safari/605.1.15',
    ],
    gpuVendor: 'Apple',
    clientHintsPlatform: 'macOS',
    clientHintsPlatformVersion: '14.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
      'Helvetica Neue, Helvetica, Arial, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 2880, height: 1800 },
      { width: 3024, height: 1964 },
      { width: 3456, height: 2234 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'macOS Firefox',
    platform: 'MacIntel',
    oscpu: 'Intel Mac OS X 10.15',
    userAgentPatterns: [
      'Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:{version}) Gecko/20100101 Firefox/{version}',
    ],
    gpuVendor: 'Apple',
    clientHintsPlatform: 'macOS',
    clientHintsPlatformVersion: '14.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'Linux Chrome',
    platform: 'Linux x86_64',
    oscpu: 'Linux x86_64',
    userAgentPatterns: [
      'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Safari/537.36',
    ],
    gpuVendor: 'NVIDIA',
    clientHintsPlatform: 'Linux',
    clientHintsPlatformVersion: '6.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      'Roboto, Oxygen, Ubuntu, Cantarell, sans-serif',
      'DejaVu Sans, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
      { width: 3840, height: 2160 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'Linux Firefox',
    platform: 'Linux x86_64',
    oscpu: 'Linux x86_64',
    userAgentPatterns: [
      'Mozilla/5.0 (X11; Linux x86_64; rv:{version}) Gecko/20100101 Firefox/{version}',
    ],
    gpuVendor: 'NVIDIA',
    clientHintsPlatform: 'Linux',
    clientHintsPlatformVersion: '6.0.0',
    clientHintsArchitecture: 'x86',
    clientHintsBitness: '64',
    fontFamilies: [
      'Roboto, Oxygen, Ubuntu, Cantarell, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 0, hasTouch: false },
    ],
  },
  {
    name: 'iPhone Safari',
    platform: 'iPhone',
    oscpu: 'iPhone OS 17_4_1 like Mac OS X',
    userAgentPatterns: [
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_4_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1',
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.3 Mobile/15E148 Safari/604.1',
    ],
    gpuVendor: 'Apple',
    clientHintsPlatform: 'iOS',
    clientHintsPlatformVersion: '17.4.0',
    clientHintsArchitecture: 'arm',
    clientHintsBitness: '64',
    fontFamilies: [
      '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 390, height: 844 },   // iPhone 14/15
      { width: 393, height: 852 },   // iPhone 15 Pro
      { width: 430, height: 932 },   // iPhone 15 Pro Max
      { width: 414, height: 896 },   // iPhone 11 Pro Max
      { width: 375, height: 812 },   // iPhone X/XS/11 Pro
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 5, hasTouch: true },
    ],
  },
  {
    name: 'iPad Safari',
    platform: 'iPad',
    oscpu: 'iPad OS 17_4_1 like Mac OS X',
    userAgentPatterns: [
      'Mozilla/5.0 (iPad; CPU OS 17_4_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1',
    ],
    gpuVendor: 'Apple',
    clientHintsPlatform: 'iPadOS',
    clientHintsPlatformVersion: '17.4.0',
    clientHintsArchitecture: 'arm',
    clientHintsBitness: '64',
    fontFamilies: [
      '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 820, height: 1180 },   // iPad Air
      { width: 834, height: 1194 },   // iPad Pro 11
      { width: 1024, height: 1366 },  // iPad Pro 12.9
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 10, hasTouch: true },
    ],
  },
  {
    name: 'Android Chrome',
    platform: 'Linux armv8l',
    oscpu: 'Linux armv8l',
    userAgentPatterns: [
      'Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Mobile Safari/537.36',
      'Mozilla/5.0 (Linux; Android 13; SM-G998B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Mobile Safari/537.36',
      'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{version}.0.0.0 Mobile Safari/537.36',
    ],
    gpuVendor: 'ARM',
    clientHintsPlatform: 'Android',
    clientHintsPlatformVersion: '14.0.0',
    clientHintsArchitecture: 'arm',
    clientHintsBitness: '64',
    fontFamilies: [
      'Roboto, sans-serif',
      'Noto Sans, sans-serif',
    ],
    typicalScreenSizes: [
      { width: 360, height: 800 },   // Android 标准
      { width: 412, height: 915 },   // Android 大屏
      { width: 393, height: 873 },   // Pixel 8
      { width: 384, height: 854 },   // Samsung S23
    ],
    supportedTouchConfigs: [
      { maxTouchPoints: 10, hasTouch: true },
    ],
  },
];

// ==================== GPU 配置池 ====================

const GPU_CONFIGS = {
  NVIDIA: [
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4090 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4080 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4070 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4060 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4060 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3090 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3080 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3070 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Ti Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (NVIDIA)', renderer: 'ANGLE (NVIDIA, NVIDIA GeForce GTX 1650 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
  ],
  Intel: [
    { vendor: 'Google Inc. (Intel)', renderer: 'ANGLE (Intel, Intel(R) Arc(TM) A770 Graphics Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (Intel)', renderer: 'ANGLE (Intel, Intel(R) UHD Graphics 770 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (Intel)', renderer: 'ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (Intel)', renderer: 'ANGLE (Intel, Intel(R) Iris(TM) Plus Graphics 640 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
  ],
  AMD: [
    { vendor: 'Google Inc. (AMD)', renderer: 'ANGLE (AMD, AMD Radeon RX 7900 XTX Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (AMD)', renderer: 'ANGLE (AMD, AMD Radeon RX 6950 XT Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (AMD)', renderer: 'ANGLE (AMD, AMD Radeon RX 6800 XT Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (AMD)', renderer: 'ANGLE (AMD, AMD Radeon RX 6700 XT Direct3D11 vs_5_0 ps_5_0, D3D11)' },
    { vendor: 'Google Inc. (AMD)', renderer: 'ANGLE (AMD, AMD Radeon RX 580 Direct3D11 vs_5_0 ps_5_0, D3D11)' },
  ],
  Apple: [
    { vendor: 'Apple Inc.', renderer: 'Apple M1' },
    { vendor: 'Apple Inc.', renderer: 'Apple M1 Pro' },
    { vendor: 'Apple Inc.', renderer: 'Apple M1 Max' },
    { vendor: 'Apple Inc.', renderer: 'Apple M2' },
    { vendor: 'Apple Inc.', renderer: 'Apple M2 Pro' },
    { vendor: 'Apple Inc.', renderer: 'Apple M3' },
    { vendor: 'Apple Inc.', renderer: 'Apple M3 Pro' },
    { vendor: 'Apple Inc.', renderer: 'Apple M3 Max' },
  ],
  ARM: [
    { vendor: 'ARM', renderer: 'Mali-G710' },
    { vendor: 'ARM', renderer: 'Mali-G78' },
    { vendor: 'ARM', renderer: 'Mali-G77' },
    { vendor: 'Qualcomm', renderer: 'Adreno (TM) 740' },
    { vendor: 'Qualcomm', renderer: 'Adreno (TM) 730' },
  ],
};

// ==================== 区域配置定义 ====================

interface RegionConfig {
  locale: string;
  timezone: string;
  languages: string[];
  // 该区域常用的屏幕分辨率
  commonScreenSizes: { width: number; height: number }[];
}

const REGION_CONFIGS: RegionConfig[] = [
  // 中国大陆
  {
    locale: 'zh-CN',
    timezone: 'Asia/Shanghai',
    languages: ['zh-CN', 'zh', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
    ],
  },
  // 中国台湾
  {
    locale: 'zh-TW',
    timezone: 'Asia/Taipei',
    languages: ['zh-TW', 'zh', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 中国香港
  {
    locale: 'zh-HK',
    timezone: 'Asia/Hong_Kong',
    languages: ['zh-HK', 'zh-TW', 'zh', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
  },
  // 日本
  {
    locale: 'ja-JP',
    timezone: 'Asia/Tokyo',
    languages: ['ja-JP', 'ja', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
    ],
  },
  // 韩国
  {
    locale: 'ko-KR',
    timezone: 'Asia/Seoul',
    languages: ['ko-KR', 'ko', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
  },
  // 新加坡
  {
    locale: 'en-SG',
    timezone: 'Asia/Singapore',
    languages: ['en-SG', 'en', 'zh', 'ms'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
  },
  // 美国
  {
    locale: 'en-US',
    timezone: 'America/New_York',
    languages: ['en-US', 'en', 'es-US', 'es'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 3440, height: 1440 },
      { width: 3840, height: 2160 },
      { width: 1366, height: 768 },
    ],
  },
  // 英国
  {
    locale: 'en-GB',
    timezone: 'Europe/London',
    languages: ['en-GB', 'en', 'cy', 'gd'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 1366, height: 768 },
    ],
  },
  // 加拿大
  {
    locale: 'en-CA',
    timezone: 'America/Toronto',
    languages: ['en-CA', 'en', 'fr-CA', 'fr'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
  },
  // 澳大利亚
  {
    locale: 'en-AU',
    timezone: 'Australia/Sydney',
    languages: ['en-AU', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 3440, height: 1440 },
    ],
  },
  // 德国
  {
    locale: 'de-DE',
    timezone: 'Europe/Berlin',
    languages: ['de-DE', 'de', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 3840, height: 2160 },
    ],
  },
  // 法国
  {
    locale: 'fr-FR',
    timezone: 'Europe/Paris',
    languages: ['fr-FR', 'fr', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
      { width: 3440, height: 1440 },
    ],
  },
  // 西班牙
  {
    locale: 'es-ES',
    timezone: 'Europe/Madrid',
    languages: ['es-ES', 'es', 'en-US', 'en', 'ca', 'gl'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 意大利
  {
    locale: 'it-IT',
    timezone: 'Europe/Rome',
    languages: ['it-IT', 'it', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 俄罗斯
  {
    locale: 'ru-RU',
    timezone: 'Europe/Moscow',
    languages: ['ru-RU', 'ru', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 巴西
  {
    locale: 'pt-BR',
    timezone: 'America/Sao_Paulo',
    languages: ['pt-BR', 'pt', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 印度
  {
    locale: 'hi-IN',
    timezone: 'Asia/Kolkata',
    languages: ['hi-IN', 'hi', 'en-IN', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 阿联酋
  {
    locale: 'ar-AE',
    timezone: 'Asia/Dubai',
    languages: ['ar-AE', 'ar', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 2560, height: 1440 },
    ],
  },
  // 泰国
  {
    locale: 'th-TH',
    timezone: 'Asia/Bangkok',
    languages: ['th-TH', 'th', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 越南
  {
    locale: 'vi-VN',
    timezone: 'Asia/Ho_Chi_Minh',
    languages: ['vi-VN', 'vi', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
  // 印度尼西亚
  {
    locale: 'id-ID',
    timezone: 'Asia/Jakarta',
    languages: ['id-ID', 'id', 'en-US', 'en'],
    commonScreenSizes: [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 },
    ],
  },
];

// ==================== 硬件配置 ====================

const HARDWARE_CONCURRENCY_OPTIONS = [2, 4, 6, 8, 12, 16, 20, 24, 32];
const DEVICE_MEMORY_OPTIONS = [2, 4, 6, 8, 12, 16, 24, 32, 64];
const COLOR_DEPTHS = [24, 30, 32, 48];
const COLOR_SCHEMES = ['light', 'dark', 'no-preference'];
const SCREEN_ORIENTATIONS = [
  { angle: 0, type: 'landscape-primary' },
  { angle: 90, type: 'portrait-primary' },
  { angle: 180, type: 'landscape-secondary' },
  { angle: 270, type: 'portrait-secondary' },
];

// 桌面端设备像素比
const DESKTOP_PIXEL_RATIOS = [1, 1.25, 1.5, 1.75, 2];
// 移动端设备像素比
const MOBILE_PIXEL_RATIOS = [2, 2.5, 3, 3.5];

// ==================== 工具函数 ====================

function randomChoice<T>(array: T[]): T {
  return array[Math.floor(Math.random() * array.length)];
}

function randomInt(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function shuffleArray<T>(array: T[]): T[] {
  const shuffled = [...array];
  for (let i = shuffled.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }
  return shuffled;
}

// 判断是否为移动端平台
function isMobilePlatform(platform: string): boolean {
  return platform === 'iPhone' || platform === 'iPad' || platform === 'Linux armv8l';
}

// ==================== 指纹接口定义 ====================

export interface GeneratedFingerprint {
  // 基础信息
  userAgent: string;
  screenSize: { width: number; height: number };
  platform: string;
  oscpu: string;
  locale: string;
  timezone: string;
  
  // 硬件信息
  hardwareConcurrency: number;
  deviceMemory: number;
  devicePixelRatio: number;
  colorDepth: number;
  
  // 语言和区域
  languages: string[];
  vendor: string;
  
  // GPU信息
  gpuVendor: string;
  gpuRenderer: string;
  
  // 显示设置
  colorScheme: string;
  touchConfig: { maxTouchPoints: number; hasTouch: boolean };
  screenOrientation: { angle: number; type: string };
  fontFamily: string;
  
  // Client Hints
  clientHintsPlatform: string;
  clientHintsPlatformVersion: string;
  clientHintsArchitecture: string;
  clientHintsBitness: string;
  clientHintsModel: string;
  clientHintsWow64: string;
  
  // 唯一标识
  fingerprintHash: string;
  
  // 平台信息（用于调试）
  platformName: string;
}

// ==================== 指纹生成器 ====================

/**
 * 生成单个指纹 - 确保所有信息相互对应
 */
export function generateFingerprint(): GeneratedFingerprint {
  // 1. 随机选择平台配置（这决定了所有其他属性）
  const platformConfig = randomChoice(PLATFORM_CONFIGS);
  
  // 2. 根据平台选择区域配置（确保时区和语言的合理性）
  let regionConfig: RegionConfig;
  if (platformConfig.name.includes('iPhone') || platformConfig.name.includes('iPad')) {
    // iOS设备主要分布在美国、中国、日本等
    const iosRegions = REGION_CONFIGS.filter(r => 
      ['en-US', 'zh-CN', 'ja-JP', 'en-GB', 'en-AU'].includes(r.locale)
    );
    regionConfig = randomChoice(iosRegions);
  } else if (platformConfig.name.includes('Android')) {
    // Android设备分布更广泛
    const androidRegions = REGION_CONFIGS.filter(r =>
      ['en-US', 'zh-CN', 'en-IN', 'pt-BR', 'ru-RU', 'id-ID', 'vi-VN'].includes(r.locale)
    );
    regionConfig = randomChoice(androidRegions);
  } else {
    // 桌面端使用所有区域
    regionConfig = randomChoice(REGION_CONFIGS);
  }
  
  // 3. 生成 User-Agent
  let userAgent: string;
  if (platformConfig.userAgentPatterns.length === 1 && platformConfig.userAgentPatterns[0].includes('{')) {
    // 需要替换版本号
    const chromeVersion = randomInt(132, 135);
    const safariVersion = randomInt(17, 18);
    const firefoxVersion = randomInt(127, 128);
    
    userAgent = platformConfig.userAgentPatterns[0]
      .replace(/{version}/g, chromeVersion.toString())
      .replace(/{safariVersion}/g, safariVersion.toString() + '.0')
      .replace(/{firefoxVersion}/g, firefoxVersion.toString());
  } else {
    // 直接选择预定义的User-Agent
    userAgent = randomChoice(platformConfig.userAgentPatterns);
  }
  
  // 4. 根据平台选择GPU
  const gpuConfig = randomChoice(GPU_CONFIGS[platformConfig.gpuVendor]);
  
  // 5. 根据平台和区域选择屏幕分辨率
  // 优先使用平台典型分辨率，但也会考虑区域常用分辨率
  let screenSize: { width: number; height: number };
  if (Math.random() < 0.7) {
    // 70%概率使用平台典型分辨率
    screenSize = randomChoice(platformConfig.typicalScreenSizes);
  } else {
    // 30%概率使用区域常用分辨率
    const commonSizes = regionConfig.commonScreenSizes.filter(size => 
      platformConfig.typicalScreenSizes.some(ps => ps.width === size.width && ps.height === size.height)
    );
    screenSize = commonSizes.length > 0 ? randomChoice(commonSizes) : randomChoice(platformConfig.typicalScreenSizes);
  }
  
  // 6. 根据平台选择触摸配置
  const touchConfig = randomChoice(platformConfig.supportedTouchConfigs);
  
  // 7. 根据平台选择设备像素比
  const isMobile = isMobilePlatform(platformConfig.platform);
  const devicePixelRatio = isMobile 
    ? randomChoice(MOBILE_PIXEL_RATIOS)
    : randomChoice(DESKTOP_PIXEL_RATIOS);
  
  // 8. 根据平台选择字体
  const fontFamily = randomChoice(platformConfig.fontFamilies);
  
  // 9. 生成其他属性
  const hardwareConcurrency = isMobile
    ? randomChoice([4, 6, 8])  // 移动端通常核心数较少
    : randomChoice(HARDWARE_CONCURRENCY_OPTIONS);
  
  const deviceMemory = isMobile
    ? randomChoice([4, 6, 8, 12])  // 移动端内存较小
    : randomChoice(DEVICE_MEMORY_OPTIONS);
  
  const colorDepth = randomChoice(COLOR_DEPTHS);
  const colorScheme = randomChoice(COLOR_SCHEMES);
  const screenOrientation = isMobile
    ? randomChoice([SCREEN_ORIENTATIONS[0], SCREEN_ORIENTATIONS[1]])  // 移动端多为竖屏或横屏
    : SCREEN_ORIENTATIONS[0];  // 桌面端固定横屏
  
  // 10. 确定 vendor
  let vendor: string;
  if (userAgent.includes('Firefox')) {
    vendor = '';  // Firefox vendor 为空
  } else if (userAgent.includes('Safari') && !userAgent.includes('Chrome')) {
    vendor = 'Apple Inc.';  // Safari
  } else if (platformConfig.gpuVendor === 'Apple') {
    vendor = 'Apple Inc.';
  } else {
    vendor = 'Google Inc.';  // Chrome/Edge on Windows/Linux
  }
  
  const fingerprint: GeneratedFingerprint = {
    userAgent,
    screenSize,
    platform: platformConfig.platform,
    oscpu: platformConfig.oscpu,
    locale: regionConfig.locale,
    timezone: regionConfig.timezone,
    hardwareConcurrency,
    deviceMemory,
    devicePixelRatio,
    colorDepth,
    languages: regionConfig.languages,
    vendor,
    gpuVendor: gpuConfig.vendor,
    gpuRenderer: gpuConfig.renderer,
    colorScheme,
    touchConfig,
    screenOrientation,
    fontFamily,
    clientHintsPlatform: platformConfig.clientHintsPlatform,
    clientHintsPlatformVersion: platformConfig.clientHintsPlatformVersion,
    clientHintsArchitecture: platformConfig.clientHintsArchitecture,
    clientHintsBitness: platformConfig.clientHintsBitness,
    clientHintsModel: '',
    clientHintsWow64: 'false',
    fingerprintHash: '',
    platformName: platformConfig.name,
  };
  
  // 生成指纹哈希
  fingerprint.fingerprintHash = generateFingerprintHash(fingerprint);
  
  return fingerprint;
}

/**
 * 生成指纹哈希
 */
function generateFingerprintHash(fingerprint: GeneratedFingerprint): string {
  const hashString = `${fingerprint.userAgent}-${fingerprint.screenSize.width}x${fingerprint.screenSize.height}-${fingerprint.platform}-${fingerprint.locale}-${fingerprint.timezone}-${fingerprint.hardwareConcurrency}-${fingerprint.deviceMemory}-${fingerprint.gpuRenderer}-${fingerprint.touchConfig.maxTouchPoints}`;
  
  let hash = 0;
  for (let i = 0; i < hashString.length; i++) {
    const char = hashString.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16).padStart(8, '0');
}

/**
 * 批量生成唯一指纹
 * @param count 生成数量
 * @param existingFingerprints 已存在的指纹列表（用于去重）
 * @returns 生成的指纹列表
 */
export function generateUniqueFingerprints(
  count: number, 
  existingFingerprints: string[] = []
): GeneratedFingerprint[] {
  const fingerprints: GeneratedFingerprint[] = [];
  const existingHashes = new Set(existingFingerprints);
  
  let attempts = 0;
  const maxAttempts = count * 100;
  
  while (fingerprints.length < count && attempts < maxAttempts) {
    const fingerprint = generateFingerprint();
    
    if (!existingHashes.has(fingerprint.fingerprintHash)) {
      existingHashes.add(fingerprint.fingerprintHash);
      fingerprints.push(fingerprint);
    }
    
    attempts++;
  }
  
  if (fingerprints.length < count) {
    console.warn(`警告：只生成了 ${fingerprints.length}/${count} 个唯一指纹`);
  }
  
  return fingerprints;
}

/**
 * 将生成的指纹转换为浏览器配置
 */
export function fingerprintToBrowserProfile(
  fingerprint: GeneratedFingerprint,
  name: string,
  index: number
): CreateProfileRequest {
  return {
    name: `${name}-${index + 1}`,
    user_agent: fingerprint.userAgent,
    viewport_width: fingerprint.screenSize.width,
    viewport_height: fingerprint.screenSize.height,
    device_scale_factor: fingerprint.devicePixelRatio,
    locale: fingerprint.locale,
    timezone_id: fingerprint.timezone,
    proxy_type: 'direct',
    canvas_spoof: true,
    webgl_spoof: true,
    audio_spoof: true,
    timezone_spoof: true,
    geolocation_spoof: true,
    font_spoof: true,
    webrtc_spoof: true,
    navigator_override: true,
    webdriver_override: true,
    headless: false,
    // 扩展字段
    hardware_concurrency: fingerprint.hardwareConcurrency,
    device_memory: fingerprint.deviceMemory,
    color_depth: fingerprint.colorDepth,
    languages: JSON.stringify(fingerprint.languages),
    vendor: fingerprint.vendor,
    gpu_vendor: fingerprint.gpuVendor,
    gpu_renderer: fingerprint.gpuRenderer,
    color_scheme: fingerprint.colorScheme,
    max_touch_points: fingerprint.touchConfig.maxTouchPoints,
    has_touch: fingerprint.touchConfig.hasTouch,
    screen_orientation_angle: fingerprint.screenOrientation.angle,
    screen_orientation_type: fingerprint.screenOrientation.type,
    font_family: fingerprint.fontFamily,
    client_hints_platform: fingerprint.clientHintsPlatform,
    client_hints_platform_version: fingerprint.clientHintsPlatformVersion,
    client_hints_architecture: fingerprint.clientHintsArchitecture,
    client_hints_bitness: fingerprint.clientHintsBitness,
    client_hints_model: fingerprint.clientHintsModel,
    client_hints_wow64: fingerprint.clientHintsWow64,
    fingerprint_hash: fingerprint.fingerprintHash,
    platform_name: fingerprint.platformName,
  };
}

/**
 * 批量生成浏览器配置
 */
export function generateBatchProfiles(
  count: number,
  baseName: string = 'Auto-Profile',
  existingFingerprints: string[] = []
): CreateProfileRequest[] {
  const fingerprints = generateUniqueFingerprints(count, existingFingerprints);
  return fingerprints.map((fp, index) => fingerprintToBrowserProfile(fp, baseName, index));
}

// 导入类型
import type { CreateProfileRequest } from './browserAutomationService';

// 导出配置供外部使用
export {
  PLATFORM_CONFIGS,
  REGION_CONFIGS,
  GPU_CONFIGS,
  HARDWARE_CONCURRENCY_OPTIONS,
  DEVICE_MEMORY_OPTIONS,
  COLOR_DEPTHS,
  COLOR_SCHEMES,
  SCREEN_ORIENTATIONS,
  DESKTOP_PIXEL_RATIOS,
  MOBILE_PIXEL_RATIOS,
};
