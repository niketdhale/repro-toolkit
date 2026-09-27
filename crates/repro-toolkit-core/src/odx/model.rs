//! In-memory representation of the subset of ISO 22901 (ODX) that matters
//! for building a UDS reprogramming sequence: diagnostic services, their
//! request/response byte layout, and flash data blocks.

/// One `DIAG-LAYER` (e.g. an ECU-VARIANT or PROTOCOL layer) and the
/// diagnostic services it declares.
#[derive(Debug, Clone, Default)]
pub struct DiagLayer {
    pub short_name: String,
    pub services: Vec<DiagService>,
}

/// One `DIAG-SERVICE`: a request plus the positive/negative responses ODX
/// says are valid for it.
#[derive(Debug, Clone)]
pub struct DiagService {
    pub short_name: String,
    /// ODX `SEMANTIC` attribute, e.g. "SESSION", "SECURITY", "FLASH". Used
    /// to recognize well-known UDS steps when building the sequence.
    pub semantic: Option<String>,
    pub request: Option<Message>,
    pub pos_responses: Vec<Message>,
    pub neg_responses: Vec<Message>,
}

/// A `REQUEST`, `POS-RESPONSE`, or `NEG-RESPONSE`: an ordered list of byte
/// parameters.
#[derive(Debug, Clone, Default)]
pub struct Message {
    pub short_name: String,
    pub params: Vec<Param>,
}

/// One `PARAM` inside a request/response. `CODED-CONST` params carry a
/// fixed, known byte value (e.g. the service ID); everything else is a
/// variable field whose value is only known at runtime (e.g. a seed,
/// an address, a data payload).
#[derive(Debug, Clone)]
pub struct Param {
    pub short_name: String,
    pub byte_position: Option<u32>,
    pub bit_length: Option<u32>,
    pub kind: ParamKind,
}

#[derive(Debug, Clone)]
pub enum ParamKind {
    /// Fixed byte(s) known at parse time, e.g. the SID or sub-function.
    CodedConst { bytes: Vec<u8> },
    /// A value supplied at runtime (physical value, table key, etc.).
    Variable { data_type: Option<String> },
}

/// One contiguous block of flash data extracted from an `odx-f` flash
/// container: an address range and the raw bytes to transfer.
#[derive(Debug, Clone)]
pub struct FlashDataBlock {
    pub short_name: String,
    pub address: u64,
    pub size: u64,
}
