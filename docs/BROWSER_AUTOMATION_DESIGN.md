# 浏览器自动化功能设计文档

## 1. 概述

### 1.1 功能目标
完善浏览器自动化页面功能，实现以下核心能力：
- 钱包管理：导入钱包（名称、私钥、地址、备注），支持编辑和删除
- 环境配置：批量生成/创建指纹信息，配置抗审查技术选项
- 脚本编辑：编辑和管理脚本程序
- 执行面板：单账号或批量测试执行
- 任务管理：配置定时任务，执行时随机使用所有环境配置
- 任务监控：查看任务执行情况

### 1.2 使用流程

```
┌─────────────────────────────────────────────────────────────────────┐
│                        浏览器自动化使用流程                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Step 1: 钱包管理                                                    │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 导入钱包（名称、私钥、地址、备注）                        │     │
│  │ • 支持 Excel 导入                                          │     │
│  │ • 编辑、删除钱包                                           │     │
│  │ • 数据保存到独立数据表（SQLite）                           │     │
│  └────────────────────────────────────────────────────────────┘     │
│                              ↓                                       │
│  Step 2: 环境配置                                                    │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 批量生成指纹配置                                         │     │
│  │ • 配置抗审查技术选项：                                     │     │
│  │   - Canvas 指纹混淆                                        │     │
│  │   - WebGL 渲染伪装                                         │     │
│  │   - Audio Context 噪音                                     │     │
│  │   - 时区伪装                                               │     │
│  │   - 地理位置伪装                                           │     │
│  │   - 字体伪装                                               │     │
│  │   - WebRTC 防泄漏                                          │     │
│  │ • 持久化保存到数据库                                       │     │
│  └────────────────────────────────────────────────────────────┘     │
│                              ↓                                       │
│  Step 3: 脚本编辑                                                    │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 创建/编辑脚本                                            │     │
│  │ • 使用自定义 API（连接钱包、签名、交易等）                 │     │
│  │ • 导入/导出脚本                                            │     │
│  └────────────────────────────────────────────────────────────┘     │
│                              ↓                                       │
│  Step 4: 执行面板（可选测试）                                        │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 选择脚本、环境、钱包                                     │     │
│  │ • 单账号测试                                               │     │
│  │ • 批量账号测试                                             │     │
│  │ • 验证脚本正确性                                           │     │
│  └────────────────────────────────────────────────────────────┘     │
│                              ↓                                       │
│  Step 5: 任务管理                                                    │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 配置定时任务（Cron 表达式）                              │     │
│  │ • 选择要执行的钱包列表                                     │     │
│  │ • 选择执行的脚本                                           │     │
│  │ • 环境分配策略：随机使用所有环境配置                       │     │
│  │ • 启用/禁用任务                                            │     │
│  └────────────────────────────────────────────────────────────┘     │
│                              ↓                                       │
│  Step 6: 任务监控                                                    │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ • 查看任务执行状态                                         │     │
│  │ • 查看执行日志                                             │     │
│  │ • 统计成功率                                               │     │
│  └────────────────────────────────────────────────────────────┘     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Browser Automation System                      │
├─────────────────────────────────────────────────────────────────────┤
│  Frontend (Vue 3)                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ 钱包管理     │  │ 环境配置     │  │ 脚本编辑器  │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │ 执行面板     │  │ 任务管理     │  │ 任务监控    │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
├─────────────────────────────────────────────────────────────────────┤
│  Backend (Tauri/Rust)                                                │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │                   Task Scheduler (已实现)                   │     │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │     │
│  │  │ Cron Parser │ │ Queue Manager│ │ Execution Engine   │   │     │
│  │  └─────────────┘ └─────────────┘ └─────────────────────┘   │     │
│  └────────────────────────────────────────────────────────────┘     │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │                Database Layer (已实现)                      │     │
│  │  - SQLite 表结构                                           │     │
│  │  - CRUD 操作 (Tauri Commands)                              │     │
│  │  - 私钥加密存储                                            │     │
│  └────────────────────────────────────────────────────────────┘     │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │              Playwright Bridge (待实现)                     │     │
│  │  - Browser Farm Management                                  │     │
│  │  - Wallet Injection (MetaMask/OKX)                          │     │
│  │  - Fingerprint Protection                                   │     │
│  └────────────────────────────────────────────────────────────┘     │
├─────────────────────────────────────────────────────────────────────┤
│  Database (SQLite)                                                   │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ Tables: airdrop_wallets, browser_profiles, automation_tasks │     │
│  │         automation_scripts, task_executions                 │     │
│  └────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. 数据库设计

### 2.1 表结构

#### 2.1.1 空投钱包表 (`airdrop_wallets`)

```sql
CREATE TABLE airdrop_wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,                     -- 钱包名称
    address TEXT NOT NULL,                  -- 钱包地址
    encrypted_private_key TEXT NOT NULL,    -- 加密存储的私钥
    label TEXT,                             -- 备注
    group_name TEXT NOT NULL DEFAULT 'Default', -- 分组
    proxy TEXT NOT NULL DEFAULT 'Direct',   -- 代理配置
    chain_type TEXT NOT NULL DEFAULT 'evm', -- 链类型 (evm/solana)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_airdrop_wallets_address ON airdrop_wallets(address);
