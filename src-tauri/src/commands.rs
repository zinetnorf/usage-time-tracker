use crate::db::{local_day, AppDayUsage, AppRow, DayTotals};
use crate::AppState;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

/// Claves de configuración expuestas a la UI (§7 + pausa).
const CONFIG_KEYS: &[&str] = &[
    "idle_threshold_sec",
    "count_idle_as_usage",
    "track_window_titles",
    "poll_interval_ms",
    "flush_interval_sec",
    "raw_retention_days",
    "autostart_enabled",
    "language",
    "top_apps_count",
    "tracking_paused",
    "onboarding_done",
];

#[derive(serde::Serialize)]
pub struct OnboardingStatus {
    pub done: bool,
    pub is_macos: bool,
    pub accessibility_granted: bool,
}

fn err_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[tauri::command]
pub fn get_today_summary(state: State<AppState>) -> Result<Vec<AppDayUsage>, String> {
    let db = state.db.lock().map_err(err_str)?;
    let today = local_day(chrono::Local::now().timestamp());
    db.day_summary(&today).map_err(err_str)
}

#[tauri::command]
pub fn get_day_summary(state: State<AppState>, day: String) -> Result<Vec<AppDayUsage>, String> {
    let db = state.db.lock().map_err(err_str)?;
    db.day_summary(&day).map_err(err_str)
}

#[tauri::command]
pub fn get_range_totals(
    state: State<AppState>,
    from_day: String,
    to_day: String,
) -> Result<Vec<DayTotals>, String> {
    let db = state.db.lock().map_err(err_str)?;
    db.range_totals(&from_day, &to_day).map_err(err_str)
}

#[tauri::command]
pub fn get_range_summary(
    state: State<AppState>,
    from_day: String,
    to_day: String,
) -> Result<Vec<AppDayUsage>, String> {
    let db = state.db.lock().map_err(err_str)?;
    db.range_summary(&from_day, &to_day).map_err(err_str)
}

#[tauri::command]
pub fn get_apps(state: State<AppState>) -> Result<Vec<AppRow>, String> {
    let db = state.db.lock().map_err(err_str)?;
    db.list_apps().map_err(err_str)
}

#[tauri::command]
pub fn rename_app(state: State<AppState>, app_id: i64, name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(err_str)?;
    db.rename_app(app_id, &name).map_err(err_str)
}

#[tauri::command]
pub fn merge_apps(state: State<AppState>, from_id: i64, into_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(err_str)?;
    db.merge_apps(from_id, into_id).map_err(err_str)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<std::collections::HashMap<String, String>, String> {
    let db = state.db.lock().map_err(err_str)?;
    let mut out = std::collections::HashMap::new();
    for key in CONFIG_KEYS {
        out.insert((*key).to_string(), db.config_str(key).map_err(err_str)?);
    }
    Ok(out)
}

#[tauri::command]
pub fn set_app_blacklisted(
    state: State<AppState>,
    app_id: i64,
    blacklisted: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(err_str)?;
    db.set_blacklisted(app_id, blacklisted).map_err(err_str)
}

#[tauri::command]
pub fn get_onboarding(state: State<AppState>) -> Result<OnboardingStatus, String> {
    let db = state.db.lock().map_err(err_str)?;
    Ok(OnboardingStatus {
        done: db.config_bool("onboarding_done").map_err(err_str)?,
        is_macos: cfg!(target_os = "macos"),
        accessibility_granted: crate::platform::accessibility_granted(),
    })
}

/// Abre el panel de Accesibilidad de Ajustes del sistema (§10).
#[tauri::command]
pub fn open_accessibility_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            None::<&str>,
        )
        .map_err(err_str)
}

#[tauri::command]
pub fn finish_onboarding(
    app: tauri::AppHandle,
    state: State<AppState>,
    autostart: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(err_str)?;
    db.set_config("onboarding_done", "true").map_err(err_str)?;
    db.set_config("autostart_enabled", if autostart { "true" } else { "false" })
        .map_err(err_str)?;
    let autolaunch = app.autolaunch();
    let result = if autostart {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(err_str)
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    if !CONFIG_KEYS.contains(&key.as_str()) {
        return Err(format!("clave de configuración desconocida: {key}"));
    }
    let db = state.db.lock().map_err(err_str)?;
    db.set_config(&key, &value).map_err(err_str)
}
