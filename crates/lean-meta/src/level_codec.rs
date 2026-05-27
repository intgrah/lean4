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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DecodedLevel {
    Zero,
    Succ(Box<DecodedLevel>),
    Max(Box<DecodedLevel>, Box<DecodedLevel>),
    IMax(Box<DecodedLevel>, Box<DecodedLevel>),
    Param(DecodedName),
    MVar(DecodedName),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DecodedName {
    Anonymous,
    Str(Box<DecodedName>, Vec<u8>),
    Num(Box<DecodedName>, u64),
}

#[derive(Debug, Eq, PartialEq)]
pub enum EncodeError {
    NumNameTooLarge,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DecodeError {
    Truncated,
    UnknownLevelTag(u8),
    UnknownNameTag(u8),
    StrTooLarge(u32),
    TrailingBytes,
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

pub fn encode_decoded_level(u: &DecodedLevel, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    match u {
        DecodedLevel::Zero => {
            out.push(TAG_ZERO);
            Ok(())
        }
        DecodedLevel::Succ(c) => {
            out.push(TAG_SUCC);
            encode_decoded_level(c, out)
        }
        DecodedLevel::Max(a, b) => {
            out.push(TAG_MAX);
            encode_decoded_level(a, out)?;
            encode_decoded_level(b, out)
        }
        DecodedLevel::IMax(a, b) => {
            out.push(TAG_IMAX);
            encode_decoded_level(a, out)?;
            encode_decoded_level(b, out)
        }
        DecodedLevel::Param(n) => {
            out.push(TAG_PARAM);
            encode_decoded_name(n, out)
        }
        DecodedLevel::MVar(n) => {
            out.push(TAG_MVAR);
            encode_decoded_name(n, out)
        }
    }
}

pub fn encode_decoded_name(n: &DecodedName, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    match n {
        DecodedName::Anonymous => {
            out.push(NAME_ANONYMOUS);
            Ok(())
        }
        DecodedName::Str(parent, bytes) => {
            out.push(NAME_STR);
            encode_decoded_name(parent, out)?;
            let len = u32::try_from(bytes.len()).map_err(|_| EncodeError::NumNameTooLarge)?;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(bytes);
            Ok(())
        }
        DecodedName::Num(parent, v) => {
            out.push(NAME_NUM);
            encode_decoded_name(parent, out)?;
            out.extend_from_slice(&v.to_le_bytes());
            Ok(())
        }
    }
}

pub fn decode_level(bytes: &[u8]) -> Result<DecodedLevel, DecodeError> {
    let mut cur = bytes;
    let lvl = decode_level_inner(&mut cur)?;
    if !cur.is_empty() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(lvl)
}

pub fn decode_name(bytes: &[u8]) -> Result<DecodedName, DecodeError> {
    let mut cur = bytes;
    let n = decode_name_inner(&mut cur)?;
    if !cur.is_empty() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(n)
}

fn read_u8(cur: &mut &[u8]) -> Result<u8, DecodeError> {
    let (&b, rest) = cur.split_first().ok_or(DecodeError::Truncated)?;
    *cur = rest;
    Ok(b)
}

fn read_u32_le(cur: &mut &[u8]) -> Result<u32, DecodeError> {
    if cur.len() < 4 {
        return Err(DecodeError::Truncated);
    }
    let (head, rest) = cur.split_at(4);
    *cur = rest;
    Ok(u32::from_le_bytes(head.try_into().unwrap()))
}

fn read_u64_le(cur: &mut &[u8]) -> Result<u64, DecodeError> {
    if cur.len() < 8 {
        return Err(DecodeError::Truncated);
    }
    let (head, rest) = cur.split_at(8);
    *cur = rest;
    Ok(u64::from_le_bytes(head.try_into().unwrap()))
}

fn read_bytes<'a>(cur: &mut &'a [u8], len: usize) -> Result<&'a [u8], DecodeError> {
    if cur.len() < len {
        return Err(DecodeError::Truncated);
    }
    let (head, rest) = cur.split_at(len);
    *cur = rest;
    Ok(head)
}

fn decode_level_inner(cur: &mut &[u8]) -> Result<DecodedLevel, DecodeError> {
    match read_u8(cur)? {
        TAG_ZERO => Ok(DecodedLevel::Zero),
        TAG_SUCC => Ok(DecodedLevel::Succ(Box::new(decode_level_inner(cur)?))),
        TAG_MAX => {
            let a = decode_level_inner(cur)?;
            let b = decode_level_inner(cur)?;
            Ok(DecodedLevel::Max(Box::new(a), Box::new(b)))
        }
        TAG_IMAX => {
            let a = decode_level_inner(cur)?;
            let b = decode_level_inner(cur)?;
            Ok(DecodedLevel::IMax(Box::new(a), Box::new(b)))
        }
        TAG_PARAM => Ok(DecodedLevel::Param(decode_name_inner(cur)?)),
        TAG_MVAR => Ok(DecodedLevel::MVar(decode_name_inner(cur)?)),
        t => Err(DecodeError::UnknownLevelTag(t)),
    }
}

fn decode_name_inner(cur: &mut &[u8]) -> Result<DecodedName, DecodeError> {
    match read_u8(cur)? {
        NAME_ANONYMOUS => Ok(DecodedName::Anonymous),
        NAME_STR => {
            let parent = Box::new(decode_name_inner(cur)?);
            let len = read_u32_le(cur)?;
            let usize_len = len as usize;
            if usize_len as u32 != len {
                return Err(DecodeError::StrTooLarge(len));
            }
            let bytes = read_bytes(cur, usize_len)?.to_vec();
            Ok(DecodedName::Str(parent, bytes))
        }
        NAME_NUM => {
            let parent = Box::new(decode_name_inner(cur)?);
            let v = read_u64_le(cur)?;
            Ok(DecodedName::Num(parent, v))
        }
        t => Err(DecodeError::UnknownNameTag(t)),
    }
}

#[cfg(test)]
mod codec_tests {
    use super::*;

    fn roundtrip_level(lvl: &DecodedLevel) {
        let mut bytes = Vec::new();
        encode_decoded_level(lvl, &mut bytes).unwrap();
        assert_eq!(&decode_level(&bytes).unwrap(), lvl);
    }

    fn roundtrip_name(n: &DecodedName) {
        let mut bytes = Vec::new();
        encode_decoded_name(n, &mut bytes).unwrap();
        assert_eq!(&decode_name(&bytes).unwrap(), n);
    }

    #[test]
    fn zero_roundtrip() {
        roundtrip_level(&DecodedLevel::Zero);
    }

    #[test]
    fn succ_chain_roundtrip() {
        let l = (0..5).fold(DecodedLevel::Zero, |acc, _| {
            DecodedLevel::Succ(Box::new(acc))
        });
        roundtrip_level(&l);
    }

    #[test]
    fn max_imax_roundtrip() {
        let l = DecodedLevel::IMax(
            Box::new(DecodedLevel::Max(
                Box::new(DecodedLevel::Zero),
                Box::new(DecodedLevel::Succ(Box::new(DecodedLevel::Zero))),
            )),
            Box::new(DecodedLevel::Param(DecodedName::Str(
                Box::new(DecodedName::Anonymous),
                b"u".to_vec(),
            ))),
        );
        roundtrip_level(&l);
    }

    #[test]
    fn nested_name_roundtrip() {
        let n = DecodedName::Num(
            Box::new(DecodedName::Str(
                Box::new(DecodedName::Str(
                    Box::new(DecodedName::Anonymous),
                    b"Lean".to_vec(),
                )),
                b"Meta".to_vec(),
            )),
            42,
        );
        roundtrip_name(&n);
    }

    #[test]
    fn decode_rejects_truncated_succ() {
        let bytes = [TAG_SUCC];
        assert_eq!(decode_level(&bytes), Err(DecodeError::Truncated));
    }

    #[test]
    fn decode_rejects_unknown_level_tag() {
        let bytes = [99u8];
        assert_eq!(decode_level(&bytes), Err(DecodeError::UnknownLevelTag(99)));
    }

    #[test]
    fn decode_rejects_trailing_bytes() {
        let bytes = [TAG_ZERO, 0xff];
        assert_eq!(decode_level(&bytes), Err(DecodeError::TrailingBytes));
    }
}

#[cfg(all(test, has_lean_runtime))]
mod runtime_tests {
    use super::*;
    use lean_expr::{Level, Name};

    #[test]
    fn encode_level_zero() {
        let z = Level::zero();
        let mut out = Vec::new();
        encode_level(z.as_ref(), &mut out).unwrap();
        assert_eq!(out, vec![TAG_ZERO]);
    }

    #[test]
    fn encode_name_anonymous() {
        let n = Name::anonymous();
        let mut out = Vec::new();
        encode_name(n.as_ref(), &mut out).unwrap();
        assert_eq!(out, vec![NAME_ANONYMOUS]);
    }

    #[test]
    fn encode_decoded_match_lean_zero() {
        let z = Level::zero();
        let mut a = Vec::new();
        encode_level(z.as_ref(), &mut a).unwrap();
        let mut b = Vec::new();
        encode_decoded_level(&DecodedLevel::Zero, &mut b).unwrap();
        assert_eq!(a, b);
    }
}
