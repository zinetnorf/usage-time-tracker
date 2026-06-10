pub mod commands;
pub mod db;
pub mod platform;
pub mod runloop;
pub mod tracker;
pub mod tray;

use std::sync::{Mutex, OnceLock};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

/// Estado compartido con los comandos. El tracker loop usa su PROPIA
/// conexión SQLite (WAL permite lector+escritor concurrentes).
pub struct AppState {
    pub db: Mutex<db::Db>,
    pub run_loop: OnceLock<runloop::LoopHandle>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Segunda instancia: enfocar el dashboard existente (§9).
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("usage.db");
            let database = db::Db::open(&db_path)?;

            // Autostart según config (§7); antes del onboarding no se toca:
            // es el onboarding quien lo decide (§10).
            if database.config_bool("onboarding_done")? {
                let autolaunch = app.autolaunch();
                if database.config_bool("autostart_enabled")? {
                    let _ = autolaunch.enable();
                } else {
                    let _ = autolaunch.disable();
                }
            }

            app.manage(AppState {
                db: Mutex::new(database),
                run_loop: OnceLock::new(),
            });

            // En macOS la app vive en la barra de menú, sin icono en dock.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handles = tray::build(app)?;
            let today_item = handles.today;
            let loop_handle = runloop::spawn(db_path, move |label| {
                let _ = today_item.set_text(label);
            });
            let _ = app
                .state::<AppState>()
                .run_loop
                .set(loop_handle);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Cerrar la ventana oculta al tray; no termina la app (§9).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_summary,
            commands::get_day_summary,
            commands::get_range_totals,
            commands::get_range_summary,
            commands::get_apps,
            commands::rename_app,
            commands::merge_apps,
            commands::set_app_blacklisted,
            commands::get_settings,
            commands::set_setting,
            commands::get_onboarding,
            commands::open_accessibility_settings,
            commands::finish_onboarding,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
