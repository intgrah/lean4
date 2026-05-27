use std::io::{self, Read, Write};

use crate::format::{FIELD_ORDER, FieldTag, FileHeader, FormatError, HEADER_LEN, RECORD_MARKER};

#[derive(Debug, Eq, PartialEq, Default)]
pub struct Record {
    pub input: Vec<u8>,
    pub state_in: Vec<u8>,
    pub output: Vec<u8>,
    pub state_out: Vec<u8>,
    pub message_delta: Vec<u8>,
}

impl Record {
    fn field(&self, tag: FieldTag) -> &[u8] {
        match tag {
            FieldTag::Input => &self.input,
            FieldTag::StateIn => &self.state_in,
            FieldTag::Output => &self.output,
            FieldTag::StateOut => &self.state_out,
            FieldTag::MessageDelta => &self.message_delta,
        }
    }

    fn field_mut(&mut self, tag: FieldTag) -> &mut Vec<u8> {
        match tag {
            FieldTag::Input => &mut self.input,
            FieldTag::StateIn => &mut self.state_in,
            FieldTag::Output => &mut self.output,
            FieldTag::StateOut => &mut self.state_out,
            FieldTag::MessageDelta => &mut self.message_delta,
        }
    }
}

pub struct RecordWriter<W: Write> {
    inner: W,
}

impl<W: Write> RecordWriter<W> {
    pub fn create(mut inner: W, header: &FileHeader) -> io::Result<Self> {
        inner.write_all(&header.encode())?;
        Ok(Self { inner })
    }

    pub fn append(inner: W) -> Self {
        Self { inner }
    }

    pub fn push(&mut self, record: &Record) -> io::Result<()> {
        let total_len: u64 = FIELD_ORDER
            .iter()
            .map(|t| 4 + 8 + record.field(*t).len() as u64)
            .sum();
        self.inner.write_all(&RECORD_MARKER.to_le_bytes())?;
        self.inner.write_all(&total_len.to_le_bytes())?;
        for tag in FIELD_ORDER {
            let payload = record.field(tag);
            self.inner.write_all(&(tag as u32).to_le_bytes())?;
            self.inner
                .write_all(&(payload.len() as u64).to_le_bytes())?;
            self.inner.write_all(payload)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<W> {
        self.inner.flush()?;
        Ok(self.inner)
    }
}

pub struct RecordReader<R: Read> {
    inner: R,
    pub header: FileHeader,
}

impl<R: Read> RecordReader<R> {
    pub fn open(mut inner: R) -> Result<Self, ReadError> {
        let mut header_bytes = [0u8; HEADER_LEN];
        inner.read_exact(&mut header_bytes).map_err(ReadError::io)?;
        let header = FileHeader::decode(&header_bytes).map_err(ReadError::Format)?;
        Ok(Self { inner, header })
    }
}

impl<R: Read> Iterator for RecordReader<R> {
    type Item = Result<Record, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut marker_bytes = [0u8; 4];
        match self.inner.read_exact(&mut marker_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return None,
            Err(e) => return Some(Err(ReadError::Io(e))),
        }
        let marker = u32::from_le_bytes(marker_bytes);
        if marker != RECORD_MARKER {
            return Some(Err(ReadError::Format(FormatError::BadRecordMarker {
                found: marker,
            })));
        }
        let mut total_len_bytes = [0u8; 8];
        if let Err(e) = self.inner.read_exact(&mut total_len_bytes) {
            return Some(Err(ReadError::Io(e)));
        }
        let _total_len = u64::from_le_bytes(total_len_bytes);
        let mut record = Record::default();
        for expected in FIELD_ORDER {
            let mut tag_bytes = [0u8; 4];
            if let Err(e) = self.inner.read_exact(&mut tag_bytes) {
                return Some(Err(ReadError::Io(e)));
            }
            let tag_raw = u32::from_le_bytes(tag_bytes);
            let tag = match FieldTag::from_u32(tag_raw) {
                Some(t) => t,
                None => {
                    return Some(Err(ReadError::Format(FormatError::UnknownFieldTag {
                        found: tag_raw,
                    })));
                }
            };
            if tag != expected {
                return Some(Err(ReadError::Format(FormatError::UnexpectedFieldOrder {
                    expected,
                    found: tag,
                })));
            }
            let mut len_bytes = [0u8; 8];
            if let Err(e) = self.inner.read_exact(&mut len_bytes) {
                return Some(Err(ReadError::Io(e)));
            }
            let len = u64::from_le_bytes(len_bytes) as usize;
            let mut payload = vec![0u8; len];
            if let Err(e) = self.inner.read_exact(&mut payload) {
                return Some(Err(ReadError::Io(e)));
            }
            *record.field_mut(tag) = payload;
        }
        Some(Ok(record))
    }
}

#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    Format(FormatError),
}

