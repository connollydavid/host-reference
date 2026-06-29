//! The mail normaliser: internet mail (.eml) through mail-parser. The skeleton is the envelope, the
//! subject with the from, to, and date headers, and the attachment count. The body view is a later
//! refinement. The source map is whole-document for now.

use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};
use mail_parser::{Address, MessageParser};

pub struct MailNormalizer;

impl Normalizer for MailNormalizer {
    fn modality(&self) -> Modality {
        Modality::Mail
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["eml"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = eml_shape(source.bytes)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
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
