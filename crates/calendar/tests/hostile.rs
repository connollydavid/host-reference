//! call/0031 hostile-input coverage for the calendar reader: a malformed or truncated calendar is
//! refused (Error::Refused), not returned as a silent zero-component skeleton (plan/0050 finding 4).

use host_reference_calendar::CalendarNormalizer;
use host_reference_core::{Error, Normalizer, Source};

#[test]
fn refuses_non_calendar_input() {
    // calcard parses leniently, so non-calendar bytes yield no VCALENDAR rather than an error.
    // The old reader reported that as a silent "0 components"; it is now refused (finding 4).
    let garbage = b"this is plainly not a calendar at all\r\njust some random prose\r\n";
    let got = CalendarNormalizer.skeleton(&Source { bytes: garbage, hint: Some("ics") });
    assert!(matches!(got, Err(Error::Refused(_))), "expected a refusal, got {got:?}");
}
