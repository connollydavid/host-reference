//! host-reference core: the contract a reference compiler is built on.
//!
//! See agentic-host call/0030 (the component), call/0031 (the threat model), and
//! call/0032 (the engineering-geometry token target). The types here describe the
//! immutable normalised layer, which is deterministic and attested; the collaborative
//! overlay layer lands in its own crate.

use std::ops::{Range, RangeInclusive};

/// The closed modality taxonomy. Every content kind maps into one cell; a new format
/// slots into an existing modality (call/0030).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modality {
    Prose,
    StructuredData,
    OfficeCompound,
    FixedLayout,
    Raster,
    Vector,
    Mail,
    EngineeringEda,
    EngineeringGeometry,
}

/// How fully a normaliser preserves structural roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Semantic {
    None,
    Partial,
    Full,
}

/// A normaliser's declared capabilities for the kind it reads. The `Default` is the
/// most restrictive setting: an undeclared capability cannot over-claim editability
/// (the Bly fail-safe rule, call/0030).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    /// The original can be reconstructed from the normalised form.
    pub round_trip: bool,
    /// An edit to the normalised view can be pushed back into the source.
    pub write_back: bool,
    /// Structural roles captured.
    pub semantic: Semantic,
    /// Optical character recognition was used.
    pub ocr: bool,
}

impl Default for Caps {
    fn default() -> Self {
        Caps { round_trip: false, write_back: false, semantic: Semantic::None, ocr: false }
    }
}

/// A content-addressed span of a source, the unit the source map resolves in both
/// directions: a normalised region to its origin, and an origin back to its region.
#[derive(Clone, Debug)]
pub struct Span {
    /// The content hash of the source this span belongs to.
    pub source: String,
    /// The byte range in that source the span derives from.
    pub origin: Range<usize>,
}

/// The bidirectional source map: every normalised region carries the span it came
/// from, so a fact is traceable and an edit is anchorable (call/0030, call/0031).
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    pub spans: Vec<Span>,
}

/// The always-resident skeleton: the token-lean, semantically-typed index a consumer
/// reads first. The token counts make the saving a measured number, not a claim.
#[derive(Clone, Debug, Default)]
pub struct Tier0 {
    pub markdown: String,
    pub source_map: SourceMap,
    pub raw_tokens: usize,
    pub normalised_tokens: usize,
}

/// A fetched-on-demand full slice, chosen by a `SpanSelector`.
#[derive(Clone, Debug, Default)]
pub struct Tier1 {
    pub markdown: String,
    pub source_map: SourceMap,
}

/// How a consumer selects a windowed, token-budgeted view of the full layer
/// (call/0030; the selector validated at the weak-agent bar in plan/0049).
#[derive(Clone, Debug)]
pub enum SpanSelector {
    PageRange(RangeInclusive<u32>),
    Section(String),
    CharOffset { start: usize, len: usize },
    TokenBudget { anchor: String, max_tokens: usize },
    ConceptUri(String),
}

/// An edit applied to a normalised view, propagated by `put` where a well-behaved lens
/// exists.
#[derive(Clone, Debug)]
pub struct Edit {
    pub at: Span,
    pub replacement: String,
}

/// A patch to the source produced by `put`.
#[derive(Clone, Debug, Default)]
pub struct Patch {
    pub bytes: Vec<u8>,
}

/// The source bytes a normaliser reads, with an optional format hint.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub bytes: &'a [u8],
    pub hint: Option<&'a str>,
}

/// A normalisation outcome other than success. A refusal is explicit and recorded,
/// never a silent partial (call/0031).
#[derive(Clone, Debug)]
pub enum Error {
    /// The operation is not supported for this kind.
    Unsupported(&'static str),
    /// The parse hit a resource bound or a hostile structure and refused.
    Refused(String),
    /// The source could not be parsed.
    Parse(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unsupported(w) => write!(f, "unsupported: {w}"),
            Error::Refused(w) => write!(f, "refused: {w}"),
            Error::Parse(w) => write!(f, "parse error: {w}"),
        }
    }
}

impl std::error::Error for Error {}

/// The contract every format normaliser implements. The output is deterministic: a
/// pure function of the source bytes and the pinned toolchain (call/0018).
pub trait Normalizer {
    /// The modality cell this normaliser serves.
    fn modality(&self) -> Modality;

    /// The capabilities this normaliser declares for the kind it reads.
    fn capabilities(&self) -> Caps;

    /// Whether this normaliser handles the given bytes (a content sniff plus the hint).
    fn detect(&self, source: &Source) -> bool;

    /// The always-resident skeleton.
    fn skeleton(&self, source: &Source) -> Result<Tier0, Error>;

    /// A windowed, token-budgeted full slice.
    fn view(&self, source: &Source, select: &SpanSelector) -> Result<Tier1, Error>;

    /// The reverse direction, where a well-behaved lens exists. The default refuses,
    /// the fail-safe for a kind that declares no write-back.
    fn put(&self, _source: &Source, _edit: &Edit) -> Result<Patch, Error> {
        Err(Error::Unsupported("put"))
    }
}

/// The content identity of a source: a short hex prefix of its SHA-256, the stable key the source
/// map and the provenance record hang on (call/0030).
pub fn content_id(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// Token count against the pinned reference tokenizer, tiktoken `o200k_base` (call/0030, settled in
/// plan/0049). The vocab is embedded in the crate, so the count is offline and deterministic.
///
/// The byte-level BPE encodes any UTF-8 text, so a count is produced for every language, including
/// Standard Chinese and other non-Latin scripts. The count is a reference yardstick: a model-native
/// tokenizer packs CJK and other scripts more tightly, so a per-consumer tokenizer behind this same
/// call site is future work; the savings ratio it reports stays meaningful in the meantime.
pub fn count_tokens(text: &str) -> usize {
    use std::sync::OnceLock;
    use tiktoken_rs::{o200k_base, CoreBPE};
    static BPE: OnceLock<CoreBPE> = OnceLock::new();
    BPE.get_or_init(|| o200k_base().expect("embedded o200k_base vocab"))
        .encode_ordinary(text)
        .len()
}

/// The canonical, deterministic serialization of a `Tier0`, the form a conformance fixture pins and
/// compares byte for byte. The spans are sorted by their source range, so the form is stable
/// regardless of the order the normaliser built them in.
pub fn serialize_tier0(t: &Tier0) -> String {
    let mut out = String::new();
    out.push_str("== markdown ==\n");
    out.push_str(&t.markdown);
    if !t.markdown.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("== source-map ==\n");
    let mut spans: Vec<(usize, usize, &str)> = t
        .source_map
        .spans
        .iter()
        .map(|s| (s.origin.start, s.origin.end, s.source.as_str()))
        .collect();
    spans.sort();
    for (start, end, src) in spans {
        out.push_str(&format!("{src}:{start}-{end}\n"));
    }
    out.push_str("== tokens ==\n");
    out.push_str(&format!("raw={} normalised={}\n", t.raw_tokens, t.normalised_tokens));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_default_is_most_restrictive() {
        let c = Caps::default();
        assert!(!c.round_trip);
        assert!(!c.write_back);
        assert_eq!(c.semantic, Semantic::None);
        assert!(!c.ocr);
    }
}
