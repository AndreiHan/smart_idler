#![allow(clippy::needless_pass_by_value)]
use std::sync::atomic::Ordering;

use anyhow::Result;
use tauri::{command, State};

use idler_utils::{
    db_ops, registry_ops,
    registry_ops::{RegistryEntries, RegistrySetting, RegistryState},
};

use crate::app_controller;
use crate::AppState;

#[command(rename_all = "snake_case")]
pub fn get_shutdown_state(channel: State<app_controller::ControllerChannel>) -> bool {
    channel.active.load(Ordering::SeqCst)
}

#[command(rename_all = "snake_case")]
pub fn get_shutdown_clock(state: State<AppState>) -> String {
    let mut setting = match state.shutdown.lock() {
        Ok(h) => h,
        Err(err) => {
            error!("Failed to lock shutdown with err: {err}");
            return String::new();
        }
    };

    let status = setting.update_local_data();
    trace!("Got shutdown status: {status:?}");
    setting.last_data.to_string()
}

#[command(rename_all = "snake_case")]
pub fn set_shutdown(
    channel_state: State<app_controller::ControllerChannel>,
    app_state: State<AppState>,
    hour: &str,
) {
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

    let mut setting = match app_state.shutdown.lock() {
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
pub fn get_data(state: State<AppState>, data: &str) -> String {
    let setting = match data {
        "force_interval" => &state.force_interval,
        "robot_input" => &state.robot_input,
        _ => {
            warn!("Found invalid data in request: {data}");
            return String::new();
        }
    };
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {state:?} with err: {err}");
            return String::new();
        }
    };
    let status = setting.update_local_data();
    trace!("Got status: {status:?}");
    setting.last_data.to_string()
}

#[command(rename_all = "snake_case")]
pub fn get_state(state: State<AppState>, data: &str) -> bool {
    let setting = if data == "logging" {
        &state.logging
    } else {
        warn!("Found invalid data in request: {state:?}");
        return false;
    };
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {state:?} with err: {err}");
            return false;
        }
    };
    let status = setting.update_local_data();
    trace!("Got state: {status:?}");
    if setting.last_data == RegistryState::Enabled.to_string() {
        return true;
    }
    false
}

#[command(rename_all = "snake_case")]
pub fn set_registry_state(state: State<AppState>, data: &str, wanted_status: bool) {
    let setting = if data == "logging" {
        &state.logging
    } else {
        warn!("Found incorrect data in request: {state:?}");
        return;
    };
    let disabled = RegistryState::Disabled.to_string();
    let enabled = RegistryState::Enabled.to_string();
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {state:?} with err: {err}");
            return;
        }
    };
    let status: Result<()> = if wanted_status {
        setting.set_registry_data(&enabled)
    } else {
        setting.set_registry_data(&disabled)
    };
    trace!("Set registry: {status:?}");
}

#[command(rename_all = "snake_case")]
pub fn set_force_interval(interval: &str) {
    let mut setting = RegistrySetting::new(&RegistryEntries::ForceInterval);
    let status = setting.set_registry_data(interval);
    trace!("Set force interval: {status:?}");
}

#[command(rename_all = "snake_case")]
pub fn tauri_get_db_count() -> Result<String, ()> {
    if registry_ops::RegistrySetting::new(&registry_ops::RegistryEntries::LogStatistics).last_data
        == registry_ops::RegistryState::Disabled.to_string()
    {
        return Ok(String::from("Disabled"));
    }
    let Some(db) = db_ops::RobotDatabase::new() else {
        error!("Failed to get db with");
        return Err(());
    };
    Ok(db.number_of_items.get().to_string())
}
