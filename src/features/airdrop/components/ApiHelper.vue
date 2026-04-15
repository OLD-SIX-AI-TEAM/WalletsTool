<script setup>
import { ref } from 'vue';
import { Message } from '@arco-design/web-vue';
import {
  IconBook,
  IconCode,
  IconCopy,
  IconCheck
} from '@arco-design/web-vue/es/icon';

const activeCategory = ref('wallet');
const searchQuery = ref('');
const copiedMethod = ref('');

// API 分类
const categories = [
  { id: 'wallet', name: '钱包连接', icon: 'wallet' },
  { id: 'sign', name: '签名操作', icon: 'edit' },
  { id: 'tx', name: '交易操作', icon: 'send' },
  { id: 'browser', name: '浏览器操作', icon: 'browser' },
  { id: 'human', name: '人类模拟', icon: 'user' },
  { id: 'utils', name: '工具函数', icon: 'tool' },
];

// API 方法定义
const apiMethods = {
  wallet: [
    {
      name: 'connectMetaMask',
      description: '连接 MetaMask 钱包',
      params: [{ name: 'options', type: 'Object', desc: '连接选项，如 { expectedChainId: "0x1", timeout: 30000 }' }],
      returns: 'Promise<string>',
      example: "await api.connectMetaMask({ expectedChainId: '0x1' })"
    },
    {
      name: 'connectOKXWallet',
      description: '连接 OKX Wallet',
      params: [{ name: 'options', type: 'Object', desc: '连接选项，如 { chainId: "0x1", timeout: 30000 }' }],
      returns: 'Promise<string>',
      example: "await api.connectOKXWallet({ chainId: '0x1' })"
    },
    {
      name: 'connectPhantom',
      description: '连接 Phantom 钱包 (Solana)',
      params: [{ name: 'options', type: 'Object', desc: '连接选项，如 { network: "mainnet-beta" }' }],
      returns: 'Promise<string>',
      example: "await api.connectPhantom({ network: 'mainnet-beta' })"
    },
    {
      name: 'connectSolflare',
      description: '连接 Solflare 钱包 (Solana)',
      params: [{ name: 'options', type: 'Object', desc: '连接选项，如 { network: "mainnet-beta" }' }],
      returns: 'Promise<string>',
      example: "await api.connectSolflare({ network: 'mainnet-beta' })"
    },
    {
      name: 'connectBackpack',
      description: '连接 Backpack 钱包',
      params: [{ name: 'options', type: 'Object', desc: '连接选项' }],
      returns: 'Promise<string>',
      example: "await api.connectBackpack()"
    },
    {
      name: 'connectWalletConnect',
      description: '通过 WalletConnect 连接',
      params: [
        { name: 'uri', type: 'string', desc: 'WalletConnect URI' },
        { name: 'options', type: 'Object', desc: '连接选项' }
      ],
      returns: 'Promise<string>',
      example: "await api.connectWalletConnect('wc:...', { chainId: '0x1' })"
    },
    {
      name: 'switchNetwork',
      description: '切换钱包网络 (EVM)',
      params: [{ name: 'chainId', type: 'string', desc: '目标网络ID (十六进制字符串，如 "0x1" 表示 Ethereum)' }],
      returns: 'Promise<boolean>',
      example: "await api.switchNetwork('0x1')"
    },
    {
      name: 'switchSolanaNetwork',
      description: '切换 Solana 网络',
      params: [{ name: 'network', type: 'string', desc: '网络名称: mainnet-beta, testnet, devnet' }],
      returns: 'Promise<boolean>',
      example: "await api.switchSolanaNetwork('mainnet-beta')"
    },
    {
      name: 'getCurrentAddress',
      description: '获取当前连接的钱包地址',
      params: [],
      returns: 'Promise<string>',
      example: "const address = await api.getCurrentAddress()"
    },
    {
      name: 'getWalletProvider',
      description: '获取当前钱包提供商信息',
      params: [],
      returns: 'Promise<Object>',
      example: "const provider = await api.getWalletProvider() // { name: 'MetaMask', chainId: '0x1' }"
    },
    {
      name: 'isWalletConnected',
      description: '检查钱包是否已连接',
      params: [],
      returns: 'Promise<boolean>',
      example: "const connected = await api.isWalletConnected()"
    },
    {
      name: 'disconnectWallet',
      description: '断开钱包连接',
      params: [],
      returns: 'Promise<void>',
      example: "await api.disconnectWallet()"
    },
    {
      name: 'addTokenToWallet',
      description: '添加 Token 到钱包显示',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token 合约地址' },
        { name: 'symbol', type: 'string', desc: 'Token 符号 (如 USDC)' },
        { name: 'decimals', type: 'number', desc: '小数位数 (如 6)' },
        { name: 'image', type: 'string', desc: 'Token 图标 URL (可选)' }
      ],
      returns: 'Promise<boolean>',
      example: "await api.addTokenToWallet('0xA0b86a33E6441e6C7D3D4B4f6c7D3D4B4f6c7D3', 'USDC', 6)"
    },
    {
      name: 'addNetwork',
      description: '添加自定义网络到钱包',
      params: [
        { name: 'network', type: 'Object', desc: '网络配置对象 { chainId, chainName, rpcUrls, nativeCurrency, blockExplorerUrls }' }
      ],
      returns: 'Promise<boolean>',
      example: `await api.addNetwork({
  chainId: '0x89',
  chainName: 'Polygon Mainnet',
  rpcUrls: ['https://polygon-rpc.com'],
  nativeCurrency: { name: 'MATIC', symbol: 'MATIC', decimals: 18 },
  blockExplorerUrls: ['https://polygonscan.com']
})`
    },
  ],
  sign: [
    {
      name: 'signMessage',
      description: '签名消息 (EVM)',
      params: [
        { name: 'message', type: 'string', desc: '要签名的消息' },
        { name: 'options', type: 'Object', desc: '可选参数 { account }' }
      ],
      returns: 'Promise<string>',
      example: "await api.signMessage('Hello, World!')"
    },
    {
      name: 'signMessageSolana',
      description: '签名消息 (Solana)',
      params: [
        { name: 'message', type: 'string|Uint8Array', desc: '要签名的消息' },
        { name: 'options', type: 'Object', desc: '可选参数' }
      ],
      returns: 'Promise<{ signature: string, publicKey: string }>',
      example: "await api.signMessageSolana('Hello Solana')"
    },
    {
      name: 'signTransaction',
      description: '签名交易 (EVM)',
      params: [{ name: 'tx', type: 'Object', desc: '交易对象 { to, value, data, gasLimit, gasPrice }' }],
      returns: 'Promise<string>',
      example: "await api.signTransaction({ to: '0x...', value: '0.1', data: '0x...' })"
    },
    {
      name: 'signTransactionSolana',
      description: '签名交易 (Solana)',
      params: [
        { name: 'transaction', type: 'Object', desc: 'Solana 交易对象' },
        { name: 'options', type: 'Object', desc: '可选参数' }
      ],
      returns: 'Promise<{ signature: string, serializedTx: string }>',
      example: `const tx = {
  recipient: 'recipientPublicKey',
  amount: 1000000000, // 1 SOL in lamports
  recentBlockhash: 'blockhash'
};
await api.signTransactionSolana(tx)`
    },
    {
      name: 'signAllTransactions',
      description: '批量签名交易 (Solana)',
      params: [
        { name: 'transactions', type: 'Array<Object>', desc: '交易对象数组' }
      ],
      returns: 'Promise<Array<{ signature: string }>>',
      example: "await api.signAllTransactions([tx1, tx2, tx3])"
    },
    {
      name: 'signTypedData',
      description: '签名 EIP-712 类型数据',
      params: [
        { name: 'domain', type: 'Object', desc: 'EIP-712 Domain' },
        { name: 'types', type: 'Object', desc: '类型定义' },
        { name: 'value', type: 'Object', desc: '要签名的值' }
      ],
      returns: 'Promise<string>',
      example: `await api.signTypedData(
  { name: 'MyApp', version: '1', chainId: 1 },
  { Person: [{ name: 'name', type: 'string' }] },
  { name: 'John' }
)`
    },
    {
      name: 'signPersonalMessage',
      description: '签名个人消息 (带以太坊前缀)',
      params: [
        { name: 'message', type: 'string', desc: '要签名的消息' }
      ],
      returns: 'Promise<string>',
      example: "await api.signPersonalMessage('Hello Ethereum')"
    },
  ],
  tx: [
    {
      name: 'sendNativeTransfer',
      description: '发送原生币转账 (EVM)',
      params: [
        { name: 'to', type: 'string', desc: '接收地址' },
        { name: 'amount', type: 'string', desc: '金额 (如 "0.1" 表示 0.1 ETH)' },
        { name: 'options', type: 'Object', desc: '可选参数 { gasLimit, gasPrice, maxFeePerGas, maxPriorityFeePerGas }' }
      ],
      returns: 'Promise<{ hash: string, status: string }>',
      example: "await api.sendNativeTransfer('0x...', '0.1', { gasLimit: 21000 })"
    },
    {
      name: 'sendSolanaTransfer',
      description: '发送 SOL 转账',
      params: [
        { name: 'to', type: 'string', desc: '接收地址 (Base58)' },
        { name: 'amount', type: 'number', desc: '金额 (lamports, 1 SOL = 1e9 lamports)' },
        { name: 'options', type: 'Object', desc: '可选参数' }
      ],
      returns: 'Promise<{ signature: string, status: string }>',
      example: "await api.sendSolanaTransfer('recipientAddress', 1000000000) // 1 SOL"
    },
    {
      name: 'sendSPLToken',
      description: '发送 SPL Token 转账 (Solana)',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token Mint 地址' },
        { name: 'to', type: 'string', desc: '接收地址' },
        { name: 'amount', type: 'number', desc: '金额 (考虑 decimals)' },
        { name: 'decimals', type: 'number', desc: 'Token 小数位数' }
      ],
      returns: 'Promise<{ signature: string, status: string }>',
      example: "await api.sendSPLToken('tokenMint', 'recipient', 1000000, 6) // 1 USDC"
    },
    {
      name: 'approveToken',
      description: 'ERC-20 Token 授权',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token 合约地址' },
        { name: 'spender', type: 'string', desc: '授权给谁 (合约地址)' },
        { name: 'amount', type: 'string', desc: '授权数量 (如 "1000" 表示 1000 个 Token)' }
      ],
      returns: 'Promise<{ hash: string, status: string }>',
      example: "await api.approveToken('0x...', '0x...', '1000')"
    },
    {
      name: 'transferToken',
      description: 'ERC-20 Token 转账',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token 合约地址' },
        { name: 'to', type: 'string', desc: '接收地址' },
        { name: 'amount', type: 'string', desc: '转账数量 (原始单位)' }
      ],
      returns: 'Promise<{ hash: string, status: string }>',
      example: "await api.transferToken('0x...', '0x...', '100')"
    },
    {
      name: 'transferNFT',
      description: 'NFT 转账 (ERC-721/ERC-1155)',
      params: [
        { name: 'nftContract', type: 'string', desc: 'NFT 合约地址' },
        { name: 'to', type: 'string', desc: '接收地址' },
        { name: 'tokenId', type: 'string', desc: 'Token ID' },
        { name: 'standard', type: 'string', desc: '标准类型: ERC721 或 ERC1155' },
        { name: 'amount', type: 'string', desc: '数量 (ERC1155 需要，默认 1)' }
      ],
      returns: 'Promise<{ hash: string, status: string }>',
      example: "await api.transferNFT('0x...', '0x...', '123', 'ERC721')"
    },
    {
      name: 'batchTransfer',
      description: '批量转账 (EVM)',
      params: [
        { name: 'transfers', type: 'Array<Object>', desc: '转账数组 [{ to, amount, tokenAddress? }]' }
      ],
      returns: 'Promise<Array<{ hash: string, status: string }>>',
      example: `await api.batchTransfer([
  { to: '0x...', amount: '0.1' },
  { to: '0x...', amount: '100', tokenAddress: '0x...' }
])`
    },
    {
      name: 'interactWithContract',
      description: '与智能合约交互',
      params: [
        { name: 'contractAddress', type: 'string', desc: '合约地址' },
        { name: 'abi', type: 'Array', desc: '合约 ABI' },
        { name: 'method', type: 'string', desc: '方法名' },
        { name: 'args', type: 'Array', desc: '方法参数数组' },
        { name: 'options', type: 'Object', desc: '交易选项 { value, gasLimit }' }
      ],
      returns: 'Promise<{ hash: string, result: any }>',
      example: `await api.interactWithContract(
  '0x...',
  contractAbi,
  'stake',
  [1000],
  { value: '0.1' }
)`
    },
    {
      name: 'callContractView',
      description: '调用合约 View 方法',
      params: [
        { name: 'contractAddress', type: 'string', desc: '合约地址' },
        { name: 'abi', type: 'Array', desc: '合约 ABI' },
        { name: 'method', type: 'string', desc: '方法名' },
        { name: 'args', type: 'Array', desc: '方法参数数组' }
      ],
      returns: 'Promise<any>',
      example: "const balance = await api.callContractView('0x...', erc20Abi, 'balanceOf', ['0x...'])"
    },
    {
      name: 'waitForTransaction',
      description: '等待交易确认',
      params: [
        { name: 'txHash', type: 'string', desc: '交易哈希' },
        { name: 'confirmations', type: 'number', desc: '确认数 (默认 1)' },
        { name: 'timeout', type: 'number', desc: '超时时间 (毫秒, 默认 60000)' }
      ],
      returns: 'Promise<{ status: string, receipt: Object, confirmations: number }>',
      example: "await api.waitForTransaction('0x...', 1, 120000)"
    },
    {
      name: 'estimateGas',
      description: '估算 Gas 费用',
      params: [
        { name: 'tx', type: 'Object', desc: '交易对象 { to, value, data }' }
      ],
      returns: 'Promise<{ gasLimit: string, gasPrice: string, maxFee: string }>',
      example: "const estimate = await api.estimateGas({ to: '0x...', value: '0.1' })"
    },
  ],
  browser: [
    {
      name: 'waitForSelector',
      description: '等待元素出现',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'timeout', type: 'number', desc: '超时时间 (毫秒, 默认30000)' }
      ],
      returns: 'Promise<Element>',
      example: "await api.waitForSelector('.submit-btn', 10000)"
    },
    {
      name: 'waitForSelectorHidden',
      description: '等待元素消失',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'timeout', type: 'number', desc: '超时时间 (毫秒, 默认30000)' }
      ],
      returns: 'Promise<boolean>',
      example: "await api.waitForSelectorHidden('.loading')"
    },
    {
      name: 'waitForPageLoad',
      description: '等待页面加载完成',
      params: [
        { name: 'state', type: 'string', desc: '加载状态: load, domcontentloaded, networkidle (默认 networkidle)' },
        { name: 'timeout', type: 'number', desc: '超时时间 (毫秒, 默认60000)' }
      ],
      returns: 'Promise<boolean>',
      example: "await api.waitForPageLoad('networkidle', 60000)"
    },
    {
      name: 'clickElement',
      description: '点击元素',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'options', type: 'Object', desc: '点击选项 { delay, button, clickCount, position, timeout, force }' }
      ],
      returns: 'Promise<void>',
      example: "await api.clickElement('.submit-btn', { delay: 100, button: 'left' })"
    },
    {
      name: 'inputText',
      description: '输入文本',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'text', type: 'string', desc: '输入文本' },
        { name: 'options', type: 'Object', desc: '输入选项 { delay, clearFirst, timeout }' }
      ],
      returns: 'Promise<void>',
      example: "await api.inputText('#address', '0x...', { delay: 50, clearFirst: true })"
    },
    {
      name: 'getElementText',
      description: '获取元素文本',
      params: [{ name: 'selector', type: 'string', desc: 'CSS 选择器' }],
      returns: 'Promise<string>',
      example: "const text = await api.getElementText('.balance')"
    },
    {
      name: 'getElementAttribute',
      description: '获取元素属性',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'attribute', type: 'string', desc: '属性名' }
      ],
      returns: 'Promise<string>',
      example: "const href = await api.getElementAttribute('a.link', 'href')"
    },
    {
      name: 'getElementProperty',
      description: '获取元素属性值',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'property', type: 'string', desc: '属性名' }
      ],
      returns: 'Promise<any>',
      example: "const checked = await api.getElementProperty('input', 'checked')"
    },
    {
      name: 'elementExists',
      description: '检查元素是否存在',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'timeout', type: 'number', desc: '超时时间 (毫秒, 默认 5000)' }
      ],
      returns: 'Promise<boolean>',
      example: "const exists = await api.elementExists('.modal')"
    },
    {
      name: 'selectOption',
      description: '选择下拉框选项',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'value', type: 'string|Array<string>', desc: '选项值' },
        { name: 'options', type: 'Object', desc: '选项 { label?, value?, index? }' }
      ],
      returns: 'Promise<void>',
      example: "await api.selectOption('#country', 'US')"
    },
    {
      name: 'checkElement',
      description: '勾选复选框/单选框',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'checked', type: 'boolean', desc: '是否勾选 (默认 true)' }
      ],
      returns: 'Promise<void>',
      example: "await api.checkElement('#agree-terms', true)"
    },
    {
      name: 'hoverElement',
      description: '悬停在元素上',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'options', type: 'Object', desc: '选项 { position, timeout, force }' }
      ],
      returns: 'Promise<void>',
      example: "await api.hoverElement('.dropdown-menu')"
    },
    {
      name: 'scrollToElement',
      description: '滚动到元素位置',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'options', type: 'Object', desc: '选项 { behavior: "smooth" | "auto" }' }
      ],
      returns: 'Promise<void>',
      example: "await api.scrollToElement('#footer', { behavior: 'smooth' })"
    },
    {
      name: 'scrollPage',
      description: '滚动页面',
      params: [
        { name: 'direction', type: 'string', desc: '方向: up, down, left, right' },
        { name: 'amount', type: 'number', desc: '滚动距离 (像素, 默认 300)' }
      ],
      returns: 'Promise<void>',
      example: "await api.scrollPage('down', 500)"
    },
    {
      name: 'uploadFile',
      description: '上传文件',
      params: [
        { name: 'selector', type: 'string', desc: '文件输入选择器' },
        { name: 'filePath', type: 'string', desc: '文件路径' }
      ],
      returns: 'Promise<void>',
      example: "await api.uploadFile('input[type=file]', '/path/to/file.png')"
    },
    {
      name: 'takeScreenshot',
      description: '截图',
      params: [
        { name: 'options', type: 'Object', desc: '选项 { path, fullPage, selector, type }' }
      ],
      returns: 'Promise<Buffer|string>',
      example: "await api.takeScreenshot({ path: 'screenshot.png', fullPage: true })"
    },
    {
      name: 'executeScript',
      description: '执行任意 JavaScript',
      params: [
        { name: 'fn', type: 'string|Function', desc: 'JavaScript 代码或函数' },
        { name: 'args', type: 'Array', desc: '参数数组' }
      ],
      returns: 'Promise<any>',
      example: "const title = await api.executeScript(() => document.title)"
    },
    {
      name: 'setViewport',
      description: '设置视口大小',
      params: [
        { name: 'viewport', type: 'Object', desc: '视口配置 { width, height, deviceScaleFactor }' }
      ],
      returns: 'Promise<void>',
      example: "await api.setViewport({ width: 1920, height: 1080 })"
    },
    {
      name: 'goto',
      description: '导航到 URL',
      params: [
        { name: 'url', type: 'string', desc: '目标 URL' },
        { name: 'options', type: 'Object', desc: '选项 { waitUntil, timeout, referer }' }
      ],
      returns: 'Promise<Response>',
      example: "await api.goto('https://example.com', { waitUntil: 'networkidle' })"
    },
    {
      name: 'goBack',
      description: '返回上一页',
      params: [
        { name: 'options', type: 'Object', desc: '选项 { waitUntil, timeout }' }
      ],
      returns: 'Promise<Response>',
      example: "await api.goBack()"
    },
    {
      name: 'goForward',
      description: '前进到下一页',
      params: [
        { name: 'options', type: 'Object', desc: '选项 { waitUntil, timeout }' }
      ],
      returns: 'Promise<Response>',
      example: "await api.goForward()"
    },
    {
      name: 'reload',
      description: '刷新页面',
      params: [
        { name: 'options', type: 'Object', desc: '选项 { waitUntil, timeout }' }
      ],
      returns: 'Promise<Response>',
      example: "await api.reload()"
    },
    {
      name: 'closePopup',
      description: '关闭弹窗/模态框',
      params: [
        { name: 'selector', type: 'string', desc: '关闭按钮选择器 (默认常见选择器)' }
      ],
      returns: 'Promise<boolean>',
      example: "await api.closePopup('.modal-close')"
    },
    {
      name: 'handleDialog',
      description: '处理对话框 (alert, confirm, prompt)',
      params: [
        { name: 'action', type: 'string', desc: '动作: accept, dismiss' },
        { name: 'promptText', type: 'string', desc: 'prompt 输入文本 (可选)' }
      ],
      returns: 'Promise<void>',
      example: "await api.handleDialog('accept')"
    },
  ],
  human: [
    {
      name: 'humanLikeClick',
      description: '模拟人类点击 (带随机偏移和延迟)',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'options', type: 'Object', desc: '选项 { offsetRange, delayRange, moveSpeed }' }
      ],
      returns: 'Promise<void>',
      example: "await api.humanLikeClick('.btn', { offsetRange: 5, delayRange: [100, 300] })"
    },
    {
      name: 'humanLikeInput',
      description: '模拟人类输入 (带随机打字速度)',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'text', type: 'string', desc: '输入文本' },
        { name: 'options', type: 'Object', desc: '选项 { minDelay, maxDelay, typoChance, typoFixDelay }' }
      ],
      returns: 'Promise<void>',
      example: "await api.humanLikeInput('#search', 'hello world', { minDelay: 50, maxDelay: 150 })"
    },
    {
      name: 'humanLikeScroll',
      description: '模拟人类滚动 (带随机停顿和速度变化)',
      params: [
        { name: 'direction', type: 'string', desc: '方向: up, down, left, right' },
        { name: 'options', type: 'Object', desc: '选项 { minDistance, maxDistance, minSteps, maxSteps, pauseChance }' }
      ],
      returns: 'Promise<void>',
      example: "await api.humanLikeScroll('down', { minDistance: 300, maxDistance: 800 })"
    },
    {
      name: 'humanLikeMove',
      description: '模拟人类鼠标移动 (贝塞尔曲线)',
      params: [
        { name: 'targetX', type: 'number', desc: '目标 X 坐标' },
        { name: 'targetY', type: 'number', desc: '目标 Y 坐标' },
        { name: 'options', type: 'Object', desc: '选项 { minSteps, maxSteps, minDelay, maxDelay, overshoot }' }
      ],
      returns: 'Promise<void>',
      example: "await api.humanLikeMove(500, 300, { minSteps: 15, maxSteps: 40 })"
    },
    {
      name: 'humanLikeHover',
      description: '模拟人类悬停 (带随机抖动)',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'options', type: 'Object', desc: '选项 { hoverTime, jitterRange }' }
      ],
      returns: 'Promise<void>',
      example: "await api.humanLikeHover('.menu-item', { hoverTime: [500, 1500] })"
    },
    {
      name: 'randomDelay',
      description: '随机延迟',
      params: [
        { name: 'minMs', type: 'number', desc: '最小延迟 (毫秒, 默认 1000)' },
        { name: 'maxMs', type: 'number', desc: '最大延迟 (毫秒, 默认 3000)' }
      ],
      returns: 'Promise<void>',
      example: "await api.randomDelay(2000, 5000)"
    },
    {
      name: 'randomThinkTime',
      description: '随机思考时间 (模拟用户阅读/思考)',
      params: [
        { name: 'minMs', type: 'number', desc: '最小时间 (毫秒, 默认 2000)' },
        { name: 'maxMs', type: 'number', desc: '最大时间 (毫秒, 默认 8000)' }
      ],
      returns: 'Promise<void>',
      example: "await api.randomThinkTime(3000, 6000)"
    },
    {
      name: 'simulateTypingPattern',
      description: '模拟真实打字模式 (带错误和修正)',
      params: [
        { name: 'selector', type: 'string', desc: 'CSS 选择器' },
        { name: 'text', type: 'string', desc: '输入文本' },
        { name: 'options', type: 'Object', desc: '选项 { wpm, accuracy, correctionDelay }' }
      ],
      returns: 'Promise<void>',
      example: "await api.simulateTypingPattern('#input', 'hello', { wpm: 60, accuracy: 0.95 })"
    },
    {
      name: 'simulateMousePath',
      description: '模拟鼠标移动路径',
      params: [
        { name: 'points', type: 'Array<{x, y}>', desc: '路径点数组' },
        { name: 'options', type: 'Object', desc: '选项 { curve, speed, randomness }' }
      ],
      returns: 'Promise<void>',
      example: "await api.simulateMousePath([{x: 100, y: 100}, {x: 200, y: 200}])"
    },
    {
      name: 'simulateReading',
      description: '模拟阅读行为 (滚动并停顿)',
      params: [
        { name: 'options', type: 'Object', desc: '选项 { scrollAmount, readTime, iterations }' }
      ],
      returns: 'Promise<void>',
      example: "await api.simulateReading({ scrollAmount: 500, readTime: [2000, 5000] })"
    },
  ],
  utils: [
    {
      name: 'getBalance',
      description: '获取钱包余额',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token 地址 (空则为主币)' }
      ],
      returns: 'Promise<string>',
      example: "const balance = await api.getBalance() // 主币\nconst usdc = await api.getBalance('0x...') // Token"
    },
    {
      name: 'getTokenInfo',
      description: '获取 Token 信息',
      params: [
        { name: 'tokenAddress', type: 'string', desc: 'Token 合约地址' }
      ],
      returns: 'Promise<{ name, symbol, decimals, totalSupply }>',
      example: "const info = await api.getTokenInfo('0x...')"
    },
    {
      name: 'getGasPrices',
      description: '获取 Gas 价格',
      params: [],
      returns: 'Promise<{ slow, standard, fast, baseFee }>',
      example: "const { slow, standard, fast } = await api.getGasPrices()"
    },
    {
      name: 'formatEther',
      description: '将 wei 转换为 ETH',
      params: [
        { name: 'wei', type: 'string|number', desc: 'wei 单位金额' }
      ],
      returns: 'string',
      example: "const eth = api.formatEther('1000000000000000000') // '1.0'"
    },
    {
      name: 'parseEther',
      description: '将 ETH 转换为 wei',
      params: [
        { name: 'ether', type: 'string', desc: 'ETH 单位金额' }
      ],
      returns: 'string',
      example: "const wei = api.parseEther('1.5') // '1500000000000000000'"
    },
    {
      name: 'formatUnits',
      description: '格式化代币金额',
      params: [
        { name: 'value', type: 'string|number', desc: '原始金额' },
        { name: 'decimals', type: 'number', desc: '小数位数 (默认 18)' }
      ],
      returns: 'string',
      example: "const amount = api.formatUnits('1000000', 6) // '1.0' (USDC)"
    },
    {
      name: 'parseUnits',
      description: '解析代币金额',
      params: [
        { name: 'value', type: 'string', desc: '可读金额' },
        { name: 'decimals', type: 'number', desc: '小数位数 (默认 18)' }
      ],
      returns: 'string',
      example: "const raw = api.parseUnits('100', 6) // '100000000' (USDC)"
    },
    {
      name: 'isAddress',
      description: '验证地址格式',
      params: [
        { name: 'address', type: 'string', desc: '要验证的地址' }
      ],
      returns: 'boolean',
      example: "const valid = api.isAddress('0x...')"
    },
    {
      name: 'getAddress',
      description: '获取校验和地址',
      params: [
        { name: 'address', type: 'string', desc: '地址' }
      ],
      returns: 'string',
      example: "const checksum = api.getAddress('0x...')"
    },
    {
      name: 'shortenAddress',
      description: '缩短地址显示',
      params: [
        { name: 'address', type: 'string', desc: '完整地址' },
        { name: 'chars', type: 'number', desc: '前后保留字符数 (默认 4)' }
      ],
      returns: 'string',
      example: "const short = api.shortenAddress('0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb', 6) // '0x742d35...5f0bEb'"
    },
    {
      name: 'randomInt',
      description: '生成随机整数',
      params: [
        { name: 'min', type: 'number', desc: '最小值 (包含)' },
        { name: 'max', type: 'number', desc: '最大值 (包含)' }
      ],
      returns: 'number',
      example: "const num = api.randomInt(1, 100)"
    },
    {
      name: 'randomFloat',
      description: '生成随机浮点数',
      params: [
        { name: 'min', type: 'number', desc: '最小值' },
        { name: 'max', type: 'number', desc: '最大值' },
        { name: 'decimals', type: 'number', desc: '小数位数 (默认 2)' }
      ],
      returns: 'number',
      example: "const price = api.randomFloat(0.1, 10, 4)"
    },
    {
      name: 'randomChoice',
      description: '从数组中随机选择',
      params: [
        { name: 'array', type: 'Array', desc: '数组' }
      ],
      returns: 'any',
      example: "const item = api.randomChoice(['a', 'b', 'c'])"
    },
    {
      name: 'shuffleArray',
      description: '随机打乱数组',
      params: [
        { name: 'array', type: 'Array', desc: '数组' }
      ],
      returns: 'Array',
      example: "const shuffled = api.shuffleArray([1, 2, 3, 4, 5])"
    },
    {
      name: 'sleep',
      description: '休眠指定时间',
      params: [
        { name: 'ms', type: 'number', desc: '毫秒数' }
      ],
      returns: 'Promise<void>',
      example: "await api.sleep(5000)"
    },
    {
      name: 'retry',
      description: '重试函数执行',
      params: [
        { name: 'fn', type: 'Function', desc: '要执行的函数' },
        { name: 'options', type: 'Object', desc: '选项 { maxRetries, delay, onRetry }' }
      ],
      returns: 'Promise<any>',
      example: `const result = await api.retry(async () => {
  return await api.clickElement('.btn')
}, { maxRetries: 3, delay: 1000 })`
    },
    {
      name: 'withTimeout',
      description: '带超时的函数执行',
      params: [
        { name: 'fn', type: 'Function', desc: '要执行的函数' },
        { name: 'timeoutMs', type: 'number', desc: '超时时间 (毫秒)' },
        { name: 'errorMessage', type: 'string', desc: '超时错误信息' }
      ],
      returns: 'Promise<any>',
      example: "const result = await api.withTimeout(async () => { ... }, 30000, '操作超时')"
    },
    {
      name: 'log',
      description: '输出日志',
      params: [
        { name: 'level', type: 'string', desc: '日志级别: info, warn, error, success, debug' },
        { name: 'message', type: 'string', desc: '日志消息' },
        { name: 'data', type: 'any', desc: '附加数据 (可选)' }
      ],
      returns: 'void',
      example: "api.log('info', '开始执行...', { wallet: address })"
    },
    {
      name: 'getTimestamp',
      description: '获取当前时间戳',
      params: [
        { name: 'inSeconds', type: 'boolean', desc: '是否返回秒级时间戳 (默认 false, 毫秒)' }
      ],
      returns: 'number',
      example: "const ts = api.getTimestamp() // 毫秒\nconst tsSec = api.getTimestamp(true) // 秒"
    },
    {
      name: 'formatDate',
      description: '格式化日期',
      params: [
        { name: 'date', type: 'Date|number|string', desc: '日期对象或时间戳' },
        { name: 'format', type: 'string', desc: '格式字符串 (默认 "YYYY-MM-DD HH:mm:ss")' }
      ],
      returns: 'string',
      example: "const date = api.formatDate(new Date(), 'YYYY-MM-DD')"
    },
    {
      name: 'generateId',
      description: '生成唯一 ID',
      params: [
        { name: 'length', type: 'number', desc: 'ID 长度 (默认 16)' }
      ],
      returns: 'string',
      example: "const id = api.generateId(12)"
    },
    {
      name: 'hashString',
      description: '计算字符串哈希',
      params: [
        { name: 'input', type: 'string', desc: '输入字符串' },
        { name: 'algorithm', type: 'string', desc: '算法: sha256, keccak256 (默认 sha256)' }
      ],
      returns: 'string',
      example: "const hash = api.hashString('hello', 'keccak256')"
    },
    {
      name: 'encodeBase64',
      description: 'Base64 编码',
      params: [
        { name: 'input', type: 'string', desc: '输入字符串' }
      ],
      returns: 'string',
      example: "const encoded = api.encodeBase64('hello world')"
    },
    {
      name: 'decodeBase64',
      description: 'Base64 解码',
      params: [
        { name: 'input', type: 'string', desc: 'Base64 字符串' }
      ],
      returns: 'string',
      example: "const decoded = api.decodeBase64('aGVsbG8=')"
    },
    {
      name: 'saveData',
      description: '保存数据到任务上下文',
      params: [
        { name: 'key', type: 'string', desc: '数据键名' },
        { name: 'value', type: 'any', desc: '数据值' }
      ],
      returns: 'void',
      example: "api.saveData('txHash', '0x...')"
    },
    {
      name: 'getData',
      description: '从任务上下文获取数据',
      params: [
        { name: 'key', type: 'string', desc: '数据键名' },
        { name: 'defaultValue', type: 'any', desc: '默认值 (可选)' }
      ],
      returns: 'any',
      example: "const txHash = api.getData('txHash', '')"
    },
  ],
};

