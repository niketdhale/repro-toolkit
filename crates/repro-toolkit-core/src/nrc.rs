//! UDS negative response codes (ISO 14229-1, Annex A.1).

use crate::sequence::{FieldKind, MessageSpec};

/// Byte position of the NRC in a UDS negative response: `7F <SID> <NRC>`.
const NRC_BYTE_POSITION: u32 = 2;

/// Returns the ISO 14229-1 name of a negative response code, if it's a
/// defined one.
pub fn describe_nrc(code: u8) -> Option<&'static str> {
    Some(match code {
        0x10 => "generalReject",
        0x11 => "serviceNotSupported",
        0x12 => "subFunctionNotSupported",
        0x13 => "incorrectMessageLengthOrInvalidFormat",
        0x14 => "responseTooLong",
        0x21 => "busyRepeatRequest",
        0x22 => "conditionsNotCorrect",
        0x24 => "requestSequenceError",
        0x25 => "noResponseFromSubnetComponent",
        0x26 => "failurePreventsExecutionOfRequestedAction",
        0x31 => "requestOutOfRange",
        0x33 => "securityAccessDenied",
        0x34 => "authenticationRequired",
        0x35 => "invalidKey",
        0x36 => "exceededNumberOfAttempts",
        0x37 => "requiredTimeDelayNotExpired",
        0x70 => "uploadDownloadNotAccepted",
        0x71 => "transferDataSuspended",
        0x72 => "generalProgrammingFailure",
        0x73 => "wrongBlockSequenceCounter",
        0x78 => "requestCorrectlyReceivedResponsePending",
        0x7E => "subFunctionNotSupportedInActiveSession",
        0x7F => "serviceNotSupportedInActiveSession",
        0x81 => "rpmTooHigh",
        0x82 => "rpmTooLow",
        0x83 => "engineIsRunning",
        0x84 => "engineIsNotRunning",
        0x85 => "engineRunTimeTooLow",
        0x86 => "temperatureTooHigh",
        0x87 => "temperatureTooLow",
        0x88 => "vehicleSpeedTooHigh",
        0x89 => "vehicleSpeedTooLow",
        0x8A => "throttleOrPedalTooHigh",
        0x8B => "throttleOrPedalTooLow",
        0x8C => "transmissionRangeNotInNeutral",
        0x8D => "transmissionRangeNotInGear",
        0x8F => "brakeSwitchesNotClosed",
        0x90 => "shifterLeverNotInPark",
        0x91 => "torqueConverterClutchLocked",
        0x92 => "voltageTooHigh",
        0x93 => "voltageTooLow",
        _ => return None,
    })
}

/// Fills in `description` for the fixed NRC field (byte 2) of a negative
/// response, unless the field already has one.
pub(crate) fn annotate_negative_response(message: &mut MessageSpec) {
    for field in &mut message.fields {
        if field.description.is_some() || field.byte_position != Some(NRC_BYTE_POSITION) {
            continue;
        }
        if let FieldKind::Fixed { value_hex } = &field.kind {
            if let Ok(code) = u8::from_str_radix(value_hex, 16) {
                field.description = describe_nrc(code).map(str::to_string);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::FieldSpec;

    fn fixed_field(byte_position: u32, value_hex: &str, description: Option<&str>) -> FieldSpec {
        FieldSpec {
            name: "F".to_string(),
            byte_position: Some(byte_position),
            bit_length: Some(8),
            description: description.map(str::to_string),
            kind: FieldKind::Fixed {
                value_hex: value_hex.to_string(),
            },
        }
    }

    #[test]
    fn describes_known_and_unknown_codes() {
        assert_eq!(describe_nrc(0x35), Some("invalidKey"));
        assert_eq!(describe_nrc(0x78), Some("requestCorrectlyReceivedResponsePending"));
        assert_eq!(describe_nrc(0x00), None);
    }

    #[test]
    fn annotates_only_the_nrc_byte_and_keeps_existing_descriptions() {
        let mut message = MessageSpec {
            name: "Neg".to_string(),
            fields: vec![
                fixed_field(0, "7F", None),
                fixed_field(2, "22", None),
            ],
        };
        annotate_negative_response(&mut message);
        assert_eq!(message.fields[0].description, None);
        assert_eq!(message.fields[1].description.as_deref(), Some("conditionsNotCorrect"));

        let mut custom = MessageSpec {
            name: "Neg".to_string(),
            fields: vec![fixed_field(2, "22", Some("vendor text"))],
        };
        annotate_negative_response(&mut custom);
        assert_eq!(custom.fields[0].description.as_deref(), Some("vendor text"));
    }
}
