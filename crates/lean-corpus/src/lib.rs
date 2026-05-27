#![allow(clippy::missing_safety_doc)]

pub mod compact;
pub mod format;
pub mod record;

pub use compact::{LeanCompactedRegion, Region};
pub use format::{FORMAT_VERSION, FieldTag, FileHeader, FormatError};
pub use record::{ReadError, Record, RecordReader, RecordWriter};
