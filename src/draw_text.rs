use crate::image_ops::overlay_rgba_on_rgb;
use ab_glyph::{FontRef, PxScale};
use image::imageops;
use image::{RgbImage, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

/// Draw 3 lines of text at the top-left of the photo area.
/// - `dst`: final RGB image (full canvas)
/// - `photo_offset`: (x,y) where the photo starts on the canvas
/// - `photo_size`: (width,height) of the photo
/// - `lines`: exactly 3 lines of text
/// - `font`: loaded TTF font
/// - `margin_px`: margin from photo edges
/// - `color`: text color (RGBA)
pub fn draw_multiline_text_top_left<S: AsRef<str>>(
    dst: &mut RgbImage,
    photo_offset: (u32, u32),
    photo_size: (u32, u32),
    lines: [S; 3],
    font: &FontRef,
    margin_px: u32,
    color: Rgba<u8>,
) {
    let (_photo_w, photo_h) = photo_size;

    // Text height ~4% of photo height (same scale logic as date)
    let line_height_px = (photo_h as f32 * 0.04).max(12.0);
    let scale = PxScale::from(line_height_px);

    // Line spacing: 120% of font size
    let line_spacing = (line_height_px * 1.2).round() as u32;

    // Create a temporary RGBA canvas large enough for 3 lines
    let tmp_w = 2000u32;
    let tmp_h = line_spacing * 3 + 10;
    let mut tmp: RgbaImage = RgbaImage::from_pixel(tmp_w, tmp_h, Rgba([0, 0, 0, 0]));

    // Draw each line
    for (i, text) in lines.iter().enumerate() {
        let y = i as u32 * line_spacing;
        draw_text_mut(&mut tmp, color, 0, y as i32, scale, font, text.as_ref());
    }

    // Crop to bounding box of non-transparent pixels
    let mut min_x = tmp_w;
    let mut min_y = tmp_h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;

    for y in 0..tmp.height() {
        for x in 0..tmp.width() {
            if tmp.get_pixel(x, y)[3] != 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return;
    }

    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;
    let text_img = imageops::crop_imm(&tmp, min_x, min_y, crop_w, crop_h).to_image();

    // Final position: top-left of photo area + margin
    let x = photo_offset.0 + margin_px;
    let y = photo_offset.1 + margin_px;

    overlay_rgba_on_rgb(dst, &text_img, x, y);
}
