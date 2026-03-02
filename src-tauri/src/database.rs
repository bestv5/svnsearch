//! SVN 文件索引 SQLite 持久化
//! 按仓库 URL 分别存储，支持多配置的索引加载与清空。
//! 表结构 v1：url, path, name（文件名或目录名）, is_dir（0=文件 1=目录）。

use rusqlite::types::Value;
use rusqlite::Connection;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// 从 path 计算名称与是否目录：目录以 / 结尾，name 为末段（去掉尾随 / 后取最后一段）
fn path_to_name_and_is_dir(path: &str) -> (String, bool) {
    let is_dir = path.ends_with('/');
    let p = path.trim_end_matches('/');
    let name = p.rsplit('/').next().unwrap_or(p).to_string();
    (name, is_dir)
}

fn fold_name_for_db(name: &str) -> String {
    // 与 search_query 默认行为保持一致：全 Unicode 小写折叠
    name.to_lowercase()
}

fn fold_name_ascii_for_db(name: &str) -> String {
    // 与 search_query 的 ascii: 修饰符保持一致：仅 ASCII 字母小写
    name.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect()
}

/// 获取数据库文件路径（位于用户本地数据目录 / svnsearch / index.db）
fn get_db_path() -> Result<PathBuf, String> {
    let base = dirs::data_local_dir().ok_or_else(|| "无法获取应用数据目录".to_string())?;
    let dir = base.join("svnsearch");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
    Ok(dir.join("index.db"))
}

/// 追加调试日志到本地文件（最佳努力，失败时静默忽略）
fn append_debug_log(line: &str) {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("svnsearch");
        let _ = std::fs::create_dir_all(&dir);
        let log_path = dir.join("svnsearch-debug.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// 打开数据库连接
fn open_db() -> Result<Connection, String> {
    let path = get_db_path()?;
    Connection::open(&path).map_err(|e| format!("打开数据库失败: {}", e))
}

/// 初始化表结构（不做历史迁移，假定用户可删除数据库重建索引）
fn init_schema(conn: &Connection) -> Result<(), String> {
    // 主表：file_index
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_index (
            url TEXT NOT NULL,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            is_dir INTEGER NOT NULL,
            name_fold TEXT NOT NULL,
            name_ascii_fold TEXT NOT NULL,
            PRIMARY KEY (url, path)
        );
        CREATE INDEX IF NOT EXISTS idx_file_index_url ON file_index(url);
        CREATE INDEX IF NOT EXISTS idx_file_index_is_dir_name_fold ON file_index(is_dir, name_fold);
        CREATE INDEX IF NOT EXISTS idx_file_index_name_fold ON file_index(name_fold);
        "#,
    )
    .map_err(|e| format!("创建表失败: {}", e))?;

    // FTS5 虚表：file_index_fts
    let fts_exists: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type='table' AND name='file_index_fts'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查 FTS 表失败: {}", e))?;

    if !fts_exists {
        conn.execute(
            "CREATE VIRTUAL TABLE file_index_fts USING fts5(url, path, name, is_dir, tokenize = 'unicode61');",
            [],
        )
        .map_err(|e| format!("创建 FTS5 表失败: {}", e))?;
    }

    Ok(())
}

/// 保存某仓库的索引（先删后插，原子性由事务保证）；每条为 path，目录以 / 结尾
pub fn save_index(url: &str, files: &[String]) -> Result<(), String> {
    let conn = open_db()?;
    init_schema(&conn)?;
    let tx = conn.unchecked_transaction().map_err(|e| format!("开启事务失败: {}", e))?;
    tx.execute("DELETE FROM file_index WHERE url = ?1", [url])
        .map_err(|e| format!("清空旧索引失败: {}", e))?;
    tx.execute("DELETE FROM file_index_fts WHERE url = ?1", [url])
        .map_err(|e| format!("清空旧 FTS 索引失败: {}", e))?;
    let mut stmt = tx
        .prepare("INSERT INTO file_index (url, path, name, is_dir, name_fold, name_ascii_fold) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .map_err(|e| format!("准备插入语句失败: {}", e))?;
    let mut fts_stmt = tx
        .prepare("INSERT INTO file_index_fts (url, path, name, is_dir) VALUES (?1, ?2, ?3, ?4)")
        .map_err(|e| format!("准备 FTS 插入语句失败: {}", e))?;
    for path in files {
        let (name, is_dir) = path_to_name_and_is_dir(path);
        let is_dir_int = if is_dir { 1i32 } else { 0i32 };
        let name_fold = fold_name_for_db(&name);
        let name_ascii_fold = fold_name_ascii_for_db(&name);
        stmt.execute(rusqlite::params![url, path.as_str(), &name, is_dir_int, &name_fold, &name_ascii_fold])
            .map_err(|e| format!("插入失败: {}", e))?;
        fts_stmt
            .execute(rusqlite::params![url, path.as_str(), &name, is_dir_int])
            .map_err(|e| format!("FTS 插入失败: {}", e))?;
    }
    drop(stmt);
    drop(fts_stmt);
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
    conn.execute("DELETE FROM file_index_fts WHERE url = ?1", [url])
        .map_err(|e| format!("清空 FTS 索引失败: {}", e))?;
    Ok(())
}

fn like_escape_literal(s: &str) -> String {
    // 供 LIKE ... ESCAPE '\' 使用：转义 \、% 、_
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            _ => out.push(ch),
        }
    }
    out
}

