pub mod draw_text;
pub mod error;
mod image_ops;
mod parse_exif;

use ab_glyph::FontRef;
use image::codecs::jpeg::PixelDensity;
use jiff::civil::DateTime;

use draw_text::{DrawPosition, FontSize, MultilineDraw, PhotoOffset, PhotoSize};
use error::AppError;
use image::{DynamicImage, GenericImage, ImageBuffer, Rgb, RgbImage, Rgba};
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use threadpool::ThreadPool;
use tracing::error;
use tracing::info;
use walkdir::WalkDir;

#[derive(Debug, clap::Parser)]
#[clap(about = "A command line tool to add dates to images and rescale them")]
pub struct App {
    #[arg(help = "Path to the directory conaining the image files to be processed")]
    pub source: PathBuf,
    #[arg(
        help = "Path to the directory conaining the folders where the processed images should be saved."
    )]
    pub target: PathBuf,
    #[clap(
        short,
        help = "The amount of cpus to use to process images. The default is all the available cpus on the computer"
    )]
    pub threads: Option<usize>,
}

const WIDTH_CM: f32 = 8.0;
const HEIGHT_CM: f32 = 6.0;
const DPI: f32 = 300.0;

const TEXT_COLOR_RGB: (u8, u8, u8) = (255, 140, 0); // orange
const MARGIN_MM: f32 = 5.0;
const BACKGROUND_RGB: (u8, u8, u8) = (255, 255, 255); // white

const YELLOW: Rgba<u8> = Rgba([255, 255, 84, 255]);
const ORANGE: Rgba<u8> = Rgba([TEXT_COLOR_RGB.0, TEXT_COLOR_RGB.1, TEXT_COLOR_RGB.2, 255]);

const fn mm_to_px(mm: f32) -> u32 {
    ((mm / 25.4) * DPI).round() as u32
}

const fn cm_to_px(cm: f32) -> u32 {
    ((cm / 2.54) * DPI).round() as u32
}

const TARGET_W: u32 = cm_to_px(WIDTH_CM);
const TARGET_H: u32 = cm_to_px(HEIGHT_CM);
const MARGIN_PX: u32 = mm_to_px(MARGIN_MM);

pub fn run_image_processing(
    App {
        source,
        target,
        threads,
    }: App,
    #[cfg(feature = "emit-progress")] emit: impl Fn(&str, String) + Clone + Send + 'static,
) -> Result<(), AppError> {
    let root = source;
    let font = image_ops::load_bold_font()?;
    let regular_font = image_ops::load_arial_bold()?;

    // =========================
    // Auto-detect start number
    // =========================
    let max_num = image_ops::find_max_number_jpg(&target)?;
    let number = max_num + 1;
    info!("Start number automatically set to: {}", number);

    // =========================
    // Collect images grouped by date
    // =========================
    let mut images = vec![];

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !image_ops::is_image_file(path) {
            continue;
        }

        // Do not process files that was previously done
        if filename_is_number_only(path)? {
            continue;
        }

        images.push(path.to_path_buf());
    }

    // =========================
    // Process by date
    // =========================
    let work_cpus = threads.unwrap_or(num_cpus::get());
    info!("Using {work_cpus} cpus to process images");
    let tp = ThreadPool::new(work_cpus);
    let number: Arc<AtomicUsize> = Arc::new(number.into());
    #[cfg(feature = "emit-progress")]
    let total: usize = images.len();

    #[cfg(feature = "emit-progress")]
    emit("process-file-total", total.to_string());
    #[cfg(feature = "emit-progress")]
    let complete: Arc<AtomicUsize> = Arc::new(0.into());
    for image_path in images.into_iter() {
        let date = parse_image_date(&image_path)?;
        let date_folder_format = date.strftime("%Y%m%d").to_string();
        let out_dir = target.join(&date_folder_format);
        fs::create_dir_all(&out_dir)?;
        info!("\n➡️ Processing date {} → folder: {:?}", date, out_dir);

        let out_dir = out_dir.clone();
        let number = number.clone();
        let font = font.clone();
        let regular_font = regular_font.clone();

        #[cfg(feature = "emit-progress")]
        let emit = emit.clone();
        #[cfg(feature = "emit-progress")]
        let complete = complete.clone();
        tp.execute(move || {
            #[cfg(feature = "emit-progress")]
            let fname = image_path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap_or_default()
                .to_string();
            #[cfg(feature = "emit-progress")]
            emit("process-file", fname.clone());

            if let Err(e) = process_image(&image_path, font, regular_font, &date, &number, out_dir)
            {
                error!(
                    "{e}, this error might have caused the cache directory not to be cleaned up."
                );
            }
            #[cfg(feature = "emit-progress")]
            {
                let comp = complete.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let pct = (comp as f32 / total as f32) * 100f32;
                emit("process-progress", pct.to_string());
                emit("process-file-done", fname);
            }
        });
    }

    tp.join();
    #[cfg(feature = "emit-progress")]
    emit("process-complete", "".to_string());

    info!("\n🎉 Done! All new photos were saved per date into separate folders and numbered.");
    Ok(())
}

