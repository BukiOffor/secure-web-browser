#![cfg(target_os = "windows")]
use std::ptr::null_mut;
use windows::{
    core::*, Win32::Foundation::*, Win32::UI::Input::KeyboardAndMouse::*,
    Win32::UI::WindowsAndMessaging::*,
};

static mut HOOK_HANDLE: HHOOK = HHOOK(null_mut());

fn is_key_pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u32 & 0x8000 as u32) != 0 }
}

unsafe extern "system" fn keyboard_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code >= 0 && (w_param.0 == WM_KEYDOWN as usize || w_param.0 == WM_SYSKEYDOWN as usize) {
        let kb: &KBDLLHOOKSTRUCT = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;

        let alt_down = is_key_pressed(VK_MENU);
        let ctrl_down = is_key_pressed(VK_CONTROL);

        let system_key = match vk {
            vk if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 => true,
            vk if vk == VK_TAB.0 as u32 && alt_down => true,
            vk if vk == VK_ESCAPE.0 as u32 && (alt_down || ctrl_down) => true,
            vk if vk == VK_F4.0 as u32 && alt_down => true,
            // Add all modifiers
            vk if vk == VK_MENU.0 as u32 => true,
            vk if vk == VK_DELETE.0 as u32 => true,
            _ => false,
        };

        if is_key_pressed(VK_MENU) {
            println!("System key intercepted and suppressed");
            return LRESULT(1); // Suppressing the key
        }

        if system_key {
            println!("System key intercepted and suppressed");
            return LRESULT(1); // Suppressing the key
        }
    }
    // pass the key to the system program
    unsafe { CallNextHookEx(Some(HHOOK(null_mut())), code, w_param, l_param) }
}

