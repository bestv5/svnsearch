//! SVN 文件索引 SQLite 持久化
//! 按仓库 URL 分别存储，支持多配置的索引加载与清空。
//! 表结构 v1：url, path, name（文件名或目录名）, is_dir（0=文件 1=目录）。

use rusqlite::Connection;
use std::path::PathBuf;

const SCHEMA_VERSION: i32 = 1;

/// 从 path 计算名称与是否目录：目录以 / 结尾，name 为末段（去掉尾随 / 后取最后一段）
fn path_to_name_and_is_dir(path: &str) -> (String, bool) {
    let is_dir = path.ends_with('/');
    let p = path.trim_end_matches('/');
    let name = p.rsplit('/').next().unwrap_or(p).to_string();
    (name, is_dir)
}

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

/// 初始化表结构并执行迁移（user_version 0 → 1：增加 name, is_dir）
fn init_schema(conn: &Connection) -> Result<(), String> {
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("读取 user_version 失败: {}", e))?;

    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    // 检查是否存在旧表（仅 url, path 两列）
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name='file_index'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查表失败: {}", e))?;

    if table_exists {
        // 迁移：旧表重命名 → 建新表 → 拷贝并计算 name/is_dir → 删旧表
        conn.execute("ALTER TABLE file_index RENAME TO file_index_old", [])
            .map_err(|e| format!("重命名旧表失败: {}", e))?;
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_index (
            url TEXT NOT NULL,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            is_dir INTEGER NOT NULL,
            PRIMARY KEY (url, path)
        );
        CREATE INDEX IF NOT EXISTS idx_file_index_url ON file_index(url);
        CREATE INDEX IF NOT EXISTS idx_file_index_is_dir_name ON file_index(is_dir, name);
        CREATE INDEX IF NOT EXISTS idx_file_index_name ON file_index(name);
        "#,
    )
    .map_err(|e| format!("创建表失败: {}", e))?;

    if table_exists {
        let mut stmt = conn
            .prepare("SELECT url, path FROM file_index_old")
            .map_err(|e| format!("准备查询旧表失败: {}", e))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| format!("查询旧表失败: {}", e))?;
        let mut insert_stmt = conn
            .prepare("INSERT INTO file_index (url, path, name, is_dir) VALUES (?1, ?2, ?3, ?4)")
            .map_err(|e| format!("准备插入失败: {}", e))?;
        for row in rows {
            let (url, path): (String, String) = row.map_err(|e| format!("读取行失败: {}", e))?;
            let (name, is_dir) = path_to_name_and_is_dir(&path);
            let is_dir_int = if is_dir { 1i32 } else { 0i32 };
            insert_stmt
                .execute(rusqlite::params![&url, &path, &name, is_dir_int])
                .map_err(|e| format!("迁移插入失败: {}", e))?;
        }
        conn.execute("DROP TABLE file_index_old", [])
            .map_err(|e| format!("删除旧表失败: {}", e))?;
    }

    conn.execute(&format!("PRAGMA user_version = {}", SCHEMA_VERSION), [])
        .map_err(|e| format!("设置 user_version 失败: {}", e))?;
    Ok(())
}

/// 保存某仓库的索引（先删后插，原子性由事务保证）；每条为 path，目录以 / 结尾
pub fn save_index(url: &str, files: &[String]) -> Result<(), String> {
    let conn = open_db()?;
    init_schema(&conn)?;
    let tx = conn.unchecked_transaction().map_err(|e| format!("开启事务失败: {}", e))?;
    tx.execute("DELETE FROM file_index WHERE url = ?1", [url])
        .map_err(|e| format!("清空旧索引失败: {}", e))?;
    let mut stmt = tx
        .prepare("INSERT INTO file_index (url, path, name, is_dir) VALUES (?1, ?2, ?3, ?4)")
        .map_err(|e| format!("准备插入语句失败: {}", e))?;
    for path in files {
        let (name, is_dir) = path_to_name_and_is_dir(path);
        let is_dir_int = if is_dir { 1i32 } else { 0i32 };
        stmt.execute(rusqlite::params![url, path.as_str(), &name, is_dir_int])
            .map_err(|e| format!("插入失败: {}", e))?;
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

/// 搜索索引：按新语法解析 query，仅对名称匹配，返回 (url, path, is_dir, name_segments)
pub fn search_index(
    query: &str,
    limit: u32,
) -> Result<Vec<(String, String, bool, Vec<(String, bool)>)>, String> {
    let conn = open_db()?;
    init_schema(&conn)?;

    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    // 读取候选行（粗筛：可后续加 LIKE 预过滤以优化性能）
    const CANDIDATE_LIMIT: i64 = 10_000;
    let mut stmt = conn
        .prepare(
            "SELECT url, path, name, is_dir FROM file_index ORDER BY url, path LIMIT ?1",
        )
        .map_err(|e| format!("准备搜索语句失败: {}", e))?;
    let rows = stmt
        .query_map([CANDIDATE_LIMIT], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })
        .map_err(|e| format!("搜索失败: {}", e))?;

    let mut out = Vec::new();
    for row in rows {
        let (url, path, name, is_dir_int): (String, String, String, i32) =
            row.map_err(|e| format!("读取行失败: {}", e))?;
        let is_dir = is_dir_int != 0;
        let (matched, ranges) =
            crate::search_query::parse_and_match(query, &name, is_dir, None)
                .map_err(|e| format!("匹配失败: {}", e))?;
        if matched {
            let segments = crate::search_query::ranges_to_segments(&name, &ranges);
            out.push((url, path, is_dir, segments));
            if out.len() >= limit as usize {
                break;
            }
        }
    }
    Ok(out)
}
