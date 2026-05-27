//! Binary format for per-function trace corpora.
//!
//! Each ported function has a corpus file containing tuples of the form
//! `(input, state_in, output, state_out, message_delta)`. The Rust replay
//! runner reads these tuples and compares them against the output of the
//! Rust impl under test; the Lean-side instrumentation produces them by
//! wrapping `@[export]`ed entrypoints with `withCorpusCapture`.
//!
//! The payload of every field is the raw byte representation of a Lean
//! `lean_object*` graph as produced by `CompactedRegion.save`; this crate
//! is responsible only for the framing around those payloads.

pub mod format;
pub mod record;

pub use format::{FORMAT_VERSION, FieldTag, FileHeader, FormatError};
pub use record::{ReadError, Record, RecordReader, RecordWriter};
