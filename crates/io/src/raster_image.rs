//! Pictures (PNG, JPEG) placed in a drawing.
//!
//! DXF and DWG only name an image file, so a picture that travels with a drawing (a student's
//! drawing handed in on Drive) would arrive without it. A placed picture is therefore stored in
//! the drawing itself: it is shrunk to at most [`MAX_SIDE`] pixels, compressed (JPEG; PNG when it
//! has transparency) and kept as extended data (xdata) of its IMAGE entity, base64 in `1000`
//! strings under the application name [`XDATA_APP`]. Other programs ignore that data and show
//! the IMAGE as a frame; CADCraft decodes it again when it reads the drawing ([`restore`]).

use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use cadcraft_doc::{Drawing, EntityKind, EntityStore, Handle, Pixels, RawTag};
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Limits};

/// The xdata application that holds an embedded picture.
pub const XDATA_APP: &str = "DISEGNOCAD_IMAGE";

/// The longest side of a stored picture, in pixels.
pub const MAX_SIDE: u32 = 2048;
/// The most compressed bytes a picture is stored in; a bigger one is shrunk further.
const MAX_STORED: usize = 3 << 20;
/// The smallest a picture is shrunk to while fitting [`MAX_STORED`].
const MIN_SIDE: u32 = 256;
/// The most pixels along a side, and the most memory, a source file may ask to decode.
const MAX_SOURCE_SIDE: u32 = 20_000;
const MAX_DECODE_BYTES: u64 = 512 << 20;
/// Characters per `1000` group (DXF strings hold at most 255).
const CHUNK: usize = 200;

/// A picture ready to be placed: its pixels and the xdata that embeds it.
pub struct Imported {
    pub pixels: Pixels,
    pub xdata: Vec<RawTag>,
}

fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, String> {
    let mut r = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| format!("the picture could not be read: {e}"))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    r.limits(limits);
    Ok(r)
}

/// Whether `bytes` start like a PNG or JPEG file.
pub fn is_picture(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G']) || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// Decode a PNG or JPEG file, shrink it to fit [`MAX_SIDE`] and prepare its embedding.
pub fn import(bytes: &[u8]) -> Result<Imported, String> {
    if !is_picture(bytes) {
        return Err("only PNG and JPEG pictures can be placed".into());
    }
    let mut img = reader(bytes)?.decode().map_err(|e| format!("the picture could not be decoded: {e}"))?;
    if img.width().max(img.height()) > MAX_SIDE {
        img = img.resize(MAX_SIDE, MAX_SIDE, FilterType::Triangle);
    }
    let transparent = img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p.0[3] != 255);
    loop {
        let (stored, ext) = encode(&img, transparent)?;
        if stored.len() <= MAX_STORED || img.width().max(img.height()) <= MIN_SIDE {
            let rgba = img.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let pixels = Pixels::new(w, h, rgba.into_raw()).ok_or("the picture is empty")?;
            return Ok(Imported { pixels, xdata: embed(ext, &stored) });
        }
        // Still too big: a quarter fewer pixels per side.
        let (w, h) = (img.width() * 3 / 4, img.height() * 3 / 4);
        img = img.resize(w.max(1), h.max(1), FilterType::Triangle);
    }
}

fn encode(img: &DynamicImage, transparent: bool) -> Result<(Vec<u8>, &'static str), String> {
    let mut out = Vec::new();
    if transparent {
        img.to_rgba8().write_to(&mut Cursor::new(&mut out), ImageFormat::Png).map_err(|e| format!("the picture could not be stored: {e}"))?;
        Ok((out, "png"))
    } else {
        let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
        rgb.write_with_encoder(enc).map_err(|e| format!("the picture could not be stored: {e}"))?;
        Ok((out, "jpg"))
    }
}

/// The xdata that embeds `stored` (a PNG or JPEG file; `ext` names which).
fn embed(ext: &str, stored: &[u8]) -> Vec<RawTag> {
    let text = STANDARD.encode(stored);
    let tag = |code: i32, value: &str| RawTag { code, value: value.to_string() };
    let mut x = vec![tag(1001, XDATA_APP), tag(1000, ext)];
    // Base64 is ASCII, so chunking by bytes never splits a character.
    x.extend(text.as_bytes().chunks(CHUNK).map(|c| tag(1000, &String::from_utf8_lossy(c))));
    x
}

/// The picture file embedded in `xdata`, if any.
fn embedded(xdata: &[RawTag]) -> Option<Vec<u8>> {
    let start = xdata.iter().position(|t| t.code == 1001 && t.value.trim().eq_ignore_ascii_case(XDATA_APP))?;
    let body = xdata.get(start + 1..)?.iter().take_while(|t| t.code != 1001).filter(|t| t.code == 1000);
    // The first string names the format; the rest is the data.
    let text: String = body.skip(1).map(|t| t.value.trim()).collect();
    STANDARD.decode(text).ok()
}

/// The pixels of a stored picture; `None` when it can't be decoded.
fn decode(bytes: &[u8]) -> Option<Pixels> {
    if !is_picture(bytes) {
        return None;
    }
    let rgba = reader(bytes).ok()?.decode().ok()?.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Pixels::new(w, h, rgba.into_raw())
}

fn restore_store(store: &mut EntityStore) {
    let todo: Vec<(Handle, Pixels)> = store
        .iter()
        .filter(|e| matches!(&e.kind, EntityKind::Image(i) if i.pixels.is_none()))
        .filter_map(|e| Some((e.handle, decode(&embedded(&e.common.xdata)?)?)))
        .collect();
    for (h, px) in todo {
        store.modify(h, |e| {
            if let EntityKind::Image(i) = &mut e.kind {
                i.pixels = Some(px);
            }
        });
    }
}

/// Decode the pictures embedded in a drawing just read, so they show.
pub fn restore(d: &mut Drawing) {
    restore_store(&mut d.model);
    for l in &mut d.layouts {
        restore_store(&mut l.entities);
    }
    for b in d.blocks.values_mut() {
        restore_store(&mut std::sync::Arc::make_mut(b).entities);
    }
}
