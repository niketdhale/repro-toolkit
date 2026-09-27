//! Parses the subset of ODX (ISO 22901) XML needed to build a UDS
//! reprogramming sequence: diagnostic layers/services from `odx-d`
//! documents, and flash data blocks from `odx-f` documents.

use roxmltree::{Document, Node};

use crate::error::{ReproError, Result};
use crate::odx::model::{DiagLayer, DiagService, FlashDataBlock, Message, Param, ParamKind};

/// Parse an `odx-d` (diagnostic data) document and return every
/// `DIAG-LAYER` it declares, each with its `DIAG-SERVICE`s.
pub fn parse_diag_layers(entry_name: &str, xml: &str) -> Result<Vec<DiagLayer>> {
    let doc = Document::parse(xml).map_err(|source| ReproError::Xml {
        name: entry_name.to_string(),
        source,
    })?;

    let mut layers = Vec::new();
    for layer_node in find_all_descendants(doc.root_element(), "DIAG-LAYER")
        .into_iter()
        .chain(find_all_descendants(doc.root_element(), "PROTOCOL"))
        .chain(find_all_descendants(doc.root_element(), "ECU-VARIANT"))
        .chain(find_all_descendants(doc.root_element(), "BASE-VARIANT"))
    {
        let short_name = child_text(layer_node, "SHORT-NAME").unwrap_or_default();
        let services = parse_diag_services(layer_node);
        if !services.is_empty() {
            layers.push(DiagLayer {
                short_name,
                services,
            });
        }
    }
    Ok(layers)
}

/// Parse an `odx-f` (flash data) document and return every flash data
/// block it declares (address + size of each contiguous memory segment).
pub fn parse_flash_data_blocks(entry_name: &str, xml: &str) -> Result<Vec<FlashDataBlock>> {
    let doc = Document::parse(xml).map_err(|source| ReproError::Xml {
        name: entry_name.to_string(),
        source,
    })?;

    let mut blocks = Vec::new();
    for node in find_all_descendants(doc.root_element(), "FLASHDATA") {
        let short_name = child_text(node, "SHORT-NAME").unwrap_or_default();
        // Address/size normally live under DATA/SEGMENT elements per ECU-MEM.
        for segment in find_all_descendants(node, "SEGMENT") {
            let address = child_text(segment, "SOURCE-START-ADDRESS")
                .or_else(|| child_text(segment, "START-ADDRESS"))
                .and_then(|s| parse_int(&s));
            let size = child_text(segment, "COMPRESSED-SIZE")
                .or_else(|| child_text(segment, "UNCOMPRESSED-SIZE"))
                .and_then(|s| parse_int(&s));
            if let (Some(address), Some(size)) = (address, size) {
                blocks.push(FlashDataBlock {
                    short_name: short_name.clone(),
                    address,
                    size,
                });
            }
        }
    }
    Ok(blocks)
}

fn parse_diag_services(layer_node: Node) -> Vec<DiagService> {
    let mut services = Vec::new();
    for svc_node in find_all_descendants(layer_node, "DIAG-SERVICE") {
        let short_name = child_text(svc_node, "SHORT-NAME").unwrap_or_default();
        let semantic = svc_node.attribute("SEMANTIC").map(str::to_string);

        let request = svc_node
            .children()
            .find(|n| n.has_tag_name("REQUEST"))
            .map(parse_message);
        let pos_responses = svc_node
            .children()
            .filter(|n| n.has_tag_name("POS-RESPONSE"))
            .map(parse_message)
            .collect();
        let neg_responses = svc_node
            .children()
            .filter(|n| n.has_tag_name("NEG-RESPONSE"))
            .map(parse_message)
            .collect();

        services.push(DiagService {
            short_name,
            semantic,
            request,
            pos_responses,
            neg_responses,
        });
    }
    services
}

fn parse_message(node: Node) -> Message {
    let short_name = child_text(node, "SHORT-NAME").unwrap_or_default();
    let mut params = Vec::new();

    if let Some(params_node) = node.children().find(|n| n.has_tag_name("PARAMS")) {
        for param_node in params_node.children().filter(|n| n.has_tag_name("PARAM")) {
            params.push(parse_param(param_node));
        }
    }

    Message { short_name, params }
}

fn parse_param(node: Node) -> Param {
    let short_name = child_text(node, "SHORT-NAME").unwrap_or_default();
    let byte_position = child_text(node, "BYTE-POSITION").and_then(|s| s.parse().ok());
    let bit_length = child_text(node, "BIT-LENGTH").and_then(|s| s.parse().ok());

    const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
    let xsi_type = node
        .attribute((XSI_NS, "type"))
        .or_else(|| node.attribute("xsi:type"));

    let kind = match xsi_type {
        Some("CODED-CONST") => {
            let bytes = child_text(node, "CODED-VALUE")
                .map(|v| coded_value_to_bytes(&v, bit_length))
                .unwrap_or_default();
            ParamKind::CodedConst { bytes }
        }
        other => {
            let data_type = other.map(str::to_string).or_else(|| {
                node.children()
                    .find(|n| n.has_tag_name("DIAG-CODED-TYPE"))
                    .and_then(|n| n.attribute("BASE-DATA-TYPE"))
                    .map(str::to_string)
            });
            ParamKind::Variable { data_type }
        }
    };

    Param {
        short_name,
        byte_position,
        bit_length,
        kind,
    }
}

/// ODX `CODED-VALUE` is a decimal integer representing the param's raw
/// value; convert it to big-endian bytes sized by the param's bit length
/// (defaulting to a single byte when unknown).
fn coded_value_to_bytes(value: &str, bit_length: Option<u32>) -> Vec<u8> {
    let Ok(v) = value.parse::<u64>() else {
        return Vec::new();
    };
    let byte_len = bit_length.map(|b| b.div_ceil(8).max(1) as usize).unwrap_or(1);
    let full = v.to_be_bytes();
    full[full.len() - byte_len.min(full.len())..].to_vec()
}

fn parse_int(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

fn child_text(node: Node, tag: &str) -> Option<String> {
    node.children()
        .find(|n| n.has_tag_name(tag))
        .and_then(|n| n.text())
        .map(str::to_string)
}

fn find_all_descendants<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Vec<Node<'a, 'input>> {
    node.descendants()
        .filter(|n| n.is_element() && n.has_tag_name(tag))
        .collect()
}
