use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

mod index_engine;
mod database;
mod commands;
mod svn_client;
mod pinyin;
mod autostart;

type AppState = Arc<Mutex<Option<index_engine::IndexEngine>>>;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path_resolver()
                .app_data_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            
            std::fs::create_dir_all(&app_data_dir).ok();
            
            let db_path = app_data_dir.join("svnsearch.db");
            
            let db = database::Database::new(db_path.to_str().unwrap())
                .expect("Failed to initialize database");
            
            let index_engine = index_engine::IndexEngine::new(db);
            let state = std::sync::Arc::new(std::sync::Mutex::new(Some(index_engine)));
            
            app.manage(state.clone());
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_files,
            commands::search_files_with_filter,
            commands::add_repository,
            commands::update_repository,
            commands::delete_repository,
            commands::get_repositories,
            commands::index_repository,
            commands::index_repository_incremental,
            commands::index_all_repositories,
            commands::get_index_status,
            commands::get_index_status_by_repo,
            commands::toggle_autostart,
            commands::get_config,
            commands::update_config,
            commands::get_db_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
