# NOTICE

This binary embeds two pre-trained OCR models:

- `models/text-detection.rten`
- `models/text-recognition.rten`

They are the ocrs models by Robert Knight, from
[robertknight/ocrs-models](https://github.com/robertknight/ocrs-models) and
[huggingface.co/robertknight/ocrs](https://huggingface.co/robertknight/ocrs),
licensed **CC-BY-SA-4.0** (Creative Commons Attribution-ShareAlike 4.0).

The models are redistributed unmodified under that licence with attribution. They are confined to
this helper binary, which `host-reference` runs at arm's length over the command line (call/0033), so
the permissive `host-reference-ocr` plugin and the rest of the workspace are an aggregation with the
models rather than a derivative of them. The helper's own source is under the workspace licence.
