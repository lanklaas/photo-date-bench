use std::{io, path::PathBuf, string::FromUtf8Error};

use ab_glyph::InvalidFont;
use image::ImageError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Image(#[from] ImageError),
    #[error(transparent)]
    InvalidFont(#[from] InvalidFont),
    #[error(transparent)]
    Exif(#[from] exif::Error),
    #[error(transparent)]
    Utf8Parse(#[from] FromUtf8Error),
    #[error(transparent)]
    DateTimeParse(#[from] jiff::Error),
    #[error("The file {0} could not be processed onto {1} as the numbered file already exists.")]
    OutNumberExists(PathBuf, PathBuf),
    #[error("Could not get a date from the file {0:?}")]
    NoParsibleDate(PathBuf),
}
