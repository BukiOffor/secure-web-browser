use crate::utils::types::{ModuleError, ServerValidatorResponse};
use crate::{utils, InitState};
use serde::Deserialize;
use serde_json::json;
use tauri::Emitter;
use tauri::{menu::MenuBuilder, Manager, Url};
use tauri_plugin_http::reqwest;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn set_server(app: tauri::AppHandle, url: String) -> Result<(), ModuleError> {
    println!("🚨 Request Logged!");
    let response = reqwest::get(format!("{}:8080/validate", url.clone())).await?;
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
        Err(ModuleError::Internal("Unexpected error".into()))
    }
}

#[tauri::command]
pub fn server_url(app: tauri::AppHandle) -> Result<Option<String>, ModuleError> {
    let store = app
        .store("store.json")
        .map_err(|e| ModuleError::Internal(e.to_string()))?;

    if let Some(value) = store.get("url") {
        println!("{}", value);
        let server_url = value
            .as_object()
            .unwrap()
            .get("value")
            .unwrap()
            .as_str()
            .unwrap();

        return Ok(Some(server_url.to_string()));
    };
    Ok(None)
}
