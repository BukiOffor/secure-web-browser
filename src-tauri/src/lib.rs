use tauri::{menu::Menu, window::Window, Manager, WebviewWindowBuilder};
use tauri::menu::MenuBuilder;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  
  tauri::Builder::default()
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
      window.set_fullscreen(is_kiosk.clone())?;
      window.set_decorations(!is_kiosk.clone())?;
      window.set_always_on_top(is_kiosk.clone())?;
      window.set_resizable(!is_kiosk.clone())?;
      //window.hide_menu()?;
      //window.remove_menu()?;
      let manager = app.handle();
      let menu = MenuBuilder::new(manager);
      //window.set_menu(Menu::new(manager)?)?;

      let menu = menu.build()?;
      window.set_menu(menu)?;
 

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
