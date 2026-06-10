pub mod commands;
pub mod db;
pub mod platform;
pub mod tracker;

use std::sync::Mutex;
use tauri::Manager;

/// Estado compartido con los comandos. El tracker loop usará su PROPIA
/// conexión SQLite (WAL permite lector+escritor concurrentes).
pub struct AppState {
    pub db: Mutex<db::Db>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database = db::Db::open(&data_dir.join("usage.db"))?;
            app.manage(AppState {
                db: Mutex::new(database),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_summary,
            commands::get_day_summary,
            commands::get_range_totals,
            commands::get_range_summary,
            commands::get_apps,
            commands::rename_app,
            commands::merge_apps,
            commands::get_settings,
            commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
