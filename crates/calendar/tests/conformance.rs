//! Conformance fixtures for the calendar normaliser, the same harness as the other readers: run the
//! normaliser, serialise tier-0 canonically, assert it equals the committed golden byte for byte.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden deliberately.

use host_reference_calendar::CalendarNormalizer;

#[test]
fn ics_events_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "events",
        "input.ics",
        "ics",
        &CalendarNormalizer,
    );
}

#[test]
fn vcf_contacts_shape() {
    host_reference_testkit::check_file(
        env!("CARGO_MANIFEST_DIR"),
        "contacts",
        "input.vcf",
        "vcf",
        &CalendarNormalizer,
    );
}
