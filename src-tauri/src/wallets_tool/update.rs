use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::{timeout, Duration};

// 国内可用的 GitHub 代理镜像列表
const GH_PROXY_MIRRORS: &[&str] = &[
    "https://gh-proxy.com/",
    "https://ghproxy.net/",
    "https://gh-proxy.org/",
];

// 单个请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 3;
// 总超时时间（秒）
const TOTAL_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Deserialize, Clone)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GithubReleaseUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub html_url: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub published_at: Option<String>,
    pub prerelease: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateDownloadProgress {
    pub status: String,
    pub progress: Option<u64>,
    pub total: Option<u64>,
    pub message: String,
}

/// 将 GitHub URL 转换为代理 URL（使用第一个可用镜像）
fn to_gh_proxy_url(input: &str) -> String {
    let url = input.trim();
    if url.is_empty() {
        return String::new();
    }
    // 检查是否已经是代理 URL
    for mirror in GH_PROXY_MIRRORS {
        if url.starts_with(*mirror) {
            return url.to_string();
        }
    }
    // 如果是 GitHub URL，添加代理前缀
    if url.starts_with("https://github.com/")
        || url.starts_with("https://api.github.com/")
        || url.starts_with("https://raw.githubusercontent.com/")
    {
        return format!("{}{}", GH_PROXY_MIRRORS[0], url);
    }
    url.to_string()
}

/// 获取所有代理 URL 变体（用于轮询）
fn get_proxy_url_variants(input: &str) -> Vec<String> {
    let url = input.trim();
    if url.is_empty() {
        return vec![];
    }
    
    // 检查是否已经是代理 URL
    for mirror in GH_PROXY_MIRRORS {
        if url.starts_with(*mirror) {
            return vec![url.to_string()];
        }
    }
    
    // 如果是 GitHub URL，生成所有代理变体
    if url.starts_with("https://github.com/")
        || url.starts_with("https://api.github.com/")
        || url.starts_with("https://raw.githubusercontent.com/")
    {
        let mut variants = vec![];
        for mirror in GH_PROXY_MIRRORS {
            variants.push(format!("{}{}", mirror, url));
        }
        // 最后添加原始 URL 作为备选
        variants.push(url.to_string());
        return variants;
    }
    
    vec![url.to_string()]
}

async fn fetch_github_release(
    client: &reqwest::Client,
    url: &str,
) -> Result<GitHubRelease, String> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub Release 失败: {e}"))?;

    let response = response
        .error_for_status()
        .map_err(|e| format!("请求 GitHub Release 失败: {e}"))?;

    response
        .json()
        .await
        .map_err(|e| format!("解析 GitHub Release 失败: {e}"))
}

fn parse_semver_triplet(input: &str) -> Result<(u32, u32, u32), String> {
    let trimmed = input.trim().trim_start_matches('v');
    let core = trimmed
        .split_once('-')
        .map(|(left, _)| left)
        .unwrap_or(trimmed);

    let mut parts = core.split('.');
    let major = parts
        .next()
        .ok_or_else(|| format!("无法解析版本号: {input}"))?
        .parse::<u32>()
        .map_err(|_| format!("无法解析版本号: {input}"))?;
    let minor = parts
        .next()
        .unwrap_or("0")
        .parse::<u32>()
        .map_err(|_| format!("无法解析版本号: {input}"))?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .parse::<u32>()
        .map_err(|_| format!("无法解析版本号: {input}"))?;
    Ok((major, minor, patch))
}

fn is_newer_version(current: &str, latest: &str) -> Result<bool, String> {
    let current_triplet = parse_semver_triplet(current)?;
    let latest_triplet = parse_semver_triplet(latest)?;
    Ok(latest_triplet > current_triplet)
}

/// 并行获取 GitHub Release，返回最快成功的结果
async fn fetch_github_release_parallel(
    client: &reqwest::Client,
    urls: Vec<String>,
) -> Result<(GitHubRelease, usize), String> {
    println!("[fetch_github_release_parallel] 并行查询 {} 个端点", urls.len());

    let futures: Vec<_> = urls
        .iter()
        .enumerate()
        .map(|(idx, url)| {
            let client = client.clone();
            let url = url.clone();
            async move {
                println!("[fetch_github_release_parallel] 开始查询端点 {}: {}", idx + 1, url);
                let start = std::time::Instant::now();
                
                let result = timeout(
                    Duration::from_secs(REQUEST_TIMEOUT_SECS),
                    fetch_github_release(&client, &url)
                ).await;
                
                let elapsed = start.elapsed();
                
                match result {
                    Ok(Ok(release)) => {
                        println!(
                            "[fetch_github_release_parallel] 端点 {} 成功，耗时 {:?}",
                            idx + 1,
                            elapsed
                        );
                        Ok((release, idx))
                    }
                    Ok(Err(e)) => {
                        println!(
                            "[fetch_github_release_parallel] 端点 {} 失败: {}，耗时 {:?}",
                            idx + 1,
                            e,
                            elapsed
                        );
                        Err(format!("端点 {}: {}", idx + 1, e))
                    }
                    Err(_) => {
                        println!(
                            "[fetch_github_release_parallel] 端点 {} 超时，耗时 {:?}",
                            idx + 1,
                            elapsed
                        );
                        Err(format!("端点 {} 超时", idx + 1))
                    }
                }
            }
        })
        .collect();

    // 使用 race 模式，获取最快成功的结果
    let results = join_all(futures).await;
    
    // 找到第一个成功的结果
    for result in results.iter() {
        if let Ok((release, idx)) = result {
            println!(
                "[fetch_github_release_parallel] 使用端点 {} 的结果",
                idx + 1
            );
            return Ok((release.clone(), *idx));
        }
    }

    // 所有请求都失败
    let errors: Vec<String> = results
        .into_iter()
        .filter_map(|r| r.err())
        .collect();
    Err(format!(
        "所有更新检查端点均失败: {}",
        errors.join("; ")
    ))
}

