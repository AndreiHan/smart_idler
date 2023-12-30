fn main() {
    let mut windows = tauri_build::WindowsAttributes::new();
    windows = windows
        .app_manifest(include_str!("idler.manifest"))
        .window_icon_path("icons/icon.ico");
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run build script");
}