fn process_image(
    path: &Path,
    font: FontRef,
    regular_font: FontRef,
    date: &DateTime,
    number: &AtomicUsize,
    out_dir: PathBuf,
) -> Result<(), AppError> {
    let number = number.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    // Save as sequential number
    let new_name = format!("{number}.jpg");
    let out_path = out_dir.join(&new_name);

    if out_path.exists() {
        return Err(AppError::OutNumberExists(path.to_path_buf(), out_path));
    }

    // This is what the tauri app is named and stores the exe in the same location on install
    let Some(proj_dir) = directories::ProjectDirs::from("", "", "photo-bench-ui") else {
        error!("Could not find path to temp directories. Could not process file: {path:?}");
        return Ok(());
    };

    // If the image is on a network drive, copy it first instead of processing over the network
    let cache_dir = proj_dir.cache_dir().to_path_buf();
    fs::create_dir_all(&cache_dir)?;

    let cache_file_path = cache_dir.join(&new_name);

    let mut source = BufReader::new(File::open(path)?);
    let mut target = BufWriter::new(File::create(&cache_file_path)?);

    io::copy(&mut source, &mut target)?;

    let img = image::open(&cache_file_path)?.to_rgb8();

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

    let mut text_draw = MultilineDraw {
        photo_size: PhotoSize {
            width: rw,
            height: rh,
        },
        photo_offset: PhotoOffset {
            x: offset_x,
            y: offset_y,
        },
        margin_px: MARGIN_PX,
        destination: &mut final_img,
    };

    let fs = FontSize { pt: 10, dpi: DPI };

    text_draw.draw_multiline_text(
        &[date.strftime("%d %m %Y").to_string()],
        &font,
        fs,
        ORANGE,
        DrawPosition::BottomRight,
    );

    let toptext = format_filename_as_image_text(path, number)?;

    let fs = FontSize { pt: 8, dpi: DPI };

    // Paste top-left relative to the photo area (not the full canvas)
    text_draw.draw_multiline_text(&toptext, &regular_font, fs, YELLOW, DrawPosition::TopLeft);

    let dyn_out = DynamicImage::ImageRgb8(final_img);

    let cache_out_file = cache_dir.join(format!("{number}_out.jpg"));
    let mut file = std::fs::File::create(&cache_out_file)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);

    // Make Word (and others) compute a sane physical size:
    // width_in_inches = pixels / 300, etc.
    encoder.set_pixel_density(PixelDensity::dpi(300));

    encoder.encode_image(&dyn_out)?;

    if let Err(e) = fs::remove_file(&cache_file_path) {
        error!("{e:?}. Could not remove cached file.");
    }

    let mut source = BufReader::new(File::open(&cache_out_file)?);
    let mut target = BufWriter::new(File::create(&out_path)?);

    io::copy(&mut source, &mut target)?;

    if let Err(e) = fs::remove_file(&cache_out_file) {
        error!("{e:?}. Could not remove cached ouput file.");
    }

    info!(
        "✅ {} → {}",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("(unknown)"),
        new_name
    );

    Ok(())
}

/// Reads additional info from the file name and formats it for rendering to the image
fn format_filename_as_image_text<P: AsRef<Path>>(
    path: P,
    number: usize,
) -> Result<Vec<String>, AppError> {
    let Some(name) = path.as_ref().file_name().and_then(|x| x.to_str()) else {
        return default_number_text(number);
    };

    let mut named_chunks = name
        .split("_")
        .filter(|x| x.chars().next().is_some_and(|x| !x.is_ascii_digit()))
        .map(ToString::to_string)
        .collect();
    let mut ret = default_number_text(number)?;
    ret.append(&mut named_chunks);
    Ok(ret)
}

fn default_number_text(number: usize) -> Result<Vec<String>, AppError> {
    Ok(vec![format!("Foto Nr.: {number}")])
}

fn filename_is_number_only(path: &Path) -> Result<bool, AppError> {
    let Some(name) = path.file_stem().and_then(|x| x.to_str()) else {
        return Ok(false);
    };

    Ok(name.parse::<usize>().is_ok())
}

fn parse_image_date<P: AsRef<Path>>(path: P) -> Result<DateTime, AppError> {
    let path = path.as_ref();
    let Some(meta_date) = parse_exif::get_image_date(path)? else {
        if let Some(date) = image_ops::date_from_filename(path) {
            return Ok(date);
        }
        error!("Could not extract date from file: {path:?}");
        return Err(AppError::NoParsibleDate(path.to_path_buf()));
    };
    Ok(meta_date)
}
