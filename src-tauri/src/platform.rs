/// Raíz del bundle .app a partir del path reportado (macOS, §11).
/// Acepta el ejecutable interno (`…/X.app/Contents/MacOS/X`) o el bundle
/// mismo (`…/X.app`), que es lo que reporta x-win.
pub fn bundle_root(exe_path: &str) -> Option<String> {
    if let Some(idx) = exe_path.find(".app/") {
        return Some(exe_path[..idx + 4].to_string());
    }
    if exe_path.ends_with(".app") {
        return Some(exe_path.to_string());
    }
    None
}

/// CFBundleIdentifier desde `<bundle>/Contents/Info.plist` (macOS, §11).
pub fn bundle_id_from_app(bundle_path: &str) -> Option<String> {
    let plist_path = std::path::Path::new(bundle_path).join("Contents/Info.plist");
    let value = plist::Value::from_file(plist_path).ok()?;
    value
        .as_dictionary()?
        .get("CFBundleIdentifier")?
        .as_string()
        .map(str::to_string)
}

use crate::db::AppInfo;
use crate::tracker::{Observation, ObservedWindow};
use std::collections::HashMap;

/// Observación con strings propios (x-win devuelve owned); se convierte
/// a `Observation` prestada para el tracker.
pub struct PolledTick {
    pub window: Option<OwnedWindow>,
    pub idle_seconds: i64,
    pub now_ts: i64,
}

pub struct OwnedWindow {
    pub resolved: ResolvedApp,
    pub process_name: String,
    pub exe_path: String,
    pub title: Option<String>,
}

impl PolledTick {
    pub fn as_observation(&self) -> Observation<'_> {
        Observation {
            window: self.window.as_ref().map(|w| ObservedWindow {
                app: AppInfo {
                    identity: &w.resolved.identity,
                    display_name: &w.resolved.display_name,
                    process_name: Some(&w.process_name),
                    exe_path: Some(&w.exe_path),
                    bundle_id: w.resolved.bundle_id.as_deref(),
                },
                title: w.title.as_deref(),
            }),
            idle_seconds: self.idle_seconds,
            now_ts: self.now_ts,
        }
    }
}

/// Poller de plataforma: ventana activa (x-win) + idle del sistema
/// (user-idle). Cachea bundle ids por bundle para no releer Info.plist.
#[derive(Default)]
pub struct Poller {
    bundle_ids: HashMap<String, Option<String>>,
}

impl Poller {
    pub fn poll(&mut self, now_ts: i64, track_titles: bool) -> PolledTick {
        let idle_seconds = user_idle::UserIdle::get_time()
            .map(|t| t.as_seconds() as i64)
            .unwrap_or(0);

        let window = x_win::get_active_window().ok().and_then(|w| {
            if w.info.path.is_empty() && w.info.exec_name.is_empty() {
                return None;
            }
            let resolved = self.resolve(&w.info.path, &w.info.exec_name, &w.info.name);
            let title = if track_titles && !w.title.is_empty() {
                Some(w.title)
            } else {
                None
            };
            Some(OwnedWindow {
                resolved,
                process_name: w.info.exec_name,
                exe_path: w.info.path,
                title,
            })
        });

        PolledTick {
            window,
            idle_seconds,
            now_ts,
        }
    }

    #[cfg(target_os = "macos")]
    fn resolve(&mut self, exe_path: &str, _exec_name: &str, app_name: &str) -> ResolvedApp {
        let bundle_id = bundle_root(exe_path).and_then(|root| {
            self.bundle_ids
                .entry(root.clone())
                .or_insert_with(|| bundle_id_from_app(&root))
                .clone()
        });
        resolve_macos(exe_path, bundle_id, app_name)
    }

