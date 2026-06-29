//! Conformance fixture for the netlist normaliser, the same harness: run the normaliser, serialise
//! tier-0 canonically, assert it equals the committed golden byte for byte. Never auto-blessed;
//! set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_netlist::SpiceNormalizer;

#[test]
fn spice_rc_filter_summary() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "rc",
        "input.cir",
        "cir",
        &SpiceNormalizer,
    );
}