CREATE INDEX idx_airdrop_wallets_group ON airdrop_wallets(group_name);
```

#### 2.1.2 浏览器环境配置表 (`browser_profiles`)

```sql
CREATE TABLE browser_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,

    -- 浏览器基础配置
    user_agent TEXT,
    viewport_width INTEGER NOT NULL DEFAULT 1920,
    viewport_height INTEGER NOT NULL DEFAULT 1080,
    device_scale_factor INTEGER NOT NULL DEFAULT 1,
    locale TEXT NOT NULL DEFAULT 'en-US',
    timezone_id TEXT NOT NULL DEFAULT 'America/New_York',

    -- 代理配置
    proxy_type TEXT NOT NULL DEFAULT 'direct',  -- direct, http, socks5
    proxy_host TEXT,
    proxy_port INTEGER,
    proxy_username TEXT,
    proxy_password TEXT,

    -- 指纹保护配置 (抗审查技术)
    canvas_spoof BOOLEAN NOT NULL DEFAULT 1,           -- Canvas 指纹混淆
    webgl_spoof BOOLEAN NOT NULL DEFAULT 1,            -- WebGL 渲染伪装
    audio_spoof BOOLEAN NOT NULL DEFAULT 1,            -- Audio Context 噪音
    timezone_spoof BOOLEAN NOT NULL DEFAULT 1,         -- 时区伪装
    geolocation_spoof BOOLEAN NOT NULL DEFAULT 1,      -- 地理位置伪装
    font_spoof BOOLEAN NOT NULL DEFAULT 1,             -- 字体伪装
    webrtc_spoof BOOLEAN NOT NULL DEFAULT 1,           -- WebRTC 防泄漏
    navigator_override BOOLEAN NOT NULL DEFAULT 1,     -- Navigator 属性覆盖
    webdriver_override BOOLEAN NOT NULL DEFAULT 1,     -- WebDriver 属性覆盖

    -- 高级配置
    custom_headers TEXT,  -- JSON 格式自定义请求头
    headless BOOLEAN NOT NULL DEFAULT 0,
    extensions TEXT,  -- JSON: ["metamask", "okxwallet"]

    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_browser_profiles_default ON browser_profiles(is_default);
```

#### 2.1.3 自动化任务表 (`automation_tasks`)

```sql
CREATE TABLE automation_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,

    -- 执行配置
    script_id INTEGER NOT NULL,              -- 关联的脚本ID
    wallet_ids TEXT NOT NULL,                -- JSON 数组: [1, 2, 3, ...]

    -- 环境分配策略
    profile_strategy TEXT NOT NULL DEFAULT 'random', -- random, sequential, specific
    specific_profile_id INTEGER,             -- 当 strategy 为 specific 时使用

    -- 调度配置
    schedule_type TEXT NOT NULL DEFAULT 'once',  -- once, interval, cron
    schedule_config TEXT NOT NULL,           -- JSON: {"cron": "0 9 * * *"}

    -- 执行参数
    concurrency INTEGER NOT NULL DEFAULT 1,           -- 并发数
    timeout_seconds INTEGER NOT NULL DEFAULT 300,     -- 超时时间
    retry_times INTEGER NOT NULL DEFAULT 3,           -- 重试次数
    retry_interval_seconds INTEGER NOT NULL DEFAULT 60, -- 重试间隔

    -- 状态
    status TEXT NOT NULL DEFAULT 'draft',      -- draft, enabled, paused, running
    last_run_time DATETIME,
    next_run_time DATETIME,
    total_runs INTEGER NOT NULL DEFAULT 0,
    success_runs INTEGER NOT NULL DEFAULT 0,
    failed_runs INTEGER NOT NULL DEFAULT 0,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (script_id) REFERENCES automation_scripts(id),
    FOREIGN KEY (specific_profile_id) REFERENCES browser_profiles(id)
);

