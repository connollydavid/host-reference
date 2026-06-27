//! host-reference CLI: a thin entry over the registered normalisers. `skeleton` prints the tier-0
//! skeleton; `view` prints a windowed slice (the consumer surface validated at the weak-agent bar,
//! plan/0049). Normalisers are registered here, one per content kind, as the build waves land them.

use std::process::ExitCode;

use host_reference_core::{serialize_tier0, Error, Normalizer, Source, SpanSelector};
#[cfg(feature = "data")]
use host_reference_data::DataNormalizer;
#[cfg(feature = "html")]
use host_reference_html::HtmlNormalizer;
#[cfg(feature = "netlist")]
use host_reference_netlist::SpiceNormalizer;
#[cfg(feature = "prose")]
use host_reference_prose::ProseNormalizer;
#[cfg(feature = "vector")]
use host_reference_vector::SvgNormalizer;

// Each enabled reader feature registers its normaliser; a build with none compiles
// to an empty registry and reports every kind as unsupported.
fn registry() -> Vec<Box<dyn Normalizer>> {
    #[allow(unused_mut)]
    let mut reg: Vec<Box<dyn Normalizer>> = Vec::new();
    #[cfg(feature = "prose")]
    reg.push(Box::new(ProseNormalizer));
    #[cfg(feature = "data")]
    reg.push(Box::new(DataNormalizer));
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
    path.rsplit('.').next()
}

fn pick<'a>(reg: &'a [Box<dyn Normalizer>], source: &Source) -> Result<&'a dyn Normalizer, Error> {
    reg.iter()
        .map(|n| n.as_ref())
        .find(|n| n.detect(source))
        .ok_or(Error::Unsupported("no normaliser is registered for this kind"))
}

fn parse_selector(s: &str) -> Result<SpanSelector, Error> {
    if let Some(title) = s.strip_prefix("section:") {
        Ok(SpanSelector::Section(title.to_string()))
    } else if let Some(rest) = s.strip_prefix("offset:") {
        let (a, b) = rest.split_once(':').ok_or(Error::Parse("offset:<start>:<len>".into()))?;
        let start = a.parse().map_err(|_| Error::Parse("offset start".into()))?;
        let len = b.parse().map_err(|_| Error::Parse("offset len".into()))?;
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
            let source = Source { bytes: &bytes, hint: hint(path) };
            let t0 = pick(&reg, &source)?.skeleton(&source)?;
            Ok(serialize_tier0(&t0))
        }
        Some("view") => {
            let path = args.get(1).ok_or(Error::Parse("view needs a source path".into()))?;
            let sel = match args.iter().position(|a| a == "--select") {
                Some(i) => args
                    .get(i + 1)
                    .ok_or(Error::Parse("--select needs a selector".into()))?,
                None => return Err(Error::Parse("view needs --select <selector>".into())),
            };
            let bytes = std::fs::read(path).map_err(|e| Error::Parse(e.to_string()))?;
            let source = Source { bytes: &bytes, hint: hint(path) };
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
