use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use windows_registry::LOCAL_MACHINE;

const APP_SUBKEY: &str = "SOFTWARE\\SmartIdler";
const SLEEP_TIME_SECONDS: u64 = 5 * 60;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum RegistryState {
    Enabled,
    Disabled,
}

impl ToString for RegistryState {
    fn to_string(&self) -> String {
        match self {
            RegistryState::Enabled => "enabled".to_string(),
            RegistryState::Disabled => "disabled".to_string(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for RegistryState {
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RegistryEntries {
    ForceInterval,
    LastRobotInput,
    StartMaintenance,
    LogStatistics,
    StartWithWindows,
    ShutdownTime,
}

impl ToString for RegistryEntries {
    fn to_string(&self) -> String {
        match self {
            RegistryEntries::ForceInterval => String::from("ForceInterval"),
            RegistryEntries::LastRobotInput => String::from("LastRobotInput"),
            RegistryEntries::StartMaintenance => String::from("StartMaintenance"),
            RegistryEntries::LogStatistics => String::from("LogStatistics"),
            RegistryEntries::StartWithWindows => String::from("StartWithWindows"),
            RegistryEntries::ShutdownTime => String::from("ShutdownTime"),
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
    #[must_use]
    pub fn new(entry: &RegistryEntries) -> RegistrySetting {
        match entry {
            RegistryEntries::ForceInterval => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: String::new(),
                };
                let _ = new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    let _ = new_settings.set_registry_data(&SLEEP_TIME_SECONDS.to_string());
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
                let _ = new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    let _ = new_settings.set_registry_data(&current_time);
                }
                new_settings
            }
            RegistryEntries::StartMaintenance | RegistryEntries::StartWithWindows => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Disabled.to_string(),
                };
                let _ = new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    let _ = new_settings.set_registry_data(RegistryState::Disabled);
                }
                new_settings
            }
            RegistryEntries::LogStatistics => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Enabled.to_string(),
                };
                let _ = new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    let _ = new_settings.set_registry_data(RegistryState::Enabled);
                }
                new_settings
            }
            RegistryEntries::ShutdownTime => {
                let mut new_settings = RegistrySetting {
                    registry_entry: entry.clone(),
                    registry_name: entry.to_string(),
                    last_data: RegistryState::Disabled.to_string(),
                };
                let _ = new_settings.update_local_data();
                if new_settings.last_data.is_empty() {
                    let _ = new_settings.set_registry_data("18:00");
                }
                new_settings
            }
        }
    }
    /// Updates the local data for the registry setting.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a problem opening the app key or getting the data from the registry.
    pub fn update_local_data(&mut self) -> Result<String> {
        let app_key = match LOCAL_MACHINE.open(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key with err {}", err);
                create_app_key();
                return Err(err.into());
            }
        };
        let data: String = match app_key.get_string(&self.registry_name) {
            Ok(data) => {
                debug!("Found data {:#?} in {}", data, self.registry_name);
                data
            }
            Err(err) => {
                error!(
                    "Failed to get data from {}, with error {}",
                    &self.registry_name, err
                );
                self.set_registry_data(&self.last_data.clone())?;
                return Err(err.into());
            }
        };
        self.last_data = data.clone();
        Ok(data)
    }
    /// Sets the registry data for the registry setting.
    ///
    /// # Arguments
    ///
    /// * `new_data` - The new data to set in the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if there is a problem setting the data in the registry.
    pub fn set_registry_data<T: Into<String>>(&mut self, new_data: T) -> Result<()> {
        let new_data = new_data.into();
        let app_key = match LOCAL_MACHINE.create(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key: {} with err {}", APP_SUBKEY, err);
                create_app_key();
                return Err(err.into());
            }
        };
        match app_key.set_string(&self.registry_name, &new_data) {
            Ok(()) => {
                info!("Set data {:#?} in {}", new_data, self.registry_name);
                self.last_data = new_data;
            }
            Err(err) => {
                error!(
                    "Failed to set data from {}, with error {}",
                    self.registry_name, err
                );
                return Err(err.into());
            }
        };
        Ok(())
    }
}

#[inline]
fn create_app_key() {
    std::thread::spawn(|| match LOCAL_MACHINE.create(APP_SUBKEY) {
        Ok(_) => info!("Created {}", APP_SUBKEY),
        Err(err) => error!("Failed to create {} with err: {}", APP_SUBKEY, err),
    });
}

#[inline]
#[must_use]
pub fn get_current_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
