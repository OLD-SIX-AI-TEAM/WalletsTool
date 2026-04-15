//! 数据库加密工具

use anyhow::{Result, anyhow};
use pbkdf2::pbkdf2;
use sha2::Sha256;
use hmac::Hmac;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_PREFIX: &[u8] = b"WalletsTool_SecureDB_v2_";

pub fn derive_db_key(password: &str) -> Result<String> {
    let salt = derive_salt(password);
    
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(
        password.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
        &mut key,
    ).map_err(|_| anyhow!("Key derivation failed"))?;
    
    Ok(hex::encode(key))
}

fn derive_salt(password: &str) -> Vec<u8> {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(SALT_PREFIX);
    hasher.update(password.as_bytes());
    hasher.finalize()[..16].to_vec()
}

pub async fn create_encrypted_pool(db_path: &str, db_key: &str) -> Result<SqlitePool> {
    let database_url = format!("sqlite://{}", db_path);
    
    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true);
    
    let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
        .max_connections(5)
        .min_connections(1)
        .after_connect({
            let key = db_key.to_string();
            move |conn, _meta| {
                let key = key.clone();
                Box::pin(async move {
                    sqlx::query(&format!("PRAGMA key = '{}'", key.replace("'", "''")))
                        .execute(&mut *conn)
                        .await?;
                    
                    sqlx::query("PRAGMA cipher_page_size = 4096")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("PRAGMA kdf_iter = 600000")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("PRAGMA cipher_memory_security = ON")
                        .execute(&mut *conn)
                        .await?;
                    
                    sqlx::query("PRAGMA foreign_keys = ON")
                        .execute(&mut *conn)
                        .await?;
                    
                    Ok(())
                })
            }
        })
        .connect_with(options)
        .await?;
    
    sqlx::query("SELECT count(*) FROM sqlite_master")
        .fetch_one(&pool)
        .await
        .map_err(|_| anyhow!("数据库密码错误或文件损坏"))?;
    
    Ok(pool)
}

pub async fn create_unencrypted_pool(db_path: &str) -> Result<SqlitePool> {
    let database_url = format!("sqlite://{}", db_path);
    
    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true);
    
    let pool = SqlitePool::connect_with(options).await?;
    
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    
    Ok(pool)
}

#[allow(dead_code)]
pub async fn is_database_encrypted(db_path: &str) -> bool {
    use tokio::fs::read as async_read;
    use std::path::Path;
    
    if !Path::new(db_path).exists() {
        return false;
    }
    
    let file_header = async_read(db_path)
        .await
        .ok()
        .and_then(|bytes| bytes.get(0..16).map(|h| h.to_vec()))
        .unwrap_or_default();
    
    let header_str = String::from_utf8_lossy(&file_header);
    !header_str.starts_with("SQLite format 3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_db_key() {
        let key1 = derive_db_key("password123").unwrap();
        let key2 = derive_db_key("password123").unwrap();
        assert_eq!(key1, key2);
        
        let key3 = derive_db_key("different").unwrap();
        assert_ne!(key1, key3);
        
        assert_eq!(key1.len(), 64);
    }

    #[test]
    fn test_derive_salt() {
        let salt1 = derive_salt("test");
        let salt2 = derive_salt("test");
        assert_eq!(salt1, salt2);
        
        assert_eq!(salt1.len(), 16);
    }
}