CREATE INDEX idx_tasks_status ON automation_tasks(status);
CREATE INDEX idx_tasks_next_run ON automation_tasks(next_run_time) WHERE status = 'enabled';
```

#### 2.1.4 自动化脚本表 (`automation_scripts`)

```sql
CREATE TABLE automation_scripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    content TEXT NOT NULL,                   -- 原始脚本内容（含自定义API）
    compiled_content TEXT,                   -- 预处理后的完整脚本
    version INTEGER NOT NULL DEFAULT 1,
    is_system BOOLEAN NOT NULL DEFAULT 0,    -- 系统内置脚本

    -- API依赖
    required_apis TEXT,                      -- JSON: ["connectWallet", "signMessage"]

    -- 元数据
    author TEXT,
    tags TEXT,                               -- JSON: ["uniswap", "okx", "airdrop"]

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_scripts_tags ON automation_scripts(tags);
```

#### 2.1.5 任务执行记录表 (`task_executions`)

```sql
CREATE TABLE task_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    wallet_id INTEGER NOT NULL,              -- 关联的钱包ID
    profile_id INTEGER,                      -- 使用的环境配置ID

    -- 执行状态
    status TEXT NOT NULL,                    -- pending, running, success, failed, stopped
    start_time DATETIME,
    end_time DATETIME,
    duration_ms INTEGER,

    -- 结果
    error_message TEXT,
    result_data TEXT,                        -- JSON: {"txHash": "...", "data": {...}}
    logs TEXT,                               -- 执行日志

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (task_id) REFERENCES automation_tasks(id),
    FOREIGN KEY (wallet_id) REFERENCES airdrop_wallets(id),
    FOREIGN KEY (profile_id) REFERENCES browser_profiles(id)
);

