use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub last_update: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                username TEXT,
                password TEXT,
                last_update TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_index (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                filename TEXT NOT NULL,
                is_dir INTEGER DEFAULT 0,
                size INTEGER DEFAULT 0,
                revision INTEGER DEFAULT 0,
                last_modified TEXT,
                FOREIGN KEY (repo_id) REFERENCES repositories(id),
                UNIQUE(repo_id, path)
            )",
            [],
        )?;

        self.create_indexes()?;
        Ok(())
    }

    fn create_indexes(&self) -> Result<()> {
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_filename ON file_index(filename)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_repo_id ON file_index(repo_id)",
            [],
        )?;

        Ok(())
    }

    pub fn add_repository(&self, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<u64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO repositories (name, url, username, password) VALUES (?1, ?2, ?3, ?4)",
            params![name, url, username, password],
        )?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    pub fn get_repositories(&self) -> Result<Vec<Repository>> {
        let mut stmt = self.conn.prepare("SELECT * FROM repositories ORDER BY name")?;
        let repo_iter = stmt.query_map([], |row| {
            Ok(Repository {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                username: row.get(3)?,
                password: row.get(4)?,
                last_update: row.get(5)?,
            })
        })?;

        repo_iter.collect()
    }

    pub fn update_repository(&self, id: u64, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE repositories SET name = ?1, url = ?2, username = ?3, password = ?4 WHERE id = ?5",
            params![name, url, username, password, id],
        )?;
        Ok(())
    }

    pub fn delete_repository(&self, id: u64) -> Result<()> {
        self.conn.execute("DELETE FROM file_index WHERE repo_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_repo_index(&self, repo_id: u64) -> Result<()> {
        self.conn.execute("DELETE FROM file_index WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }

    pub fn add_file(&self, repo_id: u64, path: &str, filename: &str, is_dir: bool, size: u64, revision: u64, last_modified: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO file_index (repo_id, path, filename, is_dir, size, revision, last_modified) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![repo_id, path, filename, is_dir as i32, size, revision, last_modified],
        )?;
        Ok(())
    }

    pub fn get_files_by_repo(&self, repo_id: u64) -> Result<Vec<crate::index_engine::FileEntry>> {
        let mut stmt = self.conn.prepare("SELECT * FROM file_index WHERE repo_id = ?1")?;
        let file_iter = stmt.query_map(params![repo_id], |row| {
            Ok(crate::index_engine::FileEntry {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                path: row.get(2)?,
                filename: row.get(3)?,
                is_dir: row.get(4)? != 0,
                size: row.get(5)?,
                revision: row.get(6)?,
                last_modified: row.get(7)?,
            })
        })?;

        file_iter.collect()
    }

    pub fn search_files(&self, query: &str) -> Result<Vec<crate::index_engine::FileEntry>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT f.*, r.name as repo_name, r.url as repo_url 
             FROM file_index f 
             JOIN repositories r ON f.repo_id = r.id 
             WHERE f.filename LIKE ?1 OR f.path LIKE ?1
             ORDER BY f.filename 
             LIMIT 1000"
        )?;
        
        let file_iter = stmt.query_map(params![pattern], |row| {
            Ok(crate::index_engine::FileEntry {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                path: row.get(2)?,
                filename: row.get(3)?,
                is_dir: row.get(4)? != 0,
                size: row.get(5)?,
                revision: row.get(6)?,
                last_modified: row.get(7)?,
            })
        })?;

        file_iter.collect()
    }

    pub fn count_files(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
