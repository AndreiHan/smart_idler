use idler_utils::db_ops;
use idler_utils::registry_ops;
use idler_utils::registry_ops::RegistryState;
use idler_utils::registry_ops::{RegistryEntries, RegistrySetting};
use tauri::State;

use crate::AppState;
use crate::Channel;

#[tauri::command(rename_all = "snake_case")]
pub fn get_shutdown_state(channel: State<Channel>) -> bool {
    *channel.active.lock().unwrap()
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_shutdown_clock(state: State<AppState>) -> String {
    let mut setting = match state.shutdown.lock() {
        Ok(h) => h,
        Err(err) => {
            error!("Failed to lock shutdown with err: {}", err);
            return "".to_string();
        }
    };

    let _ = setting.update_local_data();
    setting.last_data.clone()
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_shutdown(channel_state: State<Channel>, app_state: State<AppState>, hour: String) {
    let tx = match channel_state.tx.lock() {
        Ok(val) => val,
        Err(err) => {
            error!("Failed to lock tx, err: {}", err);
            return;
        }
    };
    debug!("Sent shutdown date:, {}", hour);
    let _ = tx.send(hour.clone());

    let mut active = match channel_state.active.lock() {
        Ok(val) => val,
        Err(err) => {
            error!("Failed to lock channel_state.active, err: {err}");
            return;
        }
    };
    *active = hour != "STOP";

    let mut setting = match app_state.shutdown.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to lock app_state.shutdown, err: {}", err);
            return;
        }
    };
    let _ = setting.set_registry_data(&hour);
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_data(state: State<AppState>, data: String) -> String {
    let setting = match data.as_str() {
        "force_interval" => &state.force_interval,
        "robot_input" => &state.robot_input,
        _ => {
            warn!("Found invalid data in request: {}", data);
            return String::new();
        }
    };
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {:?} with err: {}", state, err);
            return String::new();
        }
    };
    let _ = setting.update_local_data();
    setting.last_data.clone()
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_state(state: State<AppState>, data: String) -> bool {
    let setting = match data.as_str() {
        "logging" => &state.logging,
        "maintenance" => &state.maintenance,
        "startup" => &state.startup,
        _ => {
            warn!("Found invalid data in request: {:?}", state);
            return false;
        }
    };
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {:?} with err: {}", state, err);
            return false;
        }
    };
    let _ = setting.update_local_data();
    if setting.last_data == RegistryState::Enabled.to_string() {
        return true;
    }
    false
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_registry_state(state: State<AppState>, data: String, wanted_status: bool) {
    let setting = match data.as_str() {
        "logging" => &state.logging,
        "maintenance" => &state.maintenance,
        "startup" => &state.startup,
        _ => {
            warn!("Found incorrect data in request: {:?}", state);
            return;
        }
    };
    let disabled = RegistryState::Disabled.to_string();
    let enabled = RegistryState::Enabled.to_string();
    let mut setting = match setting.lock() {
        Ok(set) => set,
        Err(err) => {
            error!("Failed to get state: {:?} with err: {}", state, err);
            return;
        }
    };
    let _ = setting.set_registry_data(match wanted_status {
        true => &enabled,
        false => &disabled,
    });
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_force_interval(interval: String) {
    let mut setting = RegistrySetting::new(RegistryEntries::ForceInterval);
    let _ = setting.set_registry_data(&interval);
}

#[tauri::command(rename_all = "snake_case")]
pub fn tauri_get_db_count() -> Result<String, ()> {
    if registry_ops::RegistrySetting::new(registry_ops::RegistryEntries::LogStatistics).last_data
        == registry_ops::RegistryState::Disabled.to_string()
    {
        return Ok(String::from("Disabled"));
    }
    let db = db_ops::RobotDatabase::new();
    if db.is_none() {
        return Err(());
    }
    Ok(db.unwrap().number_of_items.to_string())
}