pub fn capture_key() -> Result<()> {
    unsafe {
        HOOK_HANDLE = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            Some(HINSTANCE(null_mut())),
            0,
        )?;
        if HOOK_HANDLE.0 == null_mut() {
            panic!("Failed to install hook");
        }

        println!("Keyboard hook installed. Press Ctrl+C to exit.");

        // Message loop to keep the hook alive
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, Some(HWND(null_mut())), 0, 0).into() {
            TranslateMessage(&msg).unwrap();
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

// unsafe extern "system" fn keyboard_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
//     if code >= 0 && (w_param.0 == WM_KEYDOWN as usize || w_param.0 == WM_SYSKEYDOWN as usize) {
//         let kb: &KBDLLHOOKSTRUCT = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
//         let vk = kb.vkCode;

//         if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 || vk == VK_MENU.0 as u32 || vk == VK_CONTROL.0 as u32 {
//             println!("System key intercepted and suppressed");
//             return LRESULT(1); // Suppress the key
//         }
//     }

// unsafe{
//     CallNextHookEx(Some(HHOOK(null_mut())), code, w_param, l_param)
// }
// }


use std::process::Command;
use std::str;

fn get_usb_devices() {
    // Define the multi-line PowerShell script as a Rust string.
    // Using a raw string literal r#"..."# is convenient for this.
    let ps_script = r#"
        # Get all Plug and Play devices
        $pnpDevices = Get-WmiObject -Class Win32_PnPEntity | Select-Object Name, PNPDeviceID, Capabilities

        # Filter for USB devices
        $usbDevices = $pnpDevices | Where-Object { $_.PNPDeviceID -like "USB*" }

        foreach ($device in $usbDevices) {
            # The capability value for "Removable" is 4
            $isRemovable = $device.Capabilities -contains 4

            Write-Host "Device: $($device.Name)"
            Write-Host "  PNPDeviceID: $($device.PNPDeviceID)"
            if ($isRemovable) {
                Write-Host "  Type: External (Removable)"
            } else {
                Write-Host "  Type: Internal"
            }
            Write-Host ""
        }
    "#;

    println!("Running PowerShell script to find USB devices...");

    // Execute the PowerShell command
    let output = Command::new("powershell")
        .arg("-NoProfile") // Skips loading the PowerShell profile for faster execution
        .arg("-Command")   // Specifies that the next argument is a command
        .arg(ps_script)    // Pass the entire script as the command
        .output()
        .expect("Failed to execute PowerShell command.");

    // Check if the command was successful
    if output.status.success() {
        // Convert the output bytes to a string and print it.
        // from_utf8_lossy is a safe way to handle non-UTF8 characters.
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("--- Script Output ---");
        println!("{}", stdout);
    } else {
        // If the command failed, print the error details
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("--- PowerShell Error ---");
        eprintln!("Exit Code: {}", output.status);
        eprintln!("{}", stderr);
    }
}

#[derive(Debug)]
struct UdpEndpoint {
    local_address: String,
    local_port: u16,
    process_name: Option<String>,
}

use std::process::Command;
use std::str;

fn get_udp_endpoints() -> Result<Vec<UdpEndpoint>, Box<dyn std::error::Error>> {
    let output = Command::new("powershell")
        .args([
            "-Command",
            "Get-NetUDPEndpoint | Select-Object -Property LocalAddress, LocalPort, @{Name='ProcessName'; Expression={(Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).ProcessName}} | ConvertTo-Json",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!("PowerShell failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let json = String::from_utf8(output.stdout)?;
    parse_udp_json(&json)
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawUdpEndpoint {
    #[serde(rename = "LocalAddress")]
    local_address: String,
    #[serde(rename = "LocalPort")]
    local_port: u16,
    #[serde(rename = "ProcessName")]
    process_name: Option<String>,
}

fn parse_udp_json(json: &str) -> Result<Vec<UdpEndpoint>, Box<dyn std::error::Error>> {
    let raw: serde_json::Value = serde_json::from_str(json)?;

    let entries: Vec<RawUdpEndpoint> = if raw.is_array() {
        serde_json::from_value(raw)?
    } else {
        // Sometimes PowerShell returns an object if only one result exists
        vec![serde_json::from_value(raw)?]
    };

    let result = entries
        .into_iter()
        .map(|entry| UdpEndpoint {
            local_address: entry.local_address,
            local_port: entry.local_port,
            process_name: entry.process_name,
        })
        .collect();

    Ok(result)
}

fn main() {
    match get_udp_endpoints() {
        Ok(endpoints) => {
            for ep in endpoints {
                if ep.local_port >= 10000 && ep.local_port <= 20000 {
                    println!(
                        "WebRTC-ish port used: {}:{} ({:?})",
                        ep.local_address, ep.local_port, ep.process_name
                    );
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}


#[derive(Debug)]
struct NetstatEntry {
    protocol: String,
    local_address: String,
    local_port: u16,
    foreign_address: String,
    foreign_port: u16,
    state: Option<String>, // UDP has no state
    pid: u32,
}
use std::process::Command;

fn run_netstat() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("netstat")
        .args(["-a", "-n", "-o"])
        .output()?;

    if !output.status.success() {
        return Err(format!("netstat failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    Ok(String::from_utf8(output.stdout)?)
}
fn parse_netstat_output(output: &str) -> Vec<NetstatEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("TCP") || line.starts_with("UDP") {
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Handle TCP and UDP differently
            if parts.len() >= 4 {
                let protocol = parts[0].to_string();
                let (local_ip, local_port) = split_address(parts[1]);
                let (foreign_ip, foreign_port) = split_address(parts[2]);

                let (state, pid) = if protocol == "TCP" && parts.len() >= 5 {
                    (Some(parts[3].to_string()), parts[4].parse::<u32>().unwrap_or(0))
                } else {
                    (None, parts[3].parse::<u32>().unwrap_or(0))
                };

                entries.push(NetstatEntry {
                    protocol,
                    local_address: local_ip,
                    local_port,
                    foreign_address: foreign_ip,
                    foreign_port,
                    state,
                    pid,
                });
            }
        }
    }

    entries
}

fn split_address(addr: &str) -> (String, u16) {
    if let Some(idx) = addr.rfind(':') {
        let ip = &addr[..idx];
        let port = &addr[idx + 1..];
        let port = port.parse().unwrap_or(0);
        (ip.to_string(), port)
    } else {
        (addr.to_string(), 0)
    }
}



/// DISABLE REMOTE IN REGISTRY

use std::error::Error;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug)]
pub enum RemoteAccessFeature {
    RemoteDesktop,
    RemoteAssistance,
    PowerShellRemoting,
    AdminShares,
}

pub fn set_remote_access(
    feature: RemoteAccessFeature,
    enable: bool,
) -> Result<(), Box<dyn Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    match feature {
        RemoteAccessFeature::RemoteDesktop => {
            let terminal_server = hklm.open_subkey_with_flags(
                "SYSTEM\\CurrentControlSet\\Control\\Terminal Server",
                KEY_WRITE,
            )?;
            terminal_server.set_value("fDenyTSConnections", &(if enable { 0u32 } else { 1u32 })?;
        }
        
        RemoteAccessFeature::RemoteAssistance => {
            let remote_assistance = hklm.open_subkey_with_flags(
                "SYSTEM\\CurrentControlSet\\Control\\Remote Assistance",
                KEY_WRITE,
            )?;
            remote_assistance.set_value("fAllowToGetHelp", &(if enable { 1u32 } else { 0u32 }))?;
        }
        
        RemoteAccessFeature::PowerShellRemoting => {
            let winrm = hklm.create_subkey(
                "SOFTWARE\\Policies\\Microsoft\\Windows\\WinRM\\Service",
            )?;
            winrm.0.set_value("AllowRemoteShellAccess", &(if enable { 1u32 } else { 0u32 }))?;
        }
        
        RemoteAccessFeature::AdminShares => {
            let parameters = hklm.open_subkey_with_flags(
                "SYSTEM\\CurrentControlSet\\Services\\LanmanServer\\Parameters",
                KEY_WRITE,
            )?;
            parameters.set_value("AutoShareWks", &(if enable { 1u32 } else { 0u32 }))?;
            parameters.set_value("AutoShareServer", &(if enable { 1u32 } else { 0u32 }))?;
        }
    }
    
    Ok(())
}

pub fn is_feature_enabled(feature: RemoteAccessFeature) -> Result<bool, Box<dyn Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    match feature {
        RemoteAccessFeature::RemoteDesktop => {
            let terminal_server = hklm.open_subkey(
                "SYSTEM\\CurrentControlSet\\Control\\Terminal Server",
            )?;
            let value: u32 = terminal_server.get_value("fDenyTSConnections")?;
            Ok(value == 0)
        }
        
        RemoteAccessFeature::RemoteAssistance => {
            let remote_assistance = hklm.open_subkey(
                "SYSTEM\\CurrentControlSet\\Control\\Remote Assistance",
            )?;
            let value: u32 = remote_assistance.get_value("fAllowToGetHelp")?;
            Ok(value == 1)
        }
        
        RemoteAccessFeature::PowerShellRemoting => {
            let winrm = hklm.open_subkey(
                "SOFTWARE\\Policies\\Microsoft\\Windows\\WinRM\\Service",
            );
            match winrm {
                Ok(key) => {
                    let value: u32 = key.get_value("AllowRemoteShellAccess").unwrap_or(0);
                    Ok(value == 1)
                }
                Err(_) => Ok(false), // Key doesn't exist = disabled
            }
        }
        
        RemoteAccessFeature::AdminShares => {
            let parameters = hklm.open_subkey(
                "SYSTEM\\CurrentControlSet\\Services\\LanmanServer\\Parameters",
            )?;
            let wks: u32 = parameters.get_value("AutoShareWks").unwrap_or(0);
            let server: u32 = parameters.get_value("AutoShareServer").unwrap_or(0);
            Ok(wks == 1 || server == 1)
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Example usage
    println!("Current Remote Desktop status: {}", is_feature_enabled(RemoteAccessFeature::RemoteDesktop)?);
    
    // Disable Remote Desktop
    set_remote_access(RemoteAccessFeature::RemoteDesktop, false)?;
    println!("Disabled Remote Desktop");
    
    // Enable it back
    set_remote_access(RemoteAccessFeature::RemoteDesktop, true)?;
    println!("Enabled Remote Desktop");
    
    Ok(())
}