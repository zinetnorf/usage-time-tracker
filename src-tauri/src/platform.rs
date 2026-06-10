/// Raíz del bundle .app a partir del path del ejecutable (macOS, §11).
/// `/Apps/Safari.app/Contents/MacOS/Safari` → `/Apps/Safari.app`.
pub fn bundle_root(exe_path: &str) -> Option<String> {
    let idx = exe_path.find(".app/")?;
    Some(exe_path[..idx + 4].to_string())
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
