//! Conformance fixtures for the structured-data normaliser, the same harness pattern as prose:
//! run the normaliser, serialise tier-0 canonically, assert it equals the committed golden byte
//! for byte. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_data::DataNormalizer;

#[test]
fn json_object_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "object",
        "input.json",
        "json",
        &DataNormalizer,
    );
}

#[test]
fn csv_table_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "table",
        "input.csv",
        "csv",
        &DataNormalizer,
    );
}

#[test]
fn yaml_config_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "config",
        "input.yaml",
        "yaml",
        &DataNormalizer,
    );
}

#[test]
fn xml_rss_feed_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "feed",
        "input.xml",
        "xml",
        &DataNormalizer,
    );
}

#[test]
fn ndjson_stream_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "stream",
        "input.ndjson",
        "ndjson",
        &DataNormalizer,
    );
}

#[test]
fn tsv_grid_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "grid",
        "input.tsv",
        "tsv",
        &DataNormalizer,
    );
}

#[test]
fn ipynb_notebook_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "notebook",
        "input.ipynb",
        "ipynb",
        &DataNormalizer,
    );
}

#[test]
fn toml_manifest_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "manifest",
        "input.toml",
        "toml",
        &DataNormalizer,
    );
}
