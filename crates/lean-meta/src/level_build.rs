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
    use crate::level_codec::{decode_level, decode_name, encode_level, encode_name};

    fn init_runtime() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            unsafe extern "C" {
                fn lean_initialize_runtime_module();
            }
            lean_initialize_runtime_module();
            lean_runtime_sys::lean_io_mark_end_initialization();
        });
    }

    fn level_roundtrips(d: DecodedLevel) {
        init_runtime();
        let lvl = build_level(&d);
        let mut out = Vec::new();
        encode_level(lvl.as_ref(), &mut out).unwrap();
        assert_eq!(
            decode_level(&out).unwrap(),
            d,
            "level build/encode roundtrip"
        );
    }

    fn name_roundtrips(d: DecodedName) {
        init_runtime();
        let n = build_name(&d);
        let mut out = Vec::new();
        encode_name(n.as_ref(), &mut out).unwrap();
        assert_eq!(decode_name(&out).unwrap(), d, "name build/encode roundtrip");
    }

    fn zero() -> DecodedLevel {
        DecodedLevel::Zero
    }
    fn succ(u: DecodedLevel) -> DecodedLevel {
        DecodedLevel::Succ(Box::new(u))
    }
    fn str_name(s: &[u8]) -> DecodedName {
        DecodedName::Str(Box::new(DecodedName::Anonymous), s.to_vec())
    }
    fn param(s: &[u8]) -> DecodedLevel {
        DecodedLevel::Param(str_name(s))
    }

    #[test]
    fn build_zero_roundtrip() {
        level_roundtrips(zero());
    }

    #[test]
    fn build_succ_roundtrip() {
        level_roundtrips(succ(succ(zero())));
    }

    #[test]
    fn build_max_imax_roundtrip() {
        level_roundtrips(DecodedLevel::Max(
            Box::new(succ(param(b"u"))),
            Box::new(DecodedLevel::IMax(Box::new(param(b"v")), Box::new(zero()))),
        ));
    }

    #[test]
    fn build_param_roundtrip() {
        level_roundtrips(param(b"u"));
    }

    #[test]
    fn build_mvar_roundtrip() {
        level_roundtrips(DecodedLevel::MVar(DecodedName::Num(
            Box::new(str_name(b"_uniq")),
            42,
        )));
    }

    #[test]
    fn build_anonymous_name_roundtrip() {
        name_roundtrips(DecodedName::Anonymous);
    }

    #[test]
    fn build_str_name_roundtrip() {
        name_roundtrips(str_name(b"hello"));
    }

    #[test]
    fn build_nested_name_roundtrip() {
        name_roundtrips(DecodedName::Num(
            Box::new(DecodedName::Str(
                Box::new(str_name(b"Lean")),
                b"Meta".to_vec(),
            )),
            7,
        ));
    }
}