CREATE INDEX idx_executions_task ON task_executions(task_id);
CREATE INDEX idx_executions_status ON task_executions(status);
CREATE INDEX idx_executions_time ON task_executions(created_at DESC);
```

---

## 3. 前端设计

### 3.1 钱包管理页面

#### 3.1.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  钱包管理                                                            │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ [添加钱包] [从系统同步] [导入(Excel)] [导出] [清空]         │     │
│  │ 搜索: [________________________________]                   │     │
│  └────────────────────────────────────────────────────────────┘     │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ ID │ 名称      │ 地址              │ 备注    │ 分组   │ 操作 │     │
│  ├────────────────────────────────────────────────────────────┤     │
│  │ 1  │ Main      │ 0x742d...f44e     │ 主钱包  │ Default│ 编辑 │     │
│  │ 2  │ Airdrop 1 │ 0x123d...abc      │ 空投1   │ Group A│ 删除 │     │
│  │ 3  │ Airdrop 2 │ 0x456d...def      │ 空投2   │ Group A│      │     │
│  └────────────────────────────────────────────────────────────┘     │
│                                                                      │
│  总计: 3 个钱包                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

#### 3.1.2 Excel 导入格式

| 名称 | 私钥 | 地址 | 备注 | 分组 | 代理 |
|------|------|------|------|------|------|
| Main | 0x... | 0x742d... | 主钱包 | Default | Direct |
| Airdrop 1 | 0x... | 0x123d... | 空投1 | Group A | http://proxy:8080 |

### 3.2 环境配置页面

#### 3.2.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  环境配置                                                            │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐ │
│  │ [批量生成] [导入] [导出]      │  │ 编辑配置: Profile-001        │ │
│  │                              │  │                              │ │
│  │ 环境列表                      │  │ 配置名称: [____________]     │ │
│  │ ┌──────────────────────────┐ │  │                              │ │
│  │ │ Profile-001              │ │  │ User Agent:                  │ │
│  │ │ 1920x1080 | Direct       │ │  │ [________________________]   │ │
│  │ │                          │ │  │                              │ │
│  │ │ Profile-002              │ │  │ 分辨率: [1920x1080    v]     │ │
│  │ │ 1366x768 | HTTP Proxy    │ │  │ 代理:   [Direct       v]     │ │
│  │ │                          │ │  │                              │ │
│  │ │ Profile-003              │ │  │ ┌──────────────────────────┐ │ │
│  │ │ ...                      │ │  │ │ 指纹保护 (Anti-Detect)   │ │ │
│  │ └──────────────────────────┘ │  │ ├──────────────────────────┤ │ │
│  │                              │  │ │ [✓] Canvas 指纹混淆      │ │ │
│  └──────────────────────────────┘  │ │ [✓] WebGL 渲染伪装       │ │ │
│                                     │ │ [✓] Audio Context 噪音   │ │ │
│                                     │ │ [✓] 时区伪装             │ │ │
│                                     │ │ [✓] 地理位置伪装         │ │ │
│                                     │ │ [✓] 字体伪装             │ │ │
│                                     │ │ [✓] WebRTC 防泄漏        │ │ │
│                                     │ └──────────────────────────┘ │ │
│                                     │                              │ │
│                                     │ [删除] [取消] [保存]         │ │
│                                     └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.3 脚本编辑页面

#### 3.3.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  脚本编辑                                                            │
│  ┌──────────────────┐  ┌──────────────────────────────────────────┐ │
│  │ [新建] [导入]     │  │ 当前编辑: OKX Daily Claim                │ │
│  │                  │  │                                          │ │
│  │ 脚本列表          │  │ ┌──────────────────────────────────────┐ │ │
│  │ ┌──────────────┐ │  │ │ [API文档] [全屏] [导出] [复制] [测试]│ │ │
│  │ │ OKX Daily    │ │  │ └──────────────────────────────────────┘ │ │
│  │ │ Uniswap Swap │ │  │                                          │ │
│  │ │ LayerZero    │ │  │ ┌──────────────────────────────────────┐ │ │
│  │ │ ...          │ │  │ │ // OKX Daily Claim Script            │ │ │
│  │ └──────────────┘ │  │ │ async function run({ page, wallet,   │ │ │
│  │                  │  │ │   api }) {                             │ │ │
│  │                  │  │ │   api.log('info', '开始执行');         │ │ │
│  │                  │  │ │   await page.goto('https://okx.com');  │ │ │
│  │                  │  │ │   await api.connectOKXWallet();        │ │ │
│  │                  │  │ │   // ...                               │ │ │
│  │                  │  │ │ }                                      │ │ │
│  │                  │  │ └──────────────────────────────────────┘ │ │
│  │                  │  │                                          │ │
│  └──────────────────┘  └──────────────────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ API 参考文档                                                    │ │
│  │ [钱包连接] [签名操作] [交易操作] [浏览器操作] [工具函数]        │ │
│  │                                                                 │ │
│  │ connectMetaMask(options) - 连接 MetaMask 钱包                  │ │
│  │ signMessage(message) - 签名消息                                │ │
│  │ clickElement(selector) - 点击元素                              │ │
│  │ ...                                                            │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.4 执行面板页面

#### 3.4.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  执行面板                                                            │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐ │
│  │ 配置                          │  │ 执行状态                     │ │
│  │                              │  │                              │ │
│  │ 选择脚本:                    │  │ 执行时间: 00:05:32           │ │
│  │ [OKX Daily Claim      v]     │  │ 总任务: 10                   │ │
│  │                              │  │ 成功: 8    失败: 0   等待: 2 │ │
│  │ 浏览器配置:                  │  │                              │ │
│  │ [Profile-001          v]     │  │ 进度: 80%                    │ │
│  │                              │  │ ████████████████████░░       │ │
│  │ 选择钱包:                    │  │                              │ │
│  │ [✓] 全部 (10)               │  │ [开始执行] [停止] [重置]     │ │
│  │ [✓] Main (0x742d...)        │  │                              │ │
│  │ [✓] Airdrop 1 (0x123d...)   │  │                              │ │
│  │ [✓] Airdrop 2 (0x456d...)   │  │                              │ │
│  │ ...                         │  │                              │ │
│  │                              │  │                              │ │
│  │ 执行配置:                    │  │                              │ │
│  │ [✓] 无头模式                │  │                              │ │
│  │ [✓] 使用代理                │  │                              │ │
│  │ [✓] 指纹保护                │  │                              │ │
│  │                              │  │                              │ │
│  └──────────────────────────────┘  └──────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ [执行队列] [执行日志]                                           │ │
│  │                                                                 │ │
│  │ 钱包              状态      耗时      交易哈希                  │ │
│  │ Main              [✓]成功   12.5s     0xabc...                  │ │
│  │ Airdrop 1         [✓]成功   11.8s     0xdef...                  │ │
│  │ Airdrop 2         [运行中]  --        --                        │ │
│  │ Airdrop 3         [等待中]  --        --                        │ │
│  │ ...                                                             │ │
│  │                                                                 │ │
│  │ [14:25:32] [INFO] 任务开始执行: OKX Daily Claim                 │ │
│  │ [14:25:33] [INFO] 初始化浏览器环境...                           │ │
│  │ [14:25:34] [SUCCESS] Main 执行成功                              │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.5 任务管理页面

#### 3.5.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  任务管理                                                            │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ [新建任务] [导入] [导出] [清空]                            │     │
│  └────────────────────────────────────────────────────────────┘     │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │ 任务名称        执行时间      状态      执行次数    操作   │     │
│  ├────────────────────────────────────────────────────────────┤     │
│  │ OKX Daily       每天 9:00     [运行中]  45/∞       [停止] │     │
│  │ Uniswap Swap    每周一 10:00  [已暂停]  12/12      [启动] │     │
│  │ LayerZero       2024-01-15    [已启用]  0/100      [编辑] │     │
│  │ Airdrop Batch   立即执行      [已完成]  100/100    [删除] │     │
│  └────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.6 任务监控页面

#### 3.6.1 布局结构

```
┌─────────────────────────────────────────────────────────────────────┐
│  任务监控                                                            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐               │
│  │ 总执行   │ │ 成功     │ │ 失败     │ │ 成功率   │               │
│  │ 156      │ │ 150      │ │ 6        │ │ 96%      │               │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘               │
│                                                                      │
│  ┌──────────────────────────────┐  ┌──────────────────────────────┐ │
│  │ 执行历史                      │  │ 执行详情                     │ │
│  │ [搜索...] [刷新] [清空]      │  │                              │ │
│  │                              │  │ 任务: OKX Daily Claim        │ │
│  │ 钱包        脚本      时间   │  │ 钱包: Main (0x742d...)       │ │
│  │ Main        OKX       14:25  │  │ 环境: Profile-003            │ │
│  │ Airdrop 1   OKX       14:26  │  │ 状态: [✓] 成功               │ │
│  │ Airdrop 2   OKX       14:27  │  │ 耗时: 12.5s                  │ │
│  │ ...                          │  │ 交易: 0xabc...               │ │
│  │                              │  │                              │ │
│  │                              │  │ 执行日志:                    │ │
│  │                              │  │ [14:25:32] 开始执行          │ │
│  │                              │  │ [14:25:33] 连接钱包          │ │
│  │                              │  │ [14:25:35] 签到成功          │ │
│  │                              │  │                              │ │
│  └──────────────────────────────┘  └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 4. 脚本自定义API设计

