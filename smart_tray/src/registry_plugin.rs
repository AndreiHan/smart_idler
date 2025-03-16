#![allow(clippy::needless_pass_by_value)]
use tauri::{
    Manager, Runtime, State, command,
    plugin::{Builder, TauriPlugin},
};
use tracing::{debug, error, trace, warn};

use anyhow::Result;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

use registry_ops::RegistryState;

#[command(rename_all = "snake_case")]
pub fn get_shutdown_state(channel: State<app_controller::ControllerChannel>) -> bool {
    channel.active.load(Ordering::SeqCst)
}

#[command(rename_all = "snake_case")]
pub fn get_shutdown_clock() -> String {
    let setting = match cell_data::REGISTRY_SHUTDOWN_TIME.lock() {
        Ok(h) => h,
        Err(err) => {
            error!("Failed to lock shutdown with err: {err}");
            return String::new();
        }
    };
    trace!("Got shutdown status: {:?}", setting);
    setting.last_data.to_string()
}

#[command(rename_all = "snake_case")]
pub fn set_shutdown(channel_state: State<app_controller::ControllerChannel>, hour: &str) {
    let tx = match channel_state.tx.lock() {
        Ok(val) => val,
        Err(err) => {
            error!("Failed to lock tx, err: {err}");
            return;
        }
    };
    debug!("Sent shutdown date:, {hour}");
    let _ = tx.send(hour.to_string());

    channel_state.active.store(hour != "STOP", Ordering::SeqCst);

    let mut setting = match cell_data::REGISTRY_SHUTDOWN_TIME.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to lock app_state.shutdown, err: {err}");
            return;
        }
    };
    let status = setting.set_registry_data(hour);
    trace!("Set shutdown status to: {status:?}");
}

#[command(rename_all = "snake_case")]
pub fn get_data(data: &str) -> String {
    let setting = match data {
        "force_interval" => &cell_data::REGISTRY_FORCE_INTERVAL,
        "robot_input" => &cell_data::REGISTRY_ROBOT_INPUT,
        _ => {
            warn!("Found invalid data in request: {data}");
            return String::new();
        }
    };
    let setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {data:?} with err: {err}");
            return String::new();
        }
    };
    trace!("Got data: {:?}", setting);
    setting.last_data.to_string()
}

#[command(rename_all = "snake_case")]
pub fn get_state(data: &str) -> bool {
    let setting = if data == "logging" {
        &cell_data::REGISTRY_LOG_STATISTICS
    } else {
        warn!("Found invalid data in request: {data:?}");
        return false;
    };
    let setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {data:?} with err: {err}");
            return false;
        }
    };
    trace!("Got state: {:?}", setting);
    if setting.is_enabled() {
        return true;
    }
    false
}

#[command(rename_all = "snake_case")]
pub fn set_registry_state(data: &str, wanted_status: bool) {
    let setting = if data == "logging" {
        &cell_data::REGISTRY_LOG_STATISTICS
    } else {
        warn!("Found incorrect data in request: {data:?}");
        return;
    };
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {data:?} with err: {err}");
            return;
        }
    };
    let status: Result<()> = if wanted_status {
        setting.set_registry_data(RegistryState::Enabled.to_string())
    } else {
        setting.set_registry_data(RegistryState::Disabled.to_string())
    };
    trace!("Set registry: {status:?}");
}

#[command(rename_all = "snake_case")]
pub fn set_force_interval(interval: &str) {
    let status = cell_data::REGISTRY_FORCE_INTERVAL
        .lock()
        .unwrap()
        .set_registry_data(interval);
    trace!("Set force interval: {status:?}, data: {interval:?}");
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("general")
        .setup(|app_handle| {
            let (tx, rx) = mpsc::channel();
            let tx = Mutex::new(tx);
            app_controller::close_app_remote(rx);

            app_handle.manage(app_controller::ControllerChannel {
                tx,
                active: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_data,
            get_state,
            set_registry_state,
            set_force_interval,
            get_shutdown_clock,
            get_shutdown_state,
            set_shutdown
        ])
        .build()
}
