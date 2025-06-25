#![allow(unused_imports)]
use tauri::menu::MenuBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
pub mod utils;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let args: Vec<String> = std::env::args().collect();
            println!("arguments: {:?}", args);
            let is_kiosk = args.iter().any(|arg| arg == "kiosk");
            println!("Running app in kiosk mode set to : {}", is_kiosk);

            let window = app.get_webview_window("main").unwrap();
            window.set_fullscreen(true)?;
            window.set_decorations(false)?;
            window.set_always_on_top(true)?;
            window.set_resizable(false)?;
            //window.hide_menu()?;
            //window.remove_menu()?;
            let manager = app.handle();
            let menu = MenuBuilder::new(manager);
            //window.set_menu(Menu::new(manager)?)?;
            let menu = menu.build()?;
            window.set_menu(menu)?;

            let ctrl_k_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyK);
            let cltr_alt_delete_shortcut =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Delete);
            let minimized_shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::KeyD);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        utils::build_bindings(
                            app,
                            shortcut,
                            &event,
                            &ctrl_k_shortcut,
                            &cltr_alt_delete_shortcut,
                            &minimized_shortcut,
                        );
                    })
                    .build(),
            )?;
            app.global_shortcut().register(ctrl_k_shortcut)?;
            app.global_shortcut().register(cltr_alt_delete_shortcut)?;
            app.global_shortcut().register(minimized_shortcut)?;

            // Check if running in a guest machine on windows    
            if utils::is_virtual_machine() || utils::is_running_in_rdp() {
                println!("Running in a guest machine, exiting...");
                app.handle().exit(0);
            }
            
            Ok(())
        })
        .on_window_event({
            move |_window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