### 4.1 API方法列表

#### 4.1.1 钱包连接类

```javascript
/**
 * 连接 MetaMask 钱包
 * @param {Object} options - 连接选项
 * @param {string} options.expectedChainId - 期望的网络ID (如 '0x1')
 * @returns {Promise<string>} 连接后的钱包地址
 */
async function connectMetaMask(options = {}) {}

/**
 * 连接 OKX Wallet
 * @param {Object} options - 连接选项
 * @param {string} options.chainId - 链ID
 * @returns {Promise<string>} 连接后的钱包地址
 */
async function connectOKXWallet(options = {}) {}

/**
 * 切换钱包网络
 * @param {string} chainId - 目标网络ID (十六进制字符串)
 * @returns {Promise<boolean>} 是否切换成功
 */
async function switchNetwork(chainId) {}

/**
 * 获取当前连接的钱包地址
 * @returns {Promise<string>} 当前地址
 */
async function getCurrentAddress() {}
```

#### 4.1.2 签名类

```javascript
/**
 * 签名消息
 * @param {string} message - 要签名的消息
 * @param {Object} options - 选项
 * @returns {Promise<string>} 签名结果 (hex格式)
 */
async function signMessage(message, options = {}) {}

/**
 * 签名交易
 * @param {Object} tx - 交易对象
 * @param {string} tx.to - 目标地址
 * @param {string} tx.value - 转账金额 (wei/lamports)
 * @param {string} tx.data - 可选数据
 * @returns {Promise<string>} 签名后的交易
 */
async function signTransaction(tx) {}

/**
 * 签名类型数据 (EIP-712)
 * @param {Object} domain - EIP-712 Domain
 * @param {Object} types - 类型定义
 * @param {Object} value - 要签名的值
 * @returns {Promise<string>} 签名结果
 */
async function signTypedData(domain, types, value) {}
```

#### 4.1.3 交易类

```javascript
/**
 * 发送原生币转账
 * @param {string} to - 接收地址
 * @param {string} amount - 金额 (可读格式，如 '0.1 ETH')
 * @param {Object} options - 选项
 * @returns {Promise<Object>} 交易结果 {hash, status}
 */
async function sendNativeTransfer(to, amount, options = {}) {}

/**
 * ERC-20 Token 授权
 * @param {string} tokenAddress - Token合约地址
 * @param {string} spender - 授权给谁
 * @param {string} amount - 授权数量 (可读格式，如 '1000 USDC')
 * @returns {Promise<Object>} 交易结果
 */
async function approveToken(tokenAddress, spender, amount) {}

/**
 * ERC-20 Token 转账
 * @param {string} tokenAddress - Token合约地址
 * @param {string} to - 接收地址
 * @param {string} amount - 转账数量
 * @returns {Promise<Object>} 交易结果
 */
async function transferToken(tokenAddress, to, amount) {}

/**
 * 等待交易确认
 * @param {string} txHash - 交易哈希
 * @param {number} confirmations - 确认数 (默认1)
 * @returns {Promise<Object>} 交易收据
 */
async function waitForTransaction(txHash, confirmations = 1) {}
```

