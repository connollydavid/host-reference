//! Conformance for the OpenSCAD normaliser. The real GPL parser lives out-of-process in the separate
//! `host-reference-openscad` repo and is conformance-tested there; this crate is permissive and
//! carries no parser. So the plugin's own contract, the out-of-process plumbing and the tally
//! formatting, is tested against a stub helper that emits fixed statement kinds. The stub asserts it
//! received a real `.scad` path, so the test still proves the plugin stages the source and runs the
//! helper at arm's length. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden.

use std::fs;
use std::path::PathBuf;

use host_reference_openscad::OpenscadNormalizer;

/// Write a stub standing in for `host-reference-openscad-helper`: it checks a source path was passed
/// and prints fixed statement kinds, so the plugin's plumbing and tally are exercised without the GPL
/// parser.
fn write_stub() -> PathBuf {
    let stub = std::env::temp_dir().join("host-reference-openscad-stub.sh");
    fs::write(
        &stub,
        "#!/bin/sh\n[ \"$1\" = --version ] && { echo 'host-reference-openscad-helper stub'; exit 0; }\n[ -f \"$1\" ] || { echo 'stub: no scad' >&2; exit 1; }\nprintf 'ModuleDefinition\\nAssignment\\nModuleInstantiation\\nModuleInstantiation\\n'\n",
    )
    .expect("write stub helper");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    }
    stub
}

#[test]
fn model_tallies_helper_kinds() {
    std::env::set_var("HOST_REFERENCE_OPENSCAD_HELPER", write_stub());
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "model",
        "input.scad",
        "scad",
        &OpenscadNormalizer,
    );
}
