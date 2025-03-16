#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate msvc_spectre_libs;

use tracing::{error, info};

use anyhow::Result;
use tauri::{Builder, RunEvent, generate_context};

mod registry_plugin;
mod tray;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    let _ = tracing_subscriber::fmt::try_init();
    idler_utils::ExecState::start();
    mitigations::apply_mitigations().await;

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
            idler_utils::spawn_idle_threads();
        }
        RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
        }
        RunEvent::Exit => {
            idler_utils::ExecState::stop();
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
