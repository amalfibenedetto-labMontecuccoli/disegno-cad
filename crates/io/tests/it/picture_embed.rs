//! A PNG or JPEG placed in a drawing travels with the file: it is embedded in the IMAGE entity's
//! extended data and shows again after DXF or DWG save and reopen (`raster_image`).

use cadcraft_doc::{Common, Drawing, EntityKind, Image, Space};
use cadcraft_geom::{Vec2, Vec3};
use cadcraft_io::raster_image;
use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

fn encoded(img: impl Into<image::DynamicImage>, format: ImageFormat) -> Vec<u8> {
    let mut out = Vec::new();
    img.into().write_to(&mut std::io::Cursor::new(&mut out), format).unwrap();
    out
}

/// A drawing with `bytes` placed as a picture.
fn drawing_with_picture(bytes: &[u8]) -> Drawing {
    let pic = raster_image::import(bytes).unwrap();
    let image = Image {
        insert: Vec3::new(2.0, 3.0, 0.0),
        u: Vec3::new(0.1, 0.0, 0.0),
        v: Vec3::new(0.0, 0.1, 0.0),
        size: Vec2::new(f64::from(pic.pixels.width), f64::from(pic.pixels.height)),
        path: "foto.jpg".into(),
        pixels: Some(pic.pixels),
        ..Image::default()
    };
    let mut d = Drawing::new_metric();
    d.add(&Space::Model, Common { xdata: pic.xdata, ..Common::default() }, EntityKind::Image(image)).unwrap();
    d
}

fn pictures(d: &Drawing) -> Vec<&Image> {
    d.model.iter().filter_map(|e| if let EntityKind::Image(i) = &e.kind { Some(i) } else { None }).collect()
}

fn gradient() -> RgbImage {
    RgbImage::from_fn(64, 48, |x, y| Rgb([(x * 4) as u8, (y * 5) as u8, 128]))
}

#[test]
fn a_jpeg_is_shown_after_dxf_and_dwg_save_and_reopen() {
    let d = drawing_with_picture(&encoded(gradient(), ImageFormat::Jpeg));
    for name in ["a.dxf", "a.dwg"] {
        let back = cadcraft_io::read(&cadcraft_io::write(&d, name).unwrap(), name).unwrap();
        let imgs = pictures(&back);
        assert_eq!(imgs.len(), 1, "{name}");
        let px = imgs[0].pixels.as_ref().unwrap_or_else(|| panic!("{name}: the picture lost its pixels"));
        assert_eq!((px.width, px.height), (64, 48), "{name}");
        // JPEG is lossy: the corner colours come back close.
        let first = &px.rgba[..4];
        assert!(i32::from(first[2]).abs_diff(128) < 12 && first[3] == 255, "{name}: {first:?}");
        assert_eq!(imgs[0].insert, Vec3::new(2.0, 3.0, 0.0), "{name}");
    }
}

#[test]
fn a_png_with_transparency_keeps_its_alpha() {
    let img = RgbaImage::from_fn(10, 10, |x, _| Rgba([200, 10, 10, if x < 5 { 0 } else { 255 }]));
    let d = drawing_with_picture(&encoded(img, ImageFormat::Png));
    let back = cadcraft_io::read(&cadcraft_io::write(&d, "t.dxf").unwrap(), "t.dxf").unwrap();
    let px = pictures(&back)[0].pixels.clone().unwrap();
    assert_eq!(px.rgba[3], 0);
    assert_eq!(px.rgba[(5 * 4) + 3], 255);
}

#[test]
fn a_big_picture_is_shrunk_and_stays_small() {
    let big = RgbImage::from_fn(3000, 1500, |x, y| Rgb([(x % 251) as u8, (y % 241) as u8, ((x ^ y) % 256) as u8]));
    let pic = raster_image::import(&encoded(big, ImageFormat::Png)).unwrap();
    assert_eq!(pic.pixels.width, raster_image::MAX_SIDE);
    assert_eq!(pic.pixels.height, raster_image::MAX_SIDE / 2);
    // Every group's string fits a DXF string.
    assert!(pic.xdata.iter().all(|t| t.value.len() <= 255));
}

#[test]
fn files_that_are_not_pictures_are_refused() {
    assert!(raster_image::import(b"not a picture at all").is_err());
    assert!(raster_image::import(&[0x89, b'P', b'N', b'G', 1, 2, 3]).is_err());
    assert!(raster_image::import(&[]).is_err());
}

#[test]
fn a_damaged_embedded_picture_leaves_a_frame_not_a_failure() {
    let mut d = drawing_with_picture(&encoded(gradient(), ImageFormat::Jpeg));
    let h = d.model.iter().next().unwrap().handle;
    d.model.modify(h, |e| {
        if let Some(t) = e.common.xdata.get_mut(3) {
            t.value = "!!!not base64!!!".into();
        }
    });
    let back = cadcraft_io::read(&cadcraft_io::write(&d, "d.dxf").unwrap(), "d.dxf").unwrap();
    assert!(pictures(&back)[0].pixels.is_none());
}
