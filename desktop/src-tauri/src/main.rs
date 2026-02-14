use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

mod index_engine;
mod database;
mod commands;
mod svn_client;
mod tray;
mod pinyin;
mod autostart;

type AppState = Arc<Mutex<Option<index_engine::IndexEngine>>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path_resolver().app_data_dir().unwrap_or_else(|_| {
                std::env::current_dir().unwrap()
            });
            
            let db_path = app_data_dir.join("svnsearch.db");
            std::fs::create_dir_all(&app_data_dir).ok();
            
            let db = database::Database::new(db_path.to_str().unwrap())
                .expect("Failed to initialize database");
            
            let index_engine = index_engine::IndexEngine::new(db);
            let state = Arc::new(Mutex::new(Some(index_engine)));
            
            app.manage(state.clone());
            
            let _tray = tray::create_tray(app.handle());
            
            Ok(())
        })
        .system_tray(tray::create_tray)
        .on_system_tray_event(tray::handle_tray_event)
        .invoke_handler(tauri::generate_handler![
            commands::search_files,
            commands::add_repository,
            commands::get_repositories,
            commands::index_repository,
            commands::get_index_status,
            commands::toggle_window,
            commands::toggle_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
