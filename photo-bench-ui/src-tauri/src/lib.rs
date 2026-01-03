use tauri_plugin_shell::ShellExt;

use tauri::Manager;

pub mod photobench;
mod tracing;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

struct AppState {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState {});
            tracing::init_tracing(app.app_handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            open_download_folder,
            photobench::process_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn open_download_folder(app_handle: tauri::AppHandle, target_folder: &str) {
    app_handle.shell().open(target_folder, None).unwrap();
}
