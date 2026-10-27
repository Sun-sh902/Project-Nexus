use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::{
    menu::{Menu, PredefinedMenuItem, Submenu},
    Manager,
};

#[derive(Clone, Copy, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

/// Nexus ships in English only; the enum keeps the request contract explicit and bounded.
#[derive(Clone, Copy, Deserialize, PartialEq, Debug)]
pub enum LocalePreference {
    #[serde(rename = "en-US")]
    English,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferenceRequest {
    request_id: String,
    theme: ThemePreference,
    locale: LocalePreference,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceResponse {
    contract_version: u8,
    request_id: String,
    result: &'static str,
}

#[derive(Serialize, Debug)]
pub struct PreferenceError {
    code: &'static str,
    retryable: bool,
}

// Finite application-only Action Registry. No domain effects or arbitrary parameters.
enum ApplicationPreferenceAction {
    SetMenuLanguage(LocalePreference),
    SetWindowAppearance(ThemePreference),
}
impl ApplicationPreferenceAction {
    fn execute(self, window: &tauri::WebviewWindow) -> tauri::Result<()> {
        match self {
            Self::SetMenuLanguage(locale) => {
                window
                    .app_handle()
                    .set_menu(create_menu(window.app_handle(), locale)?)?;
            }
            Self::SetWindowAppearance(theme) => {
                let theme = match theme {
                    ThemePreference::Light => Some(tauri::Theme::Light),
                    ThemePreference::Dark => Some(tauri::Theme::Dark),
                    ThemePreference::System => None,
                };
                window.set_theme(theme)?;
                window.app_handle().set_theme(theme);
            }
        }
        Ok(())
    }
}

fn validate_request(request: &PreferenceRequest, window: &str) -> Result<(), PreferenceError> {
    if window != "main"
        || request.request_id.is_empty()
        || request.request_id.len() > 64
        || !request
            .request_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(PreferenceError {
            code: "app.preferences_denied",
            retryable: false,
        });
    }
    Ok(())
}

#[tauri::command]
pub fn apply_app_preferences(
    window: tauri::WebviewWindow,
    request: PreferenceRequest,
) -> Result<PreferenceResponse, PreferenceError> {
    validate_request(&request, window.label())?;
    for action in [
        ApplicationPreferenceAction::SetMenuLanguage(request.locale),
        ApplicationPreferenceAction::SetWindowAppearance(request.theme),
    ] {
        action.execute(&window).map_err(|_| PreferenceError {
            code: "app.preferences_partial",
            retryable: true,
        })?;
    }
    Ok(PreferenceResponse {
        contract_version: 1,
        request_id: request.request_id,
        result: "applied",
    })
}

fn catalog(locale: LocalePreference) -> BTreeMap<String, String> {
    let source = match locale {
        LocalePreference::English => include_str!("../../src/locales/en-US/common.json"),
    };
    serde_json::from_str(source).expect("Bundled translation catalog must be valid")
}

pub fn create_menu(
    app: &tauri::AppHandle,
    locale: LocalePreference,
) -> tauri::Result<Menu<tauri::Wry>> {
    let messages = catalog(locale);
    let text = |key: &str| {
        Some(
            messages
                .get(key)
                .expect("Native menu translations must exist")
                .as_str(),
        )
    };
    let edit = Submenu::new(app, messages.get("menuEdit").unwrap(), true)?;
    #[cfg(target_os = "macos")]
    edit.append_items(&[
        &PredefinedMenuItem::undo(app, text("menuUndo"))?,
        &PredefinedMenuItem::redo(app, text("menuRedo"))?,
        &PredefinedMenuItem::separator(app)?,
    ])?;
    edit.append_items(&[
        &PredefinedMenuItem::cut(app, text("menuCut"))?,
        &PredefinedMenuItem::copy(app, text("menuCopy"))?,
        &PredefinedMenuItem::paste(app, text("menuPaste"))?,
        &PredefinedMenuItem::select_all(app, text("menuSelectAll"))?,
    ])?;
    let file = Submenu::with_items(
        app,
        messages.get("menuFile").unwrap(),
        true,
        &[&PredefinedMenuItem::close_window(app, text("menuClose"))?],
    )?;
    let window = Submenu::new(app, messages.get("menuWindow").unwrap(), true)?;
    window.append_items(&[
        &PredefinedMenuItem::minimize(app, text("menuMinimize"))?,
        &PredefinedMenuItem::maximize(app, text("menuMaximize"))?,
    ])?;
    #[cfg(target_os = "macos")]
    window.append(&PredefinedMenuItem::fullscreen(
        app,
        text("menuFullscreen"),
    )?)?;
    let menu = Menu::new(app)?;
    #[cfg(target_os = "macos")]
    menu.append(&Submenu::with_items(
        app,
        "Project Nexus",
        true,
        &[
            &PredefinedMenuItem::hide(app, text("menuHide"))?,
            &PredefinedMenuItem::hide_others(app, text("menuHideOthers"))?,
            &PredefinedMenuItem::show_all(app, text("menuShowAll"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, text("menuQuit"))?,
        ],
    )?)?;
    #[cfg(not(target_os = "macos"))]
    file.append(&PredefinedMenuItem::quit(app, text("menuQuit"))?)?;
    menu.append_items(&[&file, &edit, &window])?;
    Ok(menu)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_denies_unknown_fields_invalid_enums_windows_and_oversized_ids() {
        for input in [
            r#"{"requestId":"ok","theme":"dark","locale":"en-US","path":"/tmp"}"#,
            r#"{"requestId":"ok","theme":"execute","locale":"en-US"}"#,
            r#"{"requestId":"ok","theme":"dark","locale":"fr-FR"}"#,
            r#"{"requestId":"ok","theme":"dark","locale":"de-DE"}"#,
        ] {
            assert!(serde_json::from_str::<PreferenceRequest>(input).is_err());
        }
        let mut request: PreferenceRequest =
            serde_json::from_str(r#"{"requestId":"abc-123","theme":"dark","locale":"en-US"}"#)
                .unwrap();
        assert!(validate_request(&request, "main").is_ok());
        assert!(validate_request(&request, "other").is_err());
        request.request_id = "a".repeat(65);
        assert!(validate_request(&request, "main").is_err());
        request.request_id = "../bad".into();
        assert!(validate_request(&request, "main").is_err());
    }
    #[test]
    fn native_menu_catalog_is_complete() {
        let english = catalog(LocalePreference::English);
        for key in [
            "menuFile",
            "menuEdit",
            "menuWindow",
            "menuClose",
            "menuQuit",
            "menuCopy",
            "menuPaste",
            "menuCut",
            "menuSelectAll",
            "menuUndo",
            "menuRedo",
            "menuHide",
            "menuHideOthers",
            "menuShowAll",
            "menuFullscreen",
            "menuMinimize",
            "menuMaximize",
        ] {
            assert!(!english[key].is_empty());
        }
    }
}
