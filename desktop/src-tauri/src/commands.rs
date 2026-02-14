use crate::index_engine::{IndexEngine, FileEntry};
use crate::database::Database;
use crate::pinyin::contains_pinyin;
use crate::autostart::set_autostart;
use tauri::{State, Window};
use std::sync::Arc;
use tokio::sync::Mutex;

type IndexState = Arc<Mutex<Option<IndexEngine>>>;

#[tauri::command]
pub async fn search_files(query: String, index: State<'_, IndexState>) -> Vec<FileEntry> {
    if query.is_empty() {
        return Vec::new();
    }
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        let mut results = engine.search(&query).await;
        
        results.retain(|f| contains_pinyin(&f.filename, &query) || contains_pinyin(&f.path, &query));
        
        results
    } else {
        Vec::new()
    }
}

#[tauri::command]
pub async fn add_repository(
    name: String,
    url: String,
    username: Option<String>,
    password: Option<String>,
    index: State<'_, IndexState>
) -> Result<u64, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    let db_path = std::env::current_dir()
        .unwrap()
        .join("svnsearch.db");
    
    let db = Database::new(db_path.to_str().unwrap())
        .map_err(|e| e.to_string())?;
    
    db.add_repository(&name, &url, username.as_deref(), password.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_repositories(index: State<'_, IndexState>) -> Result<Vec<crate::database::Repository>, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    let db_path = std::env::current_dir()
        .unwrap()
        .join("svnsearch.db");
    
    let db = Database::new(db_path.to_str().unwrap())
        .map_err(|e| e.to_string())?;
    
    db.get_repositories().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn index_repository(
    repo_id: u64,
    index: State<'_, IndexState>
) -> Result<String, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    let db_path = std::env::current_dir()
        .unwrap()
        .join("svnsearch.db");
    
    let db = Database::new(db_path.to_str().unwrap())
        .map_err(|e| e.to_string())?;
    
    let files = db.get_files_by_repo(repo_id).map_err(|e| e.to_string())?;
    engine.add_files(files).await.map_err(|e| e.to_string())?;
    
    Ok(format!("Indexed {} files", files.len()))
}

#[tauri::command]
pub async fn get_index_status(index: State<'_, IndexState>) -> usize {
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        engine.count().await
    } else {
        0
    }
}

#[tauri::command]
pub async fn toggle_window(window: Window, show: bool) -> Result<(), String> {
    if show {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_autostart(enable: bool) -> Result<(), String> {
    set_autostart(enable).map_err(|e| e.to_string())
}
