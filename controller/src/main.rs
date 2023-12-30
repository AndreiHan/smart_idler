#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[macro_use]
extern crate log;
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{generate_context, Manager};

use idler_utils::idler_win_utils;
use idler_utils::process_ops;
use idler_utils::registry_ops::{RegistryEntries, RegistrySetting};
use idler_utils::single_instance;

mod commands;

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

fn main() {
    if cfg!(debug_assertions) {
        env_logger::init_from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
        );
    }

    let current_instance =
        single_instance::SingleInstance::new(process_ops::AppProcess::Controller);
    current_instance.check();

    idler_win_utils::ExecState::start();
    let tauri_app = tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::get_state,
            commands::set_registry_state,
            commands::restart_controller,
            commands::set_force_interval,
            commands::tauri_get_db_count,
        ])
        .build(generate_context!("tauri.conf.json"));

    let tauri_app = match tauri_app {
        Ok(app) => app,
        Err(err) => {
            error!("Failed to build controller app with err {}", err);
            return;
        }
    };

    match tauri_app.get_window("controller") {
        Some(window) => {
            let _ = window.set_title("Controller");
            let _ = window.center();
        }
        None => error!("Failed to get window"),
    }

    tauri_app.run(move |_app_handle, event| match event {
        tauri::RunEvent::Exit => {
            idler_win_utils::ExecState::stop();
            current_instance.exit();
        }
        tauri::RunEvent::ExitRequested { .. } => {
            idler_win_utils::ExecState::stop();
            current_instance.exit();
        }
        _ => {}
    });
}
