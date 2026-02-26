use tauri::State;
use sqlx::{SqlitePool, Row};
use anyhow::Result;
use serde_json;
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;

use crate::wallets_tool::airdrop::models::*;
use crate::wallets_tool::security;
use crate::wallets_tool::airdrop::models::{BrowserExtension, CreateBrowserExtensionRequest, UpdateBrowserExtensionRequest, ExtensionFolder, ManifestInfo};

/// 获取所有空投钱包
#[tauri::command]
pub async fn get_airdrop_wallets(pool: State<'_, SqlitePool>) -> Result<Vec<AirdropWallet>, String> {
    let wallets = sqlx::query_as::<_, AirdropWallet>(
        "SELECT * FROM airdrop_wallets ORDER BY id DESC"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取钱包列表失败: {}", e))?;

    Ok(wallets)
}

/// 创建空投钱包
#[tauri::command]
pub async fn create_airdrop_wallet(
    pool: State<'_, SqlitePool>,
    request: CreateAirdropWalletRequest,
) -> Result<AirdropWallet, String> {
    // 验证地址格式
    let address = request.address.trim();
    if !is_valid_address(address) {
        return Err("无效的钱包地址格式".to_string());
    }

    // 检查地址是否已存在
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM airdrop_wallets WHERE LOWER(address) = LOWER(?)"
    )
    .bind(address)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| format!("检查地址失败: {}", e))?;

    if existing.is_some() {
        return Err("该地址已存在".to_string());
    }

    // 加密私钥
    let encrypted_key = security::memory::encrypt_string(&request.private_key)
        .map_err(|e| format!("加密私钥失败: {}", e))?;

    let name = request.name.trim();
    let label = request.label.as_ref().map(|s| s.trim().to_string());
    let group_name = request.group_name.unwrap_or_else(|| "Default".to_string());
    let proxy = request.proxy.unwrap_or_else(|| "Direct".to_string());
    let chain_type = request.chain_type.unwrap_or_else(|| "evm".to_string());

    let wallet = sqlx::query_as::<_, AirdropWallet>(
        r#"
        INSERT INTO airdrop_wallets (name, address, encrypted_private_key, label, group_name, proxy, chain_type)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(name)
    .bind(address)
    .bind(&encrypted_key)
    .bind(&label)
    .bind(&group_name)
    .bind(&proxy)
    .bind(&chain_type)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("创建钱包失败: {}", e))?;

    Ok(wallet)
}

/// 更新空投钱包
#[tauri::command]
pub async fn update_airdrop_wallet(
    pool: State<'_, SqlitePool>,
    request: UpdateAirdropWalletRequest,
) -> Result<AirdropWallet, String> {
    // 检查钱包是否存在
    let existing: Option<AirdropWallet> = sqlx::query_as(
        "SELECT * FROM airdrop_wallets WHERE id = ?"
    )
    .bind(request.id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| format!("查询钱包失败: {}", e))?;

    if existing.is_none() {
        return Err("钱包不存在".to_string());
    }

    let existing = existing.unwrap();

    // 如果更新地址，验证格式
    if let Some(ref addr) = request.address {
        if !is_valid_address(addr.trim()) {
            return Err("无效的钱包地址格式".to_string());
        }

        // 检查新地址是否与其他钱包冲突
        let conflict: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM airdrop_wallets WHERE LOWER(address) = LOWER(?) AND id != ?"
        )
        .bind(addr.trim())
        .bind(request.id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| format!("检查地址冲突失败: {}", e))?;

        if conflict.is_some() {
            return Err("该地址已被其他钱包使用".to_string());
        }
    }

    let UpdateAirdropWalletRequest {
        id,
        name,
        address,
        private_key,
        label,
        group_name,
        proxy,
        chain_type,
    } = request;

    let encrypted_key = if let Some(pk) = private_key.as_deref() {
        Some(
            security::memory::encrypt_string(pk)
                .map_err(|e| format!("加密私钥失败: {}", e))?,
        )
    } else {
        None
    };

    let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new("UPDATE airdrop_wallets SET ");
    let mut separated = qb.separated(", ");
    let mut has_updates = false;

    if let Some(v) = name.as_deref() {
        separated.push("name = ").push_bind(v.trim());
        has_updates = true;
    }
    if let Some(v) = address.as_deref() {
        separated.push("address = ").push_bind(v.trim());
        has_updates = true;
    }
    if let Some(v) = encrypted_key.as_deref() {
        separated.push("encrypted_private_key = ").push_bind(v);
        has_updates = true;
    }
    if let Some(v) = label.as_deref() {
        separated.push("label = ").push_bind(v.trim());
        has_updates = true;
    }
    if let Some(v) = group_name.as_deref() {
        separated.push("group_name = ").push_bind(v.trim());
        has_updates = true;
    }
    if let Some(v) = proxy.as_deref() {
        separated.push("proxy = ").push_bind(v.trim());
        has_updates = true;
    }
    if let Some(v) = chain_type.as_deref() {
        separated.push("chain_type = ").push_bind(v.trim());
        has_updates = true;
    }

    if !has_updates {
        return Ok(existing);
    }

    separated.push("updated_at = CURRENT_TIMESTAMP");

    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");

    let wallet = qb
        .build_query_as::<AirdropWallet>()
        .fetch_one(&*pool)
        .await
        .map_err(|e| format!("更新钱包失败: {}", e))?;

    Ok(wallet)
}

/// 删除空投钱包
#[tauri::command]
pub async fn delete_airdrop_wallet(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM airdrop_wallets WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除钱包失败: {}", e))?;

    Ok(())
}

/// 批量导入空投钱包
#[tauri::command]
pub async fn import_airdrop_wallets(
    pool: State<'_, SqlitePool>,
    request: ImportWalletsRequest,
) -> Result<(Vec<AirdropWallet>, Vec<String>), String> {
    let mut wallets = vec![];
    let mut errors = vec![];

    for (idx, req) in request.wallets.into_iter().enumerate() {
        // 验证地址
        let address = req.address.trim();
        if !is_valid_address(address) {
            errors.push(format!("第{}行: 无效地址 {}", idx + 1, address));
            continue;
        }

        // 检查重复
        let existing: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM airdrop_wallets WHERE LOWER(address) = LOWER(?)"
        )
        .bind(address)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| format!("检查地址失败: {}", e))?;

        if existing.is_some() {
            errors.push(format!("第{}行: 地址 {} 已存在", idx + 1, address));
            continue;
        }

        // 加密私钥
        let encrypted_key = match security::memory::encrypt_string(&req.private_key) {
            Ok(key) => key,
            Err(e) => {
                errors.push(format!("第{}行: 加密私钥失败 {}", idx + 1, e));
                continue;
            }
        };

        let name = req.name.trim();
        let label = req.label.as_ref().map(|s| s.trim().to_string());
        let group_name = req.group_name.unwrap_or_else(|| "Default".to_string());
        let proxy = req.proxy.unwrap_or_else(|| "Direct".to_string());
        let chain_type = req.chain_type.unwrap_or_else(|| "evm".to_string());

        match sqlx::query_as::<_, AirdropWallet>(
            r#"
            INSERT INTO airdrop_wallets (name, address, encrypted_private_key, label, group_name, proxy, chain_type)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(name)
        .bind(address)
        .bind(&encrypted_key)
        .bind(&label)
        .bind(&group_name)
        .bind(&proxy)
        .bind(&chain_type)
        .fetch_one(&*pool)
        .await
        {
            Ok(wallet) => wallets.push(wallet),
            Err(e) => errors.push(format!("第{}行: 导入失败 {}", idx + 1, e)),
        }
    }

    Ok((wallets, errors))
}

