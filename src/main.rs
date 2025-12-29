mod error;
mod image_ops;
mod parse_exif;

use ab_glyph::FontRef;
use clap::Parser;
use error::AppError;
use image::{imageops, DynamicImage, GenericImage, ImageBuffer, Rgb, RgbImage, Rgba};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::{collections::BTreeMap, path::Path};
use threadpool::ThreadPool;
use tracing::error;
use tracing::{debug, info};
use tracing_subscriber::{
    fmt::format::FmtSpan, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};
use walkdir::WalkDir;

#[derive(Debug, clap::Parser)]
#[clap(about = "A command line tool to add dates to images and rescale them")]
struct App {
    #[arg(help = "Path to the directory conaining the image files.")]
    path: PathBuf,
    #[clap(short, help = "Output directory", default_value = "./output")]
    output: PathBuf,
}

const WIDTH_CM: f32 = 8.0;
const HEIGHT_CM: f32 = 6.0;
const DPI: f32 = 300.0;

const TEXT_COLOR_RGB: (u8, u8, u8) = (255, 140, 0); // orange
const MARGIN_MM: f32 = 5.0;
const BACKGROUND_RGB: (u8, u8, u8) = (255, 255, 255); // white

const fn mm_to_px(mm: f32) -> u32 {
    ((mm / 25.4) * DPI).round() as u32
}

const fn cm_to_px(cm: f32) -> u32 {
    ((cm / 2.54) * DPI).round() as u32
}

const TARGET_W: u32 = cm_to_px(WIDTH_CM);
const TARGET_H: u32 = cm_to_px(HEIGHT_CM);
const MARGIN_PX: u32 = mm_to_px(MARGIN_MM);

fn main() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let App { path, output } = App::parse();
    let root = path;

    let font = image_ops::load_font()?;

    // =========================
    // Auto-detect start number
    // =========================
    let max_num = image_ops::find_max_number_jpg(&root)?;
    let number = max_num + 1;
    info!("Start number automatically set to: {}", number);

    // =========================
    // Collect images grouped by date
    // =========================
    let mut images_by_date: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !image_ops::is_image_file(path) {
            continue;
        }

        let filename = path;
        debug!("Getting date info for {filename:?}");
        let Some(meta_date) = parse_exif::get_image_date(filename)? else {
            if let Some(date) = image_ops::date_from_filename(filename) {
                images_by_date
                    .entry(date)
                    .or_default()
                    .push(path.to_path_buf());
            }
            continue;
        };
        images_by_date
            .entry(meta_date.strftime("%Y%m%d").to_string())
            .or_default()
            .push(path.to_path_buf());
    }

    // =========================
    // Process by date
    // =========================
    let work_cpus = num_cpus::get() / 2;
    info!("Using {work_cpus} cpus to process images");
    let tp = ThreadPool::new(work_cpus);
    let number: Arc<AtomicUsize> = Arc::new(number.into());
    for (date, list) in images_by_date.into_iter() {
        let out_dir = root.join(format!("output_{}", date));
        fs::create_dir_all(&out_dir)?;

        info!("\n➡️ Processing date {} → folder: {:?}", date, out_dir);

        for path in list {
            let out_dir = out_dir.clone();
            let number = number.clone();
            let font = font.clone();
            let date = date.clone();
            tp.execute(move || {
                if let Err(e) = process_image(&path, font, &date, &number, out_dir) {
                    error!("{e}");
                }
            });
        }
    }

    tp.join();

    info!("\n🎉 Done! All new photos were saved per date into separate folders and numbered.");
    Ok(())
}

fn process_image(
    path: &Path,
    font: FontRef,
    date: &str,
    number: &AtomicUsize,
    out_dir: PathBuf,
) -> Result<(), AppError> {
    let img = image::open(path)?.to_rgb8();
    let dyn_img = DynamicImage::ImageRgb8(img);

    // Resize to fit
    let resized = image_ops::resize_to_fit(&dyn_img, TARGET_W, TARGET_H).to_rgb8();
    let (rw, rh) = (resized.width(), resized.height());

    // Create fixed-size white canvas
    let mut final_img: RgbImage = ImageBuffer::from_pixel(
        TARGET_W,
        TARGET_H,
        Rgb([BACKGROUND_RGB.0, BACKGROUND_RGB.1, BACKGROUND_RGB.2]),
    );

    let offset_x = ((TARGET_W as i32 - rw as i32) / 2).max(0) as u32;
    let offset_y = ((TARGET_H as i32 - rh as i32) / 2).max(0) as u32;

    final_img.copy_from(&resized, offset_x, offset_y)?;

    // --- Text sizing: ~4% of photo height (similar to the Python)
    // We render at some pixel height, crop, then if needed scale down to fit width constraints.
    let desired_text_h = (rh as f32) * 0.04;
    let orange = Rgba([TEXT_COLOR_RGB.0, TEXT_COLOR_RGB.1, TEXT_COLOR_RGB.2, 255]);

    let mut text_img = image_ops::render_text_crop(&font, date, desired_text_h, orange);

    // Ensure it fits inside the photo area with margins.
    let max_text_w = rw.saturating_sub(2 * MARGIN_PX).max(1);
    if text_img.width() > max_text_w {
        let scale = max_text_w as f32 / text_img.width() as f32;
        let new_w = (text_img.width() as f32 * scale).round().max(1.0) as u32;
        let new_h = (text_img.height() as f32 * scale).round().max(1.0) as u32;
        text_img = imageops::resize(&text_img, new_w, new_h, imageops::FilterType::Lanczos3);
    }

    // Paste bottom-right relative to the photo area (not the full canvas)
    let x = offset_x + rw.saturating_sub(text_img.width() + MARGIN_PX);
    let y = offset_y + rh.saturating_sub(text_img.height() + MARGIN_PX);

    // Alpha-blend RGBA text onto RGB final image
    image_ops::overlay_rgba_on_rgb(&mut final_img, &text_img, x, y);

    // Save as sequential number
    let new_name = format!("{}.jpg", number.load(std::sync::atomic::Ordering::SeqCst));
    let out_path = out_dir.join(&new_name);

    let dyn_out = DynamicImage::ImageRgb8(final_img);
    let mut file = std::fs::File::create(&out_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
    encoder.encode_image(&dyn_out)?;

    info!(
        "✅ {} → {}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("(unknown)"),
        new_name
    );

    number.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
