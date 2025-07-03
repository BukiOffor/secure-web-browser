#![allow(unused_imports)]
pub mod utils;

use crate::utils::types::Triggers;
use std::process;
use std::sync::mpsc::channel;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use chrono::Utc;
use tauri::{Emitter, Manager, Url};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_store::StoreExt;
use tokio_schedule::{every, Job};
use tokio_task_scheduler::{Scheduler, TaskBuilder};

struct AppState {
    child_process: Arc<Mutex<Option<CommandChild>>>,
}
struct InitState(RwLock<bool>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    log::info!("arguments: {:?}", args);
    let is_kiosk = args.iter().any(|arg| arg == "kiosk");
    log::info!("Running app in kiosk mode set to : {}", is_kiosk);
    let child_process: Arc<Mutex<Option<CommandChild>>> = Arc::new(Mutex::default());
    let app_state = AppState { child_process };

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .manage(InitState(RwLock::new(false)))
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;
            // request notification access from user
            match app.notification().request_permission() {
                Ok(_) => log::info!("Permission Requested for Application"),
                Err(err) => log::error!("Couldn't request permision: {}", err),
            }
            app.notification().builder().show().unwrap();

            // Check if running in a guest machine on windows
            if utils::is_virtual_machine() || utils::is_running_in_rdp() {
                log::info!("Running in a guest machine, exiting...");
                app.handle().exit(0);
            }

            #[cfg(target_os = "windows")]
            {
                log::info!("Disabling CAD commands");
                utils::disable_cad_actions(true).expect("could not disable cad command");
            }
            // get host info
            let host_info = utils::get_host_info();
            log::info!("Host Info: {:?}", host_info);

            if cfg!(target_os = "windows") {
                log::info!("Dectected Windows Environment: Running side car");
                app.handle()
                    .plugin(tauri_plugin_shell::init())
                    .expect("Failed to initialize shell plugin for Windows");

                let sidecar = app
                    .shell()
                    .sidecar("mapper")
                    .expect("Failed to get sidecar");
                let process = sidecar.spawn();

                match process {
                    Ok((mut rx, child)) => {
                        // Save the child handle
                        let app_state = app.state::<AppState>();
                        let mut child_lock = app_state.child_process.lock().unwrap();
                        *child_lock = Some(child);
                        drop(child_lock);
                        tauri::async_runtime::spawn(async move {
                            while let Some(event) = rx.recv().await {
                                match event {
                                    CommandEvent::Stdout(line) => {
                                        log::info!(
                                            "[Sidecar stdout] {:?}",
                                            String::from_utf8_lossy(&line)
                                        );
                                    }
                                    CommandEvent::Stderr(line) => {
                                        log::error!(
                                            "[Sidecar stderr] {:?}",
                                            String::from_utf8_lossy(&line)
                                        );
                                    }
                                    CommandEvent::Error(err) => {
                                        let error_message = format!("[Sidecar error] {}", err);
                                        log::error!("{}", &error_message);

                                        process::exit(1); // Exit on error
                                    }
                                    CommandEvent::Terminated(_) => {
                                        log::error!("[Sidecar] Terminated.");
                                        //process::exit(1); // Exit on error
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                    Err(err) => {
                        log::error!("Failed to spawn sidecar process: {}", err);
                        app.handle().exit(1);
                    }
                }
            }

            let kill_binding = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyK);
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
                            &kill_binding,
                            &cltr_alt_delete_shortcut,
                            &minimized_shortcut,
                        );
                    })
                    .build(),
            )?;
            app.global_shortcut().register(kill_binding)?;

            #[cfg(not(target_os = "windows"))]
            {
                app.global_shortcut().register(cltr_alt_delete_shortcut)?;
                app.global_shortcut().register(minimized_shortcut)?;
            }

            let window = app.get_webview_window("main").unwrap();
            window.set_fullscreen(true)?;
            // create a channel for listeners
            let (sender, rx) = channel::<Triggers>();
            let app_handle = app.handle().clone();

            // let weekly = tokio_schedule::Job::perform(tokio_schedule::every(1).second()
            //     .in_timezone(&Utc), || async { println!("Every Second job") });
            // tauri::async_runtime::spawn(weekly);

            //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ////////////////////////////////////                    SCHEDULE TASK FOR PASSWORD QUERYING                           //////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

                
            let app_handle_for_password = app_handle.clone();
            let query_password = every(5).minutes().at(0).in_timezone(&Utc)
            .perform(move || {
                let handle = app_handle_for_password.clone();
                async move {
                    match utils::query_password_for_server(&handle).await {
                        Ok(_) => log::info!("Task: Querying Password was Successful"),
                        Err(e) => log::error!("Querying Password Error: {}", e),
                    }
                }
            });
            tauri::async_runtime::spawn(query_password);
            //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ////////////////////////////////////                    SCHEDULE TASK FOR USB DEVICES                           ///////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

            let scheduler = Scheduler::new();

            // Define our recurring task.
            // It runs every 10 seconds.
            let task = TaskBuilder::new("input_checker", {
                let input_checker_sender = sender.clone();
                move || {
                    let response = utils::is_disallowed_device_connected();
                    if response.len() > 0 {
                        match input_checker_sender
                            .clone()
                            .send(Triggers::DisAllowedInputDectected(response))
                        {
                            Ok(_) => log::info!("send was successful"),
                            Err(e) => log::error!("send failed: {:?}", e),
                        }
                    }
                    log::info!("Task executed: No Usb Devices Found!");
                    Ok(())
                }
            })
            .every_seconds(10)
            .build();

            tauri::async_runtime::spawn({
                async move {
                    match scheduler.add_task(task).await {
                        Ok(_) => log::info!("Task: Usb Device Checker added successfully."),
                        Err(e) => log::error!("Error adding task: {:?}", e),
                    }
                    scheduler.start().await;
                }
            });

            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ////////////////////////////////////                    RECIEVE TASK REPORTS OVER A CHANNEL                           /////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

            // Controller to receive Triggers over a channel
            tauri::async_runtime::spawn({
                let app_handle = app_handle.clone();
                async move {
                    while let Ok(event) = rx.recv() {
                        match event {
                            Triggers::DisAllowedInputDectected(device) => {
                                log::info!(
                                    "Disallowed Input Detected with Description: `{}`, Exiting App",
                                    device[0].description.clone().unwrap_or("unnamed".into())
                                );
                                app_handle
                                    .notification()
                                    .builder()
                                    .title("Device Compromised")
                                    .body("An external device has been attached to your device")
                                    .show()
                                    .unwrap();
                                sleep(Duration::from_secs(9));
                                app_handle.exit(0);
                            }
                            _ => {}
                        }
                    }
                }
            });
            //app.notification().builder().title("title").body("body").show().unwrap();
            Ok(())
        })
        .on_window_event({
            move |window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let state = window.state::<InitState>();
                    let state = state.0.read();
                    if let Ok(result) = state {
                        if *result {
                            api.prevent_close();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![utils::commands::set_server, utils::commands::server_url])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(move |app_handle, event| {
            // if user exits the app, kill the thread running in the background
            if let tauri::RunEvent::ExitRequested { .. } = event {
                log::info!("🚨 Exit requested!");

                #[cfg(target_os = "windows")]
                {
                    let child_process = app_handle.state::<AppState>().child_process.clone();
                    let mut lock = child_process.lock().unwrap();
                    if let Some(child) = lock.take() {
                        let _ = child.kill();
                        log::info!("🛑 Sidecar killed on exit.");
                    }
                }
                let child_process = app_handle.state::<AppState>().child_process.clone();
                let mut lock = child_process.lock().unwrap();
                if let Some(child) = lock.take() {
                    let _ = child.kill();
                    println!("🛑 Sidecar killed on restart.");
                }
            }
        });
}
