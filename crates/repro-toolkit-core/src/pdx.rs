//! Loads a PDX archive (a ZIP container per ISO 22901 holding one or more
//! `.odx-d` / `.odx-f` / `.odx-fd` XML documents) and hands each relevant
//! entry to the ODX parser.

use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::error::{ReproError, Result};
use crate::odx::model::{DiagLayer, FlashDataBlock};
use crate::odx::parser;

/// Everything extracted from a PDX archive that the sequence builder needs.
#[derive(Debug, Default)]
pub struct PdxContents {
    pub diag_layers: Vec<DiagLayer>,
    pub flash_data_blocks: Vec<FlashDataBlock>,
}

pub fn load_pdx_file(path: &Path) -> Result<PdxContents> {
    let file = std::fs::File::open(path).map_err(|source| ReproError::OpenArchive {
        path: path.to_path_buf(),
        source,
    })?;
    load_pdx_reader(file)
}

pub fn load_pdx_bytes(bytes: Vec<u8>) -> Result<PdxContents> {
    load_pdx_reader(std::io::Cursor::new(bytes))
}

fn load_pdx_reader<R: Read + std::io::Seek>(reader: R) -> Result<PdxContents> {
    let mut archive = ZipArchive::new(reader)?;
    let mut contents = PdxContents::default();
    let mut found_diag_layer_container = false;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }

        let is_diag_data = name.ends_with(".odx-d") || name.ends_with(".odx-fd");
        let is_flash_data = name.ends_with(".odx-f");
        if !is_diag_data && !is_flash_data {
            continue;
        }

        let mut raw = Vec::new();
        entry
            .read_to_end(&mut raw)
            .map_err(|source| ReproError::ArchiveEntry {
                name: name.clone(),
                source,
            })?;
        let xml = String::from_utf8(raw).map_err(|source| ReproError::InvalidUtf8 {
            name: name.clone(),
            source,
        })?;

        if is_diag_data {
            found_diag_layer_container = true;
            contents
                .diag_layers
                .extend(parser::parse_diag_layers(&name, &xml)?);
        } else {
            contents
                .flash_data_blocks
                .extend(parser::parse_flash_data_blocks(&name, &xml)?);
        }
    }

    if !found_diag_layer_container {
        return Err(ReproError::NoDiagLayerContainer);
    }

    Ok(contents)
}
