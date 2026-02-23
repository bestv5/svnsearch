use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tracing::{info, error};

mod index_engine;
mod database;
mod commands;
mod svn_client;
mod pinyin;
mod autostart;

type AppState = Arc<Mutex<Option<index_engine::IndexEngine>>>;

fn main() {
    let log_dir = std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("SVN Search").join("logs"))
        .unwrap_or_else(|_| std::path::PathBuf::from("logs"));
    
    std::fs::create_dir_all(&log_dir).ok();
    
    let file_appender = tracing_appender::rolling::daily(&log_dir, "svnsearch");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Starting SVN Search application");
    
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}", panic_info);
    }));
    
    let result = tauri::Builder::default()
        .setup(|app| {
            info!("Running setup");
            
            let app_data_dir = app.path_resolver()
                .app_data_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            
            info!("App data dir: {:?}", app_data_dir);
            
            std::fs::create_dir_all(&app_data_dir).ok();
            
            let db_path = app_data_dir.join("svnsearch.db");
            info!("Database path: {:?}", db_path);
            
            let db = database::Database::new(db_path.to_str().unwrap())
                .expect("Failed to initialize database");
            
            info!("Database initialized");
            
            let index_engine = index_engine::IndexEngine::new(db);
            info!("Index engine created");
            
            let state = std::sync::Arc::new(std::sync::Mutex::new(Some(index_engine)));
            
            app.manage(state.clone());
            info!("State managed");
            
            info!("Setup complete");
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
            commands::get_index_status,
            commands::index_all_repositories,
            commands::get_index_status_by_repo,
            commands::toggle_autostart,
            commands::get_config,
            commands::update_config,
            commands::get_db_path
        ])
        .run(tauri::generate_context!());
    
    if let Err(e) = result {
        error!("Error running tauri application: {}", e);
        std::process::exit(1);
    }
}
