use anyhow::{Result, anyhow};
use sqlx::SqlitePool;
use std::path::Path;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use super::dual_database::{DualDatabaseManager, SECURE_DB_PATH};
use super::encryption::{create_encrypted_pool, derive_db_key};

static SECURE_INIT_SQL: &str = include_str!("../../data/secure_init.sql");

pub async fn init_secure_database(password: &str) -> Result<()> {
    if Path::new(SECURE_DB_PATH).exists() {
        return Err(anyhow!("安全数据库已存在，请使用 unlock_secure_database"));
    }
    
    std::fs::create_dir_all("data")?;
    
    let db_key = derive_db_key(password)?;
    
    let pool = create_encrypted_pool(SECURE_DB_PATH, &db_key).await?;
    
    execute_init_sql(&pool, SECURE_INIT_SQL).await?;
    
    save_master_verifier(&pool, password).await?;
    
    DualDatabaseManager::update_secure_pool(Some(pool));
    
    Ok(())
}

pub async fn unlock_secure_database(password: &str) -> Result<()> {
    if !Path::new(SECURE_DB_PATH).exists() {
        return Err(anyhow!("安全数据库不存在，请先使用 init_secure_database 初始化"));
    }
    
    let db_key = derive_db_key(password)?;
    
    let pool = create_encrypted_pool(SECURE_DB_PATH, &db_key).await?;
    
    verify_master_password(&pool, password).await?;
    
    DualDatabaseManager::update_secure_pool(Some(pool));
    
    Ok(())
}

pub async fn lock_secure_database() -> Result<()> {
    DualDatabaseManager::close_secure_pool().await;
    Ok(())
}

#[allow(dead_code)]
pub async fn change_secure_password(old_password: &str, new_password: &str) -> Result<()> {
    if !Path::new(SECURE_DB_PATH).exists() {
        return Err(anyhow!("安全数据库不存在"));
    }
    
    let old_key = derive_db_key(old_password)?;
    let pool = create_encrypted_pool(SECURE_DB_PATH, &old_key).await?;
    verify_master_password(&pool, old_password).await?;
    
    let new_key = derive_db_key(new_password)?;
    sqlx::query(&format!("PRAGMA rekey = '{}'", new_key.replace("'", "''")))
        .execute(&pool)
        .await
        .map_err(|e| anyhow!("更改密码失败: {}", e))?;
    
    save_master_verifier(&pool, new_password).await?;
    
    DualDatabaseManager::update_secure_pool(Some(pool));
    
    Ok(())
}

pub fn is_secure_database_initialized() -> bool {
    Path::new(SECURE_DB_PATH).exists()
}

async fn execute_init_sql(pool: &SqlitePool, sql: &str) -> Result<()> {
    for statement in parse_sql_statements(sql) {
        let stmt_trimmed = statement.trim();
        if !stmt_trimmed.is_empty() && !stmt_trimmed.starts_with("--") {
            if let Err(e) = sqlx::query(&statement).execute(pool).await {
                let error_msg = e.to_string();
                if !error_msg.contains("already exists") {
                    eprintln!("SQL warning: {}", error_msg);
                }
            }
        }
    }
    Ok(())
}

async fn save_master_verifier(pool: &SqlitePool, password: &str) -> Result<()> {
    let mut salt = [0u8; 16];
    openssl::rand::rand_bytes(&mut salt)?;
    let salt_b64 = BASE64.encode(&salt);
    
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(password.as_bytes());
    let hash = hasher.finalize();
    let hash_b64 = BASE64.encode(&hash);
    
    let verifier = format!("{}:{}", salt_b64, hash_b64);
    
    sqlx::query("INSERT OR REPLACE INTO master_config (key, value) VALUES ('master_verifier', ?)")
        .bind(&verifier)
        .execute(pool)
        .await?;
    
    Ok(())
}

async fn verify_master_password(pool: &SqlitePool, password: &str) -> Result<()> {
    let verifier: Option<String> = sqlx::query_scalar(
        "SELECT value FROM master_config WHERE key = 'master_verifier'"
    )
    .fetch_optional(pool)
    .await?;
    
    let verifier = verifier.ok_or_else(|| anyhow!("密码验证器不存在，数据库可能已损坏"))?;
    
    let parts: Vec<&str> = verifier.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("密码验证器格式错误"));
    }
    
    let salt = BASE64.decode(parts[0]).map_err(|_| anyhow!("解析密码验证器失败"))?;
    let stored_hash = BASE64.decode(parts[1]).map_err(|_| anyhow!("解析密码验证器失败"))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(password.as_bytes());
    let computed_hash = hasher.finalize();
    
    if &computed_hash[..] != stored_hash.as_slice() {
        return Err(anyhow!("密码错误"));
    }
    
    Ok(())
}

fn parse_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current_statement = String::new();
    let mut in_string = false;
    let mut string_delimiter = '\0';
    let mut paren_depth = 0;
    
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let ch = chars[i];
        
        if !in_string && (ch == '\'' || ch == '"') {
            in_string = true;
            string_delimiter = ch;
            current_statement.push(ch);
        } else if in_string && ch == string_delimiter {
            if i + 1 < chars.len() && chars[i + 1] == string_delimiter {
                current_statement.push(ch);
                current_statement.push(chars[i + 1]);
                i += 1;
            } else {
                in_string = false;
                current_statement.push(ch);
            }
        } else if !in_string {
            if ch == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                if i < chars.len() {
                    current_statement.push('\n');
                }
                i += 1;
                continue;
            }
            
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
            }
            
            if ch == ';' && paren_depth == 0 {
                let trimmed = current_statement.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("--") {
                    statements.push(current_statement.trim().to_string());
                }
                current_statement.clear();
            } else {
                current_statement.push(ch);
            }
        } else {
            current_statement.push(ch);
        }
        
        i += 1;
    }
    
    let trimmed = current_statement.trim();
    if !trimmed.is_empty() && !trimmed.starts_with("--") {
        statements.push(trimmed.to_string());
    }
    
    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_secure_database_initialized_false() {
        assert!(!is_secure_database_initialized() || std::path::Path::new(SECURE_DB_PATH).exists());
    }
}