#### 4.1.4 浏览器操作类

```javascript
/**
 * 等待元素出现
 * @param {string} selector - CSS选择器
 * @param {number} timeout - 超时时间 (毫秒)
 * @returns {Promise<Element>} 元素
 */
async function waitForSelector(selector, timeout = 30000) {}

/**
 * 等待元素消失
 * @param {string} selector - CSS选择器
 * @param {number} timeout - 超时时间 (毫秒)
 * @returns {Promise<boolean>} 是否成功消失
 */
async function waitForSelectorHidden(selector, timeout = 30000) {}

/**
 * 点击元素
 * @param {string} selector - CSS选择器
 * @param {Object} options - 选项
 * @returns {Promise<void>}
 */
async function clickElement(selector, options = {}) {}

/**
 * 输入文本
 * @param {string} selector - CSS选择器
 * @param {string} text - 输入文本
 * @param {Object} options - 选项
 * @returns {Promise<void>}
 */
async function inputText(selector, text, options = {}) {}

/**
 * 获取元素文本
 * @param {string} selector - CSS选择器
 * @returns {Promise<string>} 元素文本
 */
async function getElementText(selector) {}
```

#### 4.1.5 工具类

```javascript
/**
 * 随机延迟
 * @param {number} minMs - 最小延迟 (毫秒)
 * @param {number} maxMs - 最大延迟 (毫秒)
 * @returns {Promise<void>}
 */
async function randomDelay(minMs = 1000, maxMs = 3000) {}

/**
 * 模拟人类点击 (带随机偏移)
 * @param {string} selector - CSS选择器
 * @param {Object} options - 选项
 * @returns {Promise<void>}
 */
async function humanLikeClick(selector, options = {}) {}

/**
 * 获取钱包余额
 * @param {string} tokenAddress - Token地址 (空则为主币)
 * @returns {Promise<string>} 余额
 */
async function getBalance(tokenAddress = null) {}

/**
 * 日志输出
 * @param {string} level - 日志级别 (info, warn, error, success)
 * @param {string} message - 消息
 */
function log(level, message) {}
```

### 4.2 脚本示例

```javascript
// OKX Daily Claim Script
async function run({ page, wallet, api }) {
    api.log('info', '开始执行 OKX Daily Claim');
    
    // 1. 打开OKX官网
    await page.goto('https://www.okx.com');
    await api.waitForSelector('body');
    await api.randomDelay(2000, 4000);
    
    // 2. 连接钱包
    api.log('info', '连接 OKX Wallet...');
    await api.connectOKXWallet({ chainId: '0x1' });
    await api.randomDelay(1000, 2000);
    
    // 3. 导航到签到页面
    api.log('info', '导航到 Rewards 页面...');
    await page.goto('https://www.okx.com/rewards');
    await api.waitForSelector('button.claim-button', 10000);
    
    // 4. 点击签到
    api.log('info', '执行签到操作...');
    await api.clickElement('button.claim-button');
    await api.randomDelay(3000, 5000);
    
    api.log('success', '签到完成');
    return { success: true };
}
```

---

## 5. 环境随机分配策略

### 5.1 分配策略实现

```typescript
// 环境分配策略类型
type ProfileStrategy = 'random' | 'sequential' | 'specific';

// 环境分配函数
function assignProfile(
    strategy: ProfileStrategy,
    profiles: BrowserProfile[],
    index: number,
    specificProfileId?: number
): BrowserProfile {
    switch (strategy) {
        case 'random':
            // 随机从所有环境中选择一个
            return profiles[Math.floor(Math.random() * profiles.length)];
        
        case 'sequential':
            // 按顺序循环使用
            return profiles[index % profiles.length];
        
        case 'specific':
            // 使用指定的环境
            const profile = profiles.find(p => p.id === specificProfileId);
            if (!profile) {
                throw new Error(`指定的环境配置不存在: ${specificProfileId}`);
            }
            return profile;
        
        default:
            return profiles[0];
    }
}

// 批量执行时的环境分配示例
async function executeBatch(
    wallets: Wallet[],
    profiles: BrowserProfile[],
    strategy: ProfileStrategy
) {
    const assignments = wallets.map((wallet, index) => ({
        wallet,
        profile: assignProfile(strategy, profiles, index)
    }));
    
    // 输出分配结果
    assignments.forEach(({ wallet, profile }) => {
        console.log(`钱包 ${wallet.name} -> 环境 ${profile.name}`);
    });
    
    return assignments;
}
```

