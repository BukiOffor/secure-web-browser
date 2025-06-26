pub mod installer;
use std::process::Command;

use mac_address::get_mac_address;
use serde::Serialize;
use tauri::Manager;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutEvent, ShortcutState};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Default)]
pub struct HostInfo {
    pub os: String,
    pub arch: String,
    pub mac_address: Option<String>,
    pub serial_number: Option<String>,
    pub processor_id: Option<String>,
}

pub fn build_bindings(
    app: &AppHandle,
    shortcut: &Shortcut,
    event: &ShortcutEvent,
    kill_binding: &Shortcut,
    cltr_alt_delete_shortcut: &Shortcut,
    minimized_shortcut: &Shortcut,
) {
    // Handle the global shortcuts
    println!("{:?}", shortcut);
    if shortcut == kill_binding {
        match event.state() {
            ShortcutState::Pressed => {
                println!("Ctrl-K Pressed!");
            }
            ShortcutState::Released => {
                println!("Ctrl-K Released!");
                app.emit("show-password-prompt", ())
                    .expect("Failed to emit show-password-prompt");

                #[cfg(target_os = "windows")]
                {
                    let child_process = app.app_handle().state::<AppState>().child_process.clone();
                    let mut lock = child_process.lock().unwrap();
                    if let Some(child) = lock.take() {
                        let _ = child.kill();
                        println!("🛑 Sidecar killed on exit.");
                    }
                }

                app.exit(0);
            }
        }
    } else if shortcut == cltr_alt_delete_shortcut {
        match event.state() {
            ShortcutState::Pressed => {
                println!("Ctrl+Alt+Delete Pressed!");
            }
            ShortcutState::Released => {
                println!("Ctrl+Alt+Delete Released!");
                app.emit("show-ctrl-alt-delete-prompt", ())
                    .expect("Failed to emit show-ctrl-alt-delete-prompt");
            }
        }
    } else if shortcut == minimized_shortcut {
        match event.state() {
            ShortcutState::Pressed => {
                println!("Super+D Pressed!");
            }
            ShortcutState::Released => {
                println!("Super+D Released!");
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn is_virtual_machine() -> bool {
    // let cpuid = raw_cpuid::CpuId::new();
    //   if let Some(vf) = cpuid.get_vendor_info() {
    //     let vendor = vf.as_str().to_lowercase();
    //     return vendor.contains("vmware")
    //         || vendor.contains("virtualbox")
    //         || vendor.contains("qemu")
    //         || vendor.contains("kvm")
    //         || vendor.contains("microsoft hyper-v");
    // }

    // false
    use inside_vm::inside_vm;
    inside_vm()
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
