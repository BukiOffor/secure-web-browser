#![allow(unused_imports)]
pub mod utils;

use crate::utils::types::Triggers;
use std::process;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use tauri::menu::MenuBuilder;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use chrono::{Duration, Utc, Weekday};
use tokio::sync::Mutex as TokioMutex;
use tokio_task_scheduler::{scheduler, Scheduler, TaskBuilder};

pub struct SchedulerState(pub Mutex<Option<Scheduler>>);
pub struct RemoteChecker(pub Mutex<Option<Scheduler>>);

struct AppState {
    child_process: Arc<Mutex<Option<CommandChild>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    println!("arguments: {:?}", args);
    let is_kiosk = args.iter().any(|arg| arg == "kiosk");
    println!("Running app in kiosk mode set to : {}", is_kiosk);
    let child_process: Arc<Mutex<Option<CommandChild>>> = Arc::new(Mutex::default());
    let app_state = AppState { child_process };

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .manage(SchedulerState(Mutex::default()))
        .manage(RemoteChecker(Mutex::default()))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            if cfg!(target_os = "windows") {
                println!("Dectected Windows Environment: Running side car");
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
                                        println!(
                                            "[Sidecar stdout] {:?}",
                                            String::from_utf8_lossy(&line)
                                        );
                                    }
                                    CommandEvent::Stderr(line) => {
                                        eprintln!(
                                            "[Sidecar stderr] {:?}",
                                            String::from_utf8_lossy(&line)
                                        );
                                    }
                                    CommandEvent::Error(err) => {
                                        let error_message = format!("[Sidecar error] {}", err);
                                        eprintln!("{}", &error_message);

                                        process::exit(1); // Exit on error
                                    }
                                    CommandEvent::Terminated(_) => {
                                        eprintln!("[Sidecar] Terminated.");
                                        //process::exit(1); // Exit on error
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                    Err(err) => {
                        eprintln!("Failed to spawn sidecar process: {}", err);
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
            window.set_decorations(false)?;
            window.set_always_on_top(true)?;
            window.set_resizable(false)?;
            let menu = MenuBuilder::new(app.handle()).build()?;
            window.set_menu(menu)?;
            window.set_skip_taskbar(true)?;
            window.set_visible_on_all_workspaces(true)?;

            // Check if running in a guest machine on windows
            if utils::is_virtual_machine() || utils::is_running_in_rdp() {
                println!("Running in a guest machine, exiting...");
                app.handle().exit(0);
            }

            // get host info
            let host_info = utils::get_host_info();
            println!("Host Info: {:?}", host_info);

            // create a channel for listeners
            let (sender, rx) = channel::<Triggers>();
            let app_handle = app.handle().clone();

            //////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ////////////////////////////////////                    SCHEDULE TASK FOR REMOTE RTC CHECK                           //////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
            ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

            // create worker for input device check
            let remote_scheduler = Scheduler::new();

            // It runs every 30 seconds.
            let remote_checker_task = TaskBuilder::new("remote_checker", {
                let input_checker_sender = sender.clone();
                move || {
                    let report = utils::is_web_rtc_running();
                    if report.is_running() {
                        match input_checker_sender
                            .clone()
                            .send(Triggers::RemoteApplicationDectected(report))
                        {
                            Ok(_) => println!("send was successful"),
                            Err(e) => eprintln!("send failed: {:?}", e),
                        }
                    }
                    println!("Task executed!");
                    Ok(())
                }
            })
            .every_seconds(30)
            .build();

            tauri::async_runtime::spawn({
                let app_handle = app_handle.clone();
                async move {
                    match remote_scheduler.add_task(remote_checker_task).await {
                        Ok(_) => println!("Task added successfully."),
                        Err(e) => eprintln!("Error adding task: {:?}", e),
                    }
                    remote_scheduler.start().await;
                    let process = &app_handle.state::<RemoteChecker>().0;
                    let mut lock = process.lock().expect("could not lock scheduler");
                    *lock = Some(remote_scheduler);
                    drop(lock);
                }
            });

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
                            Ok(_) => println!("send was successful"),
                            Err(e) => eprintln!("send failed: {:?}", e),
                        }
                    }
                    println!("Task executed!");
                    Ok(())
                }
            })
            .every_seconds(10)
            .build();

            tauri::async_runtime::spawn({
                let app_handle = app_handle.clone();
                async move {
                    match scheduler.add_task(task).await {
                        Ok(_) => println!("Task added successfully."),
                        Err(e) => eprintln!("Error adding task: {:?}", e),
                    }
                    scheduler.start().await;
                    let process = &app_handle.state::<SchedulerState>().0;
                    let mut lock = process.lock().expect("could not lock scheduler");
                    *lock = Some(scheduler);
                    drop(lock);
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
                                println!(
                                    "Disallowed input detected with description: `{}`, exiting app",
                                    device[0].description.clone().unwrap_or("unnamed".into())
                                );
                                app_handle.exit(0);
                            }
                            _ => {}
                        }
                    }
                }
            });
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
