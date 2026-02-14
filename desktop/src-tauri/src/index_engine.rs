use std::sync::Arc;
use tokio::sync::RwLock;
use crate::database::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: u64,
    pub repo_id: u64,
    pub path: String,
    pub filename: String,
    pub is_dir: bool,
    pub size: u64,
    pub revision: u64,
    pub last_modified: Option<String>,
    pub repo_name: Option<String>,
    pub repo_url: Option<String>,
}

pub struct IndexEngine {
    db: Arc<RwLock<Database>>,
}

impl IndexEngine {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
        }
    }

    pub async fn search(&self, query: &str) -> Vec<crate::index_engine::FileEntry> {
        let db = self.db.read().await;
        db.search_files(query).unwrap_or_default()
    }

    pub async fn count(&self) -> usize {
        let db = self.db.read().await;
        db.count_files().unwrap_or(0)
    }

    pub async fn clear_repo(&self, repo_id: u64) -> Result<(), String> {
        let db = self.db.write().await;
        db.clear_repo_index(repo_id).map_err(|e| e.to_string())
    }

    pub async fn add_file(&self, entry: crate::index_engine::FileEntry) -> Result<(), String> {
        let db = self.db.write().await;
        db.add_file(
            entry.repo_id,
            &entry.path,
            &entry.filename,
            entry.is_dir,
            entry.size,
            entry.revision,
            entry.last_modified.as_deref(),
        ).map_err(|e| e.to_string())
    }

    pub async fn add_files(&self, entries: Vec<crate::index_engine::FileEntry>) -> Result<(), String> {
        let db = self.db.write().await;
        for entry in entries {
            db.add_file(
                entry.repo_id,
                &entry.path,
                &entry.filename,
                entry.is_dir,
                entry.size,
                entry.revision,
                entry.last_modified.as_deref(),
            ).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
