//! The mail normaliser: internet mail (.eml) through mail-parser. The skeleton is the envelope, the
//! subject with the from, to, and date headers, and the attachment count. The body view is a later
//! refinement. The source map is whole-document for now.

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};
use mail_parser::{Address, MessageParser};

pub struct MailNormalizer;

impl Normalizer for MailNormalizer {
    fn modality(&self) -> Modality {
        Modality::Mail
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("eml"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let id = content_id(source.bytes);
        let outline = eml_shape(source.bytes)?;
        let lossy = String::from_utf8_lossy(source.bytes);
        Ok(Tier0 {
            raw_tokens: count_tokens(&lossy),
            normalised_tokens: count_tokens(&outline),
            markdown: outline,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }

    fn view(&self, source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
        let id = content_id(source.bytes);
        Ok(Tier1 {
            markdown: eml_shape(source.bytes)?,
            source_map: SourceMap {
                spans: vec![Span { source: id, origin: 0..source.bytes.len() }],
            },
        })
    }
}

fn eml_shape(bytes: &[u8]) -> Result<String, Error> {
    let msg = MessageParser::default()
        .parse(bytes)
        .ok_or_else(|| Error::Parse("eml: not a valid message".into()))?;
    let mut out = String::new();
    if let Some(subject) = msg.subject() {
        out.push_str(&format!("subject: {subject}\n"));
    }
    if let Some(from) = msg.from() {
        out.push_str(&format!("from: {}\n", render_address(from)));
    }
    if let Some(to) = msg.to() {
        out.push_str(&format!("to: {}\n", render_address(to)));
    }
    if let Some(date) = msg.date() {
        out.push_str(&format!("date: {date}\n"));
    }
    out.push_str(&format!("attachments: {}\n", msg.attachments().count()));
    Ok(out)
}

fn render_address(address: &Address) -> String {
    let mut out = Vec::new();
    for addr in address.iter() {
        if let Some(email) = addr.address() {
            out.push(email.to_string());
        }
    }
    out.join(", ")
}
