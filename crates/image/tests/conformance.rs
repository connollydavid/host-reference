//! Conformance fixtures for the raster-image normaliser, the same harness as the other readers.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_image::ImageNormalizer;

#[test]
fn png_photo_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "photo",
        "input.png",
        "png",
        &ImageNormalizer,
    );
}

#[test]
fn jpeg_camera_exif_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "camera",
        "input.jpg",
        "jpg",
        &ImageNormalizer,
    );
}