impl ReadError {
    fn io(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "corpus io error: {e}"),
            Self::Format(e) => write!(f, "corpus format error: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FORMAT_VERSION;

    fn sample_header() -> FileHeader {
        FileHeader {
            version: FORMAT_VERSION,
            flags: 0,
            pin_sha: *b"abcdefghijklmnopqrst",
        }
    }

    #[test]
    fn empty_file_roundtrip() {
        let mut buf = Vec::new();
        let w = RecordWriter::create(&mut buf, &sample_header()).unwrap();
        w.finish().unwrap();
        let r = RecordReader::open(&buf[..]).unwrap();
        assert_eq!(r.header, sample_header());
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn single_record_roundtrip() {
        let rec = Record {
            input: b"in".to_vec(),
            state_in: b"sin".to_vec(),
            output: b"out".to_vec(),
            state_out: b"sout".to_vec(),
            message_delta: b"".to_vec(),
        };
        let mut buf = Vec::new();
        let mut w = RecordWriter::create(&mut buf, &sample_header()).unwrap();
        w.push(&rec).unwrap();
        w.finish().unwrap();
        let mut r = RecordReader::open(&buf[..]).unwrap();
        let got = r.next().unwrap().unwrap();
        assert_eq!(got, rec);
        assert!(r.next().is_none());
    }

    #[test]
    fn many_records_roundtrip() {
        let recs: Vec<Record> = (0..32u32)
            .map(|i| Record {
                input: i.to_le_bytes().to_vec(),
                state_in: vec![i as u8; 7],
                output: vec![],
                state_out: vec![(i ^ 0xff) as u8; 3],
                message_delta: vec![1, 2, 3],
            })
            .collect();
        let mut buf = Vec::new();
        let mut w = RecordWriter::create(&mut buf, &sample_header()).unwrap();
        for r in &recs {
            w.push(r).unwrap();
        }
        w.finish().unwrap();
        let reader = RecordReader::open(&buf[..]).unwrap();
        let read_back: Vec<Record> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(read_back, recs);
    }

    #[test]
    fn file_roundtrip_buffered_io() {
        let recs: Vec<Record> = (0..16u32)
            .map(|i| Record {
                input: i.to_le_bytes().to_vec(),
                state_in: vec![i as u8; 13],
                output: vec![(i * 3) as u8; 5],
                state_out: vec![(i ^ 0xa5) as u8; 9],
                message_delta: if i % 4 == 0 { vec![0x55; 3] } else { vec![] },
            })
            .collect();

        let mut path = std::env::temp_dir();
        path.push(format!("lean-corpus-roundtrip-{}.bin", std::process::id()));
        let _cleanup = scopeguard(&path);

        {
            let file = std::fs::File::create(&path).unwrap();
            let mut w =
                RecordWriter::create(std::io::BufWriter::new(file), &sample_header()).unwrap();
            for r in &recs {
                w.push(r).unwrap();
            }
            w.finish().unwrap();
        }

        let file = std::fs::File::open(&path).unwrap();
        let reader = RecordReader::open(std::io::BufReader::new(file)).unwrap();
        assert_eq!(reader.header, sample_header());
        let got: Vec<Record> = reader.map(|r| r.unwrap()).collect();
        assert_eq!(got, recs);
    }

    struct Cleanup<'a>(&'a std::path::Path);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.0);
        }
    }
    fn scopeguard(p: &std::path::Path) -> Cleanup<'_> {
        Cleanup(p)
    }

    #[test]
    fn rejects_bad_marker() {
        let mut buf = Vec::new();
        let mut w = RecordWriter::create(&mut buf, &sample_header()).unwrap();
        w.push(&Record::default()).unwrap();
        w.finish().unwrap();
        buf[HEADER_LEN] ^= 0xff;
        let mut r = RecordReader::open(&buf[..]).unwrap();
        match r.next().unwrap() {
            Err(ReadError::Format(FormatError::BadRecordMarker { .. })) => {}
            other => panic!("expected BadRecordMarker, got {other:?}"),
        }
    }
}