fn glob_to_like_pattern(glob: &str) -> String {
    // 将 * ? 转成 % _，其余按 LIKE 字面量处理（并转义 % _ \）
    let mut out = String::new();
    for ch in glob.chars() {
        match ch {
            '*' => out.push('%'),
            '?' => out.push('_'),
            _ => out.push_str(&like_escape_literal(&ch.to_string())),
        }
    }
    out
}

fn expr_has_glob(expr: &crate::search_query::Expr) -> bool {
    use crate::search_query::Expr;
    match expr {
        Expr::Or(children) | Expr::And(children) => children.iter().any(expr_has_glob),
        Expr::Not(inner) => expr_has_glob(inner),
        Expr::Term(t) | Expr::Phrase(t) => t.contains('*') || t.contains('?'),
    }
}

fn match_config_fts_compatible(config: &crate::search_query::MatchConfig) -> bool {
    // FTS5 使用 unicode61 分词器，默认大小写不敏感且会做一定的规格化。
    // 目前仅在「不区分大小写、不做 ASCII-only 折叠、不要求变音符敏感」的默认配置下走 FTS。
    !config.case_sensitive && !config.ascii_fold_only && !config.diacritics_sensitive
}

fn escape_fts_phrase(s: &str) -> String {
    // FTS 短语中双引号需要转义为两个双引号
    s.replace('"', "\"\"")
}

fn escape_fts_token(s: &str) -> String {
    // Term 默认不会包含空白和操作符，这里只做最基础的双引号转义，必要时可扩展。
    s.replace('"', "\"\"")
}

