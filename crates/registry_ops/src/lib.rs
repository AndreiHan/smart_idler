use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, error, info, trace};

const APP_SUBKEY: &str = "SOFTWARE\\SmartIdler";
const SLEEP_TIME_SECONDS: u64 = 60;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum RegistryState {
    Enabled,
    Disabled,
}

impl fmt::Display for RegistryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self == &RegistryState::Enabled {
            write!(f, "Enabled")
        } else {
            write!(f, "Disabled")
        }
    }
}

impl From<RegistryState> for String {
    fn from(val: RegistryState) -> Self {
        val.to_string()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum RegistryEntries {
    ForceInterval,
    LastRobotInput,
    LogStatistics,
    ShutdownTime,
}

impl fmt::Display for RegistryEntries {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RegistryEntries::ForceInterval => write!(f, "ForceInterval"),
            RegistryEntries::LastRobotInput => write!(f, "LastRobotInput"),
            RegistryEntries::LogStatistics => write!(f, "LogStatistics"),
            RegistryEntries::ShutdownTime => write!(f, "ShutdownTime"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RegistrySetting {
    pub registry_entry: RegistryEntries,
    pub last_data: String,
}

impl RegistrySetting {
    #[must_use]
    pub fn new(entry: &RegistryEntries) -> RegistrySetting {
        let initial_data = match entry {
            RegistryEntries::ForceInterval => SLEEP_TIME_SECONDS.to_string(),
            RegistryEntries::LastRobotInput => get_current_time(),
            RegistryEntries::LogStatistics => RegistryState::Disabled.to_string(),
            RegistryEntries::ShutdownTime => "18:00".to_string(),
        };

        let mut new_settings = RegistrySetting {
            registry_entry: *entry,
            last_data: initial_data.clone(),
        };

        let status = new_settings.update_local_from_registry();
        trace!("Got status: {status:?}");

        if status.is_err() {
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
    pub fn update_local_from_registry(&mut self) -> Result<String> {
        let app_key = match windows_registry::LOCAL_MACHINE.open(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key with err {err:?}");
                create_app_key();
                return Err(err.into());
            }
        };
        let data: String = match app_key.get_string(self.registry_entry.to_string()) {
            Ok(data) => {
                debug!(
                    "Found data {data:#?} in {}",
                    self.registry_entry.to_string()
                );
                data
            }
            Err(err) => {
                error!(
                    "Failed to get data from {}, with error {err:?}",
                    &self.registry_entry
                );
                self.set_registry_data(self.last_data.clone())?;
                return Err(err.into());
            }
        };
        self.last_data.clone_from(&data);
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
        let app_key = match windows_registry::LOCAL_MACHINE.create(APP_SUBKEY) {
            Ok(e) => e,
            Err(err) => {
                error!("Failed to open app key: {APP_SUBKEY} with err {err:?}");
                create_app_key();
                return Err(err.into());
            }
        };
        match app_key.set_string(&self.registry_entry.to_string(), &new_data) {
            Ok(()) => {
                info!(
                    "Set data {new_data:#?} in {}",
                    self.registry_entry.to_string()
                );
                self.last_data = new_data;
            }
            Err(err) => {
                error!(
                    "Failed to set data from {}, with error {err:?}",
                    self.registry_entry.to_string()
                );
                return Err(err.into());
            }
        };
        Ok(())
    }

    /// Checks if the registry setting is enabled. (i.e. `last_data` != disabled)
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.last_data != RegistryState::Disabled.to_string()
    }
}

fn create_app_key() {
    match windows_registry::LOCAL_MACHINE.create(APP_SUBKEY) {
        Ok(val) => info!("Created {APP_SUBKEY}, val: {val:?}"),
        Err(err) => error!("Failed to create {APP_SUBKEY} with err: {err}"),
    }
}

#[must_use]
pub fn get_current_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}
