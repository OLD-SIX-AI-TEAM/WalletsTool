use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

/// 空投钱包模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AirdropWallet {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub encrypted_private_key: String,
    pub label: Option<String>,
    pub group_name: String,
    pub proxy: String,
    pub chain_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建空投钱包请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAirdropWalletRequest {
    pub name: String,
    pub address: String,
    pub private_key: String,
    pub label: Option<String>,
    pub group_name: Option<String>,
    pub proxy: Option<String>,
    pub chain_type: Option<String>,
}

/// 更新空投钱包请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAirdropWalletRequest {
    pub id: i64,
    pub name: Option<String>,
    pub address: Option<String>,
    pub private_key: Option<String>,
    pub label: Option<String>,
    pub group_name: Option<String>,
    pub proxy: Option<String>,
    pub chain_type: Option<String>,
}

/// 浏览器环境配置模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BrowserProfile {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub user_agent: Option<String>,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub device_scale_factor: i32,
    pub locale: String,
    pub timezone_id: String,
    pub proxy_type: String,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub canvas_spoof: bool,
    pub webgl_spoof: bool,
    pub audio_spoof: bool,
    pub timezone_spoof: bool,
    pub geolocation_spoof: bool,
    pub font_spoof: bool,
    pub webrtc_spoof: bool,
    pub navigator_override: bool,
    pub webdriver_override: bool,
    pub custom_headers: Option<String>,
    pub headless: bool,
    pub extensions: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // 增强指纹字段
    pub hardware_concurrency: Option<i32>,
    pub device_memory: Option<i32>,
    pub color_depth: Option<i32>,
    pub languages: Option<String>,
    pub vendor: Option<String>,
    pub gpu_vendor: Option<String>,
    pub gpu_renderer: Option<String>,
    pub color_scheme: Option<String>,
    pub max_touch_points: Option<i32>,
    pub has_touch: Option<bool>,
    pub screen_orientation_angle: Option<i32>,
    pub screen_orientation_type: Option<String>,
    pub font_family: Option<String>,
    pub client_hints_platform: Option<String>,
    pub client_hints_platform_version: Option<String>,
    pub client_hints_architecture: Option<String>,
    pub client_hints_bitness: Option<String>,
    pub client_hints_model: Option<String>,
    pub client_hints_wow64: Option<String>,
    pub fingerprint_hash: Option<String>,
    pub platform_name: Option<String>,
}

/// 创建浏览器环境配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBrowserProfileRequest {
    pub name: String,
    pub description: Option<String>,
    pub user_agent: Option<String>,
    pub viewport_width: Option<i32>,
    pub viewport_height: Option<i32>,
    pub device_scale_factor: Option<i32>,
    pub locale: Option<String>,
    pub timezone_id: Option<String>,
    pub proxy_type: Option<String>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub canvas_spoof: Option<bool>,
    pub webgl_spoof: Option<bool>,
    pub audio_spoof: Option<bool>,
    pub timezone_spoof: Option<bool>,
    pub geolocation_spoof: Option<bool>,
    pub font_spoof: Option<bool>,
    pub webrtc_spoof: Option<bool>,
    pub navigator_override: Option<bool>,
    pub webdriver_override: Option<bool>,
    pub custom_headers: Option<String>,
    pub headless: Option<bool>,
    pub extensions: Option<String>,
    pub is_default: Option<bool>,
    // 增强指纹字段
    pub hardware_concurrency: Option<i32>,
    pub device_memory: Option<i32>,
    pub color_depth: Option<i32>,
    pub languages: Option<String>,
    pub vendor: Option<String>,
    pub gpu_vendor: Option<String>,
    pub gpu_renderer: Option<String>,
    pub color_scheme: Option<String>,
    pub max_touch_points: Option<i32>,
    pub has_touch: Option<bool>,
    pub screen_orientation_angle: Option<i32>,
    pub screen_orientation_type: Option<String>,
    pub font_family: Option<String>,
    pub client_hints_platform: Option<String>,
    pub client_hints_platform_version: Option<String>,
    pub client_hints_architecture: Option<String>,
    pub client_hints_bitness: Option<String>,
    pub client_hints_model: Option<String>,
    pub client_hints_wow64: Option<String>,
    pub fingerprint_hash: Option<String>,
    pub platform_name: Option<String>,
}

