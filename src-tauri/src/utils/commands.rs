use crate::utils::types::{ModuleError, ServerValidatorResponse};
use crate::{utils, InitState};
use serde::Deserialize;
use serde_json::json;
use tauri::Emitter;
use tauri::{menu::MenuBuilder, Manager, Url};
use tauri_plugin_http::reqwest;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn set_server(app: tauri::AppHandle, url: String) -> Result<(), ModuleError> {
    log::info!("🚨 Request Logged!");
    let server_url = format!("{}:8080/validate", url.clone());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ModuleError::RequsetError(e))?;

    let response = client
        .get(&server_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        let server_response: ServerValidatorResponse =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let new_url =
            Url::parse(&server_response.ip_addr).map_err(|e| format!("Invalid URL: {}", e))?;

        utils::navigate_and_adjust_window(&app, new_url)?;

        let store = app
            .store("store.json")
            .map_err(|e| ModuleError::Internal(format!("Failed to get store: {}", e)))?;
        store.set("url", json!({"value": url}));
        store
            .save()
            .map_err(|e| ModuleError::Internal(format!("Failed to save store: {}", e)))?;

        {
            let state = app.state::<InitState>();
            let mut state_guard = state
                .0
                .write()
                .map_err(|e| ModuleError::Internal(format!("Failed to lock InitState: {}", e)))?;
            *state_guard = true;
        }
        Ok(())
    } else if status.is_client_error() {
        Err(ModuleError::Internal("Request was incorrect".into()))
    } else {
        Err(ModuleError::Internal("Something Went Wrong".into()))
    }
}

#[tauri::command]
pub fn server_url(app: tauri::AppHandle) -> Result<Option<String>, ModuleError> {
    match get_server_url(&app) {
        Ok(url) => Ok(Some(url)),
        Err(ModuleError::Internal(msg)) if msg == "Couldn't get server url" => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_server_url(app: &tauri::AppHandle) -> Result<String, ModuleError> {
    let store = app
        .store("store.json")
        .map_err(|e| ModuleError::Internal(e.to_string()))?;

    let url_result = store
        .get("url")
        .and_then(|v| v.as_object().cloned())
        .and_then(|o| o.get("value").cloned())
        .and_then(|v| Some(v))
        .ok_or_else(|| {
            log::error!("❌ ❌ ❌ Couldn't find url in store");
            ModuleError::Internal("Couldn't find url in store".into())
        })?;
    let server_url = url_result
        .as_str()
        .ok_or(ModuleError::Internal("Couldn't get server url".into()))?;

    return Ok(server_url.to_string());
}

#[tauri::command]
pub async fn exit_exam(app: tauri::AppHandle, password: String) -> Result<bool, ModuleError> {
    let store = app
        .store("store.json")
        .map_err(|e| ModuleError::Internal(e.to_string()))?;

    if let Some(value) = store.get("password") {
        let state_password = match value.as_object() {
            Some(obj) => match obj.get("value") {
                Some(val) => match val.as_str() {
                    Some(s) => s,
                    None => {
                        log::error!("❌ ❌ ❌ Failed to convert 'value' to str");
                        return Err(ModuleError::Internal("Couldn't get server password".into()));
                    }
                },
                None => {
                    log::error!("❌ ❌ ❌ 'value' key not found in password object");
                    return Err(ModuleError::Internal("Couldn't get server password".into()));
                }
            },
            None => {
                log::error!("❌ ❌ ❌ Failed to convert password to object");
                return Err(ModuleError::Internal("Couldn't get server password".into()));
            }
        };
        if state_password.eq(&password) {
            app.notification()
                .builder()
                .title("Exiting")
                .body("A user has requested exit and app will shut down in 5 seconds")
                .show()
                .unwrap_or_else(|error| {
                    log::error!("❌ ❌ ❌ Error Sending Notification: {}", error)
                });

            return Ok(true);
        } else {
            Ok(false)
        }
    } else {
        utils::query_password_for_server(&app).await?;
        log::error!("Couldn't find password in store, making a new request ...");
        Err(ModuleError::Internal(
            "Couldn't find password in store".into(),
        ))
    }
}
