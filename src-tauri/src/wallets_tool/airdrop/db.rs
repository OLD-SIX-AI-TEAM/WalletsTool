use sqlx::SqlitePool;
use anyhow::Result;

/// 添加增强指纹字段到 browser_profiles 表
async fn migrate_enhanced_fingerprint_fields(pool: &SqlitePool) -> Result<()> {
    // 检查 browser_profiles 表是否存在
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='browser_profiles'"
    )
    .fetch_one(pool)
    .await?;

    if table_exists == 0 {
        return Ok(());
    }

    // 定义需要添加的新字段
    let new_columns = vec![
        ("hardware_concurrency", "INTEGER"),
        ("device_memory", "INTEGER"),
        ("color_depth", "INTEGER"),
        ("languages", "TEXT"),
        ("vendor", "TEXT"),
        ("gpu_vendor", "TEXT"),
        ("gpu_renderer", "TEXT"),
        ("color_scheme", "TEXT"),
        ("max_touch_points", "INTEGER"),
        ("has_touch", "BOOLEAN"),
        ("screen_orientation_angle", "INTEGER"),
        ("screen_orientation_type", "TEXT"),
        ("font_family", "TEXT"),
        ("client_hints_platform", "TEXT"),
        ("client_hints_platform_version", "TEXT"),
        ("client_hints_architecture", "TEXT"),
        ("client_hints_bitness", "TEXT"),
        ("client_hints_model", "TEXT"),
        ("client_hints_wow64", "TEXT"),
        ("fingerprint_hash", "TEXT"),
        ("platform_name", "TEXT"),
    ];

    // 获取当前表的列信息
    let existing_columns: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_table_info('browser_profiles')"
    )
    .fetch_all(pool)
    .await?;

    // 添加缺失的列
    for (column_name, column_type) in new_columns {
        if !existing_columns.iter().any(|c| c == column_name) {
            println!("添加列 {} 到 browser_profiles 表...", column_name);
            let sql = format!(
                "ALTER TABLE browser_profiles ADD COLUMN {} {}",
                column_name, column_type
            );
            sqlx::query(&sql).execute(pool).await?;
        }
    }

    // 创建指纹哈希索引
    let index_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_browser_profiles_fingerprint_hash'"
    )
    .fetch_one(pool)
    .await?;

    if index_exists == 0 {
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_browser_profiles_fingerprint_hash ON browser_profiles(fingerprint_hash)"
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// 检查并修复 device_scale_factor 列类型
async fn migrate_device_scale_factor(pool: &SqlitePool) -> Result<()> {
    // 检查 browser_profiles 表是否存在
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='browser_profiles'"
    )
    .fetch_one(pool)
    .await?;

    if table_exists == 0 {
        return Ok(());
    }

    // 检查 device_scale_factor 列的数据类型
    let column_info: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, type FROM pragma_table_info('browser_profiles') WHERE name = 'device_scale_factor'"
    )
    .fetch_all(pool)
    .await?;

    if let Some((_, col_type)) = column_info.first() {
        if col_type.to_uppercase() == "REAL" {
            println!("迁移 device_scale_factor 列类型从 REAL 到 INTEGER...");
            
            // 禁用外键约束检查
            sqlx::query("PRAGMA foreign_keys = OFF")
                .execute(pool)
                .await?;
            
            // SQLite 不支持直接修改列类型，需要重建表
            sqlx::query(
                r#"
                CREATE TABLE browser_profiles_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    description TEXT,
                    user_agent TEXT,
                    viewport_width INTEGER NOT NULL DEFAULT 1920,
                    viewport_height INTEGER NOT NULL DEFAULT 1080,
                    device_scale_factor INTEGER NOT NULL DEFAULT 1,
                    locale TEXT NOT NULL DEFAULT 'en-US',
                    timezone_id TEXT NOT NULL DEFAULT 'America/New_York',
                    proxy_type TEXT NOT NULL DEFAULT 'direct',
                    proxy_host TEXT,
                    proxy_port INTEGER,
                    proxy_username TEXT,
                    proxy_password TEXT,
                    canvas_spoof BOOLEAN NOT NULL DEFAULT 1,
                    webgl_spoof BOOLEAN NOT NULL DEFAULT 1,
                    audio_spoof BOOLEAN NOT NULL DEFAULT 1,
                    timezone_spoof BOOLEAN NOT NULL DEFAULT 1,
                    geolocation_spoof BOOLEAN NOT NULL DEFAULT 1,
                    font_spoof BOOLEAN NOT NULL DEFAULT 1,
                    webrtc_spoof BOOLEAN NOT NULL DEFAULT 1,
                    navigator_override BOOLEAN NOT NULL DEFAULT 1,
                    webdriver_override BOOLEAN NOT NULL DEFAULT 1,
                    custom_headers TEXT,
                    headless BOOLEAN NOT NULL DEFAULT 0,
                    extensions TEXT,
                    is_default BOOLEAN NOT NULL DEFAULT 0,
                    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                )
                "#
            )
            .execute(pool)
            .await?;

            // 复制数据
            sqlx::query(
                r#"
                INSERT INTO browser_profiles_new (
                    id, name, description, user_agent, viewport_width, viewport_height,
                    device_scale_factor, locale, timezone_id, proxy_type, proxy_host, proxy_port,
                    proxy_username, proxy_password, canvas_spoof, webgl_spoof, audio_spoof,
                    timezone_spoof, geolocation_spoof, font_spoof, webrtc_spoof,
                    navigator_override, webdriver_override, custom_headers, headless,
                    extensions, is_default, created_at, updated_at
                )
                SELECT 
                    id, name, description, user_agent, viewport_width, viewport_height,
                    CAST(device_scale_factor AS INTEGER), locale, timezone_id, proxy_type, proxy_host, proxy_port,
                    proxy_username, proxy_password, canvas_spoof, webgl_spoof, audio_spoof,
                    timezone_spoof, geolocation_spoof, font_spoof, webrtc_spoof,
                    navigator_override, webdriver_override, custom_headers, headless,
                    extensions, is_default, created_at, updated_at
                FROM browser_profiles
                "#
            )
            .execute(pool)
            .await?;

            // 删除旧表
            sqlx::query("DROP TABLE browser_profiles")
                .execute(pool)
                .await?;

            // 重命名新表
            sqlx::query("ALTER TABLE browser_profiles_new RENAME TO browser_profiles")
                .execute(pool)
                .await?;

            // 重建索引
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_browser_profiles_default ON browser_profiles(is_default)")
                .execute(pool)
                .await?;
            
            // 重新启用外键约束检查
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(pool)
                .await?;

            println!("device_scale_factor 列类型迁移完成");
        }
    }

    Ok(())
}