/// 更新浏览器环境配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBrowserProfileRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub user_agent: Option<String>,
    pub viewport_width: Option<i32>,
    pub viewport_height: Option<i32>,
    pub device_scale_factor: Option<i32>,
    pub locale: Option<String>,
    pub timezone_id: Option<String>,
    pub proxy_type: Option<String>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub canvas_spoof: Option<bool>,
    pub webgl_spoof: Option<bool>,
    pub audio_spoof: Option<bool>,
    pub timezone_spoof: Option<bool>,
    pub geolocation_spoof: Option<bool>,
    pub font_spoof: Option<bool>,
    pub webrtc_spoof: Option<bool>,
    pub navigator_override: Option<bool>,
    pub webdriver_override: Option<bool>,
    pub custom_headers: Option<String>,
    pub headless: Option<bool>,
    pub extensions: Option<String>,
    pub is_default: Option<bool>,
    // 增强指纹字段
    pub hardware_concurrency: Option<i32>,
    pub device_memory: Option<i32>,
    pub color_depth: Option<i32>,
    pub languages: Option<String>,
    pub vendor: Option<String>,
    pub gpu_vendor: Option<String>,
    pub gpu_renderer: Option<String>,
    pub color_scheme: Option<String>,
    pub max_touch_points: Option<i32>,
    pub has_touch: Option<bool>,
    pub screen_orientation_angle: Option<i32>,
    pub screen_orientation_type: Option<String>,
    pub font_family: Option<String>,
    pub client_hints_platform: Option<String>,
    pub client_hints_platform_version: Option<String>,
    pub client_hints_architecture: Option<String>,
    pub client_hints_bitness: Option<String>,
    pub client_hints_model: Option<String>,
    pub client_hints_wow64: Option<String>,
    pub fingerprint_hash: Option<String>,
    pub platform_name: Option<String>,
}

/// 自动化脚本模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AutomationScript {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub compiled_content: Option<String>,
    pub version: i32,
    pub is_system: bool,
    pub required_apis: Option<String>,
    pub author: Option<String>,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建自动化脚本请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAutomationScriptRequest {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub required_apis: Option<Vec<String>>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 更新自动化脚本请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAutomationScriptRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub required_apis: Option<Vec<String>>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// 自动化任务模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AutomationTask {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub script_id: i64,
    pub wallet_ids: String,
    pub profile_strategy: String,
    pub specific_profile_id: Option<i64>,
    pub schedule_type: String,
    pub schedule_config: String,
    pub concurrency: i32,
    pub timeout_seconds: i32,
    pub retry_times: i32,
    pub retry_interval_seconds: i32,
    pub status: String,
    pub last_run_time: Option<DateTime<Utc>>,
    pub next_run_time: Option<DateTime<Utc>>,
    pub total_runs: i32,
    pub success_runs: i32,
    pub failed_runs: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建自动化任务请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAutomationTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub script_id: i64,
    pub wallet_ids: Vec<i64>,
    pub profile_strategy: Option<String>,
    pub specific_profile_id: Option<i64>,
    pub schedule_type: Option<String>,
    pub schedule_config: serde_json::Value,
    pub concurrency: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub retry_times: Option<i32>,
    pub retry_interval_seconds: Option<i32>,
}

/// 更新自动化任务请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAutomationTaskRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub script_id: Option<i64>,
    pub wallet_ids: Option<Vec<i64>>,
    pub profile_strategy: Option<String>,
    pub specific_profile_id: Option<i64>,
    pub schedule_type: Option<String>,
    pub schedule_config: Option<serde_json::Value>,
    pub concurrency: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub retry_times: Option<i32>,
    pub retry_interval_seconds: Option<i32>,
    pub status: Option<String>,
}

/// 任务执行记录模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskExecution {
    pub id: i64,
    pub task_id: i64,
    pub wallet_id: i64,
    pub profile_id: Option<i64>,
    pub status: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub result_data: Option<String>,
    pub logs: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 任务执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionStats {
    pub total_executions: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub success_rate: f64,
}

/// 环境分配策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStrategy {
    Random,
    Sequential,
    Specific,
}

