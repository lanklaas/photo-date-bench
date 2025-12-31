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

    if let Err(e) = photo_date_bench::run_image_processing(App {source: source_folder, target: target_folder, threads: None}) {
        error!("{e}");
    }
    
    Ok(())
}