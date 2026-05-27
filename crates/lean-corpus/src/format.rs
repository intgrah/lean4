pub const MAGIC: u32 = u32::from_le_bytes(*b"LCRP");
pub const RECORD_MARKER: u32 = u32::from_le_bytes(*b"REC1");
pub const FORMAT_VERSION: u16 = 1;
pub const HEADER_LEN: usize = 32;

#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FieldTag {
    Input = 1,
    StateIn = 2,
    Output = 3,
    StateOut = 4,
    MessageDelta = 5,
}

impl FieldTag {
    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            1 => Self::Input,
            2 => Self::StateIn,
            3 => Self::Output,
            4 => Self::StateOut,
            5 => Self::MessageDelta,
            _ => return None,
        })
    }
}

pub const FIELD_ORDER: [FieldTag; 5] = [
    FieldTag::Input,
    FieldTag::StateIn,
    FieldTag::Output,
    FieldTag::StateOut,
    FieldTag::MessageDelta,
];

#[derive(Debug, Eq, PartialEq)]
pub struct FileHeader {
    pub version: u16,
    pub flags: u16,
    pub pin_sha: [u8; 20],
}

impl FileHeader {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..28].copy_from_slice(&self.pin_sha);
        out
    }

    pub fn decode(bytes: &[u8; HEADER_LEN]) -> Result<Self, FormatError> {
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != MAGIC {
            return Err(FormatError::BadMagic { found: magic });
        }
        let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        if version != FORMAT_VERSION {
            return Err(FormatError::UnsupportedVersion { found: version });
        }
        let flags = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        let mut pin_sha = [0u8; 20];
        pin_sha.copy_from_slice(&bytes[8..28]);
        Ok(Self {
            version,
            flags,
            pin_sha,
        })
    }
}

#[derive(Debug)]
pub enum FormatError {
    BadMagic { found: u32 },
    UnsupportedVersion { found: u16 },
    BadRecordMarker { found: u32 },
    UnknownFieldTag { found: u32 },
    UnexpectedFieldOrder { expected: FieldTag, found: FieldTag },
    Truncated,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic { found } => write!(f, "bad file magic: {found:#x}"),
            Self::UnsupportedVersion { found } => {
                write!(f, "unsupported corpus format version: {found}")
            }
            Self::BadRecordMarker { found } => write!(f, "bad record marker: {found:#x}"),
            Self::UnknownFieldTag { found } => write!(f, "unknown field tag: {found}"),
            Self::UnexpectedFieldOrder { expected, found } => {
                write!(f, "expected field {expected:?}, found {found:?}")
            }
            Self::Truncated => write!(f, "corpus file truncated"),
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let h = FileHeader {
            version: FORMAT_VERSION,
            flags: 0,
            pin_sha: *b"abcdefghijklmnopqrst",
        };
        let encoded = h.encode();
        let decoded = FileHeader::decode(&encoded).unwrap();
        assert_eq!(h, decoded);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..4].copy_from_slice(b"XXXX");
        assert!(matches!(
            FileHeader::decode(&bytes),
            Err(FormatError::BadMagic { .. })
        ));
    }

    #[test]
    fn field_tag_roundtrip() {
        for tag in FIELD_ORDER {
            assert_eq!(FieldTag::from_u32(tag as u32), Some(tag));
        }
        assert_eq!(FieldTag::from_u32(99), None);
    }
}
