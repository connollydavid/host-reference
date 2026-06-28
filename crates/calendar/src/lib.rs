//! The calendar normaliser: iCalendar (.ics) and vCard (.vcf), through calcard's streaming parser.
//! The iCalendar skeleton tallies the components by kind; the vCard skeleton lists the cards. The
//! source map is whole-document for now.

use calcard::{Entry, Parser};
use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct CalendarNormalizer;

impl CalendarNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for CalendarNormalizer {
    fn modality(&self) -> Modality {
        Modality::StructuredData
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: true, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("ics" | "ical" | "vcf" | "vcard"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let outline = match source.hint {
            Some("vcf" | "vcard") => vcard_shape(text)?,
            _ => calendar_shape(text)?,
        };
        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);
        let (start, end) = match select {
            SpanSelector::CharOffset { start, len } => {
                host_reference_core::char_offset_window(text, *start, *len)
            }
            _ => (0, text.len()),
        };
        Ok(Tier1 {
            markdown: text[start..end].to_string(),
            source_map: SourceMap { spans: vec![Span { source: id, origin: start..end }] },
        })
    }
}

/// Parse the input into entries, refusing on any malformed entry. calcard reports a malformed
/// line or an unbalanced component as an error `Entry`; dropping those (the old behaviour) let a
/// truncated or garbage `.ics` return a silent `0 components`. call/0031 forbids that silent
/// partial, so an error entry becomes an explicit `Error::Refused` (finding 4).
fn parse_entries(text: &str) -> Result<Vec<Entry>, Error> {
    let mut parser = Parser::new(text);
    let mut out = Vec::new();
    loop {
        match parser.entry() {
            Entry::Eof => break,
            e @ (Entry::ICalendar(_) | Entry::VCard(_)) => out.push(e),
            Entry::InvalidLine(line) => {
                return Err(Error::Refused(format!("calendar: invalid line: {line}")))
            }
            Entry::UnterminatedComponent(c) => {
                return Err(Error::Refused(format!("calendar: unterminated component: {c}")))
            }
            Entry::UnexpectedComponentEnd { expected, found } => {
                return Err(Error::Refused(format!(
                    "calendar: unexpected component end (expected {expected:?}, found {found:?})"
                )))
            }
            Entry::TooManyComponents => {
                return Err(Error::Refused("calendar: too many components".into()))
            }
            // calcard's Entry is non_exhaustive; refuse an unrecognised variant rather than
            // drop it, the fail-safe default (call/0031).
            _ => return Err(Error::Refused("calendar: unrecognised parser entry".into())),
        }
    }
    Ok(out)
}

/// The component tally of every VCALENDAR in the input, ordered by kind.
fn calendar_shape(text: &str) -> Result<String, Error> {
    let mut tally: Vec<(String, usize)> = Vec::new();
    let mut total = 0usize;
    for e in parse_entries(text)? {
        if let Entry::ICalendar(ical) = e {
            for c in &ical.components {
                total += 1;
                let kind = format!("{:?}", c.component_type);
                match tally.iter_mut().find(|(k, _)| *k == kind) {
                    Some(t) => t.1 += 1,
                    None => tally.push((kind, 1)),
                }
            }
        }
    }
    tally.sort();
    let mut out = format!("calendar: {total} components\n");
    for (kind, count) in tally {
        if count > 1 {
            out.push_str(&format!("- {kind} (x{count})\n"));
        } else {
            out.push_str(&format!("- {kind}\n"));
        }
    }
    Ok(out)
}

/// The card count, with the union of property names the cards carry (their shape).
fn vcard_shape(text: &str) -> Result<String, Error> {
    let mut count = 0usize;
    let mut props: Vec<String> = Vec::new();
    for e in parse_entries(text)? {
        if let Entry::VCard(v) = e {
            count += 1;
            for entry in &v.entries {
                let name = format!("{:?}", entry.name);
                if !props.contains(&name) {
                    props.push(name);
                }
            }
        }
    }
    props.sort();
    let mut out = format!("vcards: {count} cards\n");
    for p in props {
        out.push_str(&format!("- {p}\n"));
    }
    Ok(out)
}
