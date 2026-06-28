//! Conformance fixtures for the office normaliser. DOCX, PPTX, and XLSX are OOXML (zips of XML), so
//! the test builds minimal, fixed packages in memory (deterministic mtime keeps the content id
//! stable) rather than committing binaries, runs the normaliser, and asserts the canonical tier-0
//! equals the committed golden. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden.

use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use host_reference_core::{serialize_tier0, Normalizer, Source};
use host_reference_office::OfficeNormalizer;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

fn gen_zip(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    // Stored (no compression) so the bytes do not depend on the deflate backend, which can vary
    // with zip feature unification across the workspace and would otherwise drift the content id.
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    for (name, content) in parts {
        zip.start_file(*name, opts).expect("start_file");
        zip.write_all(content.as_bytes()).expect("write");
    }
    zip.finish().expect("finish").into_inner()
}

const DOCX_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>
"#;

const DOCX_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>
"#;

const DOCX_CORE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>Reference Report</dc:title>
</cp:coreProperties>
"#;

const DOCX_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Introduction</w:t></w:r></w:p>
    <w:p><w:r><w:t>Some introductory text.</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Background</w:t></w:r></w:p>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Conclusion</w:t></w:r></w:p>
  </w:body>
</w:document>
"#;

fn gen_docx() -> Vec<u8> {
    gen_zip(&[
        ("[Content_Types].xml", DOCX_CONTENT_TYPES),
        ("_rels/.rels", DOCX_RELS),
        ("docProps/core.xml", DOCX_CORE),
        ("word/document.xml", DOCX_DOCUMENT),
    ])
}

fn check(dir: &str, bytes: &[u8], hint: &str) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(dir);
    let tier0 = OfficeNormalizer
        .skeleton(&Source { bytes, hint: Some(hint) })
        .expect("skeleton");
    let got = serialize_tier0(&tier0);

    let golden = base.join("expected.golden");
    if std::env::var("HOST_REFERENCE_BLESS").is_ok() {
        fs::create_dir_all(&base).expect("create fixture dir");
        fs::write(&golden, &got).expect("write golden");
        return;
    }
    let want = fs::read_to_string(&golden)
        .expect("read golden; bless it first with HOST_REFERENCE_BLESS=1");
    assert_eq!(got, want, "tier-0 drifted from the golden for fixture `{dir}`");
}

#[test]
fn docx_report_shape() {
    check("report", &gen_docx(), "docx");
}
