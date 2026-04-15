#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod utils;
mod wallets_tool;
mod plugins;
mod database;

use tauri::{WindowEvent, Manager, AppHandle, Runtime, Emitter, tray::TrayIconBuilder, menu::{MenuBuilder, MenuItemBuilder}};
use wallets_tool::airdrop::scheduler::TaskScheduler;
use std::sync::Arc;


// Tauri 命令：关闭所有子窗口
#[tauri::command]
async fn close_all_child_windows<R: Runtime>(app: AppHandle<R>, main_window_label: String) -> Result<Vec<String>, String> {
    let mut closed_windows = Vec::new();

    let windows = app.webview_windows();

    for (label, window) in windows {
        if label != main_window_label {  // 只排除主窗口
            match window.close() {
                Ok(_) => {
                    closed_windows.push(label);
                }
                Err(e) => {
                    eprintln!("关闭窗口 {label} 失败: {e}");
                }
            }
        }
    }

    Ok(closed_windows)
}

// Tauri 命令：获取所有子窗口
#[tauri::command]
async fn get_all_child_windows<R: Runtime>(app: AppHandle<R>, main_window_label: String) -> Result<Vec<String>, String> {
    let windows = app.webview_windows();
    let child_windows: Vec<String> = windows.keys()
        .filter(|&label| label != &main_window_label)
        .cloned()
        .collect();
    
    Ok(child_windows)
}

// Tauri 命令：强制关闭主窗口（跳过事件处理）
#[tauri::command]
async fn force_close_main_window<R: Runtime>(_app: AppHandle<R>) -> Result<(), String> {
    // 直接退出应用程序，跳过窗口关闭事件处理
    std::process::exit(0);
}

// Tauri 命令：显示主窗口
#[tauri::command]
async fn show_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // 在Windows系统中强制窗口置顶，然后立即取消置顶状态
        // 这样可以确保窗口弹出到最上层而不会一直保持在最上层
        window.set_always_on_top(true).map_err(|e| e.to_string())?;
        window.set_always_on_top(false).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// 辅助函数：确保窗口位置在可见屏幕范围内
fn ensure_window_visible(
    window_x: f64,
    window_y: f64,
    window_width: f64,
    window_height: f64,
    monitor: &tauri::Monitor,
) -> (f64, f64) {
    let monitor_size = monitor.size();
    let monitor_position = monitor.position();

    // 计算显示器的边界（考虑缩放因子）
    let scale_factor = monitor.scale_factor();
    let monitor_left = monitor_position.x as f64;
    let monitor_top = monitor_position.y as f64;
    let monitor_right = monitor_left + (monitor_size.width as f64 / scale_factor);
    let monitor_bottom = monitor_top + (monitor_size.height as f64 / scale_factor);

    // 确保窗口至少有100px可见在水平方向
    let min_visible_width = 100.0;
    let mut final_x = window_x;
    let mut final_y = window_y;

    // 水平边界检查
    if window_x + window_width < monitor_left + min_visible_width {
        // 窗口完全在显示器左侧外
        final_x = monitor_left + 10.0;
    } else if window_x > monitor_right - min_visible_width {
        // 窗口完全在显示器右侧外
        final_x = monitor_right - window_width - 10.0;
    }

    // 垂直边界检查 - 确保窗口标题栏可见
    let min_visible_height = 30.0;
    if window_y + window_height < monitor_top + min_visible_height {
        // 窗口完全在显示器上方外
        final_y = monitor_top + 10.0;
    } else if window_y > monitor_bottom - min_visible_height {
        // 窗口完全在显示器下方外
        final_y = monitor_bottom - window_height - 10.0;
    }

    // 额外安全检查：确保不会变成负数或极大值
    if final_x < -10000.0 || final_x > 10000.0 {
        final_x = monitor_left + 10.0;
    }
    if final_y < -10000.0 || final_y > 10000.0 {
        final_y = monitor_top + 10.0;
    }

    (final_x, final_y)
}