// 过滤方法
const filteredMethods = ref(apiMethods.wallet);

const filterMethods = () => {
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase();
    const allMethods = Object.values(apiMethods).flat();
    filteredMethods.value = allMethods.filter(m =>
      m.name.toLowerCase().includes(query) ||
      m.description.toLowerCase().includes(query)
    );
  } else {
    filteredMethods.value = apiMethods[activeCategory.value] || [];
  }
};

const changeCategory = (categoryId) => {
  activeCategory.value = categoryId;
  searchQuery.value = '';
  filteredMethods.value = apiMethods[categoryId];
};

// 复制代码
const copyCode = async (text) => {
  try {
    await navigator.clipboard.writeText(text);
    copiedMethod.value = text;
    Message.success('已复制到剪贴板');
    setTimeout(() => {
      copiedMethod.value = '';
    }, 2000);
  } catch (e) {
    Message.error('复制失败');
  }
};

// 插入代码到编辑器
const emit = defineEmits(['insert-code']);
const insertCode = (example) => {
  emit('insert-code', example);
};
</script>

<template>
  <div class="api-helper">
    <div class="helper-header">
      <div class="header-title">
        <icon-code />
        <span>API 参考</span>
      </div>
    </div>

    <!-- 搜索 -->
    <div class="search-box">
      <a-input-search
        v-model="searchQuery"
        placeholder="搜索 API..."
        @search="filterMethods"
        @clear="filterMethods"
        allow-clear
      />
    </div>

    <!-- 分类标签 -->
    <div class="category-tabs">
      <div
        v-for="cat in categories"
        :key="cat.id"
        class="category-tab"
        :class="{ active: activeCategory === cat.id && !searchQuery }"
        @click="changeCategory(cat.id)"
      >
        {{ cat.name }}
      </div>
    </div>

    <!-- API 列表 -->
    <div class="api-list">
      <div v-for="method in filteredMethods" :key="method.name" class="api-item">
        <div class="api-header" @click="method.expanded = !method.expanded">
          <div class="api-name">
            <code>{{ method.name }}</code>
          </div>
          <div class="api-desc">{{ method.description }}</div>
        </div>

        <div class="api-details" v-if="method.expanded">
          <!-- 参数 -->
          <div class="detail-section">
            <div class="section-title">参数</div>
            <div class="param-list">
              <div v-for="param in method.params" :key="param.name" class="param-item">
                <code class="param-name">{{ param.name }}</code>
                <span class="param-type">{{ param.type }}</span>
                <span class="param-desc">- {{ param.desc }}</span>
              </div>
              <div v-if="method.params.length === 0" class="no-params">无参数</div>
            </div>
          </div>

          <!-- 返回值 -->
          <div class="detail-section">
            <div class="section-title">返回值</div>
            <code class="return-type">{{ method.returns }}</code>
          </div>

          <!-- 示例 -->
          <div class="detail-section">
            <div class="section-title">示例</div>
            <div class="code-example">
              <div class="code-header">
                <span>JavaScript</span>
                <a-space>
                  <a-button size="mini" @click="insertCode(method.example)">
                    插入代码
                  </a-button>
                  <a-button size="mini" @click="copyCode(method.example)">
                    <template #icon><icon-copy /></template>
                  </a-button>
                </a-space>
              </div>
              <pre><code>{{ method.example }}</code></pre>
            </div>
          </div>
        </div>
      </div>

      <div v-if="filteredMethods.length === 0" class="no-results">
        <p>未找到相关 API</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.api-helper {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-2);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  overflow: hidden;
}

