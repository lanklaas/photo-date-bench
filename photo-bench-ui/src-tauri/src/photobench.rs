use tauri::Emitter;
use photo_date_bench::App;
use tauri::AppHandle;
use std::path::PathBuf;
use tracing::error;

#[tauri::command]
pub async fn process_images(
    app: AppHandle,
    source_folder: PathBuf,
    target_folder: PathBuf,
) -> Result<(), ()> {

    let send_event = move |event: &str, payload: String| {
        println!("{event}: {payload}");
        if let Err(e) = app.emit(event, payload) {
            error!("{e}, while emitting event {event}");
        }
    };

     tauri::async_runtime::spawn_blocking(|| {
        if let Err(e) = photo_date_bench::run_image_processing(App {source: source_folder, target: target_folder, threads: None}, send_event) {
            error!("{e}");
        }
     }).await.unwrap();
    
    Ok(())
}

