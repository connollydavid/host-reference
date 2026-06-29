//! Conformance fixtures for the config normaliser, the same harness as the other readers: run the
//! normaliser, serialise tier-0 canonically, assert it equals the committed golden byte for byte.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_config::ConfigNormalizer;

#[test]
fn ini_settings_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "settings",
        "input.ini",
        "ini",
        &ConfigNormalizer,
    );
}

#[test]
fn properties_app_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "app",
        "input.properties",
        "properties",
        &ConfigNormalizer,
    );
}

#[test]
fn env_dotenv_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "dotenv",
        "input.env",
        "env",
        &ConfigNormalizer,
    );
}