.helper-header {
  padding: 12px 15px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-3);
}

.header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}

.search-box {
  padding: 10px 15px;
  border-bottom: 1px solid var(--color-border);
}

.category-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 10px;
  border-bottom: 1px solid var(--color-border);
}

.category-tab {
  padding: 6px 12px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
  color: var(--color-text-3);
}

.category-tab:hover {
  background: var(--color-fill-2);
}

.category-tab.active {
  background: rgba(var(--primary-6), 0.1);
  color: rgb(var(--primary-6));
}

.api-list {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
}

.api-item {
  margin-bottom: 10px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  overflow: hidden;
}

.api-header {
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.2s;
  background: var(--color-bg-1);
}

.api-header:hover {
  background: var(--color-fill-2);
}

.api-name code {
  font-size: 14px;
  font-weight: 600;
  color: rgb(var(--primary-6));
  background: rgba(var(--primary-6), 0.1);
  padding: 2px 6px;
  border-radius: 4px;
}

.api-desc {
  font-size: 12px;
  color: var(--color-text-3);
  margin-top: 4px;
}

.api-details {
  padding: 12px;
  background: var(--color-bg-2);
  border-top: 1px solid var(--color-border);
}

.detail-section {
  margin-bottom: 12px;
}

.detail-section:last-child {
  margin-bottom: 0;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-4);
  text-transform: uppercase;
  margin-bottom: 6px;
}

.param-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-item {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 12px;
}

.param-name {
  background: rgba(var(--primary-6), 0.1);
  color: rgb(var(--primary-6));
  padding: 1px 4px;
  border-radius: 3px;
}

.param-type {
  color: var(--color-text-3);
  font-size: 11px;
}

.param-desc {
  color: var(--color-text-3);
}

.no-params {
  font-size: 12px;
  color: var(--color-text-4);
}

.return-type {
  font-size: 12px;
  color: rgb(var(--success-6));
  background: rgba(var(--success-6), 0.1);
  padding: 2px 6px;
  border-radius: 4px;
}

.code-example {
  background: var(--color-bg-1);
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--color-border);
}

.code-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background: var(--color-fill-2);
  font-size: 11px;
  color: var(--color-text-3);
}

.code-example pre {
  margin: 0;
  padding: 10px;
  overflow-x: auto;
  font-family: 'Fira Code', monospace;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-1);
}

.no-results {
  text-align: center;
  padding: 30px;
  color: var(--color-text-4);
}
</style>
