use std::process;

use idler_utils::idler_win_utils;
use tauri::{
    AppHandle, CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

use crate::app_controller;

pub enum IdlerMenuItems {
    Show,
    Quit,
}

impl ToString for IdlerMenuItems {
    fn to_string(&self) -> String {
        match self {
            IdlerMenuItems::Show => "Show".to_owned(),
            IdlerMenuItems::Quit => "Quit".to_owned(),
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

pub(crate) fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "Show" => {
                app_controller::build_controller(app);
            }
            "Quit" => {
                idler_win_utils::ExecState::stop();
                process::exit(0);
            }
            _ => {
                warn!("Unknown menu item: {}", id);
            }
        },
        SystemTrayEvent::LeftClick { .. } | SystemTrayEvent::DoubleClick { .. } => {
            app_controller::build_controller(app);
        }
        _ => {}
    }
}
