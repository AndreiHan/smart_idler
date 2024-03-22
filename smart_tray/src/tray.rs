use std::sync::Arc;

use idler_utils::{idler_win_utils, single_instance::SingleInstance};
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
            IdlerMenuItems::Show => "Show".to_string(),
            IdlerMenuItems::Quit => "Quit".to_string(),
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

pub(crate) fn handle_system_tray_event(
    app: &AppHandle,
    event: SystemTrayEvent,
    moved_instance: &Arc<SingleInstance>,
) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => {
            let _item_handle = app.tray_handle().get_item(&id);
            let id = id.as_str();
            if id == IdlerMenuItems::Show.to_string() {
                app_controller::build_controller(app);
            } else if id == IdlerMenuItems::Quit.to_string() {
                idler_win_utils::ExecState::stop();
                let _ = moved_instance.exit();
                std::process::exit(0);
            }
        }
        SystemTrayEvent::LeftClick { .. } | SystemTrayEvent::DoubleClick { .. } => {
            app_controller::build_controller(app);
        }
        _ => {}
    }
}
