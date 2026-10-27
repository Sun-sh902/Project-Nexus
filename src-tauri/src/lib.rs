use serde::Serialize;
use tauri::Manager;
mod app_preferences;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    contract_version: u8,
    version: &'static str,
    preferred_locale: Option<String>,
}

// No inputs and no domain observations. ACL admits only the bundled main UI.
#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        contract_version: 1,
        version: env!("CARGO_PKG_VERSION"),
        preferred_locale: sys_locale::get_locale(),
    }
}

pub fn run() {
    tauri::Builder::default()
        .menu(|app| {
            let preferred = sys_locale::get_locale()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let locale = if preferred == "zh"
                || preferred.starts_with("zh-")
                || preferred.starts_with("zh_")
            {
                app_preferences::LocalePreference::Chinese
            } else {
                app_preferences::LocalePreference::English
            };
            app_preferences::create_menu(app, locale)
        })
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            app_preferences::apply_app_preferences
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // Never force the practical minimum beyond the monitor work area.
                if let Some(monitor) = window.current_monitor()? {
                    let area = monitor.work_area();
                    let size = area.size.to_logical::<f64>(monitor.scale_factor());
                    let width = size.width.min(1200.0);
                    let height = (size.height - 40.0).clamp(1.0, 800.0);
                    window.set_min_size(Some(tauri::LogicalSize::new(
                        width.min(960.0),
                        height.min(640.0),
                    )))?;
                    window.set_size(tauri::LogicalSize::new(width, height))?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Project Nexus desktop runtime failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_is_versioned_and_contains_only_application_facts() {
        let metadata = super::app_metadata();
        assert_eq!(metadata.contract_version, 1);
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
        if let Some(locale) = metadata.preferred_locale {
            assert!(!locale.is_empty());
        }
    }
}