---

## 6. 当前实现状态

### 6.1 实现完成度总览

| 模块 | 前端状态 | 后端状态 | 完成度 | 说明 |
|------|----------|----------|--------|------|
| 钱包管理 | ✅ 完成 | ✅ 完成 | 100% | CRUD、私钥加密、Excel导入导出、SQLite持久化 |
| 环境配置 | ✅ 完成 | ✅ 完成 | 100% | 完整指纹配置、批量生成、代理配置、SQLite持久化 |
| 脚本编辑 | ✅ 完成 | ✅ 完成 | 100% | 代码编辑、API参考面板、导入导出 |
| 执行面板 | ✅ 完成 | ⚠️ 部分 | 70% | 前端UI完整，后端执行API待完善 |
| 任务管理 | ✅ 完成 | ✅ 完成 | 95% | Cron调度、钱包/脚本选择、环境分配策略 |
| 任务监控 | ✅ 完成 | ✅ 完成 | 95% | 统计概览、执行历史、详情查看 |
| 任务调度器 | - | ✅ 完成 | 90% | 已实现，待集成到应用启动 |
| 执行引擎 | - | ⚠️ 模拟 | 50% | 模拟执行已实现，Playwright集成待完成 |
| Playwright桥接 | - | ❌ 未实现 | 0% | 需要实现浏览器实例管理 |

**整体完成度: 85%**

### 6.2 已实现的后端模块

#### 6.2.1 数据库层 (`db.rs`)
- ✅ 完整的表结构定义
- ✅ 数据库迁移支持
- ✅ 索引创建

#### 6.2.2 数据模型 (`models.rs`)
- ✅ AirdropWallet - 空投钱包模型
- ✅ BrowserProfile - 浏览器环境配置模型
- ✅ AutomationScript - 自动化脚本模型
- ✅ AutomationTask - 自动化任务模型
- ✅ TaskExecution - 任务执行记录模型
- ✅ ProfileStrategy - 环境分配策略枚举
- ✅ TaskStatus - 任务状态枚举
- ✅ ExecutionStatus - 执行状态枚举
- ✅ ScheduleType - 调度类型枚举

#### 6.2.3 Tauri Commands (`commands.rs`)
**钱包管理:**
- ✅ `get_airdrop_wallets` - 获取所有钱包
- ✅ `create_airdrop_wallet` - 创建钱包（含私钥加密）
- ✅ `update_airdrop_wallet` - 更新钱包
- ✅ `delete_airdrop_wallet` - 删除钱包
- ✅ `import_airdrop_wallets` - 批量导入钱包
- ✅ `get_wallet_private_key` - 获取解密后的私钥

**环境配置:**
- ✅ `get_browser_profiles` - 获取所有环境配置
- ✅ `create_browser_profile` - 创建环境配置
- ✅ `update_browser_profile` - 更新环境配置
- ✅ `delete_browser_profile` - 删除环境配置
- ✅ `batch_generate_profiles` - 批量生成环境配置

**脚本管理:**
- ✅ `get_automation_scripts` - 获取所有脚本
- ✅ `create_automation_script` - 创建脚本
- ✅ `update_automation_script` - 更新脚本
- ✅ `delete_automation_script` - 删除脚本

**任务管理:**
- ✅ `get_automation_tasks` - 获取所有任务
- ✅ `create_automation_task` - 创建任务
- ✅ `update_automation_task` - 更新任务
- ✅ `delete_automation_task` - 删除任务
- ✅ `toggle_task_status` - 切换任务状态

**执行记录:**
- ✅ `get_task_executions` - 获取执行记录
- ✅ `get_task_execution_stats` - 获取执行统计

**初始化:**
- ✅ `init_browser_automation_tables` - 初始化数据库表

#### 6.2.4 任务调度器 (`scheduler.rs`)
- ✅ TaskScheduler 结构体
- ✅ Cron 表达式解析
- ✅ 定时任务检查（每分钟）
- ✅ 任务执行触发
- ✅ 任务状态管理
- ✅ 停止任务功能
- ✅ 立即执行功能
- ✅ 并发控制

