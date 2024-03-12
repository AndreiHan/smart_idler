use tauri::{CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayMenuItem};

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