/// 初始化浏览器自动化相关的数据库表
pub async fn init_airdrop_tables(pool: &SqlitePool) -> Result<()> {
    // 执行迁移：修复 device_scale_factor 列类型
    migrate_device_scale_factor(pool).await?;
    
    // 执行迁移：添加增强指纹字段
    migrate_enhanced_fingerprint_fields(pool).await?;

    // 空投钱包表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS airdrop_wallets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            encrypted_private_key TEXT NOT NULL,
            label TEXT,
            group_name TEXT NOT NULL DEFAULT 'Default',
            proxy TEXT NOT NULL DEFAULT 'Direct',
            chain_type TEXT NOT NULL DEFAULT 'evm',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_airdrop_wallets_address ON airdrop_wallets(address)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_airdrop_wallets_group ON airdrop_wallets(group_name)"
    )
    .execute(pool)
    .await?;

    // 浏览器环境配置表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS browser_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            user_agent TEXT,
            viewport_width INTEGER NOT NULL DEFAULT 1920,
            viewport_height INTEGER NOT NULL DEFAULT 1080,
            device_scale_factor INTEGER NOT NULL DEFAULT 1,
            locale TEXT NOT NULL DEFAULT 'en-US',
            timezone_id TEXT NOT NULL DEFAULT 'America/New_York',
            proxy_type TEXT NOT NULL DEFAULT 'direct',
            proxy_host TEXT,
            proxy_port INTEGER,
            proxy_username TEXT,
            proxy_password TEXT,
            canvas_spoof BOOLEAN NOT NULL DEFAULT 1,
            webgl_spoof BOOLEAN NOT NULL DEFAULT 1,
            audio_spoof BOOLEAN NOT NULL DEFAULT 1,
            timezone_spoof BOOLEAN NOT NULL DEFAULT 1,
            geolocation_spoof BOOLEAN NOT NULL DEFAULT 1,
            font_spoof BOOLEAN NOT NULL DEFAULT 1,
            webrtc_spoof BOOLEAN NOT NULL DEFAULT 1,
            navigator_override BOOLEAN NOT NULL DEFAULT 1,
            webdriver_override BOOLEAN NOT NULL DEFAULT 1,
            custom_headers TEXT,
            headless BOOLEAN NOT NULL DEFAULT 0,
            extensions TEXT,
            is_default BOOLEAN NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_browser_profiles_default ON browser_profiles(is_default)"
    )
    .execute(pool)
    .await?;

    // 自动化脚本表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS automation_scripts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            content TEXT NOT NULL,
            compiled_content TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            is_system BOOLEAN NOT NULL DEFAULT 0,
            required_apis TEXT,
            author TEXT,
            tags TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_automation_scripts_tags ON automation_scripts(tags)"
    )
    .execute(pool)
    .await?;

    // 自动化任务表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS automation_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            script_id INTEGER NOT NULL,
            wallet_ids TEXT NOT NULL,
            profile_strategy TEXT NOT NULL DEFAULT 'random',
            specific_profile_id INTEGER,
            schedule_type TEXT NOT NULL DEFAULT 'once',
            schedule_config TEXT NOT NULL,
            concurrency INTEGER NOT NULL DEFAULT 1,
            timeout_seconds INTEGER NOT NULL DEFAULT 300,
            retry_times INTEGER NOT NULL DEFAULT 3,
            retry_interval_seconds INTEGER NOT NULL DEFAULT 60,
            status TEXT NOT NULL DEFAULT 'draft',
            last_run_time DATETIME,
            next_run_time DATETIME,
            total_runs INTEGER NOT NULL DEFAULT 0,
            success_runs INTEGER NOT NULL DEFAULT 0,
            failed_runs INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (script_id) REFERENCES automation_scripts(id),
            FOREIGN KEY (specific_profile_id) REFERENCES browser_profiles(id)
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_automation_tasks_status ON automation_tasks(status)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_automation_tasks_next_run ON automation_tasks(next_run_time) WHERE status = 'enabled'"
    )
    .execute(pool)
    .await?;

    // 任务执行记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_executions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            wallet_id INTEGER NOT NULL,
            profile_id INTEGER,
            status TEXT NOT NULL,
            start_time DATETIME,
            end_time DATETIME,
            duration_ms INTEGER,
            error_message TEXT,
            result_data TEXT,
            logs TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (task_id) REFERENCES automation_tasks(id),
            FOREIGN KEY (wallet_id) REFERENCES airdrop_wallets(id),
            FOREIGN KEY (profile_id) REFERENCES browser_profiles(id)
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_task ON task_executions(task_id)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_status ON task_executions(status)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_time ON task_executions(created_at DESC)"
    )
    .execute(pool)
    .await?;

    // 浏览器插件表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS browser_extensions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            path TEXT NOT NULL,
            version TEXT,
            author TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            tags TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_browser_extensions_enabled ON browser_extensions(enabled)"
    )
    .execute(pool)
    .await?;

    // 插入默认脚本
    insert_default_scripts(pool).await?;

    Ok(())
}

/// 插入默认的自动化脚本
/// 注意：此功能已禁用，不再自动创建系统脚本
async fn insert_default_scripts(_pool: &SqlitePool) -> Result<()> {
    // 系统脚本自动创建功能已禁用
    // 用户可以通过界面手动创建需要的脚本
    Ok(())
}
