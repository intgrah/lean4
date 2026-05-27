#![allow(clippy::missing_safety_doc)]

mod name;
mod obj;

pub use name::{Name, NameKind, NameRef};
pub use obj::{LeanObj, LeanObjRef};