#[command]
pub async fn check_github_release_update(
    owner: Option<String>,
    repo: Option<String>,
    current_version: String,
) -> Result<Option<GithubReleaseUpdateInfo>, String> {
    println!("[check_github_release_update] 开始检查更新, current_version: {}", current_version);
    
    let owner = owner.unwrap_or_else(|| "WalletsTool".to_string());
    let repo = repo.unwrap_or_else(|| "WalletsTool".to_string());

    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let url_variants = get_proxy_url_variants(&url);

    let client = reqwest::Client::builder()
        .user_agent(format!("WalletsTool/{current_version}"))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;

    // 并行查询所有代理镜像，取最快成功的结果
    let (release, success_idx) = match timeout(
        Duration::from_secs(TOTAL_TIMEOUT_SECS),
        fetch_github_release_parallel(&client, url_variants)
    ).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("检查更新超时，请检查网络连接".to_string()),
    };

    println!("[check_github_release_update] 获取到 release 信息");

    if release.draft {
        println!("[check_github_release_update] 该 release 是草稿，跳过");
        return Ok(None);
    }

    let latest_version = release.tag_name.trim().trim_start_matches('v').to_string();
    let has_update = is_newer_version(&current_version, &latest_version)?;

    println!(
        "[check_github_release_update] 版本比较: {} -> {}, has_update: {}",
        current_version,
        latest_version,
        has_update
    );

    if !has_update {
        return Ok(None);
    }

    // 判断是否使用了代理（只要不是最后一个原始URL，就是用了代理）
    let used_proxy = success_idx < GH_PROXY_MIRRORS.len();
    let html_url = if used_proxy {
        to_gh_proxy_url(&release.html_url)
    } else {
        release.html_url
    };

    Ok(Some(GithubReleaseUpdateInfo {
        current_version,
        latest_version,
        html_url,
        name: release.name,
        body: release.body,
        published_at: release.published_at,
        prerelease: release.prerelease,
    }))
}

/// 使用 Tauri Updater 检查更新
#[command]
pub async fn check_update<R: Runtime>(
    app: AppHandle<R>,
    current_version: String,
) -> Result<UpdateCheckResult, String> {
    println!("[check_update] 开始检查更新, current_version: {}", current_version);
    
    println!("[check_update] 尝试获取 updater...");
    
    let updater = app
        .updater()
        .map_err(|e| {
            let err_msg = format!("获取更新器失败: {e}");
            println!("[check_update] 获取更新器错误: {}", err_msg);
            err_msg
        })?;

    println!("[check_update] 获取到 updater, 准备检查更新...");

    println!("[check_update] 调用 updater.check() 进行网络请求...");
    
    match updater.check().await {
        Ok(Some(update)) => {
            let latest_version = update.version.clone();
            let release_notes = update.body.clone();
            let download_url = update.download_url.to_string();
            
            println!("[check_update] 发现新版本: {} -> {}", current_version, latest_version);
            
            Ok(UpdateCheckResult {
                has_update: true,
                current_version,
                latest_version,
                release_notes,
                download_url: Some(download_url),
                published_at: update.date.map(|d| d.to_string()),
            })
        }
        Ok(None) => {
            println!("[check_update] 当前已是最新版本: {}", current_version);
            Ok(UpdateCheckResult {
                has_update: false,
                latest_version: current_version.clone(),
                current_version,
                release_notes: None,
                download_url: None,
                published_at: None,
            })
        }
        Err(e) => {
            let err_msg = format!("检查更新失败: {e}");
            println!("[check_update] 网络请求失败: {}", err_msg);
            Err(err_msg)
        }
    }
}

/// 下载并安装更新
#[command]
pub async fn download_and_install_update<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("获取更新器失败: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            // 下载更新
            let bytes = update
                .download(|_chunk_length, _content_length| {
                    // 可以在这里发送进度事件到前端
                }, || {
                    // 下载完成回调
                })
                .await
                .map_err(|e| format!("下载更新失败: {e}"))?;

            // 安装更新
            update
                .install(bytes)
                .map_err(|e| format!("安装更新失败: {e}"))?;

            Ok("更新下载完成，即将重启应用".to_string())
        }
        Ok(None) => Err("没有可用的更新".to_string()),
        Err(e) => Err(format!("检查更新失败: {e}")),
    }
}

/// 仅下载更新，不安装
#[command]
pub async fn download_update_only<R: Runtime>(
    app: AppHandle<R>,
) -> Result<String, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("获取更新器失败: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => {
            // 下载更新
            let _bytes = update
                .download(|chunk_length, content_length| {
                    let progress = if let Some(total) = content_length {
                        format!("下载进度: {} / {} bytes", chunk_length, total)
                    } else {
                        format!("已下载: {} bytes", chunk_length)
                    };
                    println!("{}", progress);
                }, || {
                    println!("下载完成");
                })
                .await
                .map_err(|e| format!("下载更新失败: {e}"))?;

            Ok(format!("更新 v{} 下载完成", update.version))
        }
        Ok(None) => Err("没有可用的更新".to_string()),
        Err(e) => Err(format!("检查更新失败: {e}")),
    }
}
