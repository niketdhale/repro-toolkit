//! Builds an ordered UDS (ISO 14229) reprogramming sequence out of the
//! diagnostic services and flash data blocks parsed from a PDX archive,
//! and defines the JSON-serializable output shape.

use serde::{Deserialize, Serialize};

use crate::error::{ReproError, Result};
use crate::odx::model::{DiagLayer, DiagService, FlashDataBlock, Message, ParamKind};

/// The full generated repro sequence: every step a tool must execute, in
/// order, to reprogram the ECU described by the source PDX. This type is
/// round-trippable: the JSON it serializes to is also a valid *custom
/// sequence* input (see [`crate::custom`]), so a generated sequence can be
/// hand-edited and fed back in.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReproSequence {
    pub ecu_variant: String,
    pub step_count: usize,
    pub steps: Vec<SequenceStep>,
}

/// One request/response exchange in the sequence.
#[derive(Debug, Serialize, Deserialize)]
pub struct SequenceStep {
    pub index: usize,
    pub name: String,
    pub category: StepCategory,
    pub semantic: Option<String>,
    pub request: Option<MessageSpec>,
    #[serde(default)]
    pub expected_positive_responses: Vec<MessageSpec>,
    #[serde(default)]
    pub expected_negative_responses: Vec<MessageSpec>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StepCategory {
    SessionControl,
    SecuritySeedRequest,
    SecurityKeySend,
    EraseMemory,
    RequestDownload,
    TransferData,
    RequestTransferExit,
    CheckMemory,
    EcuReset,
    /// Also the default category for a custom step that doesn't declare one.
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSpec {
    pub name: String,
    #[serde(default)]
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: String,
    #[serde(default)]
    pub byte_position: Option<u32>,
    #[serde(default)]
    pub bit_length: Option<u32>,
    #[serde(flatten)]
    pub kind: FieldKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldKind {
    Fixed { value_hex: String },
    Variable {
        #[serde(default)]
        data_type: Option<String>,
    },
}

/// Build the canonical repro sequence from every diagnostic layer found in
/// the PDX. Services are classified by ODX `SEMANTIC` / short-name
/// keywords into the well-known UDS reprogramming steps; anything that
/// doesn't match a known category is still included, tagged `OTHER`, so
/// no information from the source ODX is silently dropped.
pub fn build_sequence(
    layers: &[DiagLayer],
    flash_blocks: &[FlashDataBlock],
) -> Result<ReproSequence> {
    let layer = layers
        .iter()
        .find(|l| !l.services.is_empty())
        .ok_or(ReproError::NoServicesFound)?;

    let mut classified: Vec<(StepCategory, &DiagService)> = layer
        .services
        .iter()
        .map(|svc| (classify(svc), svc))
        .collect();

    // Stable sort into canonical execution order; ties keep source order.
    classified.sort_by_key(|(cat, _)| category_rank(*cat));

    let mut steps = Vec::new();
    for (category, svc) in classified {
        if category == StepCategory::TransferData && !flash_blocks.is_empty() {
            for (i, block) in flash_blocks.iter().enumerate() {
                steps.push(build_step(
                    steps.len(),
                    category,
                    svc,
                    Some(format!(
                        "block {}/{} '{}': address=0x{:X} size=0x{:X} ({} bytes)",
                        i + 1,
                        flash_blocks.len(),
                        block.short_name,
                        block.address,
                        block.size,
                        block.size
                    )),
                ));
            }
        } else {
            steps.push(build_step(steps.len(), category, svc, None));
        }
    }

    Ok(ReproSequence {
        ecu_variant: layer.short_name.clone(),
        step_count: steps.len(),
        steps,
    })
}

fn build_step(
    index: usize,
    category: StepCategory,
    svc: &DiagService,
    notes: Option<String>,
) -> SequenceStep {
    SequenceStep {
        index,
        name: svc.short_name.clone(),
        category,
        semantic: svc.semantic.clone(),
        request: svc.request.as_ref().map(to_message_spec),
        expected_positive_responses: svc.pos_responses.iter().map(to_message_spec).collect(),
        expected_negative_responses: svc.neg_responses.iter().map(to_message_spec).collect(),
        notes,
    }
}

fn to_message_spec(msg: &Message) -> MessageSpec {
    MessageSpec {
        name: msg.short_name.clone(),
        fields: msg
            .params
            .iter()
            .map(|p| FieldSpec {
                name: p.short_name.clone(),
                byte_position: p.byte_position,
                bit_length: p.bit_length,
                kind: match &p.kind {
                    ParamKind::CodedConst { bytes } => FieldKind::Fixed {
                        value_hex: bytes_to_hex(bytes),
                    },
                    ParamKind::Variable { data_type } => FieldKind::Variable {
                        data_type: data_type.clone(),
                    },
                },
            })
            .collect(),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

pub(crate) fn category_rank(cat: StepCategory) -> u8 {
    match cat {
        StepCategory::SessionControl => 0,
        StepCategory::SecuritySeedRequest => 1,
        StepCategory::SecurityKeySend => 2,
        StepCategory::EraseMemory => 3,
        StepCategory::RequestDownload => 4,
        StepCategory::TransferData => 5,
        StepCategory::RequestTransferExit => 6,
        StepCategory::CheckMemory => 7,
        StepCategory::EcuReset => 8,
        StepCategory::Other => 9,
    }
}

fn classify(svc: &DiagService) -> StepCategory {
    let name = svc.short_name.to_uppercase();
    let semantic = svc.semantic.clone().unwrap_or_default().to_uppercase();
    let haystack = format!("{name} {semantic}");

    let has = |kw: &str| haystack.contains(kw);

    if has("SESSION") {
        StepCategory::SessionControl
    } else if has("SEED") {
        StepCategory::SecuritySeedRequest
    } else if has("SECURITY") || has("KEY") || has("UNLOCK") {
        StepCategory::SecurityKeySend
    } else if has("ERASE") {
        StepCategory::EraseMemory
    } else if has("REQUESTDOWNLOAD") || has("REQUEST_DOWNLOAD") || has("DOWNLOAD") {
        StepCategory::RequestDownload
    } else if has("TRANSFEREXIT") || has("TRANSFER_EXIT") || (has("TRANSFER") && has("EXIT")) {
        StepCategory::RequestTransferExit
    } else if has("TRANSFERDATA") || has("TRANSFER_DATA") || has("TRANSFER") {
        StepCategory::TransferData
    } else if has("CHECKSUM") || has("CHECKMEMORY") || has("VERIFY") {
        StepCategory::CheckMemory
    } else if has("RESET") {
        StepCategory::EcuReset
    } else {
        StepCategory::Other
    }
}
