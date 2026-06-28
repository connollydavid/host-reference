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
            // Drop an inline comment: a SPICE comment runs from `;` to the end of the line.
            let code = l.split(';').next().unwrap_or(l);
            let toks: Vec<&str> = code.split_whitespace().collect();
            if toks.is_empty() {
                continue;
            }
            let ty = toks[0].chars().next().unwrap_or('?').to_ascii_uppercase();
            *components.entry(ty).or_insert(0) += 1;
            // The node terminals follow the reference designator; how many there are is fixed by the
            // element type, so the trailing value or model name is not mistaken for a net.
            if let Some(n) = node_count(ty) {
                for net in toks.iter().skip(1).take(n) {
                    nets.insert(net);
                }
            } else {
                // Unmodelled arity (for example `X` subcircuit calls): keep the conservative previous
                // heuristic for that line, every token between the designator and the trailing one.
                if toks.len() >= 3 {
                    for net in &toks[1..toks.len() - 1] {
                        nets.insert(net);
                    }
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

/// The number of node terminals a SPICE element exposes, keyed by the leading letter of its
/// reference designator (case already folded by the caller). `None` is an element whose arity is not
/// modelled (for example `X` subcircuit calls), handled conservatively at the call site.
fn node_count(ty: char) -> Option<usize> {
    match ty {
        'R' | 'C' | 'L' | 'V' | 'I' | 'D' => Some(2),
        'Q' => Some(3),
        'M' | 'J' => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nets_of(netlist: &str) -> Vec<String> {
        let t = SpiceNormalizer
            .skeleton(&Source { bytes: netlist.as_bytes(), hint: Some("cir") })
            .expect("skeleton");
        // The nets are emitted one per `  - <net>` bullet under the `- nets (N):` header.
        t.markdown
            .lines()
            .skip_while(|l| !l.starts_with("- nets ("))
            .skip(1)
            .take_while(|l| l.starts_with("  - "))
            .map(|l| l.trim_start_matches("  - ").to_string())
            .collect()
    }

    #[test]
    fn nets_follow_element_arity_and_ignore_values_and_comments() {
        let netlist = "Mixed Arity\n\
            R1 in out 1k ; load resistor\n\
            Q1 c b e qmod\n\
            M1 d g s b nmos L=1u\n\
            * a whole-line comment, skipped\n\
            X1 a b sub1\n\
            .end\n";
        // R takes 2 nodes (in, out; the value 1k and the `;` comment are not nets); Q takes 3
        // (c, b, e; the model qmod is not); M takes 4 (d, g, s, b; nmos and L=1u are not); the `*`
        // line is skipped; X has unmodelled arity, so the conservative heuristic keeps a, b.
        assert_eq!(nets_of(netlist), ["a", "b", "c", "d", "e", "g", "in", "out", "s"]);
    }
}
