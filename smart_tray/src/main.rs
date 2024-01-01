#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use idler_utils::registry_ops::RegistryState;
use idler_utils::sch_tasker;
use log::error;
use std::sync::Arc;
use std::thread;
use tauri::api::notification::Notification;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

use idler_utils::idler_win_utils;
use idler_utils::process_ops;
use idler_utils::registry_ops::{RegistryEntries, RegistrySetting};
use idler_utils::single_instance::SingleInstance;

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

    let tauri_app = tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .on_system_tray_event(move |app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let _item_handle = app.tray_handle().get_item(&id);
                match id.as_str() {
                    "show" => {
                        process_ops::open_app(&process_ops::AppProcess::Controller, false);
                    }
                    "quit" => {
                        idler_win_utils::ExecState::stop();
                        moved_instance.exit();
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            SystemTrayEvent::LeftClick { .. } => {
                process_ops::open_app(&process_ops::AppProcess::Controller, false);
            }
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
                .title("Ready")
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