// Tauri 命令：根据dock items数量自动设置主窗口大小和位置
#[tauri::command]
async fn set_main_window_size_for_dock<R: Runtime>(app: AppHandle<R>, item_count: u32) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        // 计算dock宽度 - 基于前端实际样式
        // dock-icon: 40px, dock-item padding: 4px, 所以每个item占用: 40 + 4*2 = 48px
        let item_width = 48u32;
        // dock item之间的gap: 6px (dock的gap属性)
        let gap = 6u32;
        // 分隔线: 1px宽 + margin 0 6px = 13px实际占用
        let divider_width = 13u32;
        // dock左右padding: 16px * 2 = 32px
        let padding = 32u32;
        // 分隔线数量
        let divider_count = 2u32;
        // 最大宽度限制
        let max_width = 1360u32;

        // dock子元素: item_count个功能items + 2个固定items(设置和退出) + 2个dividers
        let total_items = item_count + 2;
        // gap数量 = 子元素总数 - 1
        let total_children = total_items + divider_count;
        let gaps_count = if total_children > 1 { total_children - 1 } else { 0 };
        
        // 计算总宽度
        let calculated_width = (total_items * item_width)              // 所有items
            + (gaps_count * gap)                                        // 所有gaps
            + (divider_count * divider_width)                           // 分隔线
            + padding;                                                  // 左右padding

        // 应用最大宽度限制
        let total_width = calculated_width.min(max_width);

        // 高度固定为dock高度 + 边距
        // 上padding 20px + 下padding 12px + dock-item上padding 8px + 图标40px + label约16px
        let total_height = 96u32;

        println!("[set_main_window_size_for_dock] item_count={}, total_items={}, gaps={}, dividers={}, calculated_width={}, final_width={}",
            item_count, total_items, gaps_count, divider_count, calculated_width, total_width);

        // 设置窗口大小
        window.set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: total_width as f64,
            height: total_height as f64,
        })).map_err(|e| e.to_string())?;

        // 设置窗口位置到屏幕底部任务栏上方
        // 获取主显示器信息
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let monitor_size = monitor.size();
            let monitor_position = monitor.position();
            let scale_factor = monitor.scale_factor();

            // 计算窗口位置：水平居中，垂直位于屏幕底部上方
            let raw_window_x = monitor_position.x as f64 + (monitor_size.width as f64 / scale_factor - total_width as f64) / 2.0;
            // 距离屏幕底部60px（留出任务栏空间，避免重叠）
            let taskbar_offset = 60u32;
            let raw_window_y = monitor_position.y as f64 + (monitor_size.height as f64 / scale_factor) - total_height as f64 - taskbar_offset as f64;

            // 确保窗口位置在可见范围内
            let (window_x, window_y) = ensure_window_visible(
                raw_window_x,
                raw_window_y,
                total_width as f64,
                total_height as f64,
                &monitor,
            );

            println!("[set_main_window_position] monitor_size={}x{}, scale={}, position=({}, {}), raw=({}, {}), final=({}, {})",
                monitor_size.width, monitor_size.height, scale_factor,
                monitor_position.x, monitor_position.y,
                raw_window_x, raw_window_y,
                window_x, window_y);

            window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: window_x,
                y: window_y,
            })).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// Tauri 命令：设置主窗口透明度
#[tauri::command]
async fn set_main_window_opacity<R: Runtime>(_app: AppHandle<R>, _opacity: f64) -> Result<(), String> {
    // TODO: Tauri v2 中透明度 API 发生了变化，需要重新实现
    // if let Some(window) = app.get_webview_window("main") {
    //     let clamped_opacity = opacity.clamp(0.1, 1.0);
    //     window.set_opacity(clamped_opacity).map_err(|e| e.to_string())?;
    // }
    Ok(())
}

// Tauri 命令：获取主窗口当前透明度
#[tauri::command]
async fn get_main_window_opacity<R: Runtime>(_app: AppHandle<R>) -> Result<f64, String> {
    // TODO: Tauri v2 中透明度 API 发生了变化，需要重新实现
    // if let Some(window) = app.get_webview_window("main") {
    //     match window.opacity() {
    //         Ok(opacity) => return Ok(opacity),
    //         Err(_) => return Ok(1.0),
    //     }
    // }
    Ok(1.0)
}

