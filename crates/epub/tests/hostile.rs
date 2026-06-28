//! call/0031 hostile-input coverage for the EPUB reader: a decompression bomb is refused before
//! rbook decompresses it (plan/0050 finding 2 instance). An EPUB is a zip, so the same central
//! -directory bound applies as for office.

use std::io::{Cursor, Write};

use host_reference_core::{Error, Normalizer, Source};
use host_reference_epub::EpubNormalizer;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

fn deflate_bomb() -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("big.bin", opts).expect("start_file");
    zip.write_all(&vec![0u8; 4 * 1024 * 1024]).expect("write");
    zip.finish().expect("finish").into_inner()
}

#[test]
fn refuses_a_decompression_bomb() {
    let got = EpubNormalizer.skeleton(&Source { bytes: &deflate_bomb(), hint: Some("epub") });
    assert!(matches!(got, Err(Error::Refused(_))), "expected a refusal, got {got:?}");
}
