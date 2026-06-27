# host-reference

A reference compiler. It reads external documentation an agentic project depends on but
does not author, in mixed shapes, and normalises it into a token-lean, attestable form an
agent can interpret in context.

The design is settled in the agentic-host milestone plan/0049 and its decisions: call/0030
(the component, a two-layer model of a deterministic immutable normalised layer and a
collaborative overlay), call/0031 (the untrusted-input threat model), and call/0032 (the
engineering-geometry token target).

## Layout

This is a Cargo workspace.

- `crates/core` (`host-reference-core`): the `Normalizer` trait, the modality taxonomy,
  and the two-layer types (the skeleton, the windowed view, the bidirectional source map,
  the capability declaration, the span selector).
- `crates/cli` (`host-reference`): the consumer surface, the `skeleton` and `view`
  commands.

The format normalisers and the low-level readers land one content kind at a time in the
build waves, each with a conformance fixture that re-derives byte for byte. The overlay,
the recognition adapter, and the reproducible-build provenance follow in their waves.
