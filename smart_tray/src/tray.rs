use std::fmt;
use tracing::{error, info, trace, warn};

use anyhow::Result;
use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, UserAttentionType,
};

#[derive(PartialEq)]
pub enum IdlerMenuItems {
    Show,
    Quit,
}

impl fmt::Display for IdlerMenuItems {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self == &IdlerMenuItems::Show {
            write!(f, "Show")
        } else {
            write!(f, "Quit")
        }
    }
}

impl From<IdlerMenuItems> for String {
    fn from(item: IdlerMenuItems) -> Self {
        item.to_string()
    }
}

pub fn get_tray_menu() -> SystemTray {
    SystemTray::new().with_menu(
        SystemTrayMenu::new()
            .add_item(CustomMenuItem::new(
                IdlerMenuItems::Show,
                IdlerMenuItems::Show,
            ))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(CustomMenuItem::new(
                IdlerMenuItems::Quit,
                IdlerMenuItems::Quit,
            )),
    )
}

fn focus_window(app: &AppHandle) -> Result<()> {
    let Some(window) = app.get_window("controller") else {
        error!("Failed to get window");
        return Err(anyhow::anyhow!("Failed to get window"));
    };

    let status = window.show();
    trace!("Show status: {status:?}");
    let status = window.set_focus();
    trace!("Focus status: {status:?}");
    let status = window.request_user_attention(Some(UserAttentionType::Informational));
    trace!("User attention status: {status:?}");
    Ok(())
}

fn create_window(app: &AppHandle) {
    if let Some(window_config) = app.config().tauri.windows.first().cloned() {
        match tauri::WindowBuilder::from_config(app, window_config).build() {
            Ok(window) => match window.show() {
                Ok(()) => {
                    trace!("Window shown");
                }
                Err(err) => {
                    error!("Failed to show window with err: {err}");
                }
            },
            Err(err) => {
                error!("Failed to create window with err: {err}");
            }
        }
    } else {
        error!("No window configuration found");
    }
}

pub(crate) fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "Show" => match focus_window(app) {
                Ok(()) => {}
                Err(err) => {
                    error!("Failed to focus window with err: {err}");
                    create_window(app);
                }
            },
            "Quit" => {
                idler_utils::ExecState::stop();
                let app_handle = cell_data::TAURI_APP_HANDLE.get().unwrap_or_else(|| {
                    error!("Failed to get app handle");
                    std::process::exit(0);
                });
                info!("Exiting app with app handle");
                app_handle.exit(0);
            }
            _ => {
                warn!("Unknown menu item: {}", id);
            }
        },
        SystemTrayEvent::LeftClick { .. } | SystemTrayEvent::DoubleClick { .. } => {
            match focus_window(app) {
                Ok(()) => {}
                Err(err) => {
                    error!("Failed to focus window with err: {err}");
                    create_window(app);
                }
            }
        }
        _ => {}
    }
}
