//! SVN 文件索引 SQLite 持久化
//! 按仓库 URL 分别存储，支持多配置的索引加载与清空。

use rusqlite::Connection;
use std::path::PathBuf;

/// 获取数据库文件路径（位于用户本地数据目录 / svnsearch / index.db）
fn get_db_path() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir().ok_or_else(|| "无法获取应用数据目录".to_string())?;
    let dir = base.join("svnsearch");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    Ok(dir.join("index.db"))
}

/// 打开数据库连接
fn open_db() -> Result<Connection, String> {
    let path = get_db_path()?;
    Connection::open(&path).map_err(|e| format!("打开数据库失败: {}", e))
}

/// 初始化表结构
fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_index (
            url TEXT NOT NULL,
            path TEXT NOT NULL,
            PRIMARY KEY (url, path)
        );
        CREATE INDEX IF NOT EXISTS idx_file_index_url ON file_index(url);
        "#,
    )
    .map_err(|e| format!("初始化表失败: {}", e))?;
    Ok(())
}

/// 保存某仓库的索引（先删后插，原子性由事务保证）
pub fn save_index(url: &str, files: &[String]) -> Result<(), String> {
    let conn = open_db()?;
    init_schema(&conn)?;
    let tx = conn.unchecked_transaction().map_err(|e| format!("开启事务失败: {}", e))?;
    tx.execute("DELETE FROM file_index WHERE url = ?1", [url])
        .map_err(|e| format!("清空旧索引失败: {}", e))?;
    let mut stmt = tx
        .prepare("INSERT INTO file_index (url, path) VALUES (?1, ?2)")
        .map_err(|e| format!("准备插入语句失败: {}", e))?;
    for path in files {
        stmt.execute([url, path]).map_err(|e| format!("插入失败: {}", e))?;
    }
    drop(stmt);
    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(())
}

/// 加载某仓库的索引
pub fn load_index(url: &str) -> Result<Vec<String>, String> {
    let conn = open_db()?;
    init_schema(&conn)?;
    let mut stmt = conn
        .prepare("SELECT path FROM file_index WHERE url = ?1 ORDER BY path")
        .map_err(|e| format!("准备查询失败: {}", e))?;
    let rows = stmt
        .query_map([url], |row| row.get::<_, String>(0))
        .map_err(|e| format!("查询失败: {}", e))?;
    let mut paths = Vec::new();
    for row in rows {
        paths.push(row.map_err(|e| format!("读取行失败: {}", e))?);
    }
    Ok(paths)
}

/// 清空某仓库的索引
pub fn clear_index(url: &str) -> Result<(), String> {
    let conn = open_db()?;
    init_schema(&conn)?;
    conn.execute("DELETE FROM file_index WHERE url = ?1", [url])
        .map_err(|e| format!("清空索引失败: {}", e))?;
    Ok(())
}

/// 转义 LIKE 中的通配符，防止注入与错误匹配（ESCAPE '\'）
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            _ => out.push(c),
        }
    }
    out
}

/// 模糊搜索：path 包含关键词即命中，返回 (url, path) 列表
pub fn search_index(query: &str, limit: u32) -> Result<Vec<(String, String)>, String> {
    let conn = open_db()?;
    init_schema(&conn)?;

    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let pattern = format!("%{}%", escape_like(query));
    let mut stmt = conn
        .prepare("SELECT url, path FROM file_index WHERE path LIKE ?1 ESCAPE '\\' ORDER BY url, path LIMIT ?2")
        .map_err(|e| format!("准备搜索语句失败: {}", e))?;
    let limit_i: i64 = limit as i64;
    let rows = stmt
        .query_map(rusqlite::params![&pattern, limit_i], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("搜索失败: {}", e))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("读取行失败: {}", e))?);
    }
    Ok(out)
}
