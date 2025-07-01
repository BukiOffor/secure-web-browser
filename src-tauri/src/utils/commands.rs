use crate::utils::types::{ModuleError, ServerValidatorResponse};
use serde::Deserialize;
use serde_json::json;
use tauri::{menu::MenuBuilder, Manager, Url};
use tauri_plugin_http::reqwest;
use tauri_plugin_store::StoreExt;
use crate::InitState;

#[tauri::command]
pub async fn set_server(app: tauri::AppHandle, url: String) -> Result<(), ModuleError> {
    println!("🚨 Request Logged!");
    let response = reqwest::get(url).await?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        let server_response: ServerValidatorResponse =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let url =
            Url::parse(&server_response.ip_addr).map_err(|e| format!("Invalid URL: {}", e))?;
        let window = app
            .app_handle()
            .get_webview_window("main")
            .ok_or(ModuleError::Internal("Couldn't get web window".into()))?;

        window
            .set_fullscreen(true)
            .map_err(|e| ModuleError::Internal(format!("Failed to set fullscreen: {}", e)))?;
        window
            .set_decorations(false)
            .map_err(|e| ModuleError::Internal(format!("Failed to set decorations: {}", e)))?;
        window
            .set_always_on_top(true)
            .map_err(|e| ModuleError::Internal(format!("Failed to set always on top: {}", e)))?;
        window
            .set_resizable(false)
            .map_err(|e| ModuleError::Internal(format!("Failed to set resizable: {}", e)))?;
        let menu = MenuBuilder::new(app.app_handle())
            .build()
            .map_err(|e| ModuleError::Internal(format!("Failed to build menu: {}", e)))?;
        window
            .set_menu(menu)
            .map_err(|e| ModuleError::Internal(format!("Failed to set menu: {}", e)))?;
        window
            .set_skip_taskbar(true)
            .map_err(|e| ModuleError::Internal(format!("Failed to set skip taskbar: {}", e)))?;
        window.set_visible_on_all_workspaces(true).map_err(|e| {
            ModuleError::Internal(format!("Failed to set visible on all workspaces: {}", e))
        })?;
        // prevent app from screen sharing
        window.set_content_protected(true).map_err(|e| {
            ModuleError::Internal(format!("Failed to set content protected: {}", e))
        })?;
        window
            .navigate(url)
            .map_err(|e| ModuleError::Internal(format!("Failed to navigate to url: {}", e)))?;
    

        let store = app.store("store.json").map_err(|e| {
            ModuleError::Internal(format!("Failed to get store: {}", e))
        })?;
        store.set("url", json!({"value": server_response.ip_addr}));
        store.save().unwrap();
        {
            let state = app.state::<InitState>();
            let mut state_guard = state.0.write().map_err(|e| ModuleError::Internal(format!("Failed to lock InitState: {}", e)))?;
            *state_guard = true;            
        }
        Ok(())
    } else if status.is_client_error() {
        Err(ModuleError::Internal("Request was incorrect".into()))
    } else {
        Err(ModuleError::Internal("Unexpected error".into()))
    }
}
