//! The columnar normaliser: Apache Parquet and Arrow IPC (Feather). The skeleton is the schema, the
//! column names with their types, and the row count, read from the file metadata. Parquet metadata is
//! uncompressed, so the reader needs no compression codec and stays pure-Rust without the C zstd
//! codec; an Arrow IPC stream is read with compression off for the same reason. The full data view is
//! future work, so `view` returns the same schema as the skeleton. The source map is whole-document.

use std::io::Cursor;

use arrow::ipc::reader::FileReader as ArrowFileReader;
use bytes::Bytes;
use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};
use parquet::file::reader::{FileReader as _, SerializedFileReader};

pub struct ColumnarNormalizer;

impl Normalizer for ColumnarNormalizer {
    fn modality(&self) -> Modality {
        Modality::StructuredData
    }

    fn capabilities(&self) -> Caps {
        // The schema is captured in full; the binary does not round-trip through a text view; no edit
        // write-back and no recognition.
        Caps { round_trip: false, write_back: false, semantic: Semantic::Full, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["parquet", "arrow", "feather", "ipc"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = shape(source)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }
}

fn shape(source: &Source) -> Result<String, Error> {
    match source.hint {
        Some("parquet") => parquet_shape(source.bytes),
        _ => arrow_shape(source.bytes),
    }
}

fn parquet_shape(bytes: &[u8]) -> Result<String, Error> {
    let reader = SerializedFileReader::new(Bytes::copy_from_slice(bytes))
        .map_err(|e| Error::Parse(format!("parquet: {e}")))?;
    let meta = reader.metadata().file_metadata();
    let schema = meta.schema_descr();
    let mut out = format!("parquet: {} columns, {} rows\n", schema.num_columns(), meta.num_rows());
    for col in schema.columns() {
        out.push_str(&format!("- {}: {:?}\n", col.name(), col.physical_type()));
    }
    Ok(out)
}

fn arrow_shape(bytes: &[u8]) -> Result<String, Error> {
    let reader = ArrowFileReader::try_new(Cursor::new(bytes), None)
        .map_err(|e| Error::Parse(format!("arrow: {e}")))?;
    let schema = reader.schema();
    let mut rows = 0usize;
    for batch in reader {
        rows += batch.map_err(|e| Error::Parse(format!("arrow: {e}")))?.num_rows();
    }
    let mut out = format!("arrow: {} columns, {rows} rows\n", schema.fields().len());
    for f in schema.fields() {
        out.push_str(&format!("- {}: {:?}\n", f.name(), f.data_type()));
    }
    Ok(out)
}
