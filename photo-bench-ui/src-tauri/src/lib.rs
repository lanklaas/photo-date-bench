use tauri_plugin_shell::ShellExt;
use tracing_subscriber::{
    fmt::format::FmtSpan, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

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
    // let mut log_dir = dirs::data_local_dir().unwrap();
    // log_dir.push("com.gmail-attachment-downloader.app");
    // fs::create_dir_all(&log_dir).unwrap();
    // log_dir.push("gdownloader.log");
    // let mut append = true;
    // let mut truncate = false;
    // let mb_size = if log_dir.exists() {
    //     let meta = log_dir.metadata().unwrap();
    //     let mb = meta.size() as f64 / 1024.0 / 1024.0;
    //     if mb > 100.0 {
    //         append = false;
    //         truncate = true;
    //     }
    //     mb
    // } else {
    //     0.
    // };
    // let debug_file = OpenOptions::new()
    //     .append(append)
    //     .truncate(truncate)
    //     .create(true)
    //     .open(&log_dir)
    //     .unwrap();
    // let (non_blocking, _guard) = tracing_appender::non_blocking(debug_file);
    // tracing_subscriber::registry()
    //     // .with(
    //     //     fmt::layer()
    //     //         .with_writer(non_blocking)
    //     //         .with_ansi(false)
    //     //         .with_filter(LevelFilter::from_level(Level::DEBUG)),
    //     // )
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "info,tiberius=error,odbc_api=error".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
    //     .init();
    // info!("Logs initialized at {log_dir:?}. Log file size: {mb_size}MB");

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
