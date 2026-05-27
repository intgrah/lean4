use lean_expr::{Level, Name};
use lean_runtime_sys::{lean_mk_string_from_bytes, lean_uint64_to_nat};

use crate::level_codec::{DecodedLevel, DecodedName};

pub fn build_level(d: &DecodedLevel) -> Level {
    match d {
        DecodedLevel::Zero => Level::zero(),
        DecodedLevel::Succ(c) => {
            let inner = build_level(c);
            Level::succ(inner.as_ref())
        }
        DecodedLevel::Max(a, b) => {
            let lhs = build_level(a);
            let rhs = build_level(b);
            Level::max(lhs.as_ref(), rhs.as_ref())
        }
        DecodedLevel::IMax(a, b) => {
            let lhs = build_level(a);
            let rhs = build_level(b);
            Level::imax(lhs.as_ref(), rhs.as_ref())
        }
        DecodedLevel::Param(n) => {
            let name = build_name(n);
            Level::param(name.as_ref())
        }
        DecodedLevel::MVar(n) => {
            let name = build_name(n);
            let id = unsafe { lean_expr::LMVarIdRef::from_name(name.as_ref()) };
            Level::mvar(id)
        }
    }
}

pub fn build_name(d: &DecodedName) -> Name {
    match d {
        DecodedName::Anonymous => Name::anonymous(),
        DecodedName::Str(parent, bytes) => {
            let parent_name = build_name(parent);
            let sym = unsafe { lean_mk_string_from_bytes(bytes.as_ptr() as *const _, bytes.len()) };
            unsafe { Name::str_unchecked(parent_name.as_ref().obj(), sym) }
        }
        DecodedName::Num(parent, v) => {
            let parent_name = build_name(parent);
            let idx = unsafe { lean_uint64_to_nat(*v) };
            unsafe { Name::num_unchecked(parent_name.as_ref().obj(), idx) }
        }
    }
}

#[cfg(all(test, has_lean_runtime))]
mod tests {
    use super::*;
    use crate::level_codec::{encode_level, encode_name};

    #[test]
    fn build_zero_roundtrip() {
        let lvl = build_level(&DecodedLevel::Zero);
        let mut out = Vec::new();
        encode_level(lvl.as_ref(), &mut out).unwrap();
        assert_eq!(out, vec![crate::level_codec::TAG_ZERO]);
    }

    #[test]
    fn build_anonymous_name_roundtrip() {
        let n = build_name(&DecodedName::Anonymous);
        let mut out = Vec::new();
        encode_name(n.as_ref(), &mut out).unwrap();
        assert_eq!(out, vec![crate::level_codec::NAME_ANONYMOUS]);
    }

    #[test]
    #[ignore = "build_level Succ path needs runtime init; see Level::succ_dispatch"]
    fn build_succ_roundtrip() {
        let d = DecodedLevel::Succ(Box::new(DecodedLevel::Zero));
        let lvl = build_level(&d);
        let mut out = Vec::new();
        encode_level(lvl.as_ref(), &mut out).unwrap();
        assert_eq!(
            out,
            vec![crate::level_codec::TAG_SUCC, crate::level_codec::TAG_ZERO]
        );
    }

    #[test]
    #[ignore = "needs runtime init; lean_mk_string_from_bytes allocates"]
    fn build_str_name_roundtrip() {
        let d = DecodedName::Str(Box::new(DecodedName::Anonymous), b"hello".to_vec());
        let n = build_name(&d);
        let mut out = Vec::new();
        encode_name(n.as_ref(), &mut out).unwrap();
        let mut expected = vec![
            crate::level_codec::NAME_STR,
            crate::level_codec::NAME_ANONYMOUS,
        ];
        expected.extend_from_slice(&5u32.to_le_bytes());
        expected.extend_from_slice(b"hello");
        assert_eq!(out, expected);
    }
}
