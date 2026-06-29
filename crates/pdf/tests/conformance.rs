//! Conformance fixture for the PDF normaliser. The test builds a minimal born-digital PDF in memory
//! via lopdf (an uncompressed content stream, so the bytes and content id are deterministic) rather
//! than committing a binary, runs the normaliser, and asserts the canonical tier-0 equals the golden.
//! Never auto-blessed; set `HOST_REFERENCE_BLESS=1` to rewrite a golden.

use host_reference_pdf::PdfNormalizer;
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};

fn gen_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal("Reference datasheet sample text.")]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let info_id =
        doc.add_object(dictionary! { "Title" => Object::string_literal("Reference Datasheet") });
    doc.trailer.set("Info", info_id);
    let mut buf = Vec::new();
    doc.save_to(&mut buf).expect("save pdf");
    buf
}

#[test]
fn pdf_datasheet_shape() {
    host_reference_testkit::check_bytes(
        env!("CARGO_MANIFEST_DIR"),
        "datasheet",
        &gen_pdf(),
        "pdf",
        &PdfNormalizer,
    );
}
