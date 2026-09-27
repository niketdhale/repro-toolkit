//! Validates a [`ReproSequence`] for problems that are valid JSON but
//! still wrong in ways that matter before the sequence is ever sent to a
//! real ECU: overlapping byte positions within one message, an invalid
//! hex value on a fixed field, a known UDS step missing a request
//! entirely, or known steps in a nonsensical order.
//!
//! This works on the sequence itself, not its JSON source, so it applies
//! identically to a sequence auto-generated from a PDX and to a
//! hand-written custom sequence (see [`crate::custom`]).

use std::collections::HashMap;
use std::fmt;

use crate::sequence::{category_rank, FieldKind, MessageSpec, ReproSequence, StepCategory};

/// How serious a [`ValidationIssue`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Something that will very likely break a real flashing tool or
    /// ECU exchange (invalid hex, overlapping byte positions).
    Error,
    /// Something suspicious but not necessarily wrong — the sequence
    /// format gives the caller full control over step order and content,
    /// so these are heads-up, not rejections.
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        })
    }
}

/// One problem found in a sequence by [`validate_sequence`].
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    /// The step this issue applies to, or `None` for a sequence-level issue.
    pub step_index: Option<usize>,
    pub message: String,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.step_index {
            Some(index) => write!(f, "[{}] step {index}: {}", self.severity, self.message),
            None => write!(f, "[{}] sequence: {}", self.severity, self.message),
        }
    }
}

/// The known UDS categories relevant here, in the order they're expected
/// to occur when more than one is present in a sequence. `EraseMemory`,
/// `CheckMemory`, `EcuReset`, and `Other` are deliberately excluded: their
/// position relative to the others varies too much across real OEM flash
/// procedures to check meaningfully.
const ORDERED_CHAIN: [StepCategory; 6] = [
    StepCategory::SessionControl,
    StepCategory::SecuritySeedRequest,
    StepCategory::SecurityKeySend,
    StepCategory::RequestDownload,
    StepCategory::TransferData,
    StepCategory::RequestTransferExit,
];

/// Validates a repro sequence, returning every issue found. An empty
/// result means the sequence is clean; the caller decides what to do with
/// [`Severity::Warning`] issues, but should treat any [`Severity::Error`]
/// as a reason not to use the sequence as-is.
pub fn validate_sequence(sequence: &ReproSequence) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for step in &sequence.steps {
        if step.category != StepCategory::Other && step.request.is_none() {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                step_index: Some(step.index),
                message: format!(
                    "step has category {:?} but no request; every known UDS step should send one",
                    step.category
                ),
            });
        }

        if let Some(request) = &step.request {
            check_message(step.index, "request", request, &mut issues);
        }
        for response in &step.expected_positive_responses {
            check_message(step.index, "positive response", response, &mut issues);
        }
        for response in &step.expected_negative_responses {
            check_message(step.index, "negative response", response, &mut issues);
        }
    }

    check_ordering(sequence, &mut issues);

    issues
}

fn check_message(
    step_index: usize,
    message_kind: &str,
    message: &MessageSpec,
    issues: &mut Vec<ValidationIssue>,
) {
    let mut ranges: Vec<(u32, u32, &str)> = Vec::new();

    for field in &message.fields {
        if let FieldKind::Fixed { value_hex } = &field.kind {
            if !is_valid_hex(value_hex) {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    step_index: Some(step_index),
                    message: format!(
                        "{message_kind} '{}' field '{}' has invalid value_hex {:?} (must be a non-empty, even-length hex string)",
                        message.name, field.name, value_hex
                    ),
                });
            }
        }

        if let (Some(byte_position), Some(bit_length)) = (field.byte_position, field.bit_length) {
            let byte_len = bit_length.div_ceil(8).max(1);
            ranges.push((byte_position, byte_position + byte_len, field.name.as_str()));
        }
    }

    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (start_a, end_a, name_a) = ranges[i];
            let (start_b, end_b, name_b) = ranges[j];
            if start_a < end_b && start_b < end_a {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    step_index: Some(step_index),
                    message: format!(
                        "{message_kind} '{}' fields '{name_a}' and '{name_b}' have overlapping byte ranges",
                        message.name
                    ),
                });
            }
        }
    }
}

