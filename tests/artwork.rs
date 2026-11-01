use listen_moe_desktop::radio::{artwork, visualizer::artwork_color};
use std::io::Cursor;

#[test]
fn small_covers_keep_original_bytes_and_color() {
    let bytes = include_bytes!("fixtures/visualizer-cover.jpg");
    let prepared = artwork::prepare(bytes.to_vec()).unwrap();
    assert_eq!(prepared.bytes, bytes);
    assert_eq!(Some(prepared.accent), artwork_color(bytes));
    assert!(artwork::prepare(b"invalid cover".to_vec()).is_none());
}

#[test]
fn large_covers_have_bounded_display_memory_without_changing_accent_or_aspect() {
    let image = image::RgbaImage::from_fn(3000, 1500, |x, _| {
        if x < 2200 {
            image::Rgba([40, 90, 190, 128])
        } else {
            image::Rgba([220, 60, 70, 255])
        }
    });
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    let expected_accent = artwork_color(&bytes);
    let prepared = artwork::prepare(bytes).unwrap();
    assert_eq!(Some(prepared.accent), expected_accent);
    let display = image::load_from_memory(&prepared.bytes).unwrap();
    assert_eq!((display.width(), display.height()), (512, 256));
    assert!(display.as_bytes().len() <= 512 * 512 * 4);
    assert_eq!(
        display.to_rgba8().get_pixel(0, 0).0[3],
        128,
        "thumbnail preparation preserves alpha"
    );
}
