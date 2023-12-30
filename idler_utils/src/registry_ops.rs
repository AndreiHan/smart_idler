use std::thread;

use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use windows_registry::LOCAL_MACHINE;

const APP_SUBKEY: &str = "SOFTWARE\\SmartIdler";
const SLEEP_TIME_SECONDS: u64 = 0;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum RegistryState {
    Enabled,
    Disabled,
}

impl ToString for RegistryState {
    fn to_string(&self) -> String {
        match self {
            RegistryState::Enabled => "enabled".to_owned(),
            RegistryState::Disabled => "disabled".to_owned(),
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
    LogStatistics,
    ShutdownTime,
}

impl ToString for RegistryEntries {
    fn to_string(&self) -> String {
        match self {
            RegistryEntries::ForceInterval => String::from("ForceInterval"),
            RegistryEntries::LastRobotInput => String::from("LastRobotInput"),
            RegistryEntries::LogStatistics => String::from("LogStatistics"),
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
        let initial_data = match entry {
            RegistryEntries::ForceInterval => SLEEP_TIME_SECONDS.to_string(),
            RegistryEntries::LastRobotInput => get_current_time(),
            RegistryEntries::LogStatistics => RegistryState::Enabled.to_string(),
            RegistryEntries::ShutdownTime => "18:00".to_string(),
        };

        let mut new_settings = RegistrySetting {
            registry_entry: entry.clone(),
            registry_name: entry.to_string(),
            last_data: initial_data.clone(),
        };

        let status = new_settings.update_local_data();
        trace!("Got status: {status:?}");
        if new_settings.last_data.is_empty() {
            let status = new_settings.set_registry_data(&initial_data);
            trace!("Got status for updating registry data: {status:?}");
        }

        new_settings
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
                error!("Failed to open app key with err {err:?}");
                create_app_key();
                return Err(err.into());
            }
        };
        let data: String = match app_key.get_string(&self.registry_name) {
            Ok(data) => {
                debug!("Found data {data:#?} in {}", self.registry_name);
                data
            }
            Err(err) => {
                error!(
                    "Failed to get data from {}, with error {err:?}",
                    &self.registry_name
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
                error!("Failed to open app key: {APP_SUBKEY} with err {err:?}");
                create_app_key();
                return Err(err.into());
            }
        };
        match app_key.set_string(&self.registry_name, &new_data) {
            Ok(()) => {
                info!("Set data {new_data:#?} in {}", self.registry_name);
                self.last_data = new_data;
            }
            Err(err) => {
                error!(
                    "Failed to set data from {}, with error {err:?}",
                    self.registry_name
                );
                return Err(err.into());
            }
        };
        Ok(())
    }
}

fn create_app_key() {
    thread::spawn(|| match LOCAL_MACHINE.create(APP_SUBKEY) {
        Ok(val) => info!("Created {APP_SUBKEY}, val: {val:?}"),
        Err(err) => error!("Failed to create {APP_SUBKEY} with err: {err}"),
    });
}

#[must_use]
pub fn get_current_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
