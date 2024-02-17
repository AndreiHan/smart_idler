use chrono::Local;
use serde::{Deserialize, Serialize};
use winreg::enums::*;
use winreg::RegKey;

const APP_SUBKEY: &str = "SOFTWARE\\VisualIdler";
const SLEEP_TIME_SECONDS: u64 = 5 * 60;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RegistryState {
    Enabled,
    Disabled,
}

impl ToString for RegistryState {
    fn to_string(&self) -> String {
        match self {
            RegistryState::Enabled => String::from("enabled"),
            RegistryState::Disabled => String::from("disabled"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RegistryEntries {
    ForceInterval,
    LastRobotInput,
    StartMaintenance,
    LogStatistics,
    StartWithWindows,
}

impl ToString for RegistryEntries {
    fn to_string(&self) -> String {
        match self {
            RegistryEntries::ForceInterval => String::from("ForceInterval"),
            RegistryEntries::LastRobotInput => String::from("LastRobotInput"),
            RegistryEntries::StartMaintenance => String::from("StartMaintenance"),
            RegistryEntries::LogStatistics => String::from("LogStatistics"),
            RegistryEntries::StartWithWindows => String::from("StartWithWindows"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RegistrySetting {
    pub registry_entry: RegistryEntries,
    pub registry_name: String,
    pub last_data: String,
}

impl RegistrySetting {
    pub fn new(entry: RegistryEntries) -> RegistrySetting {
        match entry {
            RegistryEntries::ForceInterval => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: String::new(),
                };
                new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    new_settings.set_registry_data(&SLEEP_TIME_SECONDS.to_string());
                }
                new_settings
            }
            RegistryEntries::LastRobotInput => {
                let current_time = get_current_time();
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: String::new(),
                };
                new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    new_settings.set_registry_data(&current_time);
                }
                new_settings
            }
            RegistryEntries::StartMaintenance => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Disabled.to_string(),
                };
                new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    new_settings.set_registry_data(&RegistryState::Disabled.to_string());
                }
                new_settings
            }
            RegistryEntries::LogStatistics => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Enabled.to_string(),
                };
                new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    new_settings.set_registry_data(&RegistryState::Enabled.to_string());
                }
                new_settings
            }
            RegistryEntries::StartWithWindows => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Disabled.to_string(),
                };
                new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    new_settings.set_registry_data(&RegistryState::Disabled.to_string());
                }
                new_settings
            }
        }
    }

    pub fn update_local_data(&mut self) -> String {
        let app_key: RegKey = match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key with err {}", err);
                create_app_key();
                return String::new();
            }
        };
        let data: String = match app_key.get_value(&self.registry_name) {
            Ok(data) => {
                debug!("Found data {:#?} in {}", data, self.registry_name);
                data
            }
            Err(err) => {
                error!(
                    "Failed to get data from {}, with error {}",
                    &self.registry_name, err
                );
                self.set_registry_data(&self.last_data.clone());
                return String::new();
            }
        };
        self.last_data = data.clone();
        data
    }

    pub fn set_registry_data(&mut self, new_data: &String) {
        let (app_key, _) = match RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key: {} with err {}", APP_SUBKEY, err);
                create_app_key();
                return;
            }
        };
        match app_key.set_value(&self.registry_name, new_data) {
            Ok(_) => {
                info!("Set data {:#?} in {}", new_data, self.registry_name);
                self.last_data = new_data.clone();
            }
            Err(err) => {
                error!(
                    "Failed to set data from {}, with error {}",
                    self.registry_name, err
                );
            }
        };
    }
}

#[inline]
fn create_app_key() {
    std::thread::spawn(|| {
        let hkcu = RegKey::predef(HKEY_LOCAL_MACHINE);
        match hkcu.create_subkey(APP_SUBKEY) {
            Ok(_) => info!("Created {}", APP_SUBKEY),
            Err(err) => error!("Failed to create {} with err: {}", APP_SUBKEY, err),
        }
    });
}

#[inline]
pub fn get_current_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
