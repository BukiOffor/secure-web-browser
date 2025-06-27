use std::process::Command;
pub mod types;
use crate::utils::types::{HostInfo, PortStatus, ProcessIdentifier, USBDevice, WebRtcReport};
use crate::{AppState, RemoteChecker, SchedulerState};
use mac_address::get_mac_address;
use serde::Serialize;
use std::net::UdpSocket;
use sysinfo::System;
use tauri::Manager;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutEvent, ShortcutState};

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
                app.emit("show-password-prompt", ())
                    .expect("Failed to emit show-password-prompt"); 
                app.exit(0);
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

fn is_udp_running() -> Vec<PortStatus> {
    let mut status = vec![];
    for port in 6_300..=6_535_u32 {
        match UdpSocket::bind(format!("127.0.0.1:{}", port)) {
            Ok(_) => continue, // If bind succeeds, port is free
            // If bind fails, port is likely in use
            Err(_) => {
                log::info!(
                    "Port {} refused connection, assuming Udp is running...",
                    port
                );
                status.push(PortStatus::new(port, true))
            }
        }
    }
    status
}

fn is_known_webrtc_program_running() -> Vec<ProcessIdentifier> {
    let known_apps = vec!["zoom", "teams", "skype", "discord", "team viewer"];
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes()
        .iter()
        .filter_map(|(_, process)| {
            let name = process.name().to_string_lossy();
            if known_apps.iter().any(|app| name.contains(app)) {
                Some(ProcessIdentifier {
                    process_id: process.pid().as_u32() as i32,
                    status: true, // running
                    parent: process.parent().map(|p| p.as_u32() as i32),
                    start_time: process.start_time(),
                    run_time: process.run_time(),
                    cpu_usage: process.cpu_usage(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn is_web_rtc_running() -> WebRtcReport {
    WebRtcReport {
        ports: is_udp_running(),
        processes: is_known_webrtc_program_running(),
    }
}

pub fn assign_seat_number_to_computer() {}

pub fn change_seat_number() {}

pub fn validate_otp() -> bool {
    true
}
