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