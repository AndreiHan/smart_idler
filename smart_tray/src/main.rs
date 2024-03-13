#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[macro_use]
extern crate log;

use anyhow::Result;
use idler_utils::registry_ops::RegistryState;
use idler_utils::{sch_tasker, single_instance};
use log::error;
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tauri::api::notification::Notification;

use idler_utils::idler_win_utils;
use idler_utils::process_ops;
use idler_utils::registry_ops::{RegistryEntries, RegistrySetting};
use idler_utils::single_instance::SingleInstance;

mod app_controller;
mod commands;
mod tray;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    force_interval: Mutex<RegistrySetting>,
    robot_input: Mutex<RegistrySetting>,
    logging: Mutex<RegistrySetting>,
    maintenance: Mutex<RegistrySetting>,
    startup: Mutex<RegistrySetting>,
    shutdown: Mutex<RegistrySetting>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            force_interval: Mutex::new(RegistrySetting::new(RegistryEntries::ForceInterval)),
            robot_input: Mutex::new(RegistrySetting::new(RegistryEntries::LastRobotInput)),
            logging: Mutex::new(RegistrySetting::new(RegistryEntries::LogStatistics)),
            maintenance: Mutex::new(RegistrySetting::new(RegistryEntries::StartMaintenance)),
            startup: Mutex::new(RegistrySetting::new(RegistryEntries::StartWithWindows)),
            shutdown: Mutex::new(RegistrySetting::new(RegistryEntries::ShutdownTime)),
        }
    }
}

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        env_logger::init_from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
        );
    }

    let instance_checker = Arc::new(SingleInstance::new(process_ops::AppProcess::SysTray));
    match instance_checker.check() {
        Ok(status) => {
            if status == single_instance::CheckStatus::Failed {
                exit(1);
            }
        }
        Err(err) => error!("Failed to perform single instance check, err {}", err),
    }

    idler_win_utils::ExecState::start();
    idler_win_utils::spawn_idle_threads();
    thread::spawn(move || {
        if RegistrySetting::new(RegistryEntries::StartWithWindows).last_data
            == RegistryState::Enabled.to_string()
        {
            sch_tasker::enable_rule()
        } else {
            sch_tasker::delete_rule(None)
        }
    });

    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    app_controller::close_app_remote(rx);

    let moved_instance = Arc::clone(&instance_checker);
    let tauri_app = tauri::Builder::default()
        .manage(AppState::default())
        .manage(app_controller::ControllerChannel {
            tx,
            active: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::get_state,
            commands::set_registry_state,
            commands::set_force_interval,
            commands::tauri_get_db_count,
            commands::get_shutdown_clock,
            commands::get_shutdown_state,
            commands::set_shutdown
        ])
        .system_tray(tray::get_tray_menu())
        .on_system_tray_event(move |app, event| {
            tray::handle_system_tray_event(app, event, moved_instance.clone())
        })
        .build(tauri::generate_context!("tauri.conf.json"));

    let tauri_app = match tauri_app {
        Ok(app) => app,
        Err(err) => {
            error!("Failed to build tray app with err {}", err);
            return Err(err.into());
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
            let _ = moved_instance.exit();
        }
        _ => {}
    });
    Ok(())
}
