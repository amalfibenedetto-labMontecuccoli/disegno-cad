//! IMAGEATTACH: place a PNG or JPEG picture in the drawing. The picture is embedded in the drawing
//! (see `cadcraft_io::raster_image`), so it travels with the file. The engine stays free of image
//! codecs: the host installs the decoder with [`set_picture_decoder`].

use std::sync::OnceLock;

use cadcraft_doc::{EntityKind, Image, Pixels, RawTag};
use cadcraft_geom::{Vec2, Vec3};
use serde_json::{Value, json};

use super::*;
use crate::{DocState, Result, Session};

/// A decoded picture and the extended data that embeds it in the drawing.
pub struct Picture {
    pub pixels: Pixels,
    pub xdata: Vec<RawTag>,
}

/// Decode a PNG or JPEG file into a [`Picture`].
pub type PictureDecoder = fn(&[u8]) -> std::result::Result<Picture, String>;

static DECODER: OnceLock<PictureDecoder> = OnceLock::new();

/// Install the host's picture decoder (the io layer's `raster_image::import`).
pub fn set_picture_decoder(h: PictureDecoder) {
    let _ = DECODER.set(h);
}

/// Largest picture file placed (a photo straight from a phone is a few MB).
const MAX_PICTURE_BYTES: usize = 64 << 20;
/// A new picture is as wide as this fraction of what the view shows.
const VIEW_FRACTION: f64 = 0.5;

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("imageattach", "Attach Image", run_imageattach)
            .alias(&["ima"])
            .params("{data: base64 PNG or JPEG file, name?: file name, insert?: [x,y] (lower left corner; default: centred in the view), width?: drawing units (default: half the view)} → {handle}"),
    ]
}

fn run_imageattach(s: &mut Session, p: &Value) -> Result<Value> {
    let data = str_param(p, "data").ok_or_else(|| bad("imageattach", "`data` (a base64 PNG or JPEG file) is required"))?;
    let bytes = super::file::base64_decode(data).ok_or_else(|| bad("imageattach", "invalid base64"))?;
    if bytes.len() > MAX_PICTURE_BYTES {
        return Err(bad("imageattach", "the picture is too large"));
    }
    let decode = DECODER.get().ok_or_else(|| bad("imageattach", "pictures are not available in this build"))?;
    let pic = decode(&bytes).map_err(|e| bad("imageattach", e))?;
    let (w, h) = (f64::from(pic.pixels.width), f64::from(pic.pixels.height));

    let view = s.state().map(DocState::view).unwrap_or_default();
    let aspect = (s.viewport_px.0 / s.viewport_px.1.max(1.0)).clamp(0.2, 5.0);
    let width = f64_or(p, "width", view.height * aspect * VIEW_FRACTION);
    if width <= 0.0 || !width.is_finite() {
        return Err(bad("imageattach", "`width` must be positive"));
    }
    // One pixel's side in drawing units.
    let unit = width / w;
    let insert = point_param(p, "insert").unwrap_or(view.center - Vec2::new(width / 2.0, unit * h / 2.0));

    let file = str_param(p, "name").unwrap_or("picture");
    let file: String = file.rsplit(['/', '\\']).next().unwrap_or("picture").chars().filter(|c| !c.is_control()).take(120).collect();
    let img = Image {
        insert: Vec3::new(insert.x, insert.y, 0.0),
        u: Vec3::new(unit, 0.0, 0.0),
        v: Vec3::new(0.0, unit, 0.0),
        size: Vec2::new(w, h),
        path: if file.trim().is_empty() { "picture".into() } else { file },
        pixels: Some(pic.pixels),
        ..Image::default()
    };
    let handle = s.add_entity_with_xdata(EntityKind::Image(img), pic.xdata)?;
    Ok(json!({ "handle": handle.hex(), "width": width, "height": unit * h }))
}
