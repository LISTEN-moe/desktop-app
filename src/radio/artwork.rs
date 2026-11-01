//! Prepare artwork once off the UI thread. Keep full-size covers out of GPUI's
//! image cache: even 88px artwork at 4x scaling needs only 352 physical pixels.
use super::visualizer::{Color, color_from_image};
use std::io::Cursor;

pub const MAX_DISPLAY_SIZE: u32 = 512;

pub struct Artwork {
    pub bytes: Vec<u8>,
    pub accent: Color,
}

pub fn prepare(bytes: Vec<u8>) -> Option<Artwork> {
    let image = image::load_from_memory(&bytes).ok()?;
    let accent = color_from_image(&image);
    let bytes = if image.width() > MAX_DISPLAY_SIZE || image.height() > MAX_DISPLAY_SIZE {
        let display = image.resize(
            MAX_DISPLAY_SIZE,
            MAX_DISPLAY_SIZE,
            image::imageops::FilterType::Triangle,
        );
        let mut output = Cursor::new(Vec::new());
        display
            .write_to(&mut output, image::ImageFormat::Png)
            .ok()?;
        output.into_inner()
    } else {
        bytes
    };
    Some(Artwork { bytes, accent })
}
