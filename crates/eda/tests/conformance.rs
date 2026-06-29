//! Conformance fixtures for the engineering-EDA normaliser, the same harness as the other readers.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_eda::EdaNormalizer;

#[test]
fn kicad_schematic_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "schematic",
        "input.kicad_sch",
        "kicad_sch",
        &EdaNormalizer,
    );
}

#[test]
fn eagle_schematic_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "board",
        "input.sch",
        "sch",
        &EdaNormalizer,
    );
}

#[test]
fn gerber_copper_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "copper",
        "input.gbr",
        "gbr",
        &EdaNormalizer,
    );
}