// Tauri 命令：设置主窗口始终置顶
#[tauri::command]
async fn set_main_window_always_on_top<R: Runtime>(app: AppHandle<R>, always_on_top: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_always_on_top(always_on_top).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Tauri 命令：从托盘打开功能窗口
#[tauri::command]
async fn open_function_window<R: Runtime>(app: AppHandle<R>, page_name: String) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;
    
    let (title, _icon) = match page_name.as_str() {
        "transfer" => ("💸 批量转账", "transfer"),
        "balance" => ("💰 余额查询", "balance"),
        "monitor" => ("👁️ 链上监控", "monitor"),
        _ => ("❓ 未知功能", "unknown")
    };
    
    // let display_icon = match icon {
    //     "transfer" => "💸",
    //     "balance" => "💰",
    //     "monitor" => "👁️",
    //     _ => ""
    // };
    
    // 获取当前所有窗口的标签
    let existing_windows = app.webview_windows();
    let mut window_count = 1;
    
    // 循环查找可用的窗口标签，确保不与现有窗口冲突
    let window_label = loop {
        let candidate_label = format!("{page_name}{window_count}");
        
        // 检查这个标签是否已经存在
        if !existing_windows.contains_key(&candidate_label) {
            break candidate_label;
        }
        
        // 如果存在，递增计数器继续尝试
        window_count += 1;
        
        // 防止无限循环，设置一个合理的上限
        if window_count > 100 {
            return Err("无法找到可用的窗口标签，已达到最大窗口数量限制".to_string());
        }
    };
    
    let window_url = format!("/#/{page_name}?count={window_count}");
    
    // 生成窗口标题：统一格式为 "WalletsTool - {图标} {功能名} [{序号}]"
    let window_title = if window_count > 1 {
        format!("WalletsTool - {title} [{window_count}]")
    } else {
        format!("WalletsTool - {title}")
    };
    
    // 创建新窗口
    let webview = WebviewWindowBuilder::new(&app, &window_label, tauri::WebviewUrl::App(window_url.into()))
        .title(&window_title)
        .inner_size(1350.0, 900.0)
        .resizable(true)
        .center()
        .decorations(false)
        .visible(false)
        .skip_taskbar(false)
        .build()
        .map_err(|e| e.to_string())?;
    
    // 显示窗口
    webview.show().map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tokio::main]
