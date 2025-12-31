
use ab_glyph::{FontRef, PxScale};
use crate::error::AppError;
use image::{
    imageops, DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbImage, Rgba,
    RgbaImage,
};
use imageproc::drawing::draw_text_mut;
use regex::Regex;
use std::ffi::OsStr;
use std::path::Path;
use walkdir::WalkDir;

/// Try to extract a date from filename, output "YYYY-MM-DD".
pub fn date_from_filename<P: AsRef<Path>>(path: P) -> Option<String> {
    let name = path.as_ref().file_name().unwrap_or_default().to_str().expect("Filename to be utf8");
    // Patterns:
    // 1) 20251224
    // 2) 2025-12-24 or 2025_12_24 or 2025.12.24
    // 3) 24.12.2025
    let re1 = Regex::new(r"(20\d{2})(\d{2})(\d{2})").ok()?;
    let re2 = Regex::new(r"(20\d{2})[-_.](\d{2})[-_.](\d{2})").ok()?;
    let re3 = Regex::new(r"(\d{2})[.](\d{2})[.](20\d{2})").ok()?;

    if let Some(c) = re1.captures(name) {
        return Some(format!("{}-{}-{}", &c[1], &c[2], &c[3]));
    }
    if let Some(c) = re2.captures(name) {
        return Some(format!("{}-{}-{}", &c[1], &c[2], &c[3]));
    }
    if let Some(c) = re3.captures(name) {
        return Some(format!("{}-{}-{}", &c[3], &c[2], &c[1]));
    }
    None
}

pub fn load_bold_font() -> Result<FontRef<'static>, AppError> {
    // Bundle the font with the program so it works the same on Ubuntu + Windows.
    let font_data: &[u8] = include_bytes!("../assets/arialroundedmtbold.ttf");
    
    Ok(FontRef::try_from_slice(font_data)?)
}

pub fn load_arial_bold() -> Result<FontRef<'static>, AppError> {
    // Bundle the font with the program so it works the same on Ubuntu + Windows.
    let font_data: &[u8] = include_bytes!("../assets/ARIALBD.TTF");
    
    Ok(FontRef::try_from_slice(font_data)?)
}


/// Find the maximum N in filenames matching `N.jpg` anywhere under SOURCE_FOLDER.
pub fn find_max_number_jpg(root: &Path) -> Result<usize, AppError> {
    let re = Regex::new(r"^(\d+)\.jpg$")?;
    let mut max_num = 0;

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if let Some(c) = re.captures(&name)
            && let Ok(n) = c[1].parse::<usize>() {
                max_num = max_num.max(n);
            }
    }
    Ok(max_num)
}

pub fn is_image_file(path: &Path) -> bool {
    matches!(path
        .extension()
        .and_then(OsStr::to_str)
        .map(|s| s.to_lowercase()), Some(ext) if ext == "jpg" || ext == "jpeg" || ext == "png")
}

/// Resize to fit within (target_w, target_h) preserving aspect ratio (like PIL thumbnail).
pub fn resize_to_fit(img: &DynamicImage, target_w: u32, target_h: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w <= target_w && h <= target_h {
        return img.clone();
    }
    img.resize(target_w, target_h, imageops::FilterType::Lanczos3)
}

/// Create a transparent RGBA image, render text, and return its tight bounding box crop.
pub fn render_text_crop(font: &FontRef, text: &str, px_height: f32, color: Rgba<u8>) -> RgbaImage {
    // Render on a generous canvas first, then crop to bounding box of non-transparent pixels.
    let canvas_w = 2000u32;
    let canvas_h = 800u32;
    let mut tmp: RgbaImage = ImageBuffer::from_pixel(canvas_w, canvas_h, Rgba([0, 0, 0, 0]));

    let scale = PxScale::from(px_height.max(1.0));
    draw_text_mut(&mut tmp, color, 0, 0, scale, font, text);

    // Find bounding box of non-transparent pixels.
    let mut min_x = canvas_w;
    let mut min_y = canvas_h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;

    for y in 0..canvas_h {
        for x in 0..canvas_w {
            let a = tmp.get_pixel(x, y)[3];
            if a != 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
    }

    let crop_w = (max_x - min_x + 1).max(1);
    let crop_h = (max_y - min_y + 1).max(1);
    imageops::crop_imm(&tmp, min_x, min_y, crop_w, crop_h).to_image()
}

/// Overlay premultiplied-alpha RGBA src onto RGB dst at (x,y).
pub fn overlay_premul_rgba_on_rgb(dst: &mut RgbImage, src: &RgbaImage, x: u32, y: u32) {
    for sy in 0..src.height() {
        for sx in 0..src.width() {
            let dx = x + sx;
            let dy = y + sy;
            if dx >= dst.width() || dy >= dst.height() {
                continue;
            }

            let sp = src.get_pixel(sx, sy);
            let a = sp[3] as f32 / 255.0;
            if a <= 0.0 { continue; }

            let dp = dst.get_pixel(dx, dy);

            // sp[0..2] are ALREADY multiplied by a
            let out_r = (sp[0] as f32 + dp[0] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
            let out_g = (sp[1] as f32 + dp[1] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
            let out_b = (sp[2] as f32 + dp[2] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;

            dst.put_pixel(dx, dy, Rgb([out_r, out_g, out_b]));
        }
    }
}
