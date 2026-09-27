//! `repro-toolkit-core`: parses automotive PDX/ODX diagnostic containers
//! (ISO 22901) and generates a UDS (ISO 14229) ECU reprogramming sequence
//! as JSON.
//!
//! ```no_run
//! let sequence = repro_toolkit_core::parse_pdx_file("ecu.pdx")?;
//! let json = repro_toolkit_core::to_json_string(&sequence)?;
//! # Ok::<(), repro_toolkit_core::ReproError>(())
//! ```

mod custom;
mod error;
mod nrc;
mod odx;
mod pdx;
mod sequence;
mod validate;

use std::path::Path;

pub use custom::{CustomSequenceInput, CustomStepInput};
pub use error::{ReproError, Result};
pub use nrc::describe_nrc;
pub use sequence::{
    FieldKind, FieldSpec, GenerateOptions, MessageSpec, ReproSequence, SequenceStep, StepCategory,
    DEFAULT_MAX_BLOCK_LENGTH,
};
pub use validate::{validate_sequence, Severity, ValidationIssue};

/// Parse a PDX file on disk and build its UDS reprogramming sequence with
/// default [`GenerateOptions`].
pub fn parse_pdx_file<P: AsRef<Path>>(path: P) -> Result<ReproSequence> {
    parse_pdx_file_with_options(path, &GenerateOptions::default())
}

/// Like [`parse_pdx_file`], with explicit [`GenerateOptions`] (e.g. the
/// TransferData chunk size).
pub fn parse_pdx_file_with_options<P: AsRef<Path>>(
    path: P,
    options: &GenerateOptions,
) -> Result<ReproSequence> {
    let contents = pdx::load_pdx_file(path.as_ref())?;
    sequence::build_sequence(&contents.diag_layers, &contents.flash_data_blocks, options)
}

/// Parse a PDX archive already loaded into memory (e.g. downloaded, or
/// embedded) and build its UDS reprogramming sequence.
pub fn parse_pdx_bytes(bytes: Vec<u8>) -> Result<ReproSequence> {
    parse_pdx_bytes_with_options(bytes, &GenerateOptions::default())
}

/// Like [`parse_pdx_bytes`], with explicit [`GenerateOptions`].
pub fn parse_pdx_bytes_with_options(
    bytes: Vec<u8>,
    options: &GenerateOptions,
) -> Result<ReproSequence> {
    let contents = pdx::load_pdx_bytes(bytes)?;
    sequence::build_sequence(&contents.diag_layers, &contents.flash_data_blocks, options)
}

/// Generate the repro sequence for a PDX file, optionally overridden by a
/// user-supplied custom sequence JSON file.
///
/// - `custom_sequence_path` is `None`: identical to [`parse_pdx_file`] —
///   the sequence is fully auto-generated from the PDX.
/// - `custom_sequence_path` is `Some(path)`: `path` is parsed as a
///   [`CustomSequenceInput`] and used instead of auto-generating steps
///   from the PDX. The PDX is still opened, only to supply the ECU
///   variant name when the custom file doesn't set one itself. See
///   `docs/custom-sequence-guide.md` for the custom sequence JSON schema.
pub fn generate_sequence<P: AsRef<Path>, Q: AsRef<Path>>(
    pdx_path: P,
    custom_sequence_path: Option<Q>,
) -> Result<ReproSequence> {
    generate_sequence_with_options(pdx_path, custom_sequence_path, &GenerateOptions::default())
}

/// Like [`generate_sequence`], with explicit [`GenerateOptions`]. The
/// options only affect the auto-generated sequence; a custom sequence is
/// used exactly as written.
pub fn generate_sequence_with_options<P: AsRef<Path>, Q: AsRef<Path>>(
    pdx_path: P,
    custom_sequence_path: Option<Q>,
    options: &GenerateOptions,
) -> Result<ReproSequence> {
    match custom_sequence_path {
        None => parse_pdx_file_with_options(pdx_path, options),
        Some(seq_path) => {
            let ecu_variant_fallback = pdx::load_pdx_file(pdx_path.as_ref())
                .ok()
                .and_then(|contents| {
                    contents
                        .diag_layers
                        .iter()
                        .find(|l| !l.services.is_empty())
                        .map(|l| l.short_name.clone())
                });
            custom::load_custom_sequence_file(seq_path.as_ref(), ecu_variant_fallback)
        }
    }
}

