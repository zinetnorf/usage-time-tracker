use crate::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager, Wry};

pub struct TrayHandles {
    pub today: MenuItem<Wry>,
}

/// Tray con menú (§9): Abrir dashboard · Tiempo de hoy · Pausar/Reanudar ·
/// Salir. "Salir" es la única vía de terminar el proceso.
pub fn build(app: &App) -> tauri::Result<TrayHandles> {
    let open = MenuItem::with_id(app, "open", "Abrir dashboard", true, None::<&str>)?;
    let today = MenuItem::with_id(app, "today", "Hoy: 0m", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", pause_label(app.handle()), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &today,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &quit,
        ],
    )?;

    let pause_item = pause.clone();
    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(app.default_window_icon().expect("icono por defecto").clone())
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "pause" => {
                toggle_pause(app);
                let _ = pause_item.set_text(pause_label(app));
            }
            "quit" => quit_app(app),
            _ => {}
        })
        .build(app)?;

    Ok(TrayHandles { today })
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn pause_label(app: &AppHandle) -> &'static str {
    if tracking_paused(app) {
        "Reanudar tracking"
    } else {
        "Pausar tracking"
    }
}

fn tracking_paused(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| {
            let db = state.db.lock().ok()?;
            db.config_bool("tracking_paused").ok()
        })
        .unwrap_or(false)
}

/// Invierte la pausa vía config; el loop la aplica en ≤1 tick.
fn toggle_pause(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(db) = state.db.lock() {
            let paused = db.config_bool("tracking_paused").unwrap_or(false);
            let _ = db.set_config("tracking_paused", if paused { "false" } else { "true" });
        }
    }
}

fn quit_app(app: &AppHandle) {
    // Parar el loop primero: cierra el segmento en curso limpiamente.
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(handle) = state.run_loop.get() {
            handle.shutdown();
        }
    }
    app.exit(0);
}