fn is_valid_hex(value: &str) -> bool {
    !value.is_empty() && value.len().is_multiple_of(2) && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn check_ordering(sequence: &ReproSequence, issues: &mut Vec<ValidationIssue>) {
    let mut first_occurrence: HashMap<StepCategory, usize> = HashMap::new();
    for step in &sequence.steps {
        if ORDERED_CHAIN.contains(&step.category) {
            first_occurrence.entry(step.category).or_insert(step.index);
        }
    }

    for (i, &earlier) in ORDERED_CHAIN.iter().enumerate() {
        for &later in ORDERED_CHAIN.iter().skip(i + 1) {
            debug_assert!(category_rank(earlier) < category_rank(later));

            if let (Some(&earlier_index), Some(&later_index)) =
                (first_occurrence.get(&earlier), first_occurrence.get(&later))
            {
                if earlier_index > later_index {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        step_index: None,
                        message: format!(
                            "step {earlier_index} ({earlier:?}) comes after step {later_index} ({later:?}); \
                             that's the reverse of the usual UDS flash order"
                        ),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{FieldSpec, SequenceStep};

    fn field(name: &str, byte_position: Option<u32>, bit_length: Option<u32>, kind: FieldKind) -> FieldSpec {
        FieldSpec {
            name: name.to_string(),
            byte_position,
            bit_length,
            description: None,
            kind,
        }
    }

    fn fixed(value_hex: &str) -> FieldKind {
        FieldKind::Fixed {
            value_hex: value_hex.to_string(),
        }
    }

    fn step(index: usize, category: StepCategory, request: Option<MessageSpec>) -> SequenceStep {
        SequenceStep {
            index,
            name: format!("Step{index}"),
            category,
            semantic: None,
            request,
            expected_positive_responses: Vec::new(),
            expected_negative_responses: Vec::new(),
            notes: None,
        }
    }

    fn sequence(steps: Vec<SequenceStep>) -> ReproSequence {
        ReproSequence {
            ecu_variant: "Test_ECU".to_string(),
            step_count: steps.len(),
            steps,
        }
    }

    #[test]
    fn detects_invalid_hex() {
        let msg = MessageSpec {
            name: "Req".to_string(),
            fields: vec![field("SID", Some(0), Some(8), fixed("G1"))],
        };
        let seq = sequence(vec![step(0, StepCategory::Other, Some(msg))]);

        let issues = validate_sequence(&seq);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert!(issues[0].message.contains("invalid value_hex"));
    }

    #[test]
    fn detects_overlapping_byte_ranges() {
        let msg = MessageSpec {
            name: "Req".to_string(),
            fields: vec![
                field("SID", Some(0), Some(16), fixed("1002")),
                field("SubFunction", Some(1), Some(8), fixed("02")),
            ],
        };
        let seq = sequence(vec![step(0, StepCategory::Other, Some(msg))]);

        let issues = validate_sequence(&seq);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert!(issues[0].message.contains("overlapping byte ranges"));
    }

    #[test]
    fn non_overlapping_ranges_are_clean() {
        let msg = MessageSpec {
            name: "Req".to_string(),
            fields: vec![
                field("SID", Some(0), Some(8), fixed("10")),
                field("SubFunction", Some(1), Some(8), fixed("02")),
            ],
        };
        let seq = sequence(vec![step(0, StepCategory::SessionControl, Some(msg))]);

        assert!(validate_sequence(&seq).is_empty());
    }

    #[test]
    fn warns_on_known_category_without_request() {
        let seq = sequence(vec![step(0, StepCategory::SessionControl, None)]);

        let issues = validate_sequence(&seq);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert!(issues[0].message.contains("no request"));
    }

    #[test]
    fn other_category_without_request_is_fine() {
        let seq = sequence(vec![step(0, StepCategory::Other, None)]);
        assert!(validate_sequence(&seq).is_empty());
    }

    #[test]
    fn warns_on_out_of_order_categories() {
        // TransferData before RequestDownload: backwards.
        let seq = sequence(vec![
            step(0, StepCategory::TransferData, None),
            step(1, StepCategory::RequestDownload, None),
        ]);

        let issues = validate_sequence(&seq);
        // One warning for the ordering, plus one each for the two known
        // categories missing a request.
        assert_eq!(issues.len(), 3);
        assert!(issues
            .iter()
            .any(|i| i.message.contains("reverse of the usual UDS flash order")));
    }

    #[test]
    fn omitting_a_chain_category_is_not_penalized() {
        // No security access at all: SessionControl straight to
        // RequestDownload should not warn about "missing" security steps.
        let msg = MessageSpec {
            name: "Req".to_string(),
            fields: vec![field("SID", Some(0), Some(8), fixed("10"))],
        };
        let seq = sequence(vec![
            step(0, StepCategory::SessionControl, Some(msg.clone())),
            step(1, StepCategory::RequestDownload, Some(msg)),
        ]);

        assert!(validate_sequence(&seq).is_empty());
    }
}
