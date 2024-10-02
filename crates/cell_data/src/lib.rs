use std::sync::{LazyLock, Mutex};

use once_cell::sync::OnceCell;
use tauri::AppHandle;

pub static TAURI_APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

fn get_lazy_registry_setting(
    data: registry_ops::RegistryEntries,
) -> Mutex<registry_ops::RegistrySetting> {
    Mutex::new(registry_ops::RegistrySetting::new(&data))
}

pub static REGISTRY_LOG_STATISTICS: LazyLock<Mutex<registry_ops::RegistrySetting>> =
    LazyLock::new(|| get_lazy_registry_setting(registry_ops::RegistryEntries::LogStatistics));

pub static REGISTRY_ROBOT_INPUT: LazyLock<Mutex<registry_ops::RegistrySetting>> =
    LazyLock::new(|| get_lazy_registry_setting(registry_ops::RegistryEntries::LastRobotInput));

pub static REGISTRY_FORCE_INTERVAL: LazyLock<Mutex<registry_ops::RegistrySetting>> =
    LazyLock::new(|| get_lazy_registry_setting(registry_ops::RegistryEntries::ForceInterval));

pub static REGISTRY_SHUTDOWN_TIME: LazyLock<Mutex<registry_ops::RegistrySetting>> =
    LazyLock::new(|| get_lazy_registry_setting(registry_ops::RegistryEntries::ShutdownTime));
