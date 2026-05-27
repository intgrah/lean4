use lean_expr::{LevelRef, LevelView, NameKind, NameRef};

pub const TAG_ZERO: u8 = 0;
pub const TAG_SUCC: u8 = 1;
pub const TAG_MAX: u8 = 2;
pub const TAG_IMAX: u8 = 3;
pub const TAG_PARAM: u8 = 4;
pub const TAG_MVAR: u8 = 5;

pub const NAME_ANONYMOUS: u8 = 0;
pub const NAME_STR: u8 = 1;
pub const NAME_NUM: u8 = 2;

#[derive(Debug, Eq, PartialEq)]
pub enum EncodeError {
    NumNameTooLarge,
}

pub fn encode_level(u: LevelRef<'_>, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    match u.view() {
        LevelView::Zero => {
            out.push(TAG_ZERO);
            Ok(())
        }
        LevelView::Succ(c) => {
            out.push(TAG_SUCC);
            encode_level(c, out)
        }
        LevelView::Max(a, b) => {
            out.push(TAG_MAX);
            encode_level(a, out)?;
            encode_level(b, out)
        }
        LevelView::IMax(a, b) => {
            out.push(TAG_IMAX);
            encode_level(a, out)?;
            encode_level(b, out)
        }
        LevelView::Param(n) => {
            out.push(TAG_PARAM);
            encode_name(n, out)
        }
        LevelView::MVar(id) => {
            out.push(TAG_MVAR);
            encode_name(id.name(), out)
        }
    }
}

pub fn encode_name(n: NameRef<'_>, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    match n.kind() {
        NameKind::Anonymous => {
            out.push(NAME_ANONYMOUS);
            Ok(())
        }
        NameKind::Str => {
            out.push(NAME_STR);
            let parent = n.parent().expect("Str variant has a parent");
            encode_name(parent, out)?;
            let bytes = n.str_value().expect("Str variant has a value");
            let len = u32::try_from(bytes.len()).map_err(|_| EncodeError::NumNameTooLarge)?;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(bytes);
            Ok(())
        }
        NameKind::Num => {
            out.push(NAME_NUM);
            let parent = n.parent().expect("Num variant has a parent");
            encode_name(parent, out)?;
            let v = n.num_value().ok_or(EncodeError::NumNameTooLarge)?;
            out.extend_from_slice(&v.to_le_bytes());
            Ok(())
        }
    }
}

#[cfg(all(test, has_lean_runtime))]
mod tests {
    use super::*;
    use lean_expr::Level;

    #[test]
    fn encode_level_zero() {
        let z = Level::zero();
        let mut out = Vec::new();
        encode_level(z.as_ref(), &mut out).unwrap();
        assert_eq!(out, vec![TAG_ZERO]);
    }
}
