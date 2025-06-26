// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#[cfg(target_os = "windows")]
pub mod webview_installer {
    use std::process::Command;
    use winreg::enums::*;
    use winreg::RegKey;

    // --- This is the function that checks the registry ---
    pub fn is_webview2_installed() -> bool {
        // The registry keys to check for WebView2 installation.
        // These are the standard locations for the Evergreen runtime.
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        // Check for system-wide installations (both 64-bit and 32-bit paths)
        let system_wide_64bit = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}");
        let system_wide_32bit = hklm.open_subkey(
            "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        );

        // Check for user-specific installation
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let user_wide = hkcu.open_subkey(
            "SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        );

        // If any of these keys exist, WebView2 is considered installed.
        system_wide_64bit.is_ok() || system_wide_32bit.is_ok() || user_wide.is_ok()
    }
}

#[cfg(not(target_os = "windows"))]
pub mod webview_installer {
    pub fn is_webview2_installed() -> bool {
        true
    }
}
