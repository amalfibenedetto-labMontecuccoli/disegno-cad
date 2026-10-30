//! An MTEXT's width is where its lines wrap, not how wide the text is: ZOOM Extents must fit the
//! letters, not the frame (a logo text in a real drawing had a frame 17 times wider than its
//! letters, and the drawing opened tiny in one corner of the view).

use cadcraft_doc::{Common, Drawing, EntityKind, MText, Space, entity_bounds};
use cadcraft_geom::Vec3;

fn bounds_of(contents: &str, width: f64) -> (f64, f64) {
    let mut d = Drawing::new_metric();
    let m = MText {
        insert: Vec3::new(0.0, 0.0, 0.0),
        height: 1.0,
        width,
        attach: 1,
        rotation: 0.0,
        style: "Standard".into(),
        contents: contents.into(),
        line_spacing: 1.0,
        line_spacing_exact: false,
        columns: None,
    };
    let h = d.add(&Space::Model, Common::default(), EntityKind::MText(m)).unwrap();
    let e = d.entity(h).unwrap().clone();
    let b = entity_bounds(&d, &e, 0);
    (b.width(), b.height())
}

#[test]
fn a_short_text_in_a_wide_frame_takes_only_its_own_width() {
    let (w, _) = bounds_of("dwgLAB", 17.6);
    assert!(w < 6.0, "{w}");
}

#[test]
fn a_long_text_is_limited_by_its_frame() {
    let (w, _) = bounds_of(&"word ".repeat(40), 10.0);
    assert!((w - 10.0).abs() < 1e-9, "{w}");
}

#[test]
fn formatting_codes_take_no_room() {
    let plain = bounds_of("dwgLAB", 0.0).0;
    let coded = bounds_of("{\\fArial|b0|i0|c0|p34;\\H2.5;\\C1;dwgLAB}", 0.0).0;
    assert!((plain - coded).abs() < 1e-9, "{plain} {coded}");
    // `\U+00E8` is one character, an escaped brace too.
    assert!((bounds_of("\\U+00E8\\{", 0.0).0 - bounds_of("ab", 0.0).0).abs() < 1e-9);
}