    #[cfg(target_os = "windows")]
    fn resolve(&mut self, exe_path: &str, exec_name: &str, app_name: &str) -> ResolvedApp {
        resolve_windows(exe_path, exec_name, app_name)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn resolve(&mut self, exe_path: &str, exec_name: &str, app_name: &str) -> ResolvedApp {
        resolve_windows(exe_path, exec_name, app_name)
    }
}

/// ¿Sesión bloqueada? Se sondea cada tick (§5.3) — sin plumbing de
/// eventos; la suspensión la cubre el fallback de gap de reloj.
#[cfg(target_os = "macos")]
pub fn is_session_locked() -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
        fn CFBooleanGetValue(b: *const std::ffi::c_void) -> bool;
    }

    unsafe {
        let dict_ref = CGSessionCopyCurrentDictionary();
        if dict_ref.is_null() {
            // Sin sesión gráfica (p. ej. SSH): tratar como bloqueada.
            return true;
        }
        let dict: CFDictionary = CFDictionary::wrap_under_create_rule(dict_ref);
        let key = CFString::from_static_string("CGSSessionScreenIsLocked");
        match dict.find(key.as_concrete_TypeRef() as *const std::ffi::c_void) {
            Some(value) => CFBooleanGetValue(*value),
            None => false,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn is_session_locked() -> bool {
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, OpenInputDesktop, DESKTOP_READOBJECTS, DF_ALLOWOTHERACCOUNTHOOK,
    };
    // Con la sesión bloqueada el input desktop es el secure desktop y
    // OpenInputDesktop falla para procesos normales.
    unsafe {
        match OpenInputDesktop(DF_ALLOWOTHERACCOUNTHOOK, false, DESKTOP_READOBJECTS) {
            Ok(handle) => {
                let _ = CloseDesktop(handle);
                false
            }
            Err(_) => true,
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn is_session_locked() -> bool {
    false
}

/// ¿Tiene la app permiso de Accesibilidad? (macOS, §10). Sin él los
/// títulos de ventana llegan vacíos pero el conteo por app funciona.
#[cfg(target_os = "macos")]
pub fn accessibility_granted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_granted() -> bool {
    true
}

/// Identidad resuelta de la app observada (§11).
#[derive(Debug, Clone)]
pub struct ResolvedApp {
    pub identity: String,
    pub display_name: String,
    pub bundle_id: Option<String>,
}

/// macOS: identity = bundle_id > raíz del bundle > path del ejecutable.
pub fn resolve_macos(exe_path: &str, bundle_id: Option<String>, app_name: &str) -> ResolvedApp {
    let identity = bundle_id
        .clone()
        .or_else(|| bundle_root(exe_path))
        .unwrap_or_else(|| exe_path.to_string());
    ResolvedApp {
        identity,
        display_name: display_name_or(app_name, exe_path),
        bundle_id,
    }
}

/// Windows: identity = exe_path normalizado a minúsculas (FS case-insensitive).
/// Apps UWP detrás de ApplicationFrameHost se agrupan legibles (MVP §11).
pub fn resolve_windows(exe_path: &str, exec_name: &str, app_name: &str) -> ResolvedApp {
    if exec_name.eq_ignore_ascii_case("ApplicationFrameHost.exe") {
        return ResolvedApp {
            identity: "uwp:aplicación-uwp".to_string(),
            display_name: "Aplicación UWP".to_string(),
            bundle_id: None,
        };
    }
    ResolvedApp {
        identity: exe_path.to_lowercase(),
        display_name: display_name_or(app_name, exec_name),
        bundle_id: None,
    }
}

fn display_name_or(app_name: &str, fallback: &str) -> String {
    if app_name.trim().is_empty() {
        fallback.to_string()
    } else {
        app_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_root_extracts_app_bundle_path() {
        assert_eq!(
            bundle_root("/Applications/Safari.app/Contents/MacOS/Safari"),
            Some("/Applications/Safari.app".to_string())
        );
        assert_eq!(
            bundle_root("/Applications/Visual Studio Code.app/Contents/MacOS/Electron"),
            Some("/Applications/Visual Studio Code.app".to_string())
        );
    }

    #[test]
    fn bundle_root_accepts_bundle_path_itself() {
        // x-win en macOS reporta el bundle como path, sin /Contents/...
        assert_eq!(
            bundle_root("/Applications/Cursor.app"),
            Some("/Applications/Cursor.app".to_string())
        );
    }

    #[test]
    fn bundle_root_none_for_plain_binaries() {
        assert_eq!(bundle_root("/usr/local/bin/htop"), None);
        assert_eq!(bundle_root(""), None);
    }

    #[test]
    fn bundle_id_read_from_info_plist() {
        let dir = std::env::temp_dir().join(format!("ut-test-{}", std::process::id()));
        let contents = dir.join("Fake.app/Contents");
        std::fs::create_dir_all(&contents).unwrap();
        std::fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.example.fake</string>
</dict>
</plist>"#,
        )
        .unwrap();

        let app_bundle = dir.join("Fake.app");
        assert_eq!(
            bundle_id_from_app(app_bundle.to_str().unwrap()),
            Some("com.example.fake".to_string())
        );
        assert_eq!(bundle_id_from_app("/no/existe/X.app"), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn macos_identity_prefers_bundle_id_then_bundle_then_path() {
        let r = resolve_macos(
            "/Apps/Safari.app/Contents/MacOS/Safari",
            Some("com.apple.Safari".into()),
            "Safari",
        );
        assert_eq!(r.identity, "com.apple.Safari");
        assert_eq!(r.display_name, "Safari");
        assert_eq!(r.bundle_id.as_deref(), Some("com.apple.Safari"));

        let r = resolve_macos("/Apps/Safari.app/Contents/MacOS/Safari", None, "Safari");
        assert_eq!(r.identity, "/Apps/Safari.app", "sin plist cae al bundle");

        let r = resolve_macos("/usr/local/bin/htop", None, "htop");
        assert_eq!(r.identity, "/usr/local/bin/htop", "binario suelto usa path");
    }

    #[test]
    fn windows_identity_is_lowercased_exe_path() {
        let r = resolve_windows(
            "C:\\Program Files\\VS Code\\Code.exe",
            "Code.exe",
            "Visual Studio Code",
        );
        assert_eq!(r.identity, "c:\\program files\\vs code\\code.exe");
        assert_eq!(r.display_name, "Visual Studio Code");
        assert_eq!(r.bundle_id, None);
    }

    #[test]
    fn windows_uwp_host_groups_under_readable_name() {
        // §11: ApplicationFrameHost agrupa apps UWP; agrupar legible en MVP.
        let r = resolve_windows(
            "C:\\Windows\\System32\\ApplicationFrameHost.exe",
            "ApplicationFrameHost.exe",
            "Calculadora",
        );
        assert_eq!(r.identity, "uwp:aplicación-uwp");
        assert_eq!(r.display_name, "Aplicación UWP");
    }

    #[test]
    fn empty_app_name_falls_back_to_exec_name() {
        let r = resolve_windows("C:\\x\\foo.exe", "foo.exe", "");
        assert_eq!(r.display_name, "foo.exe");
    }
}