async fn main() {
    // 启动安全保护
    wallets_tool::security::enable_protection();

    // 初始化公开数据库（链配置、RPC节点等）
    if let Err(err) = database::init_public_database().await {
        eprintln!("公开数据库初始化失败: {err:?}");
        return;
    }

    // 使用公开数据库连接池
    let sqlite_pool = database::DualDatabaseManager::public_pool();
    println!("Initializing WalletManagerService...");
    let wallet_manager_service =
        wallets_tool::wallet_manager::service::WalletManagerService::new(sqlite_pool.clone());
    
    let chain_service = database::chain_service::ChainService::new();
    
    // Initialize task scheduler for browser automation
    let task_scheduler = Arc::new(TaskScheduler::new(sqlite_pool.clone()));
    let scheduler_for_setup = task_scheduler.clone();
    
    let updater_pubkey = option_env!("WALLETSTOOL_UPDATER_PUBKEY").unwrap_or("").trim();
    let updater_plugin = if updater_pubkey.is_empty() {
        tauri_plugin_updater::Builder::new().build()
    } else {
        tauri_plugin_updater::Builder::new()
            .pubkey(updater_pubkey)
            .build()
    };
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(updater_plugin)
        .manage(sqlite_pool)
        .manage(wallet_manager_service)
        .manage(chain_service)
        .manage(task_scheduler)
        .setup(move |app| {
            // Start the task scheduler
            let scheduler = scheduler_for_setup;
            tauri::async_runtime::spawn(async move {
                scheduler.start().await;
            });

            // 主窗口直接显示
            // 在后端直接设置主窗口初始大小和位置，避免等待前端加载
            // dock items 数量：6个功能按钮
            let dock_item_count = 6u32;
            
            if let Some(window) = app.get_webview_window("main") {
                // 计算dock宽度 - 与 set_main_window_size_for_dock 函数保持一致
                let item_width = 48u32;
                let gap = 6u32;
                let divider_width = 13u32;
                let padding = 32u32;
                let divider_count = 2u32;
                let max_width = 1360u32;

                // dock子元素: dock_item_count个功能items + 2个固定items(设置和退出) + 2个dividers
                let total_items = dock_item_count + 2;
                let total_children = total_items + divider_count;
                let gaps_count = if total_children > 1 { total_children - 1 } else { 0 };
                
                let calculated_width = (total_items * item_width)
                    + (gaps_count * gap)
                    + (divider_count * divider_width)
                    + padding;

                let total_width = calculated_width.min(max_width);
                let total_height = 96u32;

                println!("[setup] Setting initial window size: {}x{}", total_width, total_height);

                // 设置窗口大小
                if let Err(e) = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: total_width as f64,
                    height: total_height as f64,
                })) {
                    eprintln!("设置主窗口初始大小失败: {e}");
                }

                // 设置窗口位置到屏幕底部
                if let Ok(Some(monitor)) = window.primary_monitor() {
                    let monitor_size = monitor.size();
                    let monitor_position = monitor.position();
                    let scale_factor = monitor.scale_factor();

                    let raw_window_x = monitor_position.x as f64 + (monitor_size.width as f64 / scale_factor - total_width as f64) / 2.0;
                    // 距离屏幕底部60px（留出任务栏空间，避免重叠）
                    let taskbar_offset = 60u32;
                    let raw_window_y = monitor_position.y as f64 + (monitor_size.height as f64 / scale_factor) - total_height as f64 - taskbar_offset as f64;

                    // 确保窗口位置在可见范围内
                    let (window_x, window_y) = ensure_window_visible(
                        raw_window_x,
                        raw_window_y,
                        total_width as f64,
                        total_height as f64,
                        &monitor,
                    );

                    println!("[setup] Setting initial window position: raw=({}, {}), final=({}, {})",
                        raw_window_x, raw_window_y, window_x, window_y);

                    if let Err(e) = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: window_x,
                        y: window_y,
                    })) {
                        eprintln!("设置主窗口初始位置失败: {e}");
                    }
                }

                // 先设置好位置和大小，最后再显示窗口，避免出现位置不对的透明轮廓
                if let Err(e) = window.show() {
                    eprintln!("显示主窗口失败: {e}");
                } else {
                    println!("[setup] Window shown successfully at correct position");
                }

                // 确保窗口获得焦点
                if let Err(e) = window.set_focus() {
                    eprintln!("设置主窗口焦点失败: {e}");
                }

                // 禁用窗口阴影，避免在圆角处显示窗口边框
                #[cfg(target_os = "windows")]
                if let Err(e) = window.set_shadow(false) {
                    eprintln!("禁用主窗口阴影失败: {e}");
                }
            }

            // 构建托盘菜单
            let show_main = MenuItemBuilder::new("显示主窗口").id("show_main").build(app)?;
            let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let batch_transfer = MenuItemBuilder::new("批量转账").id("transfer").build(app)?;
            let balance_query = MenuItemBuilder::new("余额查询").id("balance").build(app)?;
            let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::new("退出程序").id("quit").build(app)?;
            
            let menu = MenuBuilder::new(app)
                .item(&show_main)
                .item(&separator1)
                .item(&batch_transfer)
                .item(&balance_query)
                .item(&separator2)
                .item(&quit)
                .build()?;
            
            // 创建托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show_main" => {
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = show_main_window(app_handle).await {
                                    eprintln!("显示主窗口失败: {e}");
                                }
                            });
                        }
                        "transfer" => {
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = open_function_window(app_handle, "transfer".to_string()).await {
                                    eprintln!("打开批量转账窗口失败: {e}");
                                }
                            });
                        }
                        "balance" => {
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = open_function_window(app_handle, "balance".to_string()).await {
                                    eprintln!("打开余额查询窗口失败: {e}");
                                }
                            });
                        }
                        "quit" => {
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                // 先显示主窗口
                                if let Err(e) = show_main_window(app_handle.clone()).await {
                                    eprintln!("显示主窗口失败: {e}");
                                }

                                // 发送退出确认事件到前端
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    if let Err(e) = window.emit("tray-quit-requested", ()) {
                                        eprintln!("发送托盘退出事件失败: {e}");
                                    }
                                }
                            });
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |tray, event| {
                    match event {
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..  
                        } => {
                            // 左键点击显示主窗口
                            let app_handle = tray.app_handle().clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = show_main_window(app_handle).await {
                                    eprintln!("左键点击托盘显示主窗口失败: {e}");
                                }
                            });
                        }
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Right,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..  
                        } => {
                            // 右键点击事件（菜单已在创建时设置）
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let window_label = window.label().to_string();

                if window_label == "main" {
                    // 阻止默认的关闭行为
                    api.prevent_close();
                    
                    // 将主窗口置于最前端，确保用户能看到确认对话框
                    if let Err(e) = window.show() {
                        eprintln!("显示主窗口失败: {e}");
                    }
                    if let Err(e) = window.set_focus() {
                        eprintln!("设置主窗口焦点失败: {e}");
                    }
                    
                    // 强制窗口置顶以确保在Windows系统中能够真正显示在最前端
                    if let Err(e) = window.set_always_on_top(true) {
                        eprintln!("设置窗口置顶失败: {e}");
                    }
                    
                    // 发送自定义事件到前端
                    if let Err(e) = window.emit("main-window-close-requested", ()) {
                        eprintln!("发送关闭事件失败: {e}");
                    }
                    
                    // 克隆窗口引用以便在异步任务中使用
                    let window_clone = window.clone();
                    
                    // 在短暂延迟后恢复窗口的正常状态
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                        if let Err(e) = window_clone.set_always_on_top(false) {
                            eprintln!("恢复窗口正常状态失败: {e}");
                        }
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            wallets_tool::ecosystems::ethereum::chain_config::get_chain_list,
            wallets_tool::ecosystems::ethereum::chain_config::get_coin_list,
            wallets_tool::ecosystems::ethereum::chain_config::add_coin,
            wallets_tool::ecosystems::ethereum::chain_config::remove_coin,
            wallets_tool::ecosystems::ethereum::chain_config::update_coin,
            wallets_tool::ecosystems::ethereum::chain_config::update_chain_pic_urls,
            wallets_tool::ecosystems::ethereum::chain_config::update_token_abi,
            // chain management commands
            wallets_tool::ecosystems::ethereum::chain_config::add_chain,
            wallets_tool::ecosystems::ethereum::chain_config::update_chain,
            wallets_tool::ecosystems::ethereum::chain_config::remove_chain,
            wallets_tool::ecosystems::ethereum::chain_config::get_chain_detail,
            wallets_tool::utils::download_file,
            wallets_tool::utils::save_chain_icon,
            wallets_tool::utils::get_chain_icon,
            wallets_tool::utils::read_resource_file,
            wallets_tool::utils::save_file,
            wallets_tool::utils::get_temp_dir,
            wallets_tool::utils::open_file_directory,
            wallets_tool::update::check_github_release_update,
            wallets_tool::update::check_update,
            wallets_tool::update::download_and_install_update,
            wallets_tool::update::download_and_install_from_url,
            wallets_tool::update::download_update_only,
            // fs extra functions
            plugins::fs_extra::exists,
            plugins::fs_extra::open_file,
            // balance query functions
            wallets_tool::ecosystems::ethereum::simple_balance_query::query_balances_simple,
            wallets_tool::ecosystems::ethereum::simple_balance_query::query_balances_with_updates,
            wallets_tool::ecosystems::ethereum::simple_balance_query::stop_balance_query,
            wallets_tool::ecosystems::ethereum::simple_balance_query::reset_balance_query_stop,
            // window management functions
            close_all_child_windows,
            get_all_child_windows,
            force_close_main_window,
            show_main_window,
            set_main_window_size_for_dock,
            set_main_window_opacity,
            get_main_window_opacity,
            set_main_window_always_on_top,
            open_function_window,
            // database hot reload functions
            database::reload_database,
            database::check_database_schema,
            database::export_database_to_init_sql,
            database::is_wallet_db_ready,
            // dual database commands
            database::commands::get_dual_database_status,
            database::commands::init_public_db,
            database::commands::init_secure_db,
            database::commands::unlock_secure_db,
            database::commands::lock_secure_db,
            database::commands::is_secure_db_initialized,
            database::commands::is_public_db_ready,
            database::commands::is_secure_db_unlocked,
            database::commands::is_wallet_manager_ready,
            // transfer functions
            wallets_tool::transfer::base_coin_transfer,
            wallets_tool::transfer::base_coin_transfer_fast,
            wallets_tool::transfer::check_transactions_status_batch,
            wallets_tool::transfer::check_transaction_status,
            wallets_tool::transfer::query_balance,
            wallets_tool::transfer::check_wallet_recent_transfers,
            wallets_tool::transfer::stop_transfer,
            wallets_tool::transfer::reset_transfer_stop,
            // solana transfer functions
            wallets_tool::ecosystems::solana::transfer::sol_transfer,
            wallets_tool::ecosystems::solana::transfer::sol_token_transfer,
            wallets_tool::ecosystems::solana::transfer::sol_transfer_fast,
            wallets_tool::ecosystems::solana::transfer::sol_token_transfer_fast,
            wallets_tool::ecosystems::solana::transfer::sol_check_recent_transfers,
            wallets_tool::ecosystems::solana::transfer::sol_check_transactions_status_batch,
            wallets_tool::ecosystems::solana::transfer::sol_query_balances_with_updates,
            wallets_tool::ecosystems::solana::provider::test_solana_rpc_connection,

            // token transfer functions
            wallets_tool::token_transfer::token_transfer,
            wallets_tool::token_transfer::token_transfer_fast,
            wallets_tool::token_transfer::query_token_balance,
            wallets_tool::token_transfer::get_token_info,
            // provider functions
            wallets_tool::provider::get_chain_gas_price,
            wallets_tool::provider::test_rpc_url,
            wallets_tool::provider::get_multiple_gas_prices,
            // rpc management functions
            wallets_tool::ecosystems::ethereum::rpc_management::get_rpc_providers,
            wallets_tool::ecosystems::ethereum::rpc_management::add_rpc_provider,
            wallets_tool::ecosystems::ethereum::rpc_management::update_rpc_provider,
            wallets_tool::ecosystems::ethereum::rpc_management::delete_rpc_provider,
            wallets_tool::ecosystems::ethereum::rpc_management::test_rpc_connection,
            // proxy management functions
            wallets_tool::ecosystems::ethereum::proxy_commands::set_proxy_window_id,
            wallets_tool::ecosystems::ethereum::proxy_commands::save_proxy_config,
            wallets_tool::ecosystems::ethereum::proxy_commands::save_proxy_config_for_window,
            wallets_tool::ecosystems::ethereum::proxy_commands::get_proxy_config,
            wallets_tool::ecosystems::ethereum::proxy_commands::get_proxy_config_for_window,
            wallets_tool::ecosystems::ethereum::proxy_commands::test_proxy_connection,
            wallets_tool::ecosystems::ethereum::proxy_commands::get_proxy_stats,
            wallets_tool::ecosystems::ethereum::proxy_commands::get_proxy_stats_for_window,
            wallets_tool::ecosystems::ethereum::proxy_commands::clear_proxy_config_for_window,
            // wallet manager commands
            wallets_tool::wallet_manager::commands::init_wallet_manager_tables,
            wallets_tool::wallet_manager::commands::is_wallet_manager_initialized,
            wallets_tool::wallet_manager::commands::init_encrypted_db,
            wallets_tool::wallet_manager::commands::unlock_encrypted_db,
            wallets_tool::wallet_manager::commands::is_password_set,
            wallets_tool::wallet_manager::commands::init_password,
            wallets_tool::wallet_manager::commands::verify_password,
            wallets_tool::wallet_manager::commands::get_wallet_transport_public_key,
            wallets_tool::wallet_manager::commands::register_wallet_transport_key,
            wallets_tool::wallet_manager::commands::change_password,
            wallets_tool::wallet_manager::commands::create_group,
            wallets_tool::wallet_manager::commands::get_groups,
            wallets_tool::wallet_manager::commands::update_group,
            wallets_tool::wallet_manager::commands::delete_group,
            wallets_tool::wallet_manager::commands::create_wallet,
            wallets_tool::wallet_manager::commands::create_wallets,
            wallets_tool::wallet_manager::commands::get_wallets,
            wallets_tool::wallet_manager::commands::get_wallet_secrets,
            wallets_tool::wallet_manager::commands::export_wallets,
            wallets_tool::wallet_manager::commands::update_wallet,
            wallets_tool::wallet_manager::commands::delete_wallet,
            // watch address commands
            wallets_tool::wallet_manager::commands::get_watch_addresses,
            wallets_tool::wallet_manager::commands::create_watch_address,
            wallets_tool::wallet_manager::commands::create_watch_addresses,
            wallets_tool::wallet_manager::commands::update_watch_address,
            wallets_tool::wallet_manager::commands::delete_watch_address,
            wallets_tool::wallet_manager::commands::export_watch_addresses,
            // encrypted cloud backup commands
            wallets_tool::wallet_manager::commands::create_encrypted_backup,
            wallets_tool::wallet_manager::commands::restore_encrypted_backup,
            wallets_tool::wallet_manager::commands::save_backup_to_file,
            wallets_tool::wallet_manager::commands::load_backup_from_file,
            // browser automation commands
            wallets_tool::airdrop::commands::init_browser_automation_tables,
            wallets_tool::airdrop::commands::get_airdrop_wallets,
            wallets_tool::airdrop::commands::create_airdrop_wallet,
            wallets_tool::airdrop::commands::update_airdrop_wallet,
            wallets_tool::airdrop::commands::delete_airdrop_wallet,
            wallets_tool::airdrop::commands::import_airdrop_wallets,
            wallets_tool::airdrop::commands::get_wallet_private_key,
            wallets_tool::airdrop::commands::get_browser_profiles,
            wallets_tool::airdrop::commands::create_browser_profile,
            wallets_tool::airdrop::commands::update_browser_profile,
            wallets_tool::airdrop::commands::delete_browser_profile,
            wallets_tool::airdrop::commands::batch_generate_profiles,
            wallets_tool::airdrop::commands::get_automation_scripts,
            wallets_tool::airdrop::commands::create_automation_script,
            wallets_tool::airdrop::commands::update_automation_script,
            wallets_tool::airdrop::commands::delete_automation_script,
            wallets_tool::airdrop::commands::get_automation_tasks,
            wallets_tool::airdrop::commands::create_automation_task,
            wallets_tool::airdrop::commands::update_automation_task,
            wallets_tool::airdrop::commands::delete_automation_task,
            wallets_tool::airdrop::commands::toggle_task_status,
            wallets_tool::airdrop::commands::get_task_executions,
            wallets_tool::airdrop::commands::delete_task_execution,
            wallets_tool::airdrop::commands::get_task_execution_stats,
            wallets_tool::airdrop::commands::run_task_now,
            // execution commands
            wallets_tool::airdrop::commands::create_execution,
            wallets_tool::airdrop::commands::start_execution,
            wallets_tool::airdrop::commands::cancel_execution,
            wallets_tool::airdrop::commands::get_execution,
            wallets_tool::airdrop::commands::simulate_execution,
            // browser extension commands
            wallets_tool::airdrop::commands::get_browser_extensions,
            wallets_tool::airdrop::commands::create_browser_extension,
            wallets_tool::airdrop::commands::update_browser_extension,
            wallets_tool::airdrop::commands::delete_browser_extension,
            wallets_tool::airdrop::commands::toggle_browser_extension,
            wallets_tool::airdrop::commands::scan_extension_folder,
            wallets_tool::airdrop::commands::import_extension_from_folder,
            // playwright execution commands
            wallets_tool::playwright::execute_playwright_script,
            // playwright executor commands (optimized concurrent execution)
            wallets_tool::playwright::executor::playwright_create_session,
            wallets_tool::playwright::executor::playwright_start_execution,
            wallets_tool::playwright::executor::playwright_get_execution_status,
            wallets_tool::playwright::executor::playwright_cancel_execution,
            wallets_tool::playwright::executor::playwright_cleanup_session,
            // playwright recorder commands
            wallets_tool::playwright::recorder::playwright_start_recording,
            wallets_tool::playwright::recorder::playwright_stop_recording,
            wallets_tool::playwright::recorder::playwright_get_recording_session,
            wallets_tool::playwright::recorder::check_cli_tools,
            wallets_tool::playwright::recorder::install_node_environment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