impl ProfileStrategy {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileStrategy::Random => "random",
            ProfileStrategy::Sequential => "sequential",
            ProfileStrategy::Specific => "specific",
        }
    }
}

impl From<&str> for ProfileStrategy {
    fn from(s: &str) -> Self {
        match s {
            "sequential" => ProfileStrategy::Sequential,
            "specific" => ProfileStrategy::Specific,
            _ => ProfileStrategy::Random,
        }
    }
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Enabled,
    Paused,
    Running,
}

impl TaskStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Draft => "draft",
            TaskStatus::Enabled => "enabled",
            TaskStatus::Paused => "paused",
            TaskStatus::Running => "running",
        }
    }
}

impl From<&str> for TaskStatus {
    fn from(s: &str) -> Self {
        match s {
            "enabled" => TaskStatus::Enabled,
            "paused" => TaskStatus::Paused,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Draft,
        }
    }
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Stopped,
}

impl ExecutionStatus {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Success => "success",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Stopped => "stopped",
        }
    }
}

impl From<&str> for ExecutionStatus {
    fn from(s: &str) -> Self {
        match s {
            "running" => ExecutionStatus::Running,
            "success" => ExecutionStatus::Success,
            "failed" => ExecutionStatus::Failed,
            "stopped" => ExecutionStatus::Stopped,
            _ => ExecutionStatus::Pending,
        }
    }
}

/// 调度类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Once,
    Interval,
    Cron,
}

impl ScheduleType {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleType::Once => "once",
            ScheduleType::Interval => "interval",
            ScheduleType::Cron => "cron",
        }
    }
}

impl From<&str> for ScheduleType {
    fn from(s: &str) -> Self {
        match s {
            "interval" => ScheduleType::Interval,
            "cron" => ScheduleType::Cron,
            _ => ScheduleType::Once,
        }
    }
}

/// 批量生成环境配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct BatchGenerateProfilesRequest {
    pub count: i32,
    pub proxy_type: Option<String>,
    pub proxy_host_prefix: Option<String>,
    pub proxy_port_start: Option<i32>,
    pub enable_all_spoofs: Option<bool>,
}

/// 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub headless: bool,
    pub timeout: i32,
    pub retry_count: i32,
    pub concurrent_limit: i32,
    pub enable_proxy: bool,
    pub enable_fingerprint: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            headless: false,
            timeout: 300,
            retry_count: 3,
            concurrent_limit: 1,
            enable_proxy: true,
            enable_fingerprint: true,
        }
    }
}

/// 执行任务请求
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ExecuteTaskRequest {
    pub task_id: i64,
    pub wallet_ids: Option<Vec<i64>>,
    pub config: Option<ExecutionConfig>,
}

/// 批量执行任务请求
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BatchExecuteRequest {
    pub script_id: i64,
    pub wallet_ids: Vec<i64>,
    pub profile_ids: Option<Vec<i64>>,
    pub profile_strategy: Option<String>,
    pub config: Option<ExecutionConfig>,
}

/// 导入钱包请求
#[derive(Debug, Clone, Deserialize)]
pub struct ImportWalletsRequest {
    pub wallets: Vec<CreateAirdropWalletRequest>,
}

/// 导出结果
#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub file_path: String,
    pub count: usize,
}

/// 执行结果
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub execution_id: i64,
    pub message: Option<String>,
    pub tx_hash: Option<String>,
}

/// 任务统计信息
#[derive(Debug, Clone, Serialize)]
pub struct TaskStats {
    pub task_id: i64,
    pub task_name: String,
    pub total_executions: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub success_rate: f64,
    pub last_run_time: Option<DateTime<Utc>>,
    pub next_run_time: Option<DateTime<Utc>>,
}

/// 浏览器插件模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BrowserExtension {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub enabled: bool,
    pub is_builtin: bool,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建浏览器插件请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBrowserExtensionRequest {
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub enabled: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// 更新浏览器插件请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBrowserExtensionRequest {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub enabled: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// 插件文件夹扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionFolder {
    pub name: String,
    pub path: String,
    pub has_manifest: bool,
    pub manifest_info: Option<ManifestInfo>,
}

/// manifest.json 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}
