use exif::{Field, In, Tag, Value};
use jiff::civil::DateTime;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::debug;

use crate::error::AppError;

pub fn get_image_date<P: AsRef<Path>>(file_path: P) -> Result<Option<DateTime>, AppError> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut reader)?;

    // Look for the "DateTimeOriginal" tag (Tag 36867)
    let Some(Field {
        value: Value::Ascii(dates),
        ..
    }) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY)
    else {
        debug!("No original date found");
        return Ok(None);
    };

    // The dates value is a vector of bytes, convert it to a string.
    // The standard EXIF format is "YYYY:MM:DD HH:MM:SS"
    let date_str = String::from_utf8(dates[0].clone())?;
    match DateTime::strptime("%Y:%m:%d %H:%M:%S", &date_str) {
        Ok(date) => Ok(Some(date)),
        Err(e) => {
            debug!("{e}. Could not parse {date_str} as jiff Timestamp.");
            Ok(None)
        }
    }
}
