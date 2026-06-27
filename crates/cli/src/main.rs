//! host-reference CLI: a thin entry over the core. The `skeleton` and `view` commands
//! are the consumer surface (the windowed-retrieval selector validated at the weak-agent
//! bar, plan/0049). The format normalisers land per content kind in the build waves; this
//! entry wires the commands, reads the source, and refuses cleanly until a normaliser is
//! registered for the kind.

use std::process::ExitCode;

use host_reference_core::{Error, Source};

fn usage() {
    eprintln!(
        "host-reference: normalise external documentation into a token-lean, attestable form\n\
         \n\
         usage:\n\
         \x20 host-reference skeleton <source>             print the tier-0 skeleton\n\
         \x20 host-reference view <source> --select <sel>  print a windowed view\n\
         \n\
         the format normalisers land per content kind in the build waves (plan/0049)."
    );
}

fn read_source(path: &str) -> Result<(), Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::Parse(e.to_string()))?;
    let _source = Source { bytes: &bytes, hint: path.rsplit('.').next() };
    // No normaliser is registered yet; the kinds land per content kind in the build
    // waves. Refuse cleanly rather than emit a silent empty result.
    Err(Error::Unsupported("no normaliser is registered for this kind yet"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("skeleton") | Some("view") => match args.get(1) {
            Some(path) => match read_source(path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("host-reference: {e}");
                    ExitCode::from(2)
                }
            },
            None => {
                eprintln!("host-reference: '{}' needs a source path", args[0]);
                ExitCode::from(2)
            }
        },
        Some("--help") | Some("-h") | None => {
            usage();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("host-reference: unknown command '{other}'");
            usage();
            ExitCode::from(2)
        }
    }
}
