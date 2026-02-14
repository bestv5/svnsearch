use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub last_update: Option<String>,
    pub last_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub id: i32,
    pub db_path: String,
    pub auto_index_enabled: bool,
    pub auto_index_interval: i32,
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
    db_path: String,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = PathBuf::from(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        
        let conn = Connection::open(db_path)?;
        let db_path_str = db_path.to_string();
        let db = Self { conn: Arc::new(Mutex::new(conn)), db_path: db_path_str };
        db.init_tables()?;
        Ok(db)
    }

    pub fn get_db_path(&self) -> String {
        self.db_path.clone()
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_config (
                id INTEGER PRIMARY KEY,
                db_path TEXT NOT NULL,
                auto_index_enabled INTEGER DEFAULT 0,
                auto_index_interval INTEGER DEFAULT 3600
            )",
            [],
        )?;

        let config_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM app_config",
            [],
            |row| row.get(0)
        )?;
        
        if config_exists == 0 {
            conn.execute(
                "INSERT INTO app_config (id, db_path, auto_index_enabled, auto_index_interval) VALUES (1, '', 0, 3600)",
                [],
            )?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                username TEXT,
                password TEXT,
                last_update TEXT,
                last_revision INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                filename TEXT NOT NULL,
                file_type TEXT DEFAULT '',
                is_dir INTEGER DEFAULT 0,
                size INTEGER DEFAULT 0,
                revision INTEGER DEFAULT 0,
                last_modified TEXT,
                indexed_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (repo_id) REFERENCES repositories(id),
                UNIQUE(repo_id, path)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS index_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL,
                start_revision INTEGER NOT NULL,
                end_revision INTEGER NOT NULL,
                status TEXT DEFAULT 'pending',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                completed_at TEXT,
                FOREIGN KEY (repo_id) REFERENCES repositories(id)
            )",
            [],
        )?;

        self.create_indexes()?;
        Ok(())
    }

    fn create_indexes(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_filename ON file_index(filename)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_repo_id ON file_index(repo_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_type ON file_index(file_type)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_revision ON file_index(revision)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_path ON file_index(path)",
            [],
        )?;

        Ok(())
    }

    pub fn get_config(&self) -> Result<AppConfig> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, db_path, auto_index_enabled, auto_index_interval FROM app_config WHERE id = 1",
            [],
            |row| {
                Ok(AppConfig {
                    id: row.get(0)?,
                    db_path: row.get(1)?,
                    auto_index_enabled: row.get::<_, i32>(2)? != 0,
                    auto_index_interval: row.get(3)?,
                })
            }
        )
    }

    pub fn update_config(&self, db_path: Option<&str>, auto_index_enabled: Option<bool>, auto_index_interval: Option<i32>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        if let Some(path) = db_path {
            conn.execute(
                "UPDATE app_config SET db_path = ?1 WHERE id = 1",
                params![path],
            )?;
        }
        
        if let Some(enabled) = auto_index_enabled {
            conn.execute(
                "UPDATE app_config SET auto_index_enabled = ?1 WHERE id = 1",
                params![enabled as i32],
            )?;
        }
        
        if let Some(interval) = auto_index_interval {
            conn.execute(
                "UPDATE app_config SET auto_index_interval = ?1 WHERE id = 1",
                params![interval],
            )?;
        }
        
        Ok(())
    }

    pub fn add_repository(&self, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO repositories (name, url, username, password) VALUES (?1, ?2, ?3, ?4)",
            params![name, url, username, password],
        )?;
        Ok(conn.last_insert_rowid() as u64)
    }

    pub fn get_repositories(&self) -> Result<Vec<Repository>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, url, username, password, last_update, last_revision FROM repositories ORDER BY name")?;
        let repo_iter = stmt.query_map([], |row| {
            Ok(Repository {
                id: row.get::<_, u64>(0)?,
                name: row.get::<_, String>(1)?,
                url: row.get::<_, String>(2)?,
                username: row.get::<_, Option<String>>(3)?,
                password: row.get::<_, Option<String>>(4)?,
                last_update: row.get::<_, Option<String>>(5)?,
                last_revision: row.get::<_, Option<u64>>(6)?,
            })
        })?;

        repo_iter.collect()
    }

    pub fn get_repository(&self, id: u64) -> Result<Option<Repository>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, url, username, password, last_update, last_revision FROM repositories WHERE id = ?1")?;
        let repo = stmt.query_row(params![id], |row| {
            Ok(Repository {
                id: row.get::<_, u64>(0)?,
                name: row.get::<_, String>(1)?,
                url: row.get::<_, String>(2)?,
                username: row.get::<_, Option<String>>(3)?,
                password: row.get::<_, Option<String>>(4)?,
                last_update: row.get::<_, Option<String>>(5)?,
                last_revision: row.get::<_, Option<u64>>(6)?,
            })
        }).ok();

        Ok(repo)
    }

    pub fn update_repository(&self, id: u64, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE repositories SET name = ?1, url = ?2, username = ?3, password = ?4 WHERE id = ?5",
            params![name, url, username, password, id],
        )?;
        Ok(())
    }

    pub fn update_repository_revision(&self, id: u64, revision: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE repositories SET last_revision = ?1, last_update = ?2 WHERE id = ?3",
            params![revision, now, id],
        )?;
        Ok(())
    }

    pub fn delete_repository(&self, id: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_index WHERE repo_id = ?1", params![id])?;
        conn.execute("DELETE FROM index_queue WHERE repo_id = ?1", params![id])?;
        conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_repo_index(&self, repo_id: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_index WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }

    pub fn add_file(&self, repo_id: u64, path: &str, filename: &str, file_type: &str, is_dir: bool, size: u64, revision: u64, last_modified: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO file_index (repo_id, path, filename, file_type, is_dir, size, revision, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![repo_id, path, filename, file_type, is_dir as i32, size, revision, last_modified],
        )?;
        Ok(())
    }

    pub fn add_files(&self, entries: &[crate::index_engine::FileEntry]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        
        for entry in entries {
            tx.execute(
                "INSERT OR REPLACE INTO file_index (repo_id, path, filename, file_type, is_dir, size, revision, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![entry.repo_id, entry.path, entry.filename, entry.file_type, entry.is_dir as i32, entry.size, entry.revision, entry.last_modified],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }

    pub fn get_files_by_repo(&self, repo_id: u64) -> Result<Vec<crate::index_engine::FileEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, repo_id, path, filename, file_type, is_dir, size, revision, last_modified FROM file_index WHERE repo_id = ?1")?;
        let file_iter = stmt.query_map(params![repo_id], |row| {
            Ok(crate::index_engine::FileEntry {
                id: row.get::<_, u64>(0)?,
                repo_id: row.get::<_, u64>(1)?,
                path: row.get::<_, String>(2)?,
                filename: row.get::<_, String>(3)?,
                file_type: row.get::<_, String>(4)?,
                is_dir: row.get::<_, i32>(5)? != 0,
                size: row.get::<_, u64>(6)?,
                revision: row.get::<_, u64>(7)?,
                last_modified: row.get::<_, Option<String>>(8)?,
                repo_name: None,
                repo_url: None,
            })
        })?;

        file_iter.collect()
    }

    pub fn get_repo_max_revision(&self, repo_id: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let revision: u64 = conn.query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM file_index WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0)
        )?;
        Ok(revision)
    }

    pub fn delete_files_by_revision(&self, repo_id: u64, revision: u64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM file_index WHERE repo_id = ?1 AND revision > ?2",
            params![repo_id, revision],
        )?;
        Ok(count)
    }

    pub fn search_files(&self, query: &str, file_type: Option<&str>, min_size: Option<u64>, max_size: Option<u64>, from_date: Option<&str>, to_date: Option<&str>) -> Result<Vec<crate::index_engine::FileEntry>> {
        let pattern = format!("%{}%", query);
        let conn = self.conn.lock().unwrap();
        
        let mut sql = String::from(
            "SELECT f.id, f.repo_id, f.path, f.filename, f.file_type, f.is_dir, f.size, f.revision, f.last_modified, r.name as repo_name, r.url as repo_url 
             FROM file_index f 
             JOIN repositories r ON f.repo_id = r.id 
             WHERE (f.filename LIKE ?1 OR f.path LIKE ?1)"
        );
        
        if file_type.is_some() {
            sql.push_str(" AND f.file_type = ?2");
        }
        if min_size.is_some() {
            sql.push_str(" AND f.size >= ?3");
        }
        if max_size.is_some() {
            sql.push_str(" AND f.size <= ?4");
        }
        if from_date.is_some() {
            sql.push_str(" AND f.last_modified >= ?5");
        }
        if to_date.is_some() {
            sql.push_str(" AND f.last_modified <= ?6");
        }
        
        sql.push_str(" ORDER BY f.filename LIMIT 1000");
        
        let mut stmt = conn.prepare(&sql)?;
        
        let mut param_idx = 1;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(pattern)];
        
        if let Some(ft) = file_type {
            param_idx += 1;
            params_vec.push(Box::new(ft.to_string()));
        }
        if let Some(ms) = min_size {
            param_idx += 1;
            params_vec.push(Box::new(ms));
        }
        if let Some(mxs) = max_size {
            param_idx += 1;
            params_vec.push(Box::new(mxs));
        }
        if let Some(fd) = from_date {
            param_idx += 1;
            params_vec.push(Box::new(fd.to_string()));
        }
        if let Some(td) = to_date {
            param_idx += 1;
            params_vec.push(Box::new(td.to_string()));
        }
        
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        
        let file_iter = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(crate::index_engine::FileEntry {
                id: row.get::<_, u64>(0)?,
                repo_id: row.get::<_, u64>(1)?,
                path: row.get::<_, String>(2)?,
                filename: row.get::<_, String>(3)?,
                file_type: row.get::<_, String>(4)?,
                is_dir: row.get::<_, i32>(5)? != 0,
                size: row.get::<_, u64>(6)?,
                revision: row.get::<_, u64>(7)?,
                last_modified: row.get::<_, Option<String>>(8)?,
                repo_name: row.get::<_, Option<String>>(9)?,
                repo_url: row.get::<_, Option<String>>(10)?,
            })
        })?;

        file_iter.collect()
    }

    pub fn count_files(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn count_files_by_repo(&self, repo_id: u64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM file_index WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0)
        )?;
        Ok(count as usize)
    }

    pub fn add_index_task(&self, repo_id: u64, start_revision: u64, end_revision: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO index_queue (repo_id, start_revision, end_revision, status) VALUES (?1, ?2, ?3, 'pending')",
            params![repo_id, start_revision, end_revision],
        )?;
        Ok(conn.last_insert_rowid() as u64)
    }

    pub fn get_pending_index_tasks(&self) -> Result<Vec<(u64, u64, u64, u64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, repo_id, start_revision, end_revision FROM index_queue WHERE status = 'pending' ORDER BY id")?;
        let tasks = stmt.query_map([], |row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?, row.get::<_, u64>(2)?, row.get::<_, u64>(3)?))
        })?.filter_map(|r| r.ok()).collect();
        Ok(tasks)
    }

    pub fn update_index_task_status(&self, task_id: u64, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = if status == "completed" {
            Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
        } else {
            None
        };
        
        if let Some(completed_at) = now {
            conn.execute(
                "UPDATE index_queue SET status = ?1, completed_at = ?2 WHERE id = ?3",
                params![status, completed_at, task_id],
            )?;
        } else {
            conn.execute(
                "UPDATE index_queue SET status = ?1 WHERE id = ?2",
                params![status, task_id],
            )?;
        }
        Ok(())
    }
}
