use std::process::Command;
pub mod commands;
pub mod types;
use crate::utils::commands::get_server_url;
use crate::utils::types::{
    HostInfo, ModuleError, PortStatus, ProcessIdentifier, RawUdpEndpoint, ServerValidatorResponse,
    USBDevice, UdpEndpoint, WebRtcReport,
};
use crate::{AppState, InitState};
use mac_address::get_mac_address;
use serde::Serialize;
use serde_json::json;
use std::net::UdpSocket;
use std::str;
use sysinfo::System;
use tauri::menu::MenuBuilder;
use tauri::{AppHandle, Emitter};
use tauri::{Manager, Url};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutEvent, ShortcutState};
use tauri_plugin_http::reqwest;
use tauri_plugin_store::StoreExt;

const INPUT_KEYWORDS: [&str; 2] = ["mass storage", "hard disk"];

pub fn build_bindings(
    app: &AppHandle,
    shortcut: &Shortcut,
    event: &ShortcutEvent,
    kill_binding: &Shortcut,
    cltr_alt_delete_shortcut: &Shortcut,
    minimized_shortcut: &Shortcut,
) {
    // Handle the global shortcuts
    log::info!("{:?}", shortcut);
    if shortcut == kill_binding {
        match event.state() {
            ShortcutState::Pressed => {
                log::info!("Ctrl-K Pressed!");
            }
            ShortcutState::Released => {
                log::info!("Ctrl-K Released!");
                app.emit("start::exit", ())
                    .expect("Failed to emit show-password-prompt");
                let state = app.state::<InitState>();
                let state = state.0.read();
                if let Ok(result) = state {
                    if *result {
                        app.exit(0);
                    }
                }
            }
        }
    } else if shortcut == cltr_alt_delete_shortcut {
        match event.state() {
            ShortcutState::Pressed => {
                log::info!("Ctrl+Alt+Delete Pressed!");
            }
            ShortcutState::Released => {
                log::info!("Ctrl+Alt+Delete Released!");
                app.emit("show-ctrl-alt-delete-prompt", ())
                    .expect("Failed to emit show-ctrl-alt-delete-prompt");
            }
        }
    } else if shortcut == minimized_shortcut {
        match event.state() {
            ShortcutState::Pressed => {
                log::info!("Super+D Pressed!");
            }
            ShortcutState::Released => {
                log::info!("Super+D Released!");
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn is_virtual_machine() -> bool {
    let cpuid = raw_cpuid::CpuId::new();
    if let Some(vf) = cpuid.get_vendor_info() {
        let vendor = vf.as_str().to_lowercase();
        return vendor.contains("vmware")
            || vendor.contains("virtualbox")
            || vendor.contains("qemu")
            || vendor.contains("kvm")
            || vendor.contains("microsoft hyper-v");
    }

    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_virtual_machine() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn is_running_in_rdp() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
    use windows::Win32::UI::WindowsAndMessaging::SM_REMOTESESSION;

    unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn is_running_in_rdp() -> bool {
    false
}

pub fn get_windows_serial() -> Option<String> {
    let output = Command::new("wmic")
        .args(&["bios", "get", "serialnumber"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.trim().is_empty() || line.contains("SerialNumber") {
            continue;
        }
        return Some(line.trim().to_string());
    }

    None
}

pub fn get_macos_serial() -> Option<String> {
    let output = Command::new("system_profiler")
        .args(["SPHardwareDataType"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.trim_start().starts_with("Serial Number") {
            return Some(line.split(':').nth(1)?.trim().to_string());
        }
    }

    None
}

pub fn get_linux_serial() -> Option<String> {
    let output = Command::new("cat")
        .args(&["/sys/class/dmi/id/product_serial"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        return Some(line.trim().to_string());
    }

    None
}

pub fn get_host_info() -> HostInfo {
    let mut host_info = HostInfo::default();
    let mac_address = get_mac_address()
        .ok()
        .and_then(|mac| mac.map(|m| m.to_string()));
    host_info.mac_address = mac_address;
    host_info.processor_id = Some(std::process::id().to_string());
    host_info.os = std::env::consts::OS.to_string();
    host_info.arch = std::env::consts::ARCH.to_string();
    #[cfg(target_os = "windows")]
    {
        host_info.serial_number = get_windows_serial();
    }
    #[cfg(target_os = "macos")]
    {
        host_info.serial_number = get_macos_serial();
    }
    #[cfg(target_os = "linux")]
    {
        host_info.serial_number = get_linux_serial();
    }
    host_info
}

pub fn is_disallowed_device_connected() -> Vec<USBDevice> {
    let connected_devices = usb_enumeration::enumerate(None, None);
    connected_devices
        .into_iter()
        .filter(|device| {
            if let Some(description) = &device.description {
                INPUT_KEYWORDS
                    .iter()
                    .any(|keyword| description.to_lowercase().contains(keyword))
            } else {
                false
            }
        })
        .map(|device| USBDevice {
            id: device.id.to_string(),
            vendor_id: device.vendor_id,
            product_id: device.product_id,
            description: device.description.clone(),
            serial_number: device.serial_number.clone(),
        })
        .collect()
}

#[cfg(target_os = "windows")]
pub fn disable_cad_actions(enable: bool) -> std::io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    // Disable SignOut button on Action Card
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let explorer_key =
            hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer")?;
        let value = if enable { 1u32 } else { 0u32 };
        explorer_key.0.set_value("NoLogoff", &value)?; // 1 means disable
        println!("Sign out disabled successfully.");
    }

    // Disable Actions on Local Machine
    {
        let hkcu = RegKey::predef(HKEY_LOCAL_MACHINE);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
        let system_policies = hkcu.create_subkey(path)?;

        let value = if enable { 1u32 } else { 0u32 };

        system_policies.0.set_value("DisableTaskMgr", &value)?;
        system_policies
            .0
            .set_value("DisableLockWorkstation", &value)?;
        system_policies.0.set_value("DisableLogoff", &value)?;
        system_policies
            .0
            .set_value("DisableChangePassword", &value)?;
        system_policies
            .0
            .set_value("HideFastUserSwitching", &value)?;
    }

    // Disable Actions for User
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
        let system_policies = hkcu.create_subkey(path)?;

        let value = if enable { 1u32 } else { 0u32 };

        system_policies.0.set_value("DisableTaskMgr", &value)?;
        system_policies
            .0
            .set_value("DisableLockWorkstation", &value)?;
        system_policies.0.set_value("DisableLogoff", &value)?;
        system_policies
            .0
            .set_value("DisableChangePassword", &value)?;
        system_policies
            .0
            .set_value("HideFastUserSwitching", &value)?;
    }

    Ok(())
}

pub fn navigate_and_adjust_window(app: &tauri::AppHandle, url: Url) -> Result<(), ModuleError> {
    // let url =
    //     Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;

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
    window
        .set_content_protected(true)
        .map_err(|e| ModuleError::Internal(format!("Failed to set content protected: {}", e)))?;
    window
        .navigate(url)
        .map_err(|e| ModuleError::Internal(format!("Failed to navigate to url: {}", e)))?;
    Ok(())
}

pub async fn query_password_for_server(app: &AppHandle) -> Result<(), ModuleError> {
    log::info!("🚨 Request for Pasword Logged!");
    let url = get_server_url(&app)?;
    let response = reqwest::get(format!("{}:8080/password", url)).await?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        let response: types::PasswordResponse =
            serde_json::from_str(&body).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let password = response.message;
        let store = app
            .store("store.json")
            .map_err(|e| ModuleError::Internal(format!("Failed to get store: {}", e)))?;
        if let Some(value) = store.get("password") {
            let old_password = value
                .as_object()
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap();
            if old_password.eq(&password) {
                log::info!("Password has not changed, sleeping ...");
                return Ok(());
            }
        }
        log::info!("Password has changed, Writing new password ...");
        store.set("password", json!({"value": password}));
        store
            .save()
            .map_err(|e| ModuleError::Internal(format!("Failed to save store: {}", e)))?;
        Ok(())
    } else {
        Err(ModuleError::Internal("Request to Server failed".into()))
    }
}
pub fn get_current_display() -> Result<Vec<String>, ModuleError> {
    #[cfg(target_os = "windows")]
    {
        // Define the PowerShell command
        let ps_script = r#"Get-WmiObject -Namespace root\wmi -Query "Select * from WmiMonitorConnectionParams""#;
        // Run PowerShell
        let output = Command::new("powershell")
            .args(["-Command", ps_script])
            .output()
            .map_err(|e| ModuleError::Internal(format!("Failed to execute PowerShell: {}", e)))?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Split output into blocks per display
            let display_blocks: Vec<&str> = stdout
                .split("\r\n\r\n") // double newline between objects
                .filter(|block| block.to_string().contains("InstanceName"))
                .collect();
            let count = display_blocks.len();
            log::info!("Number of connected displays: {}", count);
            Ok(display_blocks.into_iter().map(|s| s.to_string()).collect())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("PowerShell error:\n{}", stderr);
            Ok(vec![])
        }
    }
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::CGDisplay;
        let display_count = CGDisplay::active_displays()
            .map_err(|e| ModuleError::Internal(format!("Could not get displays: {}", e)))?;

        let flag = display_count.iter().any(|id| {
            let screen = CGDisplay::new(*id);
            !screen.is_builtin() && screen.is_active()
        });

        if flag {
            let output = Command::new("system_profiler")
                .arg("SPDisplaysDataType")
                .output()
                .map_err(|e| ModuleError::Internal(e.to_string()))?;

            let output_str = String::from_utf8_lossy(&output.stdout).to_string();
            let mut response = vec![];
            display_count.iter().for_each(|_| {
                response.push(output_str.clone());
            });
            return Ok(response);
        } else {
            return Ok(vec![]);
        }
    }
    #[cfg(target_os = "linux")]
    {
        vec![]
    }
}

pub fn assign_seat_number_to_computer() {}

pub fn change_seat_number() {}

pub fn validate_otp() -> bool {
    true
}
