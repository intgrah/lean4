//! Rust mirrors of Lean's core data types (Name, Level, Expr, LocalContext,
//! MetavarContext, ...) as thin wrappers over `lean_object*`.
//!
//! The wrappers preserve Lean's in-memory layout exactly so that values can
//! cross the FFI boundary without conversion: a Rust [`Level`] is just an
//! owned `*mut lean_object` whose tag and slots are inspected via
//! [`lean_runtime_sys`] accessors. This is the right shape for Pass-A
//! ports, where the Rust code is interoperating with compiled Lean code
//! that already speaks this representation.

mod obj;

pub use obj::{LeanObj, LeanObjRef};