/// Serialize a repro sequence to a pretty-printed JSON string.
pub fn to_json_string(sequence: &ReproSequence) -> Result<String> {
    Ok(serde_json::to_string_pretty(sequence)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    const SAMPLE_ODX_D: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ODX MODEL-VERSION="2.2.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <DIAG-LAYER-CONTAINER>
    <ECU-VARIANTS>
      <ECU-VARIANT ID="EV_1">
        <SHORT-NAME>Sample_ECU</SHORT-NAME>
        <DIAG-COMMS>
          <DIAG-SERVICE ID="S_SESSION" SEMANTIC="SESSION">
            <SHORT-NAME>DiagnosticSessionControl_Programming</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>DSC_Programming_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>16</CODED-VALUE>
                </PARAM>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SubFunction</SHORT-NAME>
                  <BYTE-POSITION>1</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>2</CODED-VALUE>
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>DSC_Programming_PosRsp</SHORT-NAME>
              <PARAMS>
                <PARAM>
                  <SHORT-NAME>ResponseSID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <DIAG-CODED-TYPE BASE-DATA-TYPE="A_UINT32" />
                </PARAM>
              </PARAMS>
            </POS-RESPONSE>
            <NEG-RESPONSE>
              <SHORT-NAME>DSC_NegRsp</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>NegResponseSID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>127</CODED-VALUE>
                </PARAM>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>RequestSID</SHORT-NAME>
                  <BYTE-POSITION>1</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>16</CODED-VALUE>
                </PARAM>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>NRC</SHORT-NAME>
                  <BYTE-POSITION>2</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>34</CODED-VALUE>
                </PARAM>
              </PARAMS>
            </NEG-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_SEED" SEMANTIC="SECURITY">
            <SHORT-NAME>SecurityAccess_RequestSeed</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>RequestSeed_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>39</CODED-VALUE>
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>RequestSeed_PosRsp</SHORT-NAME>
              <PARAMS>
                <PARAM>
                  <SHORT-NAME>Seed</SHORT-NAME>
                  <BYTE-POSITION>2</BYTE-POSITION>
                  <DIAG-CODED-TYPE BASE-DATA-TYPE="A_BYTEFIELD" />
                </PARAM>
              </PARAMS>
            </POS-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_KEY" SEMANTIC="SECURITY">
            <SHORT-NAME>SecurityAccess_SendKey</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>SendKey_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>39</CODED-VALUE>
                </PARAM>
                <PARAM>
                  <SHORT-NAME>Key</SHORT-NAME>
                  <BYTE-POSITION>2</BYTE-POSITION>
                  <DIAG-CODED-TYPE BASE-DATA-TYPE="A_BYTEFIELD" />
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>SendKey_PosRsp</SHORT-NAME>
              <PARAMS/>
            </POS-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_DL" SEMANTIC="FLASH">
            <SHORT-NAME>RequestDownload</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>RequestDownload_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>52</CODED-VALUE>
                </PARAM>
                <PARAM>
                  <SHORT-NAME>MemoryAddress</SHORT-NAME>
                  <BYTE-POSITION>2</BYTE-POSITION>
                  <DIAG-CODED-TYPE BASE-DATA-TYPE="A_UINT32" />
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>RequestDownload_PosRsp</SHORT-NAME>
              <PARAMS/>
            </POS-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_TD" SEMANTIC="FLASH">
            <SHORT-NAME>TransferData</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>TransferData_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>54</CODED-VALUE>
                </PARAM>
                <PARAM>
                  <SHORT-NAME>BlockData</SHORT-NAME>
                  <BYTE-POSITION>2</BYTE-POSITION>
                  <DIAG-CODED-TYPE BASE-DATA-TYPE="A_BYTEFIELD" />
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>TransferData_PosRsp</SHORT-NAME>
              <PARAMS/>
            </POS-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_TE" SEMANTIC="FLASH">
            <SHORT-NAME>RequestTransferExit</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>RequestTransferExit_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>55</CODED-VALUE>
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>RequestTransferExit_PosRsp</SHORT-NAME>
              <PARAMS/>
            </POS-RESPONSE>
          </DIAG-SERVICE>
          <DIAG-SERVICE ID="S_RESET" SEMANTIC="RESET">
            <SHORT-NAME>ECUReset_Hard</SHORT-NAME>
            <REQUEST>
              <SHORT-NAME>ECUReset_Req</SHORT-NAME>
              <PARAMS>
                <PARAM xsi:type="CODED-CONST">
                  <SHORT-NAME>SID</SHORT-NAME>
                  <BYTE-POSITION>0</BYTE-POSITION>
                  <BIT-LENGTH>8</BIT-LENGTH>
                  <CODED-VALUE>17</CODED-VALUE>
                </PARAM>
              </PARAMS>
            </REQUEST>
            <POS-RESPONSE>
              <SHORT-NAME>ECUReset_PosRsp</SHORT-NAME>
              <PARAMS/>
            </POS-RESPONSE>
          </DIAG-SERVICE>
        </DIAG-COMMS>
      </ECU-VARIANT>
    </ECU-VARIANTS>
  </DIAG-LAYER-CONTAINER>
</ODX>
"#;

    const SAMPLE_ODX_F: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ODX MODEL-VERSION="2.2.0">
  <DIAG-LAYER-CONTAINER>
    <FLASHDATAS>
      <FLASHDATA ID="FD_1">
        <SHORT-NAME>ApplicationBlock</SHORT-NAME>
        <SEGMENTS>
          <SEGMENT>
            <SOURCE-START-ADDRESS>0x00080000</SOURCE-START-ADDRESS>
            <UNCOMPRESSED-SIZE>0x00010000</UNCOMPRESSED-SIZE>
          </SEGMENT>
          <SEGMENT>
            <SOURCE-START-ADDRESS>0x00090000</SOURCE-START-ADDRESS>
            <UNCOMPRESSED-SIZE>0x00008000</UNCOMPRESSED-SIZE>
          </SEGMENT>
        </SEGMENTS>
      </FLASHDATA>
    </FLASHDATAS>
  </DIAG-LAYER-CONTAINER>
</ODX>
"#;

    fn build_sample_pdx() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            writer.start_file("sample.odx-d", options).unwrap();
            writer.write_all(SAMPLE_ODX_D.as_bytes()).unwrap();
            writer.start_file("sample.odx-f", options).unwrap();
            writer.write_all(SAMPLE_ODX_F.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn default_sequence_validates_clean() {
        let sequence = parse_pdx_bytes(build_sample_pdx()).expect("should parse sample PDX");
        assert!(
            validate_sequence(&sequence).is_empty(),
            "auto-generated default sequence should never trip the validator"
        );
    }

    #[test]
    fn parses_sample_pdx_into_ordered_sequence() {
        // Block length large enough that each flash block is one chunk.
        let options = GenerateOptions {
            max_block_length: 0x1_0002,
        };
        let sequence = parse_pdx_bytes_with_options(build_sample_pdx(), &options)
            .expect("should parse sample PDX");

        assert_eq!(sequence.ecu_variant, "Sample_ECU");

        // Session -> Seed -> Key -> Download -> TransferData x2 (one per
        // flash block) -> TransferExit -> Reset.
        let names: Vec<&str> = sequence.steps.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "DiagnosticSessionControl_Programming",
                "SecurityAccess_RequestSeed",
                "SecurityAccess_SendKey",
                "RequestDownload",
                "TransferData",
                "TransferData",
                "RequestTransferExit",
                "ECUReset_Hard",
            ]
        );

        assert_eq!(sequence.steps[0].category, StepCategory::SessionControl);
        assert_eq!(
            sequence.steps[1].category,
            StepCategory::SecuritySeedRequest
        );
        assert_eq!(sequence.steps[4].category, StepCategory::TransferData);
        assert_eq!(sequence.steps[5].category, StepCategory::TransferData);
        assert!(sequence.steps[4].notes.as_ref().unwrap().contains("block 1/2"));
        assert!(sequence.steps[5].notes.as_ref().unwrap().contains("block 2/2"));

        let session_req = sequence.steps[0].request.as_ref().unwrap();
        assert_eq!(session_req.fields.len(), 2);
        match &session_req.fields[0].kind {
            FieldKind::Fixed { value_hex } => assert_eq!(value_hex, "10"),
            _ => panic!("expected fixed SID field"),
        }
        match &session_req.fields[1].kind {
            FieldKind::Fixed { value_hex } => assert_eq!(value_hex, "02"),
            _ => panic!("expected fixed sub-function field"),
        }

        let seed_pos_rsp = &sequence.steps[1].expected_positive_responses[0];
        match &seed_pos_rsp.fields[0].kind {
            FieldKind::Variable { data_type } => {
                assert_eq!(data_type.as_deref(), Some("A_BYTEFIELD"))
            }
            _ => panic!("expected variable seed field"),
        }

        let json = to_json_string(&sequence).expect("should serialize to JSON");
        assert!(json.contains("\"ecu_variant\": \"Sample_ECU\""));
    }

    #[test]
    fn rejects_archive_without_diag_layer_container() {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = SimpleFileOptions::default();
            writer.start_file("readme.txt", options).unwrap();
            writer.write_all(b"not an odx file").unwrap();
            writer.finish().unwrap();
        }

        let err = parse_pdx_bytes(buf).unwrap_err();
        assert!(matches!(err, ReproError::NoDiagLayerContainer));
    }

    #[test]
    fn generate_sequence_uses_custom_file_with_ecu_name_from_pdx() {
        use std::io::Write;

        let pdx_path = {
            let mut f = tempfile::NamedTempFile::with_suffix(".pdx").unwrap();
            f.write_all(&build_sample_pdx()).unwrap();
            f
        };

        let custom_path = {
            let mut f = tempfile::NamedTempFile::with_suffix(".json").unwrap();
            f.write_all(
                br#"{
                    "steps": [
                        {
                            "name": "CustomWait",
                            "notes": "vendor-required 500ms settle delay"
                        },
                        {
                            "name": "VendorRoutine",
                            "request": {
                                "name": "VendorRoutine_Req",
                                "fields": [
                                    { "name": "SID", "kind": "fixed", "value_hex": "31" }
                                ]
                            }
                        }
                    ]
                }"#,
            )
            .unwrap();
            f
        };

        let sequence = generate_sequence(pdx_path.path(), Some(custom_path.path()))
            .expect("should load custom sequence");

        // ECU name comes from the PDX since the custom file didn't set one.
        assert_eq!(sequence.ecu_variant, "Sample_ECU");
        assert_eq!(sequence.step_count, 2);
        assert_eq!(sequence.steps[0].name, "CustomWait");
        assert_eq!(sequence.steps[0].category, StepCategory::Other);
        assert_eq!(sequence.steps[1].name, "VendorRoutine");
    }

    #[test]
    fn generate_sequence_without_custom_file_matches_default() {
        use std::io::Write;
        let pdx_path = {
            let mut f = tempfile::NamedTempFile::with_suffix(".pdx").unwrap();
            f.write_all(&build_sample_pdx()).unwrap();
            f
        };

        let sequence = generate_sequence::<_, &Path>(pdx_path.path(), None)
            .expect("should build default sequence");
        // 6 non-transfer steps + 17 + 9 TransferData chunks at the default
        // 0x0FFF block length (4093 data bytes per chunk).
        assert_eq!(sequence.step_count, 32);
    }

    fn transfer_steps(sequence: &ReproSequence) -> Vec<&SequenceStep> {
        sequence
            .steps
            .iter()
            .filter(|s| s.category == StepCategory::TransferData)
            .collect()
    }

    fn counter_of(step: &SequenceStep) -> String {
        let req = step.request.as_ref().unwrap();
        let field = req
            .fields
            .iter()
            .find(|f| f.name == "BlockSequenceCounter")
            .expect("chunk should carry a block sequence counter");
        assert_eq!(field.byte_position, Some(1));
        match &field.kind {
            FieldKind::Fixed { value_hex } => value_hex.clone(),
            _ => panic!("counter should be fixed"),
        }
    }

    #[test]
    fn chunks_transfer_data_by_max_block_length() {
        let sequence = parse_pdx_bytes(build_sample_pdx()).unwrap();
        let chunks = transfer_steps(&sequence);
        assert_eq!(chunks.len(), 26);

        let first = chunks[0].notes.as_deref().unwrap();
        assert!(first.contains("block 1/2 'ApplicationBlock' chunk 1/17"), "{first}");
        assert!(first.contains("address=0x80000 length=4093 bytes"), "{first}");

        // Last chunk of block 1: 65536 - 16 * 4093 = 48 bytes.
        let last_of_block1 = chunks[16].notes.as_deref().unwrap();
        assert!(last_of_block1.contains("chunk 17/17"), "{last_of_block1}");
        assert!(last_of_block1.contains("length=48 bytes"), "{last_of_block1}");

        // Counter starts at 01 and keeps counting into the second block.
        assert_eq!(counter_of(chunks[0]), "01");
        assert_eq!(counter_of(chunks[16]), "11");
        assert_eq!(counter_of(chunks[17]), "12");

        // Counter byte is inserted between SID (byte 0) and data (byte 2).
        let names: Vec<&str> = chunks[0]
            .request
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(names, vec!["SID", "BlockSequenceCounter", "BlockData"]);

        assert!(validate_sequence(&sequence).is_empty());
    }

    #[test]
    fn block_sequence_counter_wraps_from_ff_to_00() {
        // 3-byte blocks carry 1 data byte each: 0x10000 + 0x8000 chunks.
        let options = GenerateOptions { max_block_length: 3 };
        let sequence = parse_pdx_bytes_with_options(build_sample_pdx(), &options).unwrap();
        let chunks = transfer_steps(&sequence);
        assert_eq!(chunks.len(), 0x10000 + 0x8000);
        assert_eq!(counter_of(chunks[254]), "FF");
        assert_eq!(counter_of(chunks[255]), "00");
        assert_eq!(counter_of(chunks[256]), "01");
    }

    #[test]
    fn rejects_too_small_max_block_length() {
        let options = GenerateOptions { max_block_length: 2 };
        let err = parse_pdx_bytes_with_options(build_sample_pdx(), &options).unwrap_err();
        assert!(matches!(err, ReproError::InvalidMaxBlockLength(2)));
    }

    #[test]
    fn labels_nrc_in_negative_responses() {
        let sequence = parse_pdx_bytes(build_sample_pdx()).unwrap();
        let neg = &sequence.steps[0].expected_negative_responses[0];
        let nrc = neg.fields.iter().find(|f| f.name == "NRC").unwrap();
        assert_eq!(nrc.description.as_deref(), Some("conditionsNotCorrect"));
        // Only the NRC byte gets a description.
        assert!(neg.fields.iter().filter(|f| f.description.is_some()).count() == 1);

        let json = to_json_string(&sequence).unwrap();
        assert!(json.contains("\"description\": \"conditionsNotCorrect\""));
    }
}
