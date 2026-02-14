use std::sync::Arc;
use tokio::sync::RwLock;
use crate::database::{Database, AppConfig};
use crate::svn_client::SvnClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: u64,
    pub repo_id: u64,
    pub path: String,
    pub filename: String,
    pub file_type: String,
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

    pub async fn search(&self, query: &str) -> Vec<FileEntry> {
        let db = self.db.read().await;
        db.search_files(query, None, None, None, None, None).unwrap_or_default()
    }

    pub async fn search_with_filter(
        &self, 
        query: &str, 
        file_type: Option<&str>,
        min_size: Option<u64>,
        max_size: Option<u64>,
        from_date: Option<&str>,
        to_date: Option<&str>
    ) -> Vec<FileEntry> {
        let db = self.db.read().await;
        db.search_files(query, file_type, min_size, max_size, from_date, to_date).unwrap_or_default()
    }

    pub async fn count(&self) -> usize {
        let db = self.db.read().await;
        db.count_files().unwrap_or(0)
    }

    pub async fn count_by_repo(&self, repo_id: u64) -> usize {
        let db = self.db.read().await;
        db.count_files_by_repo(repo_id).unwrap_or(0)
    }

    pub async fn clear_repo(&self, repo_id: u64) -> Result<(), String> {
        let db = self.db.write().await;
        db.clear_repo_index(repo_id).map_err(|e| e.to_string())
    }

    pub async fn add_files(&self, entries: Vec<FileEntry>) -> Result<(), String> {
        let db = self.db.write().await;
        db.add_files(&entries).map_err(|e| e.to_string())
    }

    pub async fn add_repository(&self, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<u64, String> {
        let db = self.db.write().await;
        db.add_repository(name, url, username, password).map_err(|e| e.to_string())
    }

    pub async fn get_repositories(&self) -> Result<Vec<crate::database::Repository>, String> {
        let db = self.db.read().await;
        db.get_repositories().map_err(|e| e.to_string())
    }

    pub async fn get_repository(&self, id: u64) -> Result<Option<crate::database::Repository>, String> {
        let db = self.db.read().await;
        db.get_repository(id).map_err(|e| e.to_string())
    }

    pub async fn update_repository(&self, id: u64, name: &str, url: &str, username: Option<&str>, password: Option<&str>) -> Result<(), String> {
        let db = self.db.write().await;
        db.update_repository(id, name, url, username, password).map_err(|e| e.to_string())
    }

    pub async fn delete_repository(&self, id: u64) -> Result<(), String> {
        let db = self.db.write().await;
        db.delete_repository(id).map_err(|e| e.to_string())
    }

    pub async fn index_repository(&self, repo_id: u64) -> Result<String, String> {
        let db = self.db.read().await;
        let repo = db.get_repository(repo_id).map_err(|e| e.to_string())?;
        let repo = repo.ok_or("Repository not found")?;
        drop(db);

        let svn_client = SvnClient::new(None);

        let files = svn_client.list_directory(
            &repo.url,
            repo.username.as_deref(),
            repo.password.as_deref(),
            true
        ).map_err(|e| format!("SVN list failed: {}", e))?;

        let count = files.len();
        let batch_size = 1000;

        for batch in files.chunks(batch_size) {
            let mut entries = Vec::new();
            for file in batch {
                entries.push(FileEntry {
                    id: 0,
                    repo_id: repo_id,
                    path: file.path.clone(),
                    filename: file.filename.clone(),
                    file_type: get_file_type(&file.filename),
                    is_dir: file.is_dir,
                    size: file.size,
                    revision: file.revision,
                    last_modified: file.last_modified.clone(),
                    repo_name: Some(repo.name.clone()),
                    repo_url: Some(repo.url.clone()),
                });
            }
            self.add_files(entries).await.map_err(|e| e.to_string())?;
        }

        let db = self.db.read().await;
        if let Some(max_rev) = files.iter().map(|f| f.revision).max() {
            db.update_repository_revision(repo_id, max_rev).ok();
        }

        Ok(format!("Indexed {} files", count))
    }

    pub async fn index_repository_incremental(&self, repo_id: u64) -> Result<String, String> {
        let db = self.db.read().await;
        let repo = db.get_repository(repo_id).map_err(|e| e.to_string())?;
        let repo = repo.ok_or("Repository not found")?;
        
        let current_max_revision = db.get_repo_max_revision(repo_id).unwrap_or(0);
        drop(db);

        let svn_client = SvnClient::new(None);

        let latest_revision = svn_client.get_latest_revision(
            &repo.url,
            repo.username.as_deref(),
            repo.password.as_deref()
        ).map_err(|e| format!("Failed to get latest revision: {}", e))?;

        if latest_revision <= current_max_revision {
            return Ok("Already up to date".to_string());
        }

        let files = svn_client.list_directory_at_revision(
            &repo.url,
            repo.username.as_deref(),
            repo.password.as_deref(),
            latest_revision
        ).map_err(|e| format!("SVN list failed: {}", e))?;

        let count = files.len();
        let batch_size = 1000;

        for batch in files.chunks(batch_size) {
            let mut entries = Vec::new();
            for file in batch {
                entries.push(FileEntry {
                    id: 0,
                    repo_id: repo_id,
                    path: file.path.clone(),
                    filename: file.filename.clone(),
                    file_type: get_file_type(&file.filename),
                    is_dir: file.is_dir,
                    size: file.size,
                    revision: file.revision,
                    last_modified: file.last_modified.clone(),
                    repo_name: Some(repo.name.clone()),
                    repo_url: Some(repo.url.clone()),
                });
            }
            self.add_files(entries).await.map_err(|e| e.to_string())?;
        }

        let db = self.db.read().await;
        db.update_repository_revision(repo_id, latest_revision).ok();

        Ok(format!("Indexed {} new files (revisions {} -> {})", count, current_max_revision, latest_revision))
    }

    pub async fn get_config(&self) -> Result<AppConfig, String> {
        let db = self.db.read().await;
        db.get_config().map_err(|e| e.to_string())
    }

    pub async fn update_config(&self, db_path: Option<&str>, auto_index_enabled: Option<bool>, auto_index_interval: Option<i32>) -> Result<(), String> {
        let db = self.db.read().await;
        db.update_config(db_path, auto_index_enabled, auto_index_interval).map_err(|e| e.to_string())
    }

    pub async fn get_db_path(&self) -> String {
        let db = self.db.read().await;
        db.get_db_path()
    }
}

fn get_file_type(filename: &str) -> String {
    if let Some(dot_idx) = filename.rfind('.') {
        let ext = &filename[dot_idx + 1..];
        match ext.to_lowercase().as_str() {
            "txt" | "md" | "log" => "text".to_string(),
            "js" | "ts" | "jsx" | "tsx" | "vue" | "py" | "rs" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "swift" | "kt" | "scala" => "code".to_string(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => "image".to_string(),
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" => "audio".to_string(),
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "video".to_string(),
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => "archive".to_string(),
            "pdf" => "pdf".to_string(),
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "document".to_string(),
            _ => "other".to_string(),
        }
    } else {
        "other".to_string()
    }
}