fn build_fts_match_from_expr(expr: &crate::search_query::Expr) -> String {
    use crate::search_query::Expr;
    match expr {
        Expr::Or(children) => {
            let parts: Vec<String> = children.iter().map(build_fts_match_from_expr).collect();
            if parts.len() == 1 {
                parts[0].clone()
            } else {
                format!("({})", parts.join(" OR "))
            }
        }
        Expr::And(children) => {
            let parts: Vec<String> = children.iter().map(build_fts_match_from_expr).collect();
            if parts.len() == 1 {
                parts[0].clone()
            } else {
                format!("({})", parts.join(" AND "))
            }
        }
        Expr::Not(inner) => format!("NOT ({})", build_fts_match_from_expr(inner)),
        Expr::Term(t) => escape_fts_token(t),
        Expr::Phrase(p) => format!("\"{}\"", escape_fts_phrase(p)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_has_glob_detection() {
        let parsed = crate::search_query::parse("foo bar").unwrap();
        let expr = parsed.expr.as_ref().unwrap();
        assert!(!expr_has_glob(expr));

        let parsed_glob = crate::search_query::parse("foo* bar").unwrap();
        let expr_glob = parsed_glob.expr.as_ref().unwrap();
        assert!(expr_has_glob(expr_glob));
    }

    #[test]
    fn test_build_fts_match_basic_logic() {
        let parsed = crate::search_query::parse("foo bar|!baz \"hello world\"").unwrap();
        let expr = parsed.expr.as_ref().unwrap();
        let fts = build_fts_match_from_expr(expr);
        // 基本结构校验：包含 AND / OR / NOT 与短语
        assert!(fts.contains("AND"));
        assert!(fts.contains("OR"));
        assert!(fts.contains("NOT"));
        assert!(fts.contains("\"hello world\""));
    }
}

fn build_sql_where_from_expr(
    expr: &crate::search_query::Expr,
    name_col: &str,
    params: &mut Vec<Value>,
    config: &crate::search_query::MatchConfig,
) -> Result<String, String> {
    use crate::search_query::Expr;
    match expr {
        Expr::Or(children) => {
            let mut parts = Vec::new();
            for c in children {
                parts.push(build_sql_where_from_expr(c, name_col, params, config)?);
            }
            Ok(format!("({})", parts.join(" OR ")))
        }
        Expr::And(children) => {
            let mut parts = Vec::new();
            for c in children {
                parts.push(build_sql_where_from_expr(c, name_col, params, config)?);
            }
            Ok(format!("({})", parts.join(" AND ")))
        }
        Expr::Not(inner) => Ok(format!(
            "(NOT {})",
            build_sql_where_from_expr(inner, name_col, params, config)?
        )),
        Expr::Term(t) | Expr::Phrase(t) => {
            if t.is_empty() {
                return Ok("(1=1)".to_string());
            }
            let folded = if config.case_sensitive {
                t.clone()
            } else if config.ascii_fold_only {
                fold_name_ascii_for_db(t)
            } else {
                fold_name_for_db(t)
            };
            let like_pat = if t.contains('*') || t.contains('?') {
                // 原逻辑的 glob 是“全串匹配”，但这里用 LIKE 做数据库匹配会偏“子串匹配”；
                // 为了贴近“全串匹配”，加上两端锚定：不额外包 %。
                glob_to_like_pattern(&folded)
            } else {
                format!("%{}%", like_escape_literal(&folded))
            };
            params.push(Value::Text(like_pat));
            // SQLite 要求 ESCAPE 表达式是单个字符，这里设置为反斜杠 '\'
            Ok(format!("({} LIKE ? ESCAPE '\\')", name_col))
        }
    }
}

/// 搜索索引：按新语法解析 query，优先使用 FTS5 做匹配与排序，Rust 内存仅负责高亮与兜底校验
pub fn search_index(
    query: &str,
    limit: u32,
    sort_by: Option<&str>,
) -> Result<Vec<(String, String, bool, Vec<(String, bool)>)>, String> {
    let conn = open_db()?;
    init_schema(&conn)?;

    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let parsed = crate::search_query::parse(query)?;
    let expr = match &parsed.expr {
        Some(e) => e,
        None => return Ok(Vec::new()),
    };

    // 归一化排序键：relevance / name / path / type
    let sort_key = match sort_by.unwrap_or("relevance") {
        "name" => "name",
        "path" => "path",
        "type" => "type",
        _ => "relevance",
    };

    // 先决定是否可以使用 FTS5：
    // - 仅在默认大小写/变音设置，且查询中不含通配符，且未开启 path: 时启用；
    // - 对包含中文（CJK）的查询：先尝试 FTS，如果完全没有结果，再自动回退到 LIKE 子串匹配，
    //   既保留中文整词/长词查询的性能，又保证“学习环境”“学习环”等前缀/子串场景有兜底结果。
    let has_glob = expr_has_glob(expr);
    let has_cjk = query.chars().any(|c| {
        // 基本 CJK 统一表意文字块 + 扩展 A（覆盖常见简繁中文）
        (c >= '\u{4E00}' && c <= '\u{9FFF}') || (c >= '\u{3400}' && c <= '\u{4DBF}')
    });
    let use_fts =
        match_config_fts_compatible(&parsed.config) && !has_glob && !parsed.config.path_only;

    // 先收集候选行，再用统一的内存逻辑做高亮与兜底校验
    let mut candidates: Vec<(String, String, String, i32)> = Vec::new();

    // 优先尝试 FTS，失败或不兼容时回退到 LIKE 路径
    let mut used_fts = false;

    if use_fts {
        let fts_result: Result<(), String> = (|| {
            append_debug_log(&format!(
                "[svnsearch][fts] 即将走 FTS 搜索，原始 query=\"{}\", config={:?}",
                query, parsed.config
            ));
            let fts_expr = build_fts_match_from_expr(expr);
            append_debug_log(&format!(
                "[svnsearch][fts] 构造出的 FTS match 表达式: {}",
                fts_expr
            ));

            // 使用显式编号的占位符，避免参数个数与占位符个数不一致。
            // 只在 name 列上做 FTS 匹配，保证「项目1 / folder: 项目1」都仅按名称命中。
            let mut sql =
                String::from("SELECT url, path, name, is_dir FROM file_index_fts WHERE name MATCH ?1 ");
            // 类型过滤（file:/folder:）
            match parsed.config.type_filter {
                crate::search_query::TypeFilter::Both => {}
                crate::search_query::TypeFilter::FileOnly => sql.push_str("AND is_dir = 0 "),
                crate::search_query::TypeFilter::FolderOnly => sql.push_str("AND is_dir = 1 "),
            }
            // 取一个略大于前端 limit 的上限，避免高亮兜底后不足
            let db_limit: i64 = ((limit as i64) * 5).clamp(limit as i64, 10_000);
            // 按排序键决定 ORDER BY
            match sort_key {
                "name" => sql.push_str("ORDER BY name COLLATE NOCASE, url, path "),
                "path" => sql.push_str("ORDER BY path COLLATE NOCASE "),
                "type" => sql.push_str("ORDER BY is_dir DESC, name COLLATE NOCASE, url, path "),
                _ => sql.push_str("ORDER BY bm25(file_index_fts), url, path "),
            }
            sql.push_str("LIMIT ?2");
            // 日志中记录完整 SQL（含 ORDER BY 和 LIMIT）
            append_debug_log(&format!(
                "[svnsearch][fts] FTS SQL: {} | limit={} (db_limit={})",
                sql, limit, db_limit
            ));

            let mut stmt = conn.prepare(&sql).map_err(|e| {
                let msg = e.to_string();
                append_debug_log(&format!(
                    "[svnsearch][fts] 准备 FTS 搜索语句失败: {} | sql={}",
                    msg, sql
                ));
                msg
            })?;
            let rows = stmt
                .query_map(rusqlite::params![fts_expr, db_limit], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i32>(3)?,
                    ))
                })
                .map_err(|e| {
                    let msg = e.to_string();
                    append_debug_log(&format!(
                        "[svnsearch][fts] FTS 搜索失败: {} | sql={} | match_expr={} | limit={}",
                        msg, sql, fts_expr, db_limit
                    ));
                    msg
                })?;
            for row in rows {
                candidates.push(row.map_err(|e| {
                    let msg = e.to_string();
                    append_debug_log(&format!("[svnsearch][fts] 读取行失败: {}", msg));
                    msg
                })?);
            }
            Ok(())
        })();

        if let Err(e) = fts_result {
            append_debug_log(&format!(
                "[svnsearch][fts] FTS 分支出现错误，将回退到 LIKE 搜索: {}",
                e
            ));
        } else {
            used_fts = true;
        }
    }

    if !used_fts || (has_cjk && candidates.is_empty()) {
        // 回退到原有 LIKE 逻辑：
        // - 默认按名称列匹配（name/name_fold/name_ascii_fold）
        // - 当配置为 path_only（path: 修饰符）时，按完整 URL+path 匹配
        let name_col = if parsed.config.case_sensitive {
            "name"
        } else if parsed.config.ascii_fold_only {
            "name_ascii_fold"
        } else {
            "name_fold"
        };
        let full_path_col = "url || '/' || path";

        let mut params: Vec<Value> = Vec::new();
        let mut where_parts = Vec::new();

        if parsed.config.path_only {
            // path: 修饰符：对完整 URL+path 做匹配，支持 path 片段/层级搜索
            where_parts.push(build_sql_where_from_expr(
                expr,
                full_path_col,
                &mut params,
                &parsed.config,
            )?);
        } else {
            // 默认：仅按名称匹配
            where_parts.push(build_sql_where_from_expr(
                expr,
                name_col,
                &mut params,
                &parsed.config,
            )?);
        }
        match parsed.config.type_filter {
            crate::search_query::TypeFilter::Both => {}
            crate::search_query::TypeFilter::FileOnly => {
                where_parts.push("(is_dir = 0)".to_string())
            }
            crate::search_query::TypeFilter::FolderOnly => {
                where_parts.push("(is_dir = 1)".to_string())
            }
        }

        let db_limit: i64 = ((limit as i64) * 50).clamp(500, 50_000);
        params.push(Value::Integer(db_limit));

        let mut sql = format!(
            "SELECT url, path, name, is_dir FROM file_index WHERE {} ",
            where_parts.join(" AND ")
        );
        match sort_key {
            "name" => sql.push_str("ORDER BY name COLLATE NOCASE, url, path "),
            "path" => sql.push_str("ORDER BY path COLLATE NOCASE "),
            "type" => sql.push_str("ORDER BY is_dir DESC, name COLLATE NOCASE, url, path "),
            // 回退路径下没有 bm25，只能按 URL+path 做稳定排序
            _ => sql.push_str("ORDER BY url, path "),
        }
        sql.push_str("LIMIT ?");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("准备搜索语句失败: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                ))
            })
            .map_err(|e| format!("搜索失败: {}", e))?;
        for row in rows {
            candidates.push(row.map_err(|e| format!("读取行失败: {}", e))?);
        }
    }

    let mut out = Vec::new();
    for (url, path, name, is_dir_int) in candidates {
        let is_dir = is_dir_int != 0;

        // SQL 已经根据表达式完成了一次完整过滤，这里只用于生成名称高亮，
        // 不再作为「命中与否」的二次裁决，以避免路径查询被按纯名称错误过滤。
        let segments = match crate::search_query::parse_and_match(query, &name, is_dir, None) {
            Ok((_matched, ranges)) => crate::search_query::ranges_to_segments(&name, &ranges),
            Err(e) => {
                append_debug_log(&format!(
                    "[svnsearch][match] parse_and_match 失败，将退化为无高亮: {}",
                    e
                ));
                crate::search_query::ranges_to_segments(&name, &[])
            }
        };

        out.push((url, path, is_dir, segments));
        if out.len() >= limit as usize {
            break;
        }
    }
    Ok(out)
}

