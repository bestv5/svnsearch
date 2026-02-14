use crate::index_engine::{IndexEngine, FileEntry};
use crate::database::AppConfig;
use crate::pinyin::contains_pinyin;
use crate::autostart::set_autostart;
use tauri::{State, Window};
use std::sync::Arc;
use tokio::sync::Mutex;

type IndexState = Arc<Mutex<Option<IndexEngine>>>;

#[tauri::command]
pub async fn search_files(query: String, index: State<'_, IndexState>) -> Result<Vec<FileEntry>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        let mut results = engine.search(&query).await;
        
        results.retain(|f| contains_pinyin(&f.filename, &query) || contains_pinyin(&f.path, &query));
        
        Ok(results)
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub async fn search_files_with_filter(
    query: String,
    file_type: Option<String>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    from_date: Option<String>,
    to_date: Option<String>,
    index: State<'_, IndexState>
) -> Result<Vec<FileEntry>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        let results = engine.search_with_filter(
            &query,
            file_type.as_deref(),
            min_size,
            max_size,
            from_date.as_deref(),
            to_date.as_deref()
        ).await;
        
        Ok(results)
    } else {
        Ok(Vec::new())
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
    
    engine.add_repository(&name, &url, username.as_deref(), password.as_deref()).await
}

#[tauri::command]
pub async fn get_repositories(index: State<'_, IndexState>) -> Result<Vec<crate::database::Repository>, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.get_repositories().await
}

#[tauri::command]
pub async fn index_repository(
    repo_id: u64,
    index: State<'_, IndexState>
) -> Result<String, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.index_repository(repo_id).await
}

#[tauri::command]
pub async fn index_repository_incremental(
    repo_id: u64,
    index: State<'_, IndexState>
) -> Result<String, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.index_repository_incremental(repo_id).await
}

#[tauri::command]
pub async fn index_all_repositories(
    index: State<'_, IndexState>
) -> Result<String, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    let repos = engine.get_repositories().await?;
    let mut total_indexed = 0;
    
    for repo in repos {
        match engine.index_repository_incremental(repo.id).await {
            Ok(result) => {
                println!("Indexed repo {}: {}", repo.name, result);
                total_indexed += 1;
            }
            Err(e) => {
                println!("Failed to index repo {}: {}", repo.name, e);
            }
        }
    }
    
    Ok(format!("Indexed {} repositories", total_indexed))
}

#[tauri::command]
pub async fn get_index_status(index: State<'_, IndexState>) -> Result<usize, String> {
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        Ok(engine.count().await)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn get_index_status_by_repo(
    repo_id: u64,
    index: State<'_, IndexState>
) -> Result<usize, String> {
    let guard = index.lock().await;
    if let Some(engine) = guard.as_ref() {
        Ok(engine.count_by_repo(repo_id).await)
    } else {
        Ok(0)
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

#[tauri::command]
pub async fn update_repository(
    id: u64,
    name: String,
    url: String,
    username: Option<String>,
    password: Option<String>,
    index: State<'_, IndexState>
) -> Result<(), String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.update_repository(id, &name, &url, username.as_deref(), password.as_deref()).await
}

#[tauri::command]
pub async fn delete_repository(
    id: u64,
    index: State<'_, IndexState>
) -> Result<(), String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.delete_repository(id).await
}

#[tauri::command]
pub async fn get_config(index: State<'_, IndexState>) -> Result<AppConfig, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.get_config().await
}

#[tauri::command]
pub async fn update_config(
    db_path: Option<String>,
    auto_index_enabled: Option<bool>,
    auto_index_interval: Option<i32>,
    index: State<'_, IndexState>
) -> Result<(), String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    engine.update_config(
        db_path.as_deref(),
        auto_index_enabled,
        auto_index_interval
    ).await
}

#[tauri::command]
pub async fn get_db_path(index: State<'_, IndexState>) -> Result<String, String> {
    let guard = index.lock().await;
    let engine = guard.as_ref().ok_or("Index engine not initialized")?;
    
    Ok(engine.get_db_path().await)
}
