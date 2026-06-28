//! The out-of-process OCR helper (call/0033). It carries the ocrs/rten engine and the CC-BY-SA-4.0
//! ocrs models embedded below, so the licence-encumbered weights stay on this side of the arms-length
//! boundary. The permissive `host-reference-ocr` plugin runs this binary as a separate process and
//! reads its stdout, so the two are aggregated rather than linked. It reads one image path from argv
//! and prints the recognised text to stdout.

use std::error::Error;

use ocrs::{DimOrder, ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_tensor::prelude::*;
use rten_tensor::NdTensor;

// The ocrs models, CC-BY-SA-4.0, by robertknight/ocrs-models. See NOTICE.md. Embedding them keeps the
// helper self-contained and the weights off the permissive plugin's side of the boundary.
static DETECTION_MODEL: &[u8] = include_bytes!("../models/text-detection.rten");
static RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.rten");

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: host-reference-ocr-helper <image>")?;
    let image = image::open(&path)?.into_rgb8();
    let (width, height) = image.dimensions();
    let tensor = NdTensor::from_data([height as usize, width as usize, 3], image.into_raw());
    let source = ImageSource::from_tensor(tensor.view(), DimOrder::Hwc)?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(Model::load_static_slice(DETECTION_MODEL)?),
        recognition_model: Some(Model::load_static_slice(RECOGNITION_MODEL)?),
        ..Default::default()
    })?;

    let input = engine.prepare_input(source)?;
    print!("{}", engine.get_text(&input)?);
    Ok(())
}
