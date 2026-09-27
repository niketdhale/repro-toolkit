//! `repro-toolkit-core`: parses automotive PDX/ODX diagnostic containers
//! (ISO 22901) and generates a UDS (ISO 14229) ECU reprogramming sequence
//! as JSON.
//!
//! ```no_run
//! let sequence = repro_toolkit_core::parse_pdx_file("ecu.pdx")?;
//! let json = repro_toolkit_core::to_json_string(&sequence)?;
//! # Ok::<(), repro_toolkit_core::ReproError>(())
//! ```

mod error;
mod odx;
mod pdx;
mod sequence;

use std::path::Path;

pub use error::{ReproError, Result};
pub use sequence::{FieldKind, FieldSpec, MessageSpec, ReproSequence, SequenceStep, StepCategory};

/// Parse a PDX file on disk and build its UDS reprogramming sequence.
pub fn parse_pdx_file<P: AsRef<Path>>(path: P) -> Result<ReproSequence> {
    let contents = pdx::load_pdx_file(path.as_ref())?;
    sequence::build_sequence(&contents.diag_layers, &contents.flash_data_blocks)
}

/// Parse a PDX archive already loaded into memory (e.g. downloaded, or
/// embedded) and build its UDS reprogramming sequence.
pub fn parse_pdx_bytes(bytes: Vec<u8>) -> Result<ReproSequence> {
    let contents = pdx::load_pdx_bytes(bytes)?;
    sequence::build_sequence(&contents.diag_layers, &contents.flash_data_blocks)
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
    fn parses_sample_pdx_into_ordered_sequence() {
        let sequence = parse_pdx_bytes(build_sample_pdx()).expect("should parse sample PDX");

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
}