/// 获取钱包私钥（解密）
#[tauri::command]
pub async fn get_wallet_private_key(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<String, String> {
    let encrypted_key: String = sqlx::query_scalar(
        "SELECT encrypted_private_key FROM airdrop_wallets WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| format!("查询钱包失败: {}", e))?
    .ok_or_else(|| "钱包不存在".to_string())?;

    let private_key = security::memory::decrypt_string(&encrypted_key)
        .map_err(|e| format!("解密私钥失败: {}", e))?;

    Ok(private_key)
}

/// 获取所有浏览器环境配置
#[tauri::command]
pub async fn get_browser_profiles(pool: State<'_, SqlitePool>) -> Result<Vec<BrowserProfile>, String> {
    let profiles = sqlx::query_as::<_, BrowserProfile>(
        "SELECT * FROM browser_profiles ORDER BY id DESC"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取环境配置列表失败: {}", e))?;

    Ok(profiles)
}

/// 创建浏览器环境配置
#[tauri::command]
pub async fn create_browser_profile(
    pool: State<'_, SqlitePool>,
    request: CreateBrowserProfileRequest,
) -> Result<BrowserProfile, String> {
    let profile = sqlx::query_as::<_, BrowserProfile>(
        r#"
        INSERT INTO browser_profiles (
            name, description, user_agent, viewport_width, viewport_height,
            device_scale_factor, locale, timezone_id, proxy_type, proxy_host,
            proxy_port, proxy_username, proxy_password, canvas_spoof, webgl_spoof,
            audio_spoof, timezone_spoof, geolocation_spoof, font_spoof, webrtc_spoof,
            navigator_override, webdriver_override, custom_headers, headless,
            extensions, is_default,
            hardware_concurrency, device_memory, color_depth, languages, vendor,
            gpu_vendor, gpu_renderer, color_scheme, max_touch_points, has_touch,
            screen_orientation_angle, screen_orientation_type, font_family,
            client_hints_platform, client_hints_platform_version, client_hints_architecture,
            client_hints_bitness, client_hints_model, client_hints_wow64,
            fingerprint_hash, platform_name
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                  ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.user_agent)
    .bind(request.viewport_width.unwrap_or(1920))
    .bind(request.viewport_height.unwrap_or(1080))
    .bind(request.device_scale_factor.unwrap_or(1))
    .bind(request.locale.unwrap_or_else(|| "en-US".to_string()))
    .bind(request.timezone_id.unwrap_or_else(|| "America/New_York".to_string()))
    .bind(request.proxy_type.unwrap_or_else(|| "direct".to_string()))
    .bind(&request.proxy_host)
    .bind(request.proxy_port)
    .bind(&request.proxy_username)
    .bind(&request.proxy_password)
    .bind(request.canvas_spoof.unwrap_or(true))
    .bind(request.webgl_spoof.unwrap_or(true))
    .bind(request.audio_spoof.unwrap_or(true))
    .bind(request.timezone_spoof.unwrap_or(true))
    .bind(request.geolocation_spoof.unwrap_or(true))
    .bind(request.font_spoof.unwrap_or(true))
    .bind(request.webrtc_spoof.unwrap_or(true))
    .bind(request.navigator_override.unwrap_or(true))
    .bind(request.webdriver_override.unwrap_or(true))
    .bind(&request.custom_headers)
    .bind(request.headless.unwrap_or(false))
    .bind(&request.extensions)
    .bind(request.is_default.unwrap_or(false))
    // 增强指纹字段
    .bind(request.hardware_concurrency)
    .bind(request.device_memory)
    .bind(request.color_depth)
    .bind(&request.languages)
    .bind(&request.vendor)
    .bind(&request.gpu_vendor)
    .bind(&request.gpu_renderer)
    .bind(&request.color_scheme)
    .bind(request.max_touch_points)
    .bind(request.has_touch)
    .bind(request.screen_orientation_angle)
    .bind(&request.screen_orientation_type)
    .bind(&request.font_family)
    .bind(&request.client_hints_platform)
    .bind(&request.client_hints_platform_version)
    .bind(&request.client_hints_architecture)
    .bind(&request.client_hints_bitness)
    .bind(&request.client_hints_model)
    .bind(&request.client_hints_wow64)
    .bind(&request.fingerprint_hash)
    .bind(&request.platform_name)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("创建环境配置失败: {}", e))?;

    Ok(profile)
}

/// 更新浏览器环境配置
#[tauri::command]
pub async fn update_browser_profile(
    pool: State<'_, SqlitePool>,
    request: UpdateBrowserProfileRequest,
) -> Result<BrowserProfile, String> {
    let profile = sqlx::query_as::<_, BrowserProfile>(
        r#"
        UPDATE browser_profiles SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            user_agent = COALESCE(?, user_agent),
            viewport_width = COALESCE(?, viewport_width),
            viewport_height = COALESCE(?, viewport_height),
            device_scale_factor = COALESCE(?, device_scale_factor),
            locale = COALESCE(?, locale),
            timezone_id = COALESCE(?, timezone_id),
            proxy_type = COALESCE(?, proxy_type),
            proxy_host = COALESCE(?, proxy_host),
            proxy_port = COALESCE(?, proxy_port),
            proxy_username = COALESCE(?, proxy_username),
            proxy_password = COALESCE(?, proxy_password),
            canvas_spoof = COALESCE(?, canvas_spoof),
            webgl_spoof = COALESCE(?, webgl_spoof),
            audio_spoof = COALESCE(?, audio_spoof),
            timezone_spoof = COALESCE(?, timezone_spoof),
            geolocation_spoof = COALESCE(?, geolocation_spoof),
            font_spoof = COALESCE(?, font_spoof),
            webrtc_spoof = COALESCE(?, webrtc_spoof),
            navigator_override = COALESCE(?, navigator_override),
            webdriver_override = COALESCE(?, webdriver_override),
            custom_headers = COALESCE(?, custom_headers),
            headless = COALESCE(?, headless),
            extensions = COALESCE(?, extensions),
            is_default = COALESCE(?, is_default),
            hardware_concurrency = COALESCE(?, hardware_concurrency),
            device_memory = COALESCE(?, device_memory),
            color_depth = COALESCE(?, color_depth),
            languages = COALESCE(?, languages),
            vendor = COALESCE(?, vendor),
            gpu_vendor = COALESCE(?, gpu_vendor),
            gpu_renderer = COALESCE(?, gpu_renderer),
            color_scheme = COALESCE(?, color_scheme),
            max_touch_points = COALESCE(?, max_touch_points),
            has_touch = COALESCE(?, has_touch),
            screen_orientation_angle = COALESCE(?, screen_orientation_angle),
            screen_orientation_type = COALESCE(?, screen_orientation_type),
            font_family = COALESCE(?, font_family),
            client_hints_platform = COALESCE(?, client_hints_platform),
            client_hints_platform_version = COALESCE(?, client_hints_platform_version),
            client_hints_architecture = COALESCE(?, client_hints_architecture),
            client_hints_bitness = COALESCE(?, client_hints_bitness),
            client_hints_model = COALESCE(?, client_hints_model),
            client_hints_wow64 = COALESCE(?, client_hints_wow64),
            fingerprint_hash = COALESCE(?, fingerprint_hash),
            platform_name = COALESCE(?, platform_name),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.user_agent)
    .bind(request.viewport_width)
    .bind(request.viewport_height)
    .bind(request.device_scale_factor)
    .bind(&request.locale)
    .bind(&request.timezone_id)
    .bind(&request.proxy_type)
    .bind(&request.proxy_host)
    .bind(request.proxy_port)
    .bind(&request.proxy_username)
    .bind(&request.proxy_password)
    .bind(request.canvas_spoof)
    .bind(request.webgl_spoof)
    .bind(request.audio_spoof)
    .bind(request.timezone_spoof)
    .bind(request.geolocation_spoof)
    .bind(request.font_spoof)
    .bind(request.webrtc_spoof)
    .bind(request.navigator_override)
    .bind(request.webdriver_override)
    .bind(&request.custom_headers)
    .bind(request.headless)
    .bind(&request.extensions)
    .bind(request.is_default)
    // 增强指纹字段
    .bind(request.hardware_concurrency)
    .bind(request.device_memory)
    .bind(request.color_depth)
    .bind(&request.languages)
    .bind(&request.vendor)
    .bind(&request.gpu_vendor)
    .bind(&request.gpu_renderer)
    .bind(&request.color_scheme)
    .bind(request.max_touch_points)
    .bind(request.has_touch)
    .bind(request.screen_orientation_angle)
    .bind(&request.screen_orientation_type)
    .bind(&request.font_family)
    .bind(&request.client_hints_platform)
    .bind(&request.client_hints_platform_version)
    .bind(&request.client_hints_architecture)
    .bind(&request.client_hints_bitness)
    .bind(&request.client_hints_model)
    .bind(&request.client_hints_wow64)
    .bind(&request.fingerprint_hash)
    .bind(&request.platform_name)
    .bind(request.id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("更新环境配置失败: {}", e))?;

    Ok(profile)
}

/// 删除浏览器环境配置
#[tauri::command]
pub async fn delete_browser_profile(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM browser_profiles WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除环境配置失败: {}", e))?;

    Ok(())
}

/// 批量生成浏览器环境配置
#[tauri::command]
pub async fn batch_generate_profiles(
    pool: State<'_, SqlitePool>,
    request: BatchGenerateProfilesRequest,
) -> Result<Vec<BrowserProfile>, String> {
    use rand::seq::SliceRandom;

    let user_agents = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    ];

    let viewports = vec![
        (1920, 1080),
        (1366, 768),
        (1440, 900),
        (1536, 864),
        (2560, 1440),
        (1280, 720),
        (390, 844),
        (414, 896),
    ];

    let locales = vec!["en-US", "zh-CN", "ja-JP", "ko-KR", "de-DE", "fr-FR", "en-GB"];
    let timezones = vec![
        "America/New_York", "America/Los_Angeles", "Europe/London",
        "Europe/Paris", "Asia/Tokyo", "Asia/Shanghai", "Asia/Singapore",
    ];

    let mut profiles = vec![];

    for i in 0..request.count {
        let (ua, width, height, locale, timezone) = {
            let mut rng = rand::thread_rng();
            let ua = user_agents.choose(&mut rng).unwrap().to_string();
            let (width, height) = *viewports.choose(&mut rng).unwrap();
            let locale = locales.choose(&mut rng).unwrap().to_string();
            let timezone = timezones.choose(&mut rng).unwrap().to_string();
            (ua, width, height, locale, timezone)
        };

        let proxy_type = request.proxy_type.clone().unwrap_or_else(|| "direct".to_string());
        let proxy_host = request.proxy_host_prefix.as_ref().map(|prefix| {
            format!("{}.{}", prefix, i + 1)
        });
        let proxy_port = request.proxy_port_start.map(|start| start + i);

        let enable_all = request.enable_all_spoofs.unwrap_or(true);
        let device_scale_factor = 1i32;

        let profile = sqlx::query_as::<_, BrowserProfile>(
            r#"
            INSERT INTO browser_profiles (
                name, user_agent, viewport_width, viewport_height, device_scale_factor,
                locale, timezone_id, proxy_type, proxy_host, proxy_port,
                canvas_spoof, webgl_spoof, audio_spoof, timezone_spoof,
                geolocation_spoof, font_spoof, webrtc_spoof,
                navigator_override, webdriver_override
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(format!("Auto-Profile-{}", i + 1))
        .bind(&ua)
        .bind(width)
        .bind(height)
        .bind(device_scale_factor)
        .bind(&locale)
        .bind(&timezone)
        .bind(&proxy_type)
        .bind(&proxy_host)
        .bind(proxy_port)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .bind(enable_all)
        .fetch_one(&*pool)
        .await
        .map_err(|e| format!("生成环境配置失败: {}", e))?;

        profiles.push(profile);
    }

    Ok(profiles)
}

/// 获取所有自动化脚本
#[tauri::command]
pub async fn get_automation_scripts(pool: State<'_, SqlitePool>) -> Result<Vec<AutomationScript>, String> {
    let scripts = sqlx::query_as::<_, AutomationScript>(
        "SELECT * FROM automation_scripts ORDER BY is_system DESC, id DESC"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取脚本列表失败: {}", e))?;

    Ok(scripts)
}

/// 创建自动化脚本
#[tauri::command]
pub async fn create_automation_script(
    pool: State<'_, SqlitePool>,
    request: CreateAutomationScriptRequest,
) -> Result<AutomationScript, String> {
    let required_apis = request.required_apis.map(|apis| serde_json::to_string(&apis).unwrap_or_default());
    let tags = request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    let script = sqlx::query_as::<_, AutomationScript>(
        r#"
        INSERT INTO automation_scripts (name, description, content, required_apis, author, tags)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.content)
    .bind(&required_apis)
    .bind(&request.author)
    .bind(&tags)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("创建脚本失败: {}", e))?;

    Ok(script)
}

/// 更新自动化脚本
#[tauri::command]
pub async fn update_automation_script(
    pool: State<'_, SqlitePool>,
    request: UpdateAutomationScriptRequest,
) -> Result<AutomationScript, String> {
    let required_apis = request.required_apis.map(|apis| serde_json::to_string(&apis).unwrap_or_default());
    let tags = request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    let script = sqlx::query_as::<_, AutomationScript>(
        r#"
        UPDATE automation_scripts SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            content = COALESCE(?, content),
            required_apis = COALESCE(?, required_apis),
            author = COALESCE(?, author),
            tags = COALESCE(?, tags),
            version = version + 1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ? AND is_system = 0
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.content)
    .bind(&required_apis)
    .bind(&request.author)
    .bind(&tags)
    .bind(request.id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("更新脚本失败: {}", e))?;

    Ok(script)
}

/// 删除自动化脚本
#[tauri::command]
pub async fn delete_automation_script(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM automation_scripts WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除脚本失败: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("脚本不存在".to_string());
    }

    Ok(())
}

/// 获取所有自动化任务
#[tauri::command]
pub async fn get_automation_tasks(pool: State<'_, SqlitePool>) -> Result<Vec<AutomationTask>, String> {
    let tasks = sqlx::query_as::<_, AutomationTask>(
        "SELECT * FROM automation_tasks ORDER BY id DESC"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取任务列表失败: {}", e))?;

    Ok(tasks)
}

/// 创建自动化任务
#[tauri::command]
pub async fn create_automation_task(
    pool: State<'_, SqlitePool>,
    request: CreateAutomationTaskRequest,
) -> Result<AutomationTask, String> {
    let wallet_ids = serde_json::to_string(&request.wallet_ids)
        .map_err(|e| format!("序列化钱包ID失败: {}", e))?;
    let schedule_config = serde_json::to_string(&request.schedule_config)
        .map_err(|e| format!("序列化调度配置失败: {}", e))?;

    let task = sqlx::query_as::<_, AutomationTask>(
        r#"
        INSERT INTO automation_tasks (
            name, description, script_id, wallet_ids, profile_strategy,
            specific_profile_id, schedule_type, schedule_config, concurrency,
            timeout_seconds, retry_times, retry_interval_seconds
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(request.script_id)
    .bind(&wallet_ids)
    .bind(request.profile_strategy.unwrap_or_else(|| "random".to_string()))
    .bind(request.specific_profile_id)
    .bind(request.schedule_type.unwrap_or_else(|| "once".to_string()))
    .bind(&schedule_config)
    .bind(request.concurrency.unwrap_or(1))
    .bind(request.timeout_seconds.unwrap_or(300))
    .bind(request.retry_times.unwrap_or(3))
    .bind(request.retry_interval_seconds.unwrap_or(60))
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("创建任务失败: {}", e))?;

    Ok(task)
}

/// 更新自动化任务
#[tauri::command]
pub async fn update_automation_task(
    pool: State<'_, SqlitePool>,
    request: UpdateAutomationTaskRequest,
) -> Result<AutomationTask, String> {
    let wallet_ids = request.wallet_ids.map(|ids| {
        serde_json::to_string(&ids).unwrap_or_default()
    });
    let schedule_config = request.schedule_config.map(|cfg| {
        serde_json::to_string(&cfg).unwrap_or_default()
    });

    let task = sqlx::query_as::<_, AutomationTask>(
        r#"
        UPDATE automation_tasks SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            script_id = COALESCE(?, script_id),
            wallet_ids = COALESCE(?, wallet_ids),
            profile_strategy = COALESCE(?, profile_strategy),
            specific_profile_id = COALESCE(?, specific_profile_id),
            schedule_type = COALESCE(?, schedule_type),
            schedule_config = COALESCE(?, schedule_config),
            concurrency = COALESCE(?, concurrency),
            timeout_seconds = COALESCE(?, timeout_seconds),
            retry_times = COALESCE(?, retry_times),
            retry_interval_seconds = COALESCE(?, retry_interval_seconds),
            status = COALESCE(?, status),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(request.script_id)
    .bind(&wallet_ids)
    .bind(&request.profile_strategy)
    .bind(request.specific_profile_id)
    .bind(&request.schedule_type)
    .bind(&schedule_config)
    .bind(request.concurrency)
    .bind(request.timeout_seconds)
    .bind(request.retry_times)
    .bind(request.retry_interval_seconds)
    .bind(&request.status)
    .bind(request.id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("更新任务失败: {}", e))?;

    Ok(task)
}

/// 删除自动化任务
#[tauri::command]
pub async fn delete_automation_task(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM automation_tasks WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除任务失败: {}", e))?;

    Ok(())
}

/// 切换任务状态
#[tauri::command]
pub async fn toggle_task_status(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<AutomationTask, String> {
    let task: AutomationTask = sqlx::query_as(
        "SELECT * FROM automation_tasks WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("查询任务失败: {}", e))?;

    let new_status = match task.status.as_str() {
        "enabled" => "paused",
        "paused" => "enabled",
        "draft" => "enabled",
        _ => "enabled",
    };

    let updated = sqlx::query_as::<_, AutomationTask>(
        "UPDATE automation_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *"
    )
    .bind(new_status)
    .bind(id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("切换任务状态失败: {}", e))?;

    Ok(updated)
}

/// 立即执行任务
#[tauri::command]
pub async fn run_task_now(
    pool: State<'_, SqlitePool>,
    task_id: i64,
) -> Result<(), String> {
    use chrono::Utc;

    let task: AutomationTask = sqlx::query_as(
        "SELECT * FROM automation_tasks WHERE id = ?"
    )
    .bind(task_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("获取任务失败: {}", e))?;

    let wallet_ids: Vec<i64> = serde_json::from_str(&task.wallet_ids)
        .map_err(|e| format!("解析钱包ID失败: {}", e))?;

    if wallet_ids.is_empty() {
        return Err("任务没有配置钱包".to_string());
    }

    let profiles: Vec<BrowserProfile> = sqlx::query_as(
        "SELECT * FROM browser_profiles"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取环境配置失败: {}", e))?;

    if profiles.is_empty() {
        return Err("没有可用的环境配置".to_string());
    }

    let now = Utc::now();

    for wallet_id in wallet_ids {
        let profile = profiles.choose(&mut rand::thread_rng()).cloned().unwrap_or_else(|| profiles[0].clone());
        
        sqlx::query(
            r#"
            INSERT INTO task_executions (task_id, wallet_id, profile_id, status, start_time)
            VALUES (?, ?, ?, 'pending', ?)
            "#
        )
        .bind(task_id)
        .bind(wallet_id)
        .bind(profile.id)
        .bind(now)
        .execute(&*pool)
        .await
        .map_err(|e| format!("创建执行记录失败: {}", e))?;
    }

    sqlx::query(
        "UPDATE automation_tasks SET last_run_time = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(now)
    .bind(task_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("更新任务时间失败: {}", e))?;

    Ok(())
}

/// 获取任务执行记录
#[tauri::command]
pub async fn get_task_executions(
    pool: State<'_, SqlitePool>,
    task_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<TaskExecution>, String> {
    let query = match (task_id, limit) {
        (Some(tid), Some(lim)) => sqlx::query_as::<_, TaskExecution>(
            "SELECT * FROM task_executions WHERE task_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(tid)
        .bind(lim),
        (Some(tid), None) => sqlx::query_as::<_, TaskExecution>(
            "SELECT * FROM task_executions WHERE task_id = ? ORDER BY created_at DESC",
        )
        .bind(tid),
        (None, Some(lim)) => sqlx::query_as::<_, TaskExecution>(
            "SELECT * FROM task_executions ORDER BY created_at DESC LIMIT ?",
        )
        .bind(lim),
        (None, None) => sqlx::query_as::<_, TaskExecution>(
            "SELECT * FROM task_executions ORDER BY created_at DESC",
        ),
    };

    let executions = query
        .fetch_all(&*pool)
        .await
        .map_err(|e| format!("获取执行记录失败: {}", e))?;

    Ok(executions)
}

/// 删除执行记录
#[tauri::command]
pub async fn delete_task_execution(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM task_executions WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除执行记录失败: {}", e))?;

    Ok(())
}

/// 获取任务统计信息
#[tauri::command]
pub async fn get_task_execution_stats(
    pool: State<'_, SqlitePool>,
    task_id: Option<i64>,
) -> Result<TaskExecutionStats, String> {
    let (total, success, failed): (i64, i64, i64) = if let Some(tid) = task_id {
        sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN status = 'success' THEN 1 END) as success,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed
            FROM task_executions
            WHERE task_id = ?
            "#
        )
        .bind(tid)
        .fetch_one(&*pool)
        .await
        .map_err(|e| format!("获取统计信息失败: {}", e))?
    } else {
        sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN status = 'success' THEN 1 END) as success,
                COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed
            FROM task_executions
            "#
        )
        .fetch_one(&*pool)
        .await
        .map_err(|e| format!("获取统计信息失败: {}", e))?
    };

    let success_rate = if total > 0 {
        (success as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(TaskExecutionStats {
        total_executions: total,
        success_count: success,
        failed_count: failed,
        success_rate,
    })
}

/// 初始化浏览器自动化表
#[tauri::command]
pub async fn init_browser_automation_tables(pool: State<'_, SqlitePool>) -> Result<(), String> {
    crate::wallets_tool::airdrop::db::init_airdrop_tables(&*pool)
        .await
        .map_err(|e| format!("初始化表失败: {}", e))?;

    Ok(())
}

/// 创建执行会话请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateExecutionRequest {
    pub script_id: i64,
    pub wallet_ids: Vec<i64>,
    pub profile_ids: Option<Vec<i64>>,
    #[allow(dead_code)]
    pub parallel_mode: Option<bool>,
    #[allow(dead_code)]
    pub max_parallel: Option<i32>,
}

/// 执行会话
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionSession {
    pub id: i64,
    pub script_id: i64,
    pub status: String,
    pub total_tasks: i32,
    pub completed_tasks: i32,
    pub success_count: i32,
    pub failed_count: i32,
    pub results: Vec<ExecutionResult>,
    pub logs: Vec<ExecutionLog>,
}

/// 执行结果
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub id: i64,
    pub wallet_id: i64,
    pub wallet_name: String,
    pub wallet_address: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub logs: Option<String>,
}

/// 执行日志
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionLog {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// 创建执行会话
#[tauri::command]
pub async fn create_execution(
    pool: State<'_, SqlitePool>,
    request: CreateExecutionRequest,
) -> Result<ExecutionSession, String> {
    use chrono::Utc;
    use rand::seq::SliceRandom;

    let _script: AutomationScript = sqlx::query_as(
        "SELECT * FROM automation_scripts WHERE id = ?"
    )
    .bind(request.script_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("获取脚本失败: {}", e))?;

    let profiles: Vec<BrowserProfile> = if let Some(ref profile_ids) = request.profile_ids {
        let placeholders: Vec<String> = profile_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT * FROM browser_profiles WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut query = sqlx::query_as::<_, BrowserProfile>(&sql);
        for id in profile_ids {
            query = query.bind(id);
        }
        query.fetch_all(&*pool).await.map_err(|e| format!("获取环境配置失败: {}", e))?
    } else {
        sqlx::query_as::<_, BrowserProfile>("SELECT * FROM browser_profiles")
            .fetch_all(&*pool)
            .await
            .map_err(|e| format!("获取环境配置失败: {}", e))?
    };

    if profiles.is_empty() {
        return Err("没有可用的环境配置".to_string());
    }

    let placeholders: Vec<String> = request.wallet_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT * FROM airdrop_wallets WHERE id IN ({})",
        placeholders.join(",")
    );
    let mut query = sqlx::query_as::<_, AirdropWallet>(&sql);
    for id in &request.wallet_ids {
        query = query.bind(id);
    }
    let wallets: Vec<AirdropWallet> = query.fetch_all(&*pool).await.map_err(|e| format!("获取钱包失败: {}", e))?;

    let mut results = Vec::new();
    for (_idx, wallet) in wallets.iter().enumerate() {
        let profile = profiles.choose(&mut rand::thread_rng()).cloned().unwrap_or_else(|| profiles[0].clone());
        
        let execution_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO task_executions (task_id, wallet_id, profile_id, status, start_time)
            VALUES (0, ?, ?, 'pending', ?)
            RETURNING id
            "#
        )
        .bind(wallet.id)
        .bind(profile.id)
        .bind(Utc::now())
        .fetch_one(&*pool)
        .await
        .map_err(|e| format!("创建执行记录失败: {}", e))?;

        results.push(ExecutionResult {
            id: execution_id,
            wallet_id: wallet.id,
            wallet_name: wallet.name.clone(),
            wallet_address: wallet.address.clone(),
            status: "pending".to_string(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
            logs: None,
        });
    }

    Ok(ExecutionSession {
        id: 0,
        script_id: request.script_id,
        status: "pending".to_string(),
        total_tasks: results.len() as i32,
        completed_tasks: 0,
        success_count: 0,
        failed_count: 0,
        results,
        logs: vec![ExecutionLog {
            timestamp: Utc::now().to_rfc3339(),
            level: "info".to_string(),
            message: format!("执行会话已创建，共 {} 个任务", request.wallet_ids.len()),
        }],
    })
}

/// 启动执行（模拟执行）
#[tauri::command]
pub async fn start_execution(
    pool: State<'_, SqlitePool>,
    execution_id: i64,
) -> Result<(), String> {
    use chrono::Utc;

    let execution: TaskExecution = sqlx::query_as(
        "SELECT * FROM task_executions WHERE id = ?"
    )
    .bind(execution_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("获取执行记录失败: {}", e))?;

    if execution.status != "pending" {
        return Err("任务不在等待状态".to_string());
    }

    sqlx::query(
        "UPDATE task_executions SET status = 'running', start_time = ? WHERE id = ?"
    )
    .bind(Utc::now())
    .bind(execution_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("更新执行状态失败: {}", e))?;

    Ok(())
}

/// 取消执行
#[tauri::command]
pub async fn cancel_execution(
    pool: State<'_, SqlitePool>,
    execution_id: i64,
) -> Result<(), String> {
    use chrono::Utc;

    sqlx::query(
        "UPDATE task_executions SET status = 'stopped', end_time = ? WHERE id = ? AND status IN ('pending', 'running')"
    )
    .bind(Utc::now())
    .bind(execution_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("取消执行失败: {}", e))?;

    Ok(())
}

/// 获取执行状态
#[tauri::command]
pub async fn get_execution(
    pool: State<'_, SqlitePool>,
    execution_ids: Vec<i64>,
) -> Result<ExecutionSession, String> {

    if execution_ids.is_empty() {
        return Err("没有有效的执行ID".to_string());
    }

    let placeholders: Vec<String> = execution_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT te.*, aw.name as wallet_name, aw.address as wallet_address 
         FROM task_executions te 
         JOIN airdrop_wallets aw ON te.wallet_id = aw.id 
         WHERE te.id IN ({})",
        placeholders.join(",")
    );
    
    let mut query = sqlx::query(&sql);
    for id in &execution_ids {
        query = query.bind(id);
    }

    let rows = query.fetch_all(&*pool).await.map_err(|e| format!("获取执行记录失败: {}", e))?;

    let mut results = Vec::new();
    let mut completed = 0;
    let mut success = 0;
    let mut failed = 0;

    for row in rows {
        let id: i64 = row.try_get("id").unwrap_or(0);
        let wallet_id: i64 = row.try_get("wallet_id").unwrap_or(0);
        let wallet_name: String = row.try_get("wallet_name").unwrap_or_default();
        let wallet_address: String = row.try_get("wallet_address").unwrap_or_default();
        let status: String = row.try_get("status").unwrap_or_default();
        let started_at: Option<String> = row.try_get("start_time").ok().map(|t: chrono::DateTime<chrono::Utc>| t.to_rfc3339());
        let completed_at: Option<String> = row.try_get("end_time").ok().map(|t: chrono::DateTime<chrono::Utc>| t.to_rfc3339());
        let duration_ms: Option<i32> = row.try_get("duration_ms").ok();
        let error_message: Option<String> = row.try_get("error_message").ok();
        let logs: Option<String> = row.try_get("logs").ok();

        if status == "success" || status == "failed" || status == "stopped" {
            completed += 1;
            if status == "success" {
                success += 1;
            } else if status == "failed" {
                failed += 1;
            }
        }

        results.push(ExecutionResult {
            id,
            wallet_id,
            wallet_name,
            wallet_address,
            status,
            started_at,
            completed_at,
            duration_ms,
            error_message,
            logs,
        });
    }

    let is_all_completed = results.iter().all(|r| r.status == "success" || r.status == "failed" || r.status == "stopped");
    let session_status = if is_all_completed {
        "completed".to_string()
    } else if results.iter().any(|r| r.status == "running") {
        "running".to_string()
    } else {
        "pending".to_string()
    };

    Ok(ExecutionSession {
        id: 0,
        script_id: 0,
        status: session_status,
        total_tasks: results.len() as i32,
        completed_tasks: completed,
        success_count: success,
        failed_count: failed,
        results,
        logs: vec![],
    })
}

/// 模拟执行任务（用于测试）
#[tauri::command]
pub async fn simulate_execution(
    pool: State<'_, SqlitePool>,
    execution_id: i64,
) -> Result<(), String> {
    use chrono::Utc;
    use rand::Rng;

    let execution: TaskExecution = sqlx::query_as(
        "SELECT * FROM task_executions WHERE id = ?"
    )
    .bind(execution_id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("获取执行记录失败: {}", e))?;

    if execution.status != "running" {
        return Err("任务不在运行状态".to_string());
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut logs = execution.logs.clone().unwrap_or_default();
    logs.push_str(&format!("[{}] [INFO] 开始执行任务...\n", Utc::now().to_rfc3339()));

    sqlx::query(
        "UPDATE task_executions SET logs = ? WHERE id = ?"
    )
    .bind(&logs)
    .bind(execution_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("更新日志失败: {}", e))?;

    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

    logs.push_str(&format!("[{}] [INFO] 初始化浏览器环境...\n", Utc::now().to_rfc3339()));

    sqlx::query(
        "UPDATE task_executions SET logs = ? WHERE id = ?"
    )
    .bind(&logs)
    .bind(execution_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("更新日志失败: {}", e))?;

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    logs.push_str(&format!("[{}] [INFO] 执行脚本逻辑...\n", Utc::now().to_rfc3339()));

    sqlx::query(
        "UPDATE task_executions SET logs = ? WHERE id = ?"
    )
    .bind(&logs)
    .bind(execution_id)
    .execute(&*pool)
    .await
    .map_err(|e| format!("更新日志失败: {}", e))?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let success = rand::thread_rng().gen_bool(0.9);
    let now = Utc::now();

    if success {
        logs.push_str(&format!("[{}] [SUCCESS] 任务执行成功\n", now.to_rfc3339()));

        sqlx::query(
            r#"
            UPDATE task_executions 
            SET status = 'success', 
                end_time = ?, 
                duration_ms = CAST((julianday(?) - julianday(start_time)) * 24 * 60 * 60 * 1000 AS INTEGER),
                logs = ?
            WHERE id = ?
            "#
        )
        .bind(&now)
        .bind(&now)
        .bind(&logs)
        .bind(execution_id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("更新执行结果失败: {}", e))?;
    } else {
        logs.push_str(&format!("[{}] [ERROR] 任务执行失败: 模拟失败\n", now.to_rfc3339()));

        sqlx::query(
            r#"
            UPDATE task_executions 
            SET status = 'failed', 
                end_time = ?, 
                duration_ms = CAST((julianday(?) - julianday(start_time)) * 24 * 60 * 60 * 1000 AS INTEGER),
                error_message = ?,
                logs = ?
            WHERE id = ?
            "#
        )
        .bind(&now)
        .bind(&now)
        .bind("模拟执行失败")
        .bind(&logs)
        .bind(execution_id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("更新执行结果失败: {}", e))?;
    }

    Ok(())
}

/// 验证地址格式
fn is_valid_address(address: &str) -> bool {
    let evm_regex = regex::Regex::new(r"^0x[a-fA-F0-9]{40}$").unwrap();
    let solana_regex = regex::Regex::new(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$").unwrap();

    evm_regex.is_match(address) || solana_regex.is_match(address)
}

/// 获取所有浏览器插件
#[tauri::command]
pub async fn get_browser_extensions(pool: State<'_, SqlitePool>) -> Result<Vec<BrowserExtension>, String> {
    let extensions = sqlx::query_as::<_, BrowserExtension>(
        "SELECT * FROM browser_extensions ORDER BY enabled DESC, id DESC"
    )
    .fetch_all(&*pool)
    .await
    .map_err(|e| format!("获取插件列表失败: {}", e))?;

    Ok(extensions)
}

/// 创建浏览器插件
#[tauri::command]
pub async fn create_browser_extension(
    pool: State<'_, SqlitePool>,
    request: CreateBrowserExtensionRequest,
) -> Result<BrowserExtension, String> {
    let tags = request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    let extension = sqlx::query_as::<_, BrowserExtension>(
        r#"
        INSERT INTO browser_extensions (name, description, path, version, author, enabled, tags)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.path)
    .bind(&request.version)
    .bind(&request.author)
    .bind(request.enabled.unwrap_or(true))
    .bind(&tags)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("创建插件失败: {}", e))?;

    Ok(extension)
}

/// 更新浏览器插件
#[tauri::command]
pub async fn update_browser_extension(
    pool: State<'_, SqlitePool>,
    request: UpdateBrowserExtensionRequest,
) -> Result<BrowserExtension, String> {
    let tags = request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    let extension = sqlx::query_as::<_, BrowserExtension>(
        r#"
        UPDATE browser_extensions SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            path = COALESCE(?, path),
            version = COALESCE(?, version),
            author = COALESCE(?, author),
            enabled = COALESCE(?, enabled),
            tags = COALESCE(?, tags),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(&request.name)
    .bind(&request.description)
    .bind(&request.path)
    .bind(&request.version)
    .bind(&request.author)
    .bind(request.enabled)
    .bind(&tags)
    .bind(request.id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("更新插件失败: {}", e))?;

    Ok(extension)
}

/// 删除浏览器插件
#[tauri::command]
pub async fn delete_browser_extension(
    pool: State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM browser_extensions WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| format!("删除插件失败: {}", e))?;

    Ok(())
}

/// 切换插件启用状态
#[tauri::command]
pub async fn toggle_browser_extension(
    pool: State<'_, SqlitePool>,
    id: i64,
    enabled: bool,
) -> Result<BrowserExtension, String> {
    let extension = sqlx::query_as::<_, BrowserExtension>(
        "UPDATE browser_extensions SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *"
    )
    .bind(enabled)
    .bind(id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("切换插件状态失败: {}", e))?;

    Ok(extension)
}

/// 扫描插件目录
#[tauri::command]
pub async fn scan_extension_folder(
    folder_path: String,
) -> Result<Vec<ExtensionFolder>, String> {
    use std::fs;
    use std::path::Path;

    log::info!("========== 开始扫描插件目录 ==========");
    log::info!("输入路径: {}", folder_path);

    let path = Path::new(&folder_path);
    log::info!("Path 对象: {:?}", path);
    log::info!("路径是否存在: {}", path.exists());
    log::info!("是否是目录: {}", path.is_dir());
    
    // 尝试获取绝对路径，如果失败也继续
    match path.canonicalize() {
        Ok(abs_path) => log::info!("绝对路径: {:?}", abs_path),
        Err(e) => log::warn!("获取绝对路径失败: {}，将继续使用原始路径", e),
    }

    if !path.exists() {
        log::error!("目录不存在: {}", folder_path);
        return Err(format!("目录不存在: {}", folder_path));
    }
    if !path.is_dir() {
        log::error!("不是有效目录: {}", folder_path);
        return Err(format!("不是有效目录: {}", folder_path));
    }

    let mut results = Vec::new();

    // 首先检查当前目录是否直接包含 manifest.json（单层结构）
    let direct_manifest = path.join("manifest.json");
    let direct_manifest_str = direct_manifest.to_string_lossy().to_string();
    log::info!("检查直接 manifest.json: {:?}", direct_manifest);
    log::info!("manifest.json 路径字符串: {}", direct_manifest_str);
    log::info!("直接 manifest 是否存在 (Path.exists): {}", direct_manifest.exists());
    
    // 尝试读取目录内容，确认是否有权限问题
    match fs::read_dir(path) {
        Ok(entries) => {
            let entries: Vec<_> = entries.collect();
            log::info!("目录中的条目数: {}", entries.len());
            for (i, entry) in entries.iter().enumerate() {
                match entry {
                    Ok(e) => {
                        let file_name = e.file_name();
                        let file_name_str = file_name.to_string_lossy();
                        log::info!("  [{}] 文件名: {:?}, 路径: {:?}", i, file_name_str, e.path());
                        // 特别检查 manifest.json
                        if file_name_str == "manifest.json" {
                            log::info!("  [{}] ******* 找到 manifest.json! *******", i);
                        }
                    }
                    Err(e) => log::warn!("  [{}] 读取条目失败: {}", i, e),
                }
            }
        }
        Err(e) => log::error!("读取目录内容失败: {}", e),
    }

    // 使用 metadata 检查文件是否存在（更可靠的方式）
    let manifest_exists = match fs::metadata(&direct_manifest) {
        Ok(metadata) => {
            log::info!("manifest.json metadata: is_file={}, len={}", metadata.is_file(), metadata.len());
            metadata.is_file()
        }
        Err(e) => {
            log::warn!("无法获取 manifest.json metadata: {}，路径: {:?}", e, direct_manifest);
            false
        }
    };
    
    if manifest_exists || direct_manifest.exists() {
        log::info!("找到直接 manifest.json: {:?}", direct_manifest);
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let manifest_info = match fs::read_to_string(&direct_manifest) {
            Ok(content) => {
                log::info!("成功读取 manifest.json，内容长度: {} 字节", content.len());
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        log::info!("成功解析 manifest.json");
                        Some(ManifestInfo {
                            name: json.get("name").and_then(|v| v.as_str()).map(String::from),
                            version: json.get("version").and_then(|v| v.as_str()).map(String::from),
                            description: json.get("description").and_then(|v| v.as_str()).map(String::from),
                        })
                    }
                    Err(e) => {
                        log::warn!("解析 manifest.json 失败: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::warn!("读取 manifest.json 失败: {}", e);
                None
            }
        };

        results.push(ExtensionFolder {
            name,
            path: path.to_string_lossy().to_string(),
            has_manifest: true,
            manifest_info,
        });

        log::info!("单层结构插件扫描完成，返回 {} 个结果", results.len());
        return Ok(results);
    }

    // 遍历子目录（支持 Chrome 扩展的多层目录结构）
    log::info!("开始遍历子目录...");
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("读取目录失败: {} - 错误: {}", folder_path, e);
            return Err(format!("读取目录失败: {} - 错误: {}", folder_path, e));
        }
    };

    let mut entry_count = 0;
    for entry in entries {
        entry_count += 1;
        match entry {
            Ok(entry) => {
                let entry_path = entry.path();
                log::info!("[条目 {}] 路径: {:?}, 是否是目录: {}", entry_count, entry_path, entry_path.is_dir());

                if entry_path.is_dir() {
                    let name = entry_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    log::info!("[条目 {}] 扫描子目录: {:?}", entry_count, entry_path);

                    // 检查当前目录是否有 manifest.json
                    let manifest_path = entry_path.join("manifest.json");
                    let has_manifest = manifest_path.exists();
                    log::info!("[条目 {}] manifest.json 路径: {:?}, 存在: {}", entry_count, manifest_path, has_manifest);

                    let mut manifest_info = if has_manifest {
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                Some(ManifestInfo {
                                    name: json.get("name").and_then(|v| v.as_str()).map(String::from),
                                    version: json.get("version").and_then(|v| v.as_str()).map(String::from),
                                    description: json.get("description").and_then(|v| v.as_str()).map(String::from),
                                })
                            } else {
                                log::warn!("[条目 {}] 解析 manifest.json 失败", entry_count);
                                None
                            }
                        } else {
                            log::warn!("[条目 {}] 读取 manifest.json 失败", entry_count);
                            None
                        }
                    } else {
                        None
                    };

                    // 如果当前目录没有 manifest.json，递归检查子目录（Chrome扩展版本目录结构）
                    if !has_manifest {
                        log::info!("[条目 {}] 当前目录没有 manifest.json，检查子目录...", entry_count);
                        match fs::read_dir(&entry_path) {
                            Ok(sub_entries) => {
                                let mut sub_entry_count = 0;
                                for sub_entry in sub_entries {
                                    sub_entry_count += 1;
                                    if let Ok(sub_entry) = sub_entry {
                                        let sub_path = sub_entry.path();
                                        log::info!("[条目 {}][子条目 {}] 子路径: {:?}, 是否是目录: {}",
                                            entry_count, sub_entry_count, sub_path, sub_path.is_dir());

                                        if sub_path.is_dir() {
                                            let sub_manifest = sub_path.join("manifest.json");
                                            let sub_manifest_exists = sub_manifest.exists();
                                            log::info!("[条目 {}][子条目 {}] 检查 manifest: {:?}, 存在: {}",
                                                entry_count, sub_entry_count, sub_manifest, sub_manifest_exists);

                                            if sub_manifest_exists {
                                                match fs::read_to_string(&sub_manifest) {
                                                    Ok(content) => {
                                                        match serde_json::from_str::<serde_json::Value>(&content) {
                                                            Ok(json) => {
                                                                manifest_info = Some(ManifestInfo {
                                                                    name: json.get("name").and_then(|v| v.as_str()).map(String::from),
                                                                    version: json.get("version").and_then(|v| v.as_str()).map(String::from),
                                                                    description: json.get("description").and_then(|v| v.as_str()).map(String::from),
                                                                });
                                                                log::info!("[条目 {}][子条目 {}] 找到有效插件: {:?}",
                                                                    entry_count, sub_entry_count, manifest_info);
                                                                results.push(ExtensionFolder {
                                                                    name: name.clone(),
                                                                    path: sub_path.to_string_lossy().to_string(),
                                                                    has_manifest: true,
                                                                    manifest_info: manifest_info.clone(),
                                                                });
                                                                break;
                                                            }
                                                            Err(e) => {
                                                                log::warn!("[条目 {}][子条目 {}] 解析 manifest.json 失败: {}",
                                                                    entry_count, sub_entry_count, e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::warn!("[条目 {}][子条目 {}] 读取 manifest.json 失败: {}",
                                                            entry_count, sub_entry_count, e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                log::info!("[条目 {}] 子目录扫描完成，共检查 {} 个子条目", entry_count, sub_entry_count);
                                continue;
                            }
                            Err(e) => {
                                log::warn!("[条目 {}] 读取子目录失败: {} - 错误: {}", entry_count, entry_path.display(), e);
                            }
                        }
                    }

                    results.push(ExtensionFolder {
                        name,
                        path: entry_path.to_string_lossy().to_string(),
                        has_manifest,
                        manifest_info,
                    });
                }
            }
            Err(e) => {
                log::error!("[条目 {}] 读取条目失败: {}", entry_count, e);
            }
        }
    }

    log::info!("========== 扫描完成 ==========");
    log::info!("共遍历 {} 个条目，找到 {} 个结果", entry_count, results.len());
    for (i, result) in results.iter().enumerate() {
        log::info!("结果[{}]: name={}, path={}, has_manifest={}",
            i, result.name, result.path, result.has_manifest);
    }

    Ok(results)
}

/// 从目录导入插件
#[tauri::command]
pub async fn import_extension_from_folder(
    pool: State<'_, SqlitePool>,
    folder_path: String,
    name: Option<String>,
) -> Result<BrowserExtension, String> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err("目录不存在或不是有效目录".to_string());
    }

    log::info!("导入插件，目录: {:?}", folder_path);

    // 首先检查当前目录是否有 manifest.json
    let mut manifest_path = path.join("manifest.json");
    let mut actual_extension_path = folder_path.clone();

    if !manifest_path.exists() {
        log::info!("当前目录没有 manifest.json，尝试查找子目录...");
        // 如果没有，尝试查找子目录（支持 Chrome 扩展版本目录结构）
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let sub_manifest = entry_path.join("manifest.json");
                        if sub_manifest.exists() {
                            manifest_path = sub_manifest;
                            actual_extension_path = entry_path.to_string_lossy().to_string();
                            log::info!("在子目录找到 manifest.json: {:?}", manifest_path);
                            break;
                        }
                    }
                }
            }
        }
    }

    if !manifest_path.exists() {
        return Err("目录中未找到 manifest.json 文件".to_string());
    }

    let manifest_content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 manifest.json 失败: {}", e))?;

    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("解析 manifest.json 失败: {}", e))?;

    let ext_name = name.unwrap_or_else(|| {
        manifest.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Extension")
            .to_string()
    });

    let version = manifest.get("version")
        .and_then(|v| v.as_str())
        .map(String::from);

    let description = manifest.get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    log::info!("导入插件: name={}, version={:?}, path={:?}", ext_name, version, actual_extension_path);

    let extension = sqlx::query_as::<_, BrowserExtension>(
        r#"
        INSERT INTO browser_extensions (name, description, path, version, enabled)
        VALUES (?, ?, ?, ?, 1)
        RETURNING *
        "#
    )
    .bind(&ext_name)
    .bind(&description)
    .bind(&actual_extension_path)
    .bind(&version)
    .fetch_one(&*pool)
    .await
    .map_err(|e| format!("导入插件失败: {}", e))?;

    Ok(extension)
}
