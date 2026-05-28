use lean_corpus::Record;
use lean_meta::level_codec::{decode_level_prefix, encode_decoded_level};

use crate::registry::{ReplayFn, ReplayOutcome};

pub struct IsLevelDefEq;

impl ReplayFn for IsLevelDefEq {
    fn replay(&self, record: &Record) -> ReplayOutcome {
        let (lhs, rest) = match decode_level_prefix(&record.input) {
            Ok(p) => p,
            Err(e) => {
                return ReplayOutcome::Diverged {
                    detail: format!("decode lhs: {e:?}"),
                };
            }
        };
        let (rhs, rest) = match decode_level_prefix(rest) {
            Ok(p) => p,
            Err(e) => {
                return ReplayOutcome::Diverged {
                    detail: format!("decode rhs: {e:?}"),
                };
            }
        };
        if !rest.is_empty() {
            return ReplayOutcome::Diverged {
                detail: format!("input has {} trailing bytes after lhs+rhs", rest.len()),
            };
        }
        let mut roundtrip = Vec::with_capacity(record.input.len());
        if let Err(e) = encode_decoded_level(&lhs, &mut roundtrip) {
            return ReplayOutcome::Diverged {
                detail: format!("encode lhs: {e:?}"),
            };
        }
        if let Err(e) = encode_decoded_level(&rhs, &mut roundtrip) {
            return ReplayOutcome::Diverged {
                detail: format!("encode rhs: {e:?}"),
            };
        }
        if roundtrip != record.input {
            return ReplayOutcome::Diverged {
                detail: format!(
                    "roundtrip {} bytes != original {} bytes",
                    roundtrip.len(),
                    record.input.len()
                ),
            };
        }
        ReplayOutcome::Match
    }
}

pub const FQN: &str = "Lean.Meta.isLevelDefEqAuxImpl";

pub fn register(registry: &mut crate::registry::Registry) {
    registry.register(FQN, Box::new(IsLevelDefEq));
}

#[cfg(test)]
mod tests {
    use super::*;
    use lean_meta::level_codec::{DecodedLevel, DecodedName, encode_decoded_level};

    fn encode_pair(lhs: &DecodedLevel, rhs: &DecodedLevel) -> Vec<u8> {
        let mut out = Vec::new();
        encode_decoded_level(lhs, &mut out).unwrap();
        encode_decoded_level(rhs, &mut out).unwrap();
        out
    }

    fn record_with(input: Vec<u8>) -> Record {
        Record {
            input,
            state_in: vec![],
            output: vec![],
            state_out: vec![],
            message_delta: vec![],
        }
    }

    #[test]
    fn zero_pair_matches() {
        let bytes = encode_pair(&DecodedLevel::Zero, &DecodedLevel::Zero);
        assert_eq!(
            IsLevelDefEq.replay(&record_with(bytes)),
            ReplayOutcome::Match
        );
    }

    #[test]
    fn nontrivial_pair_matches() {
        let lhs = DecodedLevel::Succ(Box::new(DecodedLevel::Zero));
        let rhs = DecodedLevel::Max(
            Box::new(DecodedLevel::Zero),
            Box::new(DecodedLevel::Param(DecodedName::Str(
                Box::new(DecodedName::Anonymous),
                b"u".to_vec(),
            ))),
        );
        let bytes = encode_pair(&lhs, &rhs);
        assert_eq!(
            IsLevelDefEq.replay(&record_with(bytes)),
            ReplayOutcome::Match
        );
    }

    #[test]
    fn truncated_input_diverges() {
        let bytes = vec![lean_meta::level_codec::TAG_SUCC];
        match IsLevelDefEq.replay(&record_with(bytes)) {
            ReplayOutcome::Diverged { .. } => {}
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    #[test]
    fn trailing_bytes_diverge() {
        let mut bytes = encode_pair(&DecodedLevel::Zero, &DecodedLevel::Zero);
        bytes.push(0xff);
        match IsLevelDefEq.replay(&record_with(bytes)) {
            ReplayOutcome::Diverged { .. } => {}
            other => panic!("expected Diverged, got {other:?}"),
        }
    }
}