/// 为本地开发环境生成一些伪造索引数据，便于测试中文搜索与 FTS 性能
pub fn seed_dummy_dev_data() -> Result<(), String> {
    let conn = open_db()?;
    init_schema(&conn)?;

    // 使用几个固定的「伪仓库 URL」，避免污染未来真实配置
    let dev_urls = vec![
        "dev://repo-中文测试-1".to_string(),
        "dev://repo-中文测试-2".to_string(),
    ];

    for (idx, url) in dev_urls.iter().enumerate() {
        // 先清理旧数据
        clear_index(url)?;

        // 构造若干包含中文名称的路径
        let mut files: Vec<String> = Vec::new();
        let base_prefix = format!("项目{}/模块", idx + 1);
        let chinese_words = ["数据", "门户", "配置", "测试", "日志", "报表", "用户", "权限"];

        // 大约生成几千条记录：目录 + 文件混合
        for i in 0..50u32 {
            let dir_path = format!("{}/目录{}/", base_prefix, i);
            files.push(dir_path.clone());

            for j in 0..50u32 {
                let word = chinese_words[((i + j) as usize) % chinese_words.len()];
                let file_path = format!("{}/文件{}_{}_{}.txt", dir_path, i, j, word);
                files.push(file_path);
            }
        }

        save_index(url, &files)?;
    }

    Ok(())
}