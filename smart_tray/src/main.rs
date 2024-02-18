#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[macro_use]
extern crate log;

use chrono::{Local, NaiveTime};
use idler_utils::registry_ops::RegistryState;
use idler_utils::sch_tasker;
use log::error;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::{process, thread};
use tauri::api::notification::Notification;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    UserAttentionType,
};

use idler_utils::idler_win_utils;
use idler_utils::process_ops;
use idler_utils::registry_ops::{RegistryEntries, RegistrySetting};
use idler_utils::single_instance::SingleInstance;

mod commands;

#[derive(Debug)]
struct Channel {
    tx: Arc<Mutex<mpsc::Sender<String>>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    force_interval: Arc<Mutex<RegistrySetting>>,
    robot_input: Arc<Mutex<RegistrySetting>>,
    logging: Arc<Mutex<RegistrySetting>>,
    maintenance: Arc<Mutex<RegistrySetting>>,
    startup: Arc<Mutex<RegistrySetting>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            force_interval: Arc::new(Mutex::new(RegistrySetting::new(
                RegistryEntries::ForceInterval,
            ))),
            robot_input: Arc::new(Mutex::new(RegistrySetting::new(
                RegistryEntries::LastRobotInput,
            ))),
            logging: Arc::new(Mutex::new(RegistrySetting::new(
                RegistryEntries::LogStatistics,
            ))),
            maintenance: Arc::new(Mutex::new(RegistrySetting::new(
                RegistryEntries::StartMaintenance,
            ))),
            startup: Arc::new(Mutex::new(RegistrySetting::new(
                RegistryEntries::StartWithWindows,
            ))),
        }
    }
}

fn build_controller(app: &tauri::AppHandle) {
    match app.get_window("main") {
        Some(win) => {
            info!("Found 'main' window setting focus");
            let _ = win.set_focus();
            let _ = win.request_user_attention(Some(UserAttentionType::Informational));
            return;
        }
        None => info!("Could not find 'main' window, launching it"),
    };

    let current_app = app.clone();
    thread::spawn(move || {
        match tauri::WindowBuilder::new(&current_app, "main", tauri::WindowUrl::App("ui".into()))
            .fullscreen(false)
            .resizable(false)
            .title("Controller")
            .center()
            .inner_size(900.into(), 425.into())
            .build()
        {
            Ok(handle) => {
                let _ = handle.set_focus();
                let _ = handle.request_user_attention(Some(UserAttentionType::Informational));
            }
            Err(e) => error!("Failed to create controller app, err: {}", e),
        }
    });
}

fn close_app_remote(rx: Receiver<String>) {
    thread::spawn(move || {
        let mut _sender: Option<mpsc::Sender<()>> = None;
        loop {
            match rx.recv() {
                Ok(hour) => {
                    debug!("Received time: {:?}", hour);
                    let received_time = match NaiveTime::parse_from_str(&hour, "%H:%M") {
                        Ok(val) => val,
                        Err(_) => {
                            info!("Received non time value, {}. Ignorring", hour);
                            _sender = None;
                            continue;
                        }
                    };
                    let (sen, receiver) = mpsc::channel::<()>();
                    _sender = Some(sen);
                    thread::spawn(move || loop {
                        let diff = received_time - Local::now().time();
                        if diff.num_seconds() <= 0 {
                            warn!("Shutdown");
                            idler_win_utils::ExecState::stop();
                            process::exit(0);
                        }
                        thread::sleep(Duration::from_millis(500));
                        match receiver.try_recv() {
                            Ok(_) | Err(TryRecvError::Disconnected) => {
                                info!("Cancelling task for: {}", received_time);
                                break;
                            }
                            Err(TryRecvError::Empty) => {}
                        }
                    });
                    thread::sleep(Duration::from_millis(500));
                }
                Err(err) => {
                    debug!("Received err: {}", err);
                    return;
                }
            }
        }
    });
}

fn main() {
    if cfg!(debug_assertions) {
        env_logger::init_from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
        );
    }

    let instance_checker = Arc::new(SingleInstance::new(process_ops::AppProcess::SysTray));
    instance_checker.check();
    idler_win_utils::ExecState::start();

    let _ = RegistrySetting::new(RegistryEntries::LastRobotInput);
    thread::spawn(move || idler_win_utils::idle_time);

    thread::spawn(move || {
        let _ = idler_win_utils::spawn_window();
    });

    thread::spawn(move || {
        if RegistrySetting::new(RegistryEntries::StartWithWindows).last_data
            == RegistryState::Enabled.to_string()
        {
            sch_tasker::enable_rule()
        } else {
            sch_tasker::delete_rule(None)
        }
    });

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let moved_instance = Arc::clone(&instance_checker);
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(tx));
    let channel = Channel { tx };
    close_app_remote(rx);

    let tauri_app = tauri::Builder::default()
        .manage(AppState::default())
        .manage(channel)
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::get_state,
            commands::set_registry_state,
            commands::set_force_interval,
            commands::tauri_get_db_count,
            commands::set_shutdown
        ])
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(move |app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let _item_handle = app.tray_handle().get_item(&id);
                match id.as_str() {
                    "show" => {
                        build_controller(app);
                    }
                    "quit" => {
                        idler_win_utils::ExecState::stop();
                        moved_instance.exit();
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            SystemTrayEvent::LeftClick { .. } => build_controller(app),

            SystemTrayEvent::DoubleClick { .. } => build_controller(app),
            _ => {}
        })
        .build(tauri::generate_context!("tauri.conf.json"));

    let tauri_app = match tauri_app {
        Ok(app) => app,
        Err(err) => {
            error!("Failed to build tray app with err {}", err);
            return;
        }
    };

    let moved_instance = Arc::clone(&instance_checker);
    tauri_app.run(move |_app_handle, event| match event {
        tauri::RunEvent::Ready => {
            let _ = Notification::new(&_app_handle.config().tauri.bundle.identifier)
                .title("Smart Idler")
                .body("Smart Idler has started")
                .show();
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        tauri::RunEvent::Exit => {
            idler_win_utils::ExecState::stop();
            moved_instance.exit();
        }
        _ => {}
    });
}
