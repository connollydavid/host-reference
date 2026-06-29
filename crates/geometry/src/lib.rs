//! The engineering-geometry normaliser: a deterministic structure-and-metadata summary per format
//! (call/0032). STL gives the triangle count; glTF gives the scene, node, and mesh counts; DXF gives
//! the entity counts by type. More geometry formats join this crate behind the geometry feature. The
//! source map is whole-document for now.

use std::io::Cursor;

use host_reference_core::{Caps, Error, Modality, Normalizer, Semantic, Source, Tier0};

pub struct GeometryNormalizer;

impl Normalizer for GeometryNormalizer {
    fn modality(&self) -> Modality {
        Modality::EngineeringGeometry
    }

    fn capabilities(&self) -> Caps {
        Caps { round_trip: false, write_back: false, semantic: Semantic::Partial, ocr: false }
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["stl", "gltf", "glb", "dxf", "obj", "ply", "gcode", "nc", "3mf", "amf", "step", "stp"]
    }

    fn skeleton(&self, source: &Source) -> Result<Tier0, Error> {
        let outline = shape(source)?;
        Ok(host_reference_core::Tier0::whole(source.bytes, outline))
    }
}

fn shape(source: &Source) -> Result<String, Error> {
    match source.hint {
        Some("gltf" | "glb") => gltf_shape(source.bytes),
        Some("dxf") => dxf_shape(source.bytes),
        Some("obj") => obj_shape(source.bytes),
        Some("ply") => ply_shape(source.bytes),
        Some("gcode" | "nc") => gcode_shape(source.bytes),
        Some("3mf") => threemf_shape(source.bytes),
        Some("amf") => amf_shape(source.bytes),
        Some("step" | "stp") => step_shape(source.bytes),
        _ => stl_shape(source.bytes),
    }
}

fn obj_shape(bytes: &[u8]) -> Result<String, Error> {
    let mut cursor = Cursor::new(bytes);
    let (models, _) = tobj::load_obj_buf(
        &mut cursor,
        &tobj::LoadOptions { triangulate: true, ..Default::default() },
        |_| Ok((Vec::new(), Default::default())),
    )
    .map_err(|e| Error::Parse(format!("obj: {e}")))?;
    let vertices: usize = models.iter().map(|m| m.mesh.positions.len() / 3).sum();
    let faces: usize = models.iter().map(|m| m.mesh.indices.len() / 3).sum();
    Ok(format!("obj: {} models, {vertices} vertices, {faces} faces\n", models.len()))
}

fn ply_shape(bytes: &[u8]) -> Result<String, Error> {
    let mut cursor = Cursor::new(bytes);
    let parser = ply_rs_bw::parser::Parser::<ply_rs_bw::ply::DefaultElement>::new();
    let ply = parser.read_ply(&mut cursor).map_err(|e| Error::Parse(format!("ply: {e}")))?;
    let mut out = String::from("ply:\n");
    for (name, def) in &ply.header.elements {
        out.push_str(&format!("- {name}: {}\n", def.count));
    }
    Ok(out)
}

fn gcode_shape(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::Parse(format!("gcode: {e}")))?;
    let program = gcode::parse(text).map_err(|e| Error::Parse(format!("gcode: {e:?}")))?;
    let commands: usize = program.blocks.iter().map(|b| b.codes.len()).sum();
    Ok(format!("gcode: {} blocks, {commands} commands\n", program.blocks.len()))
}

fn threemf_shape(bytes: &[u8]) -> Result<String, Error> {
    let models =
        threemf::read(Cursor::new(bytes)).map_err(|e| Error::Parse(format!("3mf: {e:?}")))?;
    let mut meshes = 0usize;
    let mut vertices = 0usize;
    let mut triangles = 0usize;
    for model in &models {
        for object in &model.resources.object {
            if let Some(mesh) = &object.mesh {
                meshes += 1;
                vertices += mesh.vertices.vertex.len();
                triangles += mesh.triangles.triangle.len();
            }
        }
    }
    Ok(format!("3mf: {meshes} meshes, {vertices} vertices, {triangles} triangles\n"))
}

fn amf_shape(bytes: &[u8]) -> Result<String, Error> {
    if bytes.starts_with(b"PK") {
        return Err(Error::Unsupported(
            "compressed AMF (zip) is not yet supported; uncompressed AMF only",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| Error::Parse(format!("amf: {e}")))?;
    let doc = roxmltree::Document::parse(text).map_err(|e| Error::Parse(format!("amf: {e}")))?;
    let count = |name: &str| {
        doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == name).count()
    };
    Ok(format!(
        "amf: {} objects, {} meshes, {} vertices, {} triangles\n",
        count("object"),
        count("mesh"),
        count("vertex"),
        count("triangle")
    ))
}

fn step_shape(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::Parse(format!("step: {e}")))?;
    let (_, exchange) = ruststep::parser::exchange::exchange_file(text)
        .map_err(|e| Error::Parse(format!("step: {e:?}")))?;
    let mut tally: Vec<(String, usize)> = Vec::new();
    let mut total = 0usize;
    for section in &exchange.data {
        for entity in &section.entities {
            total += 1;
            let name = match entity {
                ruststep::ast::EntityInstance::Simple { record, .. } => record.name.clone(),
                ruststep::ast::EntityInstance::Complex { .. } => "Complex".to_string(),
            };
            match tally.iter_mut().find(|(k, _)| *k == name) {
                Some(t) => t.1 += 1,
                None => tally.push((name, 1)),
            }
        }
    }
    tally.sort();
    let mut out = format!("step: {total} entities\n");
    for (kind, count) in tally {
        if count > 1 {
            out.push_str(&format!("- {kind} (x{count})\n"));
        } else {
            out.push_str(&format!("- {kind}\n"));
        }
    }
    Ok(out)
}

fn stl_shape(bytes: &[u8]) -> Result<String, Error> {
    let mut cursor = Cursor::new(bytes);
    let mesh = stl_io::read_stl(&mut cursor).map_err(|e| Error::Parse(format!("stl: {e}")))?;
    Ok(format!("stl: {} vertices, {} triangles\n", mesh.vertices.len(), mesh.faces.len()))
}

fn gltf_shape(bytes: &[u8]) -> Result<String, Error> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| Error::Parse(format!("gltf: {e}")))?;
    Ok(format!(
        "gltf: {} scenes, {} nodes, {} meshes\n",
        gltf.scenes().count(),
        gltf.nodes().count(),
        gltf.meshes().count()
    ))
}

fn dxf_shape(bytes: &[u8]) -> Result<String, Error> {
    let mut cursor = Cursor::new(bytes);
    let drawing = dxf::Drawing::load(&mut cursor).map_err(|e| Error::Parse(format!("dxf: {e}")))?;
    let mut tally: Vec<(String, usize)> = Vec::new();
    let mut total = 0usize;
    for entity in drawing.entities() {
        total += 1;
        let kind = format!("{:?}", entity.specific);
        let kind = kind.split(['(', ' ', '{']).next().unwrap_or("Entity").to_string();
        match tally.iter_mut().find(|(k, _)| *k == kind) {
            Some(t) => t.1 += 1,
            None => tally.push((kind, 1)),
        }
    }
    tally.sort();
    let mut out = format!("dxf: {total} entities\n");
    for (kind, count) in tally {
        if count > 1 {
            out.push_str(&format!("- {kind} (x{count})\n"));
        } else {
            out.push_str(&format!("- {kind}\n"));
        }
    }
    Ok(out)
}
