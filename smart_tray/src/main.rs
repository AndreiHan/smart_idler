#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate link_cplusplus;
extern crate msvc_spectre_libs;

#[macro_use]
extern crate log;

use anyhow::Result;
use idler_utils::{cell_data, idler_win_utils, win_mitigations};
use tauri::{generate_context, Builder, RunEvent};

mod app_controller;
mod cli;
mod registry_plugin;
mod tray;

fn main() -> Result<()> {
    idler_win_utils::ExecState::start();
    win_mitigations::apply_mitigations();
    if cfg!(debug_assertions) {
        env_logger::init_from_env(
            env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "trace"),
        );
    }

    if !cfg!(debug_assertions) {
        cli::parse_args();
    }
    idler_win_utils::spawn_idle_threads();

    let tauri_app = Builder::default()
        .plugin(registry_plugin::init())
        .system_tray(tray::get_tray_menu())
        .on_system_tray_event(move |app, event| {
            tray::handle_system_tray_event(app, event);
        })
        .setup(|app| {
            let app_handle = app.handle();
            let _ = cell_data::TAURI_APP_HANDLE.set(app_handle);
            Ok(())
        })
        .build(generate_context!("tauri.conf.json"));

    match tauri_app {
        Ok(app) => app,
        Err(err) => {
            error!("Failed to build tray app with err {}", err);
            return Err(err.into());
        }
    }
    .run(move |_, event| match event {
        RunEvent::Ready => {
            info!("App is ready");
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Exit => {
            idler_win_utils::ExecState::stop();
            let app_handle = cell_data::TAURI_APP_HANDLE.get().unwrap_or_else(|| {
                error!("Failed to get app handle");
                std::process::exit(0);
            });
            info!("Exiting app with app handle");
            app_handle.exit(0);
        }
        _ => {}
    });
    Ok(())
}
