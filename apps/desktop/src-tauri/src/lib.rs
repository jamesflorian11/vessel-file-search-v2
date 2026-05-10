mod commands;
mod config;
mod content_extract;
mod db;
mod dto;
mod index;
mod job_types;
mod jobs;
mod path_norm;
mod roots;
mod scan;
mod search;

#[cfg(windows)]
mod windows_explorer;

use std::sync::Arc;

use commands::{
    cancel_job, get_indexing_status, get_settings, open_file, reveal_in_explorer, save_settings,
    search_files, start_scan, AppState,
};
use jobs::JobManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.maximize();
            }
            let app_data = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app data dir: {e}"))?;
            std::fs::create_dir_all(&app_data).map_err(|e| e.to_string())?;
            let db_path = app_data.join("vessel.db");
            let mut conn = db::open(&db_path).map_err(|e| e.to_string())?;
            roots::migrate_normalize_paths(&mut conn).map_err(|e| e.to_string())?;
            drop(conn);
            let config_path = app_data.join("config.json");
            let jobs: Arc<JobManager> = Arc::new(JobManager::new());
            app.manage(AppState {
                db_path,
                config_path,
                jobs,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            start_scan,
            cancel_job,
            get_indexing_status,
            search_files,
            open_file,
            reveal_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}