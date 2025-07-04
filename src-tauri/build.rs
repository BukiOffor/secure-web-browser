fn main() {
    #[cfg(not(target_os = "windows"))]
    {
        tauri_build::build()
    }

    //#[cfg(target_os = "windows")]
    {
        let mut windows = tauri_build::WindowsAttributes::new();
        windows = windows.app_manifest(include_str!("app.manifest"));
        tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
            .unwrap_or_else(|e| eprintln!("Error: {}", e));
    }
}
