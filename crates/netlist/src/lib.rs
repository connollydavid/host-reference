//! The netlist normaliser: SPICE. A netlist's interpretable content is its connectivity, so the
//! skeleton summarises it: the title, the component tally by type letter, the distinct nets, and
//! the directives. Component values and the exact topology are dropped, so the summary is lossy
//! with no round-trip or write-back. Node detection is a heuristic: the tokens between the
//! reference designator and the trailing value or model. Robust per-element arity (and the SPICE
//! source-position-aware parse) is future work. The source map is whole-document.

use std::collections::{BTreeMap, BTreeSet};

use host_reference_core::{
    content_id, count_tokens, Caps, Error, Modality, Normalizer, Semantic, Source, SourceMap, Span,
    SpanSelector, Tier0, Tier1,
};

pub struct SpiceNormalizer;

impl SpiceNormalizer {
    fn text<'a>(&self, source: &Source<'a>) -> Result<&'a str, Error> {
        std::str::from_utf8(source.bytes).map_err(|e| Error::Parse(format!("not UTF-8: {e}")))
    }
}

impl Normalizer for SpiceNormalizer {
    fn modality(&self) -> Modality {
        Modality::EngineeringEda
    }

    fn capabilities(&self) -> Caps {
        // The summary drops values and the exact topology, so it is lossy: no round-trip and no
        // write-back; the component tally and net set give partial semantics.
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn detect(&self, source: &Source) -> bool {
        matches!(source.hint, Some("cir" | "spice" | "sp" | "net"))
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let text = self.text(source)?;
        let id = content_id(source.bytes);

        let mut lines = text.lines();
        // By SPICE convention the first line is the title.
        let title = lines.next().unwrap_or("").trim().to_string();

        let mut components: BTreeMap<char, usize> = BTreeMap::new();
        let mut nets: BTreeSet<&str> = BTreeSet::new();
        let mut directives: BTreeSet<&str> = BTreeSet::new();
        for line in lines {
            let l = line.trim();
            if l.is_empty() || l.starts_with('*') || l.starts_with('+') {
                continue;
            }
            if let Some(rest) = l.strip_prefix('.') {
                directives.insert(rest.split_whitespace().next().unwrap_or(""));
                continue;
            }
            let toks: Vec<&str> = l.split_whitespace().collect();
            let ty = toks[0].chars().next().unwrap_or('?').to_ascii_uppercase();
            *components.entry(ty).or_insert(0) += 1;
            // Nodes are the tokens between the designator and the trailing value or model.
            if toks.len() >= 3 {
                for net in &toks[1..toks.len() - 1] {
                    nets.insert(net);
                }
            }
        }

        let mut out = format!("- title: {title}\n- components:\n");
        for (ty, n) in &components {
            out.push_str(&format!("  - {ty}: {n}\n"));
        }
        out.push_str(&format!("- nets ({}):\n", nets.len()));
        for net in &nets {
            out.push_str(&format!("  - {net}\n"));
        }
        if !directives.is_empty() {
            out.push_str("- directives:\n");
            for d in &directives {
                out.push_str(&format!("  - .{d}\n"));
            }
        }

        Ok(Tier0 {
            raw_tokens: count_tokens(text),
            normalised_tokens: count_tokens(&out),
            markdown: out,
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
