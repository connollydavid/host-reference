//! host-reference CLI: a thin entry over the registered normalisers. `skeleton` prints the tier-0
//! skeleton; `view` prints a windowed slice (the consumer surface validated at the weak-agent bar,
//! plan/0049). Normalisers are registered here, one per content kind, as the build waves land them.

use std::process::ExitCode;

#[cfg(feature = "asciidoc")]
use host_reference_asciidoc::AsciidocNormalizer;
#[cfg(feature = "av")]
use host_reference_av::AvNormalizer;
#[cfg(feature = "bibtex")]
use host_reference_bibtex::BibtexNormalizer;
#[cfg(feature = "calendar")]
use host_reference_calendar::CalendarNormalizer;
#[cfg(feature = "columnar")]
use host_reference_columnar::ColumnarNormalizer;
#[cfg(feature = "config")]
use host_reference_config::ConfigNormalizer;
use host_reference_core::{serialize_tier0, Error, Normalizer, Source, SpanSelector};
#[cfg(feature = "data")]
use host_reference_data::DataNormalizer;
#[cfg(feature = "eda")]
use host_reference_eda::EdaNormalizer;
#[cfg(feature = "epub")]
use host_reference_epub::EpubNormalizer;
#[cfg(feature = "geometry")]
use host_reference_geometry::GeometryNormalizer;
#[cfg(feature = "html")]
use host_reference_html::HtmlNormalizer;
#[cfg(feature = "image")]
use host_reference_image::ImageNormalizer;
#[cfg(feature = "mail")]
use host_reference_mail::MailNormalizer;
#[cfg(feature = "netlist")]
use host_reference_netlist::SpiceNormalizer;
#[cfg(feature = "ocr")]
use host_reference_ocr::OcrNormalizer;
#[cfg(feature = "office")]
use host_reference_office::OfficeNormalizer;
#[cfg(feature = "openscad")]
use host_reference_openscad::OpenscadNormalizer;
#[cfg(feature = "org")]
use host_reference_org::OrgNormalizer;
#[cfg(feature = "pdf")]
use host_reference_pdf::PdfNormalizer;
#[cfg(feature = "prose")]
use host_reference_prose::ProseNormalizer;
#[cfg(feature = "rst")]
use host_reference_rst::RstNormalizer;
#[cfg(feature = "rtf")]
use host_reference_rtf::RtfNormalizer;
#[cfg(feature = "vector")]
use host_reference_vector::SvgNormalizer;

// Each enabled reader feature registers its normaliser; a build with none compiles
// to an empty registry and reports every kind as unsupported.
// The registry is built by cfg-gated pushes, so it cannot be a `vec!` literal.
#[allow(clippy::vec_init_then_push)]
fn registry() -> Vec<Box<dyn Normalizer>> {
    #[allow(unused_mut)]
    let mut reg: Vec<Box<dyn Normalizer>> = Vec::new();
    #[cfg(feature = "prose")]
    reg.push(Box::new(ProseNormalizer));
    #[cfg(feature = "data")]
    reg.push(Box::new(DataNormalizer));
    #[cfg(feature = "config")]
    reg.push(Box::new(ConfigNormalizer));
    #[cfg(feature = "calendar")]
    reg.push(Box::new(CalendarNormalizer));
    #[cfg(feature = "columnar")]
    reg.push(Box::new(ColumnarNormalizer));
    #[cfg(feature = "bibtex")]
    reg.push(Box::new(BibtexNormalizer));
    #[cfg(feature = "rst")]
    reg.push(Box::new(RstNormalizer));
    #[cfg(feature = "org")]
    reg.push(Box::new(OrgNormalizer));
    #[cfg(feature = "asciidoc")]
    reg.push(Box::new(AsciidocNormalizer));
    #[cfg(feature = "rtf")]
    reg.push(Box::new(RtfNormalizer));
    #[cfg(feature = "epub")]
    reg.push(Box::new(EpubNormalizer));
    #[cfg(feature = "office")]
    reg.push(Box::new(OfficeNormalizer));
    #[cfg(feature = "mail")]
    reg.push(Box::new(MailNormalizer));
    #[cfg(feature = "pdf")]
    reg.push(Box::new(PdfNormalizer));
    #[cfg(feature = "geometry")]
    reg.push(Box::new(GeometryNormalizer));
    #[cfg(feature = "eda")]
    reg.push(Box::new(EdaNormalizer));
    #[cfg(feature = "image")]
    reg.push(Box::new(ImageNormalizer));
    #[cfg(feature = "av")]
    reg.push(Box::new(AvNormalizer));
    #[cfg(feature = "ocr")]
    reg.push(Box::new(OcrNormalizer));
    #[cfg(feature = "openscad")]
    reg.push(Box::new(OpenscadNormalizer));
    #[cfg(feature = "html")]
    reg.push(Box::new(HtmlNormalizer));
    #[cfg(feature = "vector")]
    reg.push(Box::new(SvgNormalizer));
    #[cfg(feature = "netlist")]
    reg.push(Box::new(SpiceNormalizer));
    reg
}

fn usage() {
    eprintln!(
        "host-reference: normalise external documentation into a token-lean, attestable form\n\
         \n\
         usage:\n\
         \x20 host-reference skeleton <source>                  print the tier-0 skeleton\n\
         \x20 host-reference view <source> --select <selector>  print a windowed view\n\
         \n\
         selectors: section:<title>  |  offset:<start>:<len>"
    );
}

fn hint(path: &str) -> Option<&str> {
    // The extension of the file name, not of the whole path: a dotless name (`net`, `org`) has no
    // hint and must not route by its bare name (the dotless-path finding), and a dot in a directory
    // is not an extension.
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map(|(_, ext)| ext)
}

