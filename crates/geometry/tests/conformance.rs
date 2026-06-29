//! Conformance fixtures for the engineering-geometry normaliser, the same harness as the other
//! readers. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_geometry::GeometryNormalizer;

#[test]
fn stl_cube_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "cube",
        "input.stl",
        "stl",
        &GeometryNormalizer,
    );
}

#[test]
fn gltf_scene_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "scene",
        "input.gltf",
        "gltf",
        &GeometryNormalizer,
    );
}

#[test]
fn dxf_drawing_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "drawing",
        "input.dxf",
        "dxf",
        &GeometryNormalizer,
    );
}

#[test]
fn obj_mesh_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "mesh",
        "input.obj",
        "obj",
        &GeometryNormalizer,
    );
}

#[test]
fn ply_points_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "points",
        "input.ply",
        "ply",
        &GeometryNormalizer,
    );
}

#[test]
fn gcode_toolpath_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "toolpath",
        "input.gcode",
        "gcode",
        &GeometryNormalizer,
    );
}

#[test]
fn threemf_print_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "print",
        "input.3mf",
        "3mf",
        &GeometryNormalizer,
    );
}

#[test]
fn amf_additive_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "additive",
        "input.amf",
        "amf",
        &GeometryNormalizer,
    );
}

#[test]
fn step_exchange_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "exchange",
        "input.step",
        "step",
        &GeometryNormalizer,
    );
}
