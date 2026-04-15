use anyhow::Result;
use sqlx::SqlitePool;

use super::dual_database::{DualDatabaseManager, PUBLIC_DB_PATH};
use super::encryption::create_unencrypted_pool;

static PUBLIC_INIT_SQL: &str = include_str!("../../data/public_init.sql");

pub async fn init_public_database() -> Result<()> {
    std::fs::create_dir_all("data")?;

    let pool = create_unencrypted_pool(PUBLIC_DB_PATH).await?;

    // 执行 public_init.sql 创建表结构并插入初始数据
    execute_init_sql(&pool, PUBLIC_INIT_SQL).await?;

    // 保留向后兼容性调用（现在 public_init.sql 已包含所有数据）
    insert_default_chain_data(&pool).await?;

    DualDatabaseManager::init_public_pool(pool);
    DualDatabaseManager::init_secure_pool_placeholder();

    Ok(())
}

/// 插入默认链和 RPC 数据
/// 注意：现在完全由 public_init.sql 提供，此函数保留为空以保持向后兼容
async fn insert_default_chain_data(_pool: &SqlitePool) -> Result<()> {
    // public_init.sql 已包含所有默认链和 RPC 数据
    // 此函数保留为空，避免重复插入导致的数据不一致
    Ok(())
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
    fn test_parse_sql_statements() {
        let sql = "CREATE TABLE a (id INT); INSERT INTO a VALUES (1);";
        let stmts = parse_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn test_parse_sql_with_comments() {
        let sql = "-- comment\nCREATE TABLE a (id INT);";
        let stmts = parse_sql_statements(sql);
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("CREATE TABLE"));
    }
}
