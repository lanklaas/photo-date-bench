use crate::image_ops::overlay_premul_rgba_on_rgb;
use ab_glyph::{FontRef, PxScale};
use image::imageops;
use image::{RgbImage, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;

#[derive(Debug, Clone, Copy, Default)]
pub enum DrawPosition {
    #[default]
    TopLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy)]
pub struct PhotoSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PhotoOffset {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug)]
pub struct MultilineDraw<'a> {
    /// - `photo_size`: (width,height) of the photo
    pub photo_size: PhotoSize,
    /// - `photo_offset`: (x,y) where the photo starts on the canvas
    pub photo_offset: PhotoOffset,
    /// - `margin_px`: margin from photo edges
    pub margin_px: u32,
    /// - `dst`: final RGB image (full canvas)
    pub destination: &'a mut RgbImage,
}

const fn pt_to_px(pt: usize, dpi: f32) -> f32 {
    pt as f32 * (dpi / 72.)
}

#[derive(Debug, Clone, Copy)]
pub struct FontSize {
    pub pt: usize,
    pub dpi: f32,
}

impl FontSize {
    fn as_px_scale(&self) -> PxScale {
        let px = pt_to_px(self.pt, self.dpi);
        PxScale::from(px)
    }
}

impl<'a> MultilineDraw<'a> {
    /// Draw lines of text at the specified of the photo area.
    /// - `lines`: exactly 3 lines of text
    /// - `font`: loaded TTF font
    /// - `color`: text color (RGBA)
    pub fn draw_multiline_text<S: AsRef<str>>(
        &mut self,
        lines: &[S],
        font: &FontRef,
        font_size: FontSize,
        color: Rgba<u8>,
        position: DrawPosition,
    ) {
        let &mut Self {
            ref photo_size,
            ref photo_offset,
            ref margin_px,
            ref mut destination,
        } = self;
        // Text height ~4% of photo height (same scale logic as date)
        let line_height_px = (photo_size.height as f32 * 0.04).max(12.0);
        let scale = font_size.as_px_scale();

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

        match position {
            DrawPosition::TopLeft => {
                // Final position: top-left of photo area + margin
                let x = photo_offset.x + margin_px;
                let y = photo_offset.y + margin_px;

                overlay_premul_rgba_on_rgb(destination, &text_img, x, y);
            }
            DrawPosition::BottomRight => {
                // Paste bottom-right relative to the photo area (not the full canvas)
                let x = photo_offset.x
                    + photo_size
                        .width
                        .saturating_sub(text_img.width() + margin_px);
                let y = photo_offset.y
                    + photo_size
                        .height
                        .saturating_sub(text_img.height() + margin_px);
                overlay_premul_rgba_on_rgb(destination, &text_img, x, y);
            }
        }
    }
}
