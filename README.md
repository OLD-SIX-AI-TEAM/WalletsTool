[![GitHub release](https://img.shields.io/github/v/release/WalletsTool/WalletsTool)](https://github.com/WalletsTool/WalletsTool/releases)
[![Release](https://github.com/WalletsTool/WalletsTool/actions/workflows/release.yml/badge.svg)](https://github.com/WalletsTool/WalletsTool/actions/workflows/release.yml)
![Tauri](https://img.shields.io/badge/Tauri-Rust-orange) ![Vue 3](https://img.shields.io/badge/Vue-3-42b883) ![Desktop](https://img.shields.io/badge/Platform-Desktop-blue)

<div align="center" style="margin-bottom: 20px;">
    <img src="app-icon.png" width="80" height="80" alt="应用logo"/>
</div>

<p align="center">
  <a href="https://github.com/WalletsTool/WalletsTool/releases">快速开始 | Download</a>
</p>

> [!WARNING]
> 本项目仅供区块链工作室交流学习使用。使用本工具需自行承担风险，请确保在使用前充分了解相关区块链网络的规则和风险，谨慎操作您的数字资产。


**Web3 多链钱包管理工具** —— 专为工作室和专业用户打造的桌面端神器。

基于 Vue 3 + Tauri (Rust) 构建，以**极致安全**和**工业级性能**为核心，打破 EVM 与 Solana 生态壁垒，提供批量归集、分发、查询的一站式解决方案。

## 🌟 核心优势

### 🛡️ 银行级安全架构
我们深知私钥是资产的生命线。与网页插件不同，WalletsTool 采用**零信任**安全设计：
- **内存隐身 (Memory Only)**：私钥**永不落盘**（不存数据库、不存文件），仅在易失性内存中驻留。
- **动态加密**：采用 **AES-256-CBC** 对内存中的私钥进行加密，仅在签名瞬间解密。
- **即用即焚 (Zeroize)**：签名完成后，内存中的敏感数据立即执行物理擦除，防止冷启动攻击。
- **反调试护盾**：内置运行时反调试机制，阻断恶意软件的内存扫描。
- **数据库加密**：使用 SQLCipher 进行全盘加密，PBKDF2 密钥派生（600,000 次迭代）。

### ⚡ 狂暴模式 (Fury Mode)
告别单线程的龟速等待。基于 Rust `tokio` 异步运行时，释放硬件潜能：
- **90+ 线程并发**：支持同时对上百个地址进行交互，速度仅受限于 RPC 节点限制。
- **批量提交+统一确认**：狂暴模式下批量提交交易后统一确认，最大化吞吐量。
- **智能 Gas 策略**：批量操作时自动估算并优化 Gas，不仅快，而且省。
- **智能防重**：内置链上状态检测与智能重试机制（最多3轮），杜绝因网络抖动导致的重复转账。
- **Gas 监控**：实时监控 Gas 价格，异常时自动暂停保护资产安全。

### 💼 工作室级工作流
专为高频交互场景优化，让繁琐操作变得丝滑：
- **Excel 无缝集成**：支持直接导入 Excel 表格生成/导入钱包，查询结果一键导出报表，财务对账零压力。
- **多窗口并行**：支持同时打开多个子窗口（如主窗口监控余额，子窗口执行转账），多任务并行处理互不干扰。
- **全球网络畅连**：内置 **HTTP/SOCKS5 代理管理器**，确保每一笔交易都能触达全球节点，保护隐私。
- **虚拟滚动表格**：基于 `@tanstack/vue-virtual` 的虚拟滚动，轻松处理万级数据不卡顿。

### 🔗 全栈多链支持
- **Ethereum 生态**：完美支持 ETH 主网及所有 EVM 兼容链（BSC, Arbitrum, Optimism, Polygon, Base 等）。
- **Solana 生态**：原生支持 Solana 高性能链，SOL 和 SPL 代币全支持。
- **统一资产管理**：打破生态隔阂，一个界面统一管理多链资产。

### 🤖 浏览器自动化
- **Playwright 驱动**：基于 Playwright 的浏览器自动化框架。
- **钱包并发执行**：支持多钱包同时执行交互任务。
- **脚本化操作**：可编写自定义脚本执行复杂的 DApp 交互。
- **任务管理器**：可视化任务编排和状态监控。

## 🚀 快速开始

### 一键启动（推荐）

本项目内置智能依赖管理，只需一条命令即可自动安装 Node.js、Rust 等所有环境并启动：

```bash
yarn start
```

### 环境要求

- **Node.js** >= 18.0.0
- **Rust** >= 1.70.0
- **Tauri CLI** >= 2.0.0
- **Perl** >= 5.30.0 (用于构建依赖) - [下载地址](https://strawberryperl.com/)

### Windows 额外依赖

本项目使用 **SQLCipher** 进行数据库加密，需要 **OpenSSL** 支持：

1. **安装 OpenSSL**
   - 下载安装 [OpenSSL Light for Windows](https://slproweb.com/products/Win32OpenSSL.html)（选择 Win64 版本）
   - 建议安装路径：`C:\Program Files\OpenSSL-Win64`

2. **设置环境变量**

   **PowerShell (推荐)**:
   ```powershell
   # 临时设置（当前终端会话）
   $env:OPENSSL_DIR = "C:\Program Files\OpenSSL-Win64"
   ```

   **系统环境变量 (永久生效)**:
   ```powershell
   # 以管理员身份运行 PowerShell
   [Environment]::SetEnvironmentVariable("OPENSSL_DIR", "C:\Program Files\OpenSSL-Win64", "Machine")
   ```

   **或者手动设置**:
   - 打开「系统属性」→「高级」→「环境变量」
   - 新建系统变量 `OPENSSL_DIR`，值为 `C:\Program Files\OpenSSL-Win64`

3. **验证配置**
   ```powershell
   # 检查环境变量是否设置成功
   echo $env:OPENSSL_DIR
   # 预期输出: C:\Program Files\OpenSSL-Win64
   ```

> [!IMPORTANT]
> 如果看到错误 `Missing environment variable OPENSSL_DIR`，说明环境变量未正确设置。请确保在**新的终端窗口**中运行命令，或重启 IDE/终端以应用更改。

### 开发命令

```bash
# 前端开发服务器
yarn dev

# 完整开发环境（推荐）
yarn tauri-dev

# 生产构建
yarn tauri-build

# Rust 测试
cargo test
```

## 📖 功能概览

| 模块 | 功能描述 |
|------|----------|
| **链配置** | 自定义添加 EVM/Solana 网络，管理 RPC 节点与代币列表 |
| **钱包管理** | 批量导入/生成钱包，支持 Excel 导入导出，私钥安全加密，分组管理 |
| **资产查询** | 多线程批量查询原生代币及 ERC-20/SPL 代币余额，支持导出 Excel |
| **批量转账** | 支持一对多（分发）、多对多（归集/交互）批量转账，狂暴模式，实时状态监控 |
| **链上监控** | 监控地址余额与 Nonce 变化，实时推送通知 |
| **浏览器自动化** | Playwright 驱动的浏览器自动化，支持多钱包并发执行交互任务 |
| **代理设置** | HTTP/SOCKS5 代理管理，全球节点畅连 |
| **数据库热重载** | 支持数据库热重载，无需重启应用 |

## 🏗️ 技术栈

### 前端
- **Framework**: Vue 3.5 + Composition API
- **Router**: Vue Router 4.6
- **State**: Pinia 3.0
- **Build**: Vite 7
- **UI Components**: Arco Design Vue 2.57 + PrimeVue 4.5
- **Icons**: PrimeIcons 7.0 + Iconify Vue 5.0
- **Blockchain**: Ethers 6.13, @solana/web3.js 1.91
- **Utils**: XLSX 0.18, QRCode 1.5, @tanstack/vue-virtual 3.13

### 后端
- **Framework**: Tauri 2.0 (Rust)
- **Async Runtime**: Tokio 1.47
- **Database**: SQLite + SQLCipher (加密)
- **ORM**: SQLx 0.7
- **Ethereum**: Alloy 1.4 (alloy-provider, alloy-primitives, alloy-signer-local)
- **Solana**: solana-sdk 2.2, spl-token 7.0
- **Security**: AES-256, Zeroize, PBKDF2, SHA2, HMAC
- **Automation**: Playwright

### 安全特性
- **内存加密**: AES-256-CBC 动态加密
- **内存擦除**: Zeroize 物理擦除
- **反调试**: Windows 平台运行时保护
- **传输加密**: RSA-OAEP / AES-256-GCM 加密传输
- **数据库加密**: SQLCipher PBKDF2 (600,000 iterations)

## 📁 项目结构

```
./
├── src/                      # Vue 3 前端 (Feature-based)
│   ├── features/             # 业务模块
│   │   ├── ethereum/         # EVM 功能 (transfer/balance/monitor)
│   │   ├── solana/           # Solana 功能 (transfer/balance)
│   │   ├── airdrop/          # 浏览器自动化
│   │   ├── wallet_manager/   # 钱包管理
│   │   └── home/             # 首页
│   ├── components/           # 全局组件
│   ├── stores/               # Pinia 状态管理
│   ├── router/               # 路由配置
│   └── utils/                # 工具函数
├── src-tauri/                # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── database/         # SQLite 数据库层
│   │   └── wallets_tool/     # 核心业务逻辑
│   │       ├── ecosystems/   # 链实现 (ethereum/solana)
│   │       ├── security/     # 安全模块
│   │       ├── wallet_manager/  # 钱包管理
│   │       ├── airdrop/      # 空投自动化
│   │       └── playwright/   # 浏览器自动化
│   └── data/                 # 数据库文件
└── docs/                     # 文档
```

## 🤝 贡献与支持

欢迎提交 [Issue](https://github.com/WalletsTool/WalletsTool/issues) 或 [Pull Request](https://github.com/WalletsTool/WalletsTool/pulls) 参与贡献。

## 📄 许可证

Copyright © 2026 EzBan. All rights reserved.

---

**免责声明**：本工具仅供学习研究，使用者需自行承担风险。
