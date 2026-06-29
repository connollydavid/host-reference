//! Conformance fixture for the EPUB normaliser. An EPUB is a zip of XHTML, so the test builds a
//! minimal, fixed EPUB in memory (a deterministic mtime keeps the content id stable) rather than
//! committing an opaque binary, runs the normaliser, and asserts the canonical tier-0 equals the
//! committed golden. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden.

use std::io::{Cursor, Write};

use host_reference_epub::EpubNormalizer;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

const CONTAINER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>
"#;

const OPF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="bookid">urn:uuid:example-0001</dc:identifier>
    <dc:title>Example Reference Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>
"#;

const NAV: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
  <head><title>Contents</title></head>
  <body>
    <nav epub:type="toc" id="toc">
      <ol>
        <li><a href="chapter1.xhtml">Chapter One</a>
          <ol>
            <li><a href="chapter1.xhtml#sec-a">Section A</a></li>
          </ol>
        </li>
        <li><a href="chapter2.xhtml">Chapter Two</a></li>
      </ol>
    </nav>
  </body>
</html>
"#;

const CH1: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter One</title></head>
  <body><h1>Chapter One</h1><h2 id="sec-a">Section A</h2><p>Content of chapter one.</p></body>
</html>
"#;

const CH2: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter Two</title></head>
  <body><h1>Chapter Two</h1><p>Content of chapter two.</p></body>
</html>
"#;

fn gen_epub() -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let when = zip::DateTime::default();
    // The mimetype entry is stored uncompressed and first, per the EPUB OCF spec.
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(when);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(b"application/epub+zip").unwrap();

    // Stored (no compression) so the bytes do not depend on the deflate backend, which can vary with
    // zip feature unification across the workspace and would otherwise drift the content id.
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(when);
    for (name, content) in [
        ("META-INF/container.xml", CONTAINER),
        ("OEBPS/content.opf", OPF),
        ("OEBPS/nav.xhtml", NAV),
        ("OEBPS/chapter1.xhtml", CH1),
        ("OEBPS/chapter2.xhtml", CH2),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn epub_book_shape() {
    host_reference_testkit::check_bytes(
        env!("CARGO_MANIFEST_DIR"),
        "book",
        &gen_epub(),
        "epub",
        &EpubNormalizer,
    );
}