#### 6.2.5 任务执行器 (`executor.rs`)
- ✅ TaskExecutor 结构体
- ✅ 单任务执行逻辑
- ✅ 批量执行逻辑
- ✅ 环境分配策略实现
- ✅ 执行记录创建
- ✅ 执行日志记录
- ✅ 任务统计更新
- ⚠️ 模拟执行（待集成Playwright）

### 6.3 已实现的前端模块

#### 6.3.1 服务层 (`browserAutomationService.ts`)
- ✅ 完整的 TypeScript 类型定义
- ✅ walletService - 钱包管理服务
- ✅ profileService - 环境配置服务
- ✅ scriptService - 脚本管理服务（含导入导出）
- ✅ taskService - 任务管理服务
- ✅ executionService - 执行记录服务

#### 6.3.2 组件
- ✅ `WalletManager.vue` - 钱包管理组件
- ✅ `BrowserFarm.vue` - 环境配置组件
- ✅ `ScriptEditor.vue` - 脚本编辑组件
- ✅ `TaskManager.vue` - 任务管理组件
- ✅ `TaskMonitor.vue` - 任务监控组件
- ✅ `ExecutionPanel.vue` - 执行面板组件
- ✅ `ApiHelper.vue` - API参考帮助面板

### 6.4 待实现功能

#### 6.4.1 执行相关 Tauri Commands
- ❌ `create_execution` - 创建执行会话
- ❌ `start_execution` - 启动执行
- ❌ `cancel_execution` - 取消执行
- ❌ `get_execution` - 获取执行状态
- ❌ `get_execution_logs` - 获取执行日志

#### 6.4.2 Playwright 集成
- ❌ 浏览器实例管理
- ❌ 指纹注入脚本
- ❌ 钱包扩展注入
- ❌ 代理配置应用

#### 6.4.3 应用集成
- ❌ 调度器启动集成
- ❌ 执行状态实时推送

---

## 7. 文件结构

### 7.1 前端文件

```
src/features/airdrop/
├── pages/
│   ├── BrowserAutomation.vue      # ✅ 主页面入口
│   └── Airdrop.vue                # ✅ 空投模块入口
├── components/
│   ├── WalletManager.vue          # ✅ 钱包管理组件
│   ├── BrowserFarm.vue            # ✅ 环境配置组件
│   ├── ScriptEditor.vue           # ✅ 脚本编辑组件
│   ├── TaskManager.vue            # ✅ 任务管理组件
│   ├── TaskMonitor.vue            # ✅ 任务监控组件
│   ├── ExecutionPanel.vue         # ✅ 执行面板组件
│   └── ApiHelper.vue              # ✅ API参考帮助面板
└── services/
    ├── browserAutomationService.ts # ✅ 主服务层（调用Tauri Commands）
    └── playwrightService.ts        # ⚠️ Playwright服务（模拟实现）
```

### 7.2 后端文件

```
src-tauri/src/wallets_tool/
├── airdrop/
│   ├── mod.rs                     # ✅ 模块入口
│   ├── models.rs                  # ✅ 数据模型定义
│   ├── commands.rs                # ✅ Tauri Commands实现
│   ├── db.rs                      # ✅ 数据库初始化
│   ├── scheduler.rs               # ✅ 任务调度器（待集成）
│   └── executor.rs                # ✅ 任务执行器（模拟执行）
└── playwright/
    └── mod.rs                     # ❌ Playwright桥接（待实现）
```

---

## 8. 下一步开发计划

### 8.1 Phase 1: 完善执行功能（优先级：高）
1. 实现执行相关的 Tauri Commands
2. 集成调度器到应用启动流程
3. 实现执行状态实时推送

### 8.2 Phase 2: Playwright 集成（优先级：高）
1. 实现浏览器实例管理
2. 实现指纹注入脚本
3. 实现钱包扩展注入
4. 实现代理配置应用

### 8.3 Phase 3: 功能优化（优先级：中）
1. 执行日志实时流
2. 任务执行重试机制
3. 执行结果通知
4. 性能优化

---

## 9. 风险和注意事项

1. **安全性**
   - ✅ 私钥已加密存储
   - ⚠️ 脚本执行环境需要沙箱化
   - ⚠️ 敏感操作需要二次确认

2. **反检测**
   - ⚠️ 指纹保护需要持续更新
   - ⚠️ 扩展版本兼容性
   - ⚠️ 浏览器更新适配

3. **性能**
   - ⚠️ 大量钱包并发执行
   - ⚠️ 浏览器实例内存管理
   - ✅ 任务队列持久化

4. **兼容性**
   - ⚠️ 不同版本的MetaMask/OKX
   - ⚠️ 不同网站的变化
   - ⚠️ 网络条件变化