fn pick<'a>(reg: &'a [Box<dyn Normalizer>], source: &Source) -> Result<&'a dyn Normalizer, Error> {
    // First match in registration order is unambiguous for a single-claim kind. When more than one
    // reader claims the same source (image and OCR both detect rasters, finding 6), surface the
    // collision rather than silently picking the first; the operator enables only the wanted reader.
    let mut hits = reg.iter().map(|n| n.as_ref()).filter(|n| n.detect(source));
    let first =
        hits.next().ok_or(Error::Unsupported("no normaliser is registered for this kind"))?;
    if hits.next().is_some() {
        return Err(Error::Refused(
            "more than one reader claims this kind (e.g. image and OCR both read rasters); \
             build with only the wanted reader feature"
                .into(),
        ));
    }
    Ok(first)
}

fn parse_selector(s: &str) -> Result<SpanSelector, Error> {
    if let Some(title) = s.strip_prefix("section:") {
        Ok(SpanSelector::Section(title.to_string()))
    } else if let Some(rest) = s.strip_prefix("offset:") {
        let (a, b) = rest.split_once(':').ok_or(Error::Parse("offset:<start>:<len>".into()))?;
        let start: usize = a.parse().map_err(|_| Error::Parse("offset start".into()))?;
        let len: usize = b.parse().map_err(|_| Error::Parse("offset len".into()))?;
        // Reject a window whose end cannot be represented before the reader ever slices it
        // (finding 1: an unbounded len overflowed `start + len` into a panic). A len merely larger
        // than the document is fine; the reader clamps it via core::char_offset_window.
        start.checked_add(len).ok_or(Error::Parse("offset start+len overflows".into()))?;
        Ok(SpanSelector::CharOffset { start, len })
    } else {
        Err(Error::Parse(format!("unknown selector {s:?}")))
    }
}

fn run(args: &[String]) -> Result<String, Error> {
    let reg = registry();
    match args.first().map(String::as_str) {
        Some("skeleton") => {
            let path = args.get(1).ok_or(Error::Parse("skeleton needs a source path".into()))?;
            let bytes = std::fs::read(path).map_err(|e| Error::Parse(e.to_string()))?;
            let lc = hint(path).map(str::to_ascii_lowercase);
            let source = Source { bytes: &bytes, hint: lc.as_deref() };
            let t0 = pick(&reg, &source)?.skeleton(&source)?;
            Ok(serialize_tier0(&t0))
        }
        Some("view") => {
            let path = args.get(1).ok_or(Error::Parse("view needs a source path".into()))?;
            let sel = match args.iter().position(|a| a == "--select") {
                Some(i) => {
                    args.get(i + 1).ok_or(Error::Parse("--select needs a selector".into()))?
                }
                None => return Err(Error::Parse("view needs --select <selector>".into())),
            };
            let bytes = std::fs::read(path).map_err(|e| Error::Parse(e.to_string()))?;
            let lc = hint(path).map(str::to_ascii_lowercase);
            let source = Source { bytes: &bytes, hint: lc.as_deref() };
            let t1 = pick(&reg, &source)?.view(&source, &parse_selector(sel)?)?;
            Ok(t1.markdown)
        }
        _ => Err(Error::Unsupported("command")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--help") | Some("-h") | None => {
            usage();
            ExitCode::SUCCESS
        }
        _ => match run(&args) {
            Ok(out) => {
                print!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("host-reference: {e}");
                usage();
                ExitCode::from(2)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use host_reference_core::{Caps, Modality, Tier0, Tier1};

    #[test]
    fn hint_is_the_file_extension_only() {
        assert_eq!(hint("notes.txt"), Some("txt"));
        assert_eq!(hint("/a/b/photo.JPG"), Some("JPG"));
        // a dotless name has no hint, so it does not route by its bare name (dotless-path finding).
        assert_eq!(hint("net"), None);
        // a dot in a directory is not an extension.
        assert_eq!(hint("/home/user.name/notes"), None);
    }

    #[test]
    fn parse_selector_rejects_an_overflowing_offset() {
        // finding 1: an unbounded len that would overflow start+len is refused at parse.
        assert!(matches!(parse_selector("offset:1:18446744073709551615"), Err(Error::Parse(_))));
        assert!(parse_selector("offset:0:5").is_ok());
    }

    // Two stand-in readers that both claim the same source, the image/OCR collision shape.
    struct ClaimsAll;
    impl Normalizer for ClaimsAll {
        fn modality(&self) -> Modality {
            Modality::Raster
        }
        fn capabilities(&self) -> Caps {
            Caps::default()
        }
        fn extensions(&self) -> &'static [&'static str] {
            &["png"]
        }
        fn detect(&self, _source: &Source) -> bool {
            true
        }
        fn skeleton(&self, _source: &Source) -> Result<Tier0, Error> {
            Ok(Tier0::default())
        }
        fn view(&self, _source: &Source, _select: &SpanSelector) -> Result<Tier1, Error> {
            Ok(Tier1::default())
        }
    }

    #[test]
    fn pick_surfaces_a_collision_instead_of_picking_the_first() {
        let source = Source { bytes: b"x", hint: Some("png") };
        let one: Vec<Box<dyn Normalizer>> = vec![Box::new(ClaimsAll)];
        assert!(pick(&one, &source).is_ok());
        let two: Vec<Box<dyn Normalizer>> = vec![Box::new(ClaimsAll), Box::new(ClaimsAll)];
        assert!(matches!(pick(&two, &source), Err(Error::Refused(_))));
        let none: Vec<Box<dyn Normalizer>> = vec![];
        assert!(matches!(pick(&none, &source), Err(Error::Unsupported(_))));
    }
}
