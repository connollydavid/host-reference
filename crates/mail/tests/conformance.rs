//! Conformance fixture for the mail normaliser, the same harness as the other readers. Never
//! auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_mail::MailNormalizer;

#[test]
fn eml_message_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "message",
        "input.eml",
        "eml",
        &MailNormalizer,
    );
}
