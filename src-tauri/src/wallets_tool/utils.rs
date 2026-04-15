use std::path::{Path, PathBuf};
use tauri::{command, Manager};
use crate::database::chain_service::ChainService;
use crate::database::get_database_pool;
use base64::{Engine as _, engine::general_purpose};
use tauri::State;
use sqlx;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
// 下载文件到指定路径
#[command]
pub fn download_file(_url: &str, _file_path: &Path) {
    // let client = ClientBuilder::new().build().unwrap();
    // // 请求文件
    // let response = client.send(
    //     HttpRequestBuilder::new("GET", url)
    //         .unwrap()
    //         .response_type(ResponseType::Binary)
    // ).await;
    // // 写入文件
    // return if let Ok(mut response) = response {
    //     response::write_to_file(&mut response, file_path)?;
    //     Ok(())
    // } else {
    //     Err("获取文件失败！".into())
    // }
}

// 保存链图标数据到数据库
#[command]
pub async fn save_chain_icon(
    chain_key: String,
    file_name: String, 
    file_data: Vec<u8>,
    chain_service: State<'_, ChainService>
) -> Result<String, String> {
    // 将文件数据转换为Base64编码
    let base64_data = general_purpose::STANDARD.encode(&file_data);
    
    // 检查链是否存在
    let chain = chain_service.get_chain_by_key(&chain_key).await
        .map_err(|e| format!("获取链信息失败: {e}"))?;
    
    if let Some(chain) = chain {
        // 链已存在，更新图标数据
        let pool = get_database_pool();
        sqlx::query(
            "UPDATE chains SET pic_data = ?, pic_url = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&base64_data)
        .bind(&file_name)
        .bind(chrono::Utc::now())
        .bind(chain.id)
        .execute(&pool)
        .await
        .map_err(|e| format!("更新图标数据失败: {e}"))?;
        
        println!("图标数据更新成功: {chain_key} -> {file_name}");
    } else {
        // 链不存在，这是新增链的情况，直接返回base64数据供前端使用
        println!("为新增链准备图标数据: {chain_key} -> {file_name}");
    }
    
    Ok(base64_data)
}

// 获取链图标数据
#[command]
pub async fn get_chain_icon(
    chain_key: String,
    chain_service: State<'_, ChainService>
) -> Result<Option<String>, String> {
    let chain = chain_service.get_chain_by_key(&chain_key).await
        .map_err(|e| format!("获取链信息失败: {e}"))?;
    
    match chain {
        Some(chain) => Ok(chain.pic_data),
        None => Ok(None)
    }
}

#[command]
pub async fn read_resource_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    relative_path: String
) -> Result<Vec<u8>, String> {
    // 尝试从 Tauri 资源目录读取（打包后的应用）
    let resource_path = app.path()
        .resource_dir()
        .map_err(|e| format!("获取资源目录失败: {e}"))?
        .join("template")
        .join(&relative_path);

    if resource_path.exists() {
        return std::fs::read(&resource_path)
            .map_err(|e| format!("读取资源文件失败: {e}"));
    }

    // 开发环境回退：从 public 目录读取
    let dev_path = PathBuf::from("..")
        .join("public")
        .join("template")
        .join(&relative_path);

    std::fs::read(&dev_path)
        .map_err(|e| format!("读取资源文件失败: {e} (尝试路径: {:?}, {:?})", resource_path, dev_path))
}

#[command]
pub async fn save_file(file_path: String, content: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = PathBuf::from(&file_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {e}"))?;
    }

    std::fs::write(&file_path, content)
        .map_err(|e| format!("保存文件失败: {e}"))?;

    Ok(())
}

#[command]
pub fn get_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

#[command]
pub fn open_file_directory(file_path: String) {
    let path = PathBuf::from(&file_path);
    if let Some(_parent) = path.parent() {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .args(["/select,", &file_path])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", &file_path])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(parent) = path.parent() {
                std::process::Command::new("xdg-open")
                    .arg(parent)
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
        }
    }
}