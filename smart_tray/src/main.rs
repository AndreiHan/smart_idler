#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[macro_use]
extern crate log;

use anyhow::Result;

use log::error;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::AtomicBool,
    {mpsc, Mutex},
};
use tauri::{
    api::notification::Notification,
    {generate_context, generate_handler, Builder, RunEvent},
};

use idler_utils::{
    idler_win_utils,
    registry_ops::{RegistryEntries, RegistrySetting},
};

mod app_controller;
mod commands;
mod tray;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppState {
    force_interval: Mutex<RegistrySetting>,
    robot_input: Mutex<RegistrySetting>,
    logging: Mutex<RegistrySetting>,
    shutdown: Mutex<RegistrySetting>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            force_interval: Mutex::new(RegistrySetting::new(&RegistryEntries::ForceInterval)),
            robot_input: Mutex::new(RegistrySetting::new(&RegistryEntries::LastRobotInput)),
            logging: Mutex::new(RegistrySetting::new(&RegistryEntries::LogStatistics)),
            shutdown: Mutex::new(RegistrySetting::new(&RegistryEntries::ShutdownTime)),
        }
    }
}

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        env_logger::init_from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
        );
    }

    idler_win_utils::ExecState::start();
    idler_win_utils::spawn_idle_threads();

    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    app_controller::close_app_remote(rx);

    let tauri_app = Builder::default()
        .manage(AppState::default())
        .manage(app_controller::ControllerChannel {
            tx,
            active: AtomicBool::new(false),
        })
        .invoke_handler(generate_handler![
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
            tray::handle_system_tray_event(app, event);
        })
        .build(generate_context!("tauri.conf.json"));

    match tauri_app {
        Ok(app) => app,
        Err(err) => {
            error!("Failed to build tray app with err {}", err);
            return Err(err.into());
        }
    }
    .run(move |app_handle, event| match event {
        RunEvent::Ready => {
            let identifier = app_handle.config().tauri.bundle.identifier.clone();
            let status = Notification::new(identifier)
                .title("Smart Idler")
                .body("Smart Idler has started")
                .show();
            trace!("Notification status: {status:?}");
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Exit => {
            idler_win_utils::ExecState::stop();
        }
        _ => {}
    });
    Ok(())
}
