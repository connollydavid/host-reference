//! call/0031 hostile-input coverage for the office reader: a decompression bomb (a tiny archive
//! that declares a large expansion at a high ratio) is refused before undoc decompresses it
//! (plan/0050 finding 2 instance).

use std::io::{Cursor, Write};

use host_reference_core::{Error, Normalizer, Source};
use host_reference_office::OfficeNormalizer;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

fn deflate_bomb() -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("big.bin", opts).expect("start_file");
    // Four MiB of zeros deflates to a few KiB, a compression ratio well past the cap.
    zip.write_all(&vec![0u8; 4 * 1024 * 1024]).expect("write");
    zip.finish().expect("finish").into_inner()
}

#[test]
fn refuses_a_decompression_bomb() {
    let got = OfficeNormalizer.skeleton(&Source { bytes: &deflate_bomb(), hint: Some("docx") });
    assert!(matches!(got, Err(Error::Refused(_))), "expected a refusal, got {got:?}");
}
