use once_cell::sync::OnceCell;
use tauri::AppHandle;

pub static TAURI_APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
