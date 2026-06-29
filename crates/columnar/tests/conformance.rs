//! Conformance fixtures for the columnar normaliser. Parquet and Arrow IPC are binary, so the test
//! writes a fixed, uncompressed buffer in memory (deterministic for the pinned writer) rather than
//! committing an opaque binary, runs the normaliser, and asserts the canonical tier-0 equals the
//! committed golden. Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden.

use std::sync::Arc;

use arrow::array::{Float64Array, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::FileWriter;
use host_reference_columnar::ColumnarNormalizer;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

fn sample_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec!["alpha", "beta", "gamma"])),
            Arc::new(Float64Array::from(vec![9.5, 7.0, 8.25])),
        ],
    )
    .expect("build sample batch")
}

fn gen_parquet() -> Vec<u8> {
    let batch = sample_batch();
    // Pin `created_by` to a fixed string so the buffer (and thus the golden) does not embed the
    // parquet-rs version. Otherwise the golden's content id and raw_tokens track the dependency
    // patch version rather than the source under test.
    let props = WriterProperties::builder()
        .set_compression(Compression::UNCOMPRESSED)
        .set_created_by("host-reference conformance fixture".to_string())
        .build();
    let mut buf = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, batch.schema(), Some(props)).expect("writer");
    writer.write(&batch).expect("write");
    writer.close().expect("close");
    buf
}

fn gen_arrow() -> Vec<u8> {
    let batch = sample_batch();
    let mut buf = Vec::new();
    {
        let mut writer = FileWriter::try_new(&mut buf, &batch.schema()).expect("writer");
        writer.write(&batch).expect("write");
        writer.finish().expect("finish");
    }
    buf
}

#[test]
fn parquet_table_shape() {
    host_reference_testkit::check_bytes(
        env!("CARGO_MANIFEST_DIR"),
        "table",
        &gen_parquet(),
        "parquet",
        &ColumnarNormalizer,
    );
}

#[test]
fn arrow_frame_shape() {
    host_reference_testkit::check_bytes(
        env!("CARGO_MANIFEST_DIR"),
        "frame",
        &gen_arrow(),
        "arrow",
        &ColumnarNormalizer,
    );
}
