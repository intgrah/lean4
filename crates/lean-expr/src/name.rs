//! Rust mirror of Lean's `Name`.
//!
//! Lean source: `src/Init/Prelude.lean:4692` (the inductive) and
//! `src/Lean/Data/Name.lean` (most helpers). C-ABI layout:
//!
//! * `Name.anonymous` is the scalar `lean_box(0)` (tag 0, no allocation).
//! * `Name.str p s` is ctor tag 1, two object slots (parent: Name, sym:
//!   String), plus a cached `UInt64` hash as a computed_field.
//! * `Name.num p n` is ctor tag 2, two object slots (parent: Name, n:
//!   Nat), plus a cached `UInt64` hash.
//!
//! Construction goes through Lean's existing C-ABI exports
//! (`lean_name_mk_string`, `lean_name_mk_numeral`); equality and hash use
//! the lean.h externs already present in [`lean_runtime_sys`].

use lean_runtime_sys::{
    b_lean_obj_arg, lean_box, lean_ctor_get, lean_inc_ref, lean_name_eq, lean_name_hash,
    lean_obj_arg,
};

use crate::obj::{LeanObj, LeanObjRef};

// Hand-written externs for Lean-side Name constructors. These are
// `@[export]`ed from `src/Init/Prelude.lean` and so aren't in lean.h.
unsafe extern "C" {
    fn lean_name_mk_string(parent: lean_obj_arg, sym: lean_obj_arg) -> lean_obj_arg;
    fn lean_name_mk_numeral(parent: lean_obj_arg, idx: lean_obj_arg) -> lean_obj_arg;
}

/// Owned Name handle.
#[repr(transparent)]
#[derive(Debug)]
pub struct Name {
    obj: LeanObj,
}

impl Name {
    /// `Name.anonymous`.
    #[inline]
    pub fn anonymous() -> Self {
        unsafe {
            let ptr = lean_box(0);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_box(0) returned null"),
            }
        }
    }

    /// `Name.str parent sym`. Bumps the parent's RC; takes ownership of
    /// `sym` (a Lean `String` `lean_object*`).
    ///
    /// # Safety
    /// `sym` must be a valid owned `lean_object*` for a Lean `String`.
    pub unsafe fn str_unchecked(parent: LeanObjRef<'_>, sym: lean_obj_arg) -> Self {
        unsafe {
            lean_inc_ref(parent.as_ptr());
            let ptr = lean_name_mk_string(parent.as_ptr(), sym);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_name_mk_string returned null"),
            }
        }
    }

    /// `Name.num parent idx`. Bumps the parent's RC; takes ownership of
    /// `idx` (a Lean `Nat`).
    ///
    /// # Safety
    /// `idx` must be a valid owned `lean_object*` for a Lean `Nat`.
    pub unsafe fn num_unchecked(parent: LeanObjRef<'_>, idx: lean_obj_arg) -> Self {
        unsafe {
            lean_inc_ref(parent.as_ptr());
            let ptr = lean_name_mk_numeral(parent.as_ptr(), idx);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_name_mk_numeral returned null"),
            }
        }
    }

    /// Wrap an existing owning `lean_object*` that is known to point at a `Name`.
    ///
    /// # Safety
    /// `obj` must in fact be a `Name`.
    #[inline]
    pub unsafe fn from_obj(obj: LeanObj) -> Self {
        Self { obj }
    }

    /// Underlying owned handle (consume).
    #[inline]
    pub fn into_obj(self) -> LeanObj {
        self.obj
    }

    /// Borrowed view of the underlying object.
    #[inline]
    pub fn as_ref(&self) -> NameRef<'_> {
        NameRef {
            obj: self.obj.as_ref(),
        }
    }
}

impl Clone for Name {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
        }
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for Name {}

/// Borrowed view of a Name.
#[derive(Copy, Clone)]
pub struct NameRef<'a> {
    obj: LeanObjRef<'a>,
}

impl<'a> NameRef<'a> {
    /// Wrap a borrowed pointer known to point at a `Name`.
    ///
    /// # Safety
    /// `ptr` must be a valid `lean_object*` for a `Name` whose owner outlives `'a`.
    #[inline]
    pub unsafe fn from_borrowed(ptr: b_lean_obj_arg) -> Option<Self> {
        unsafe { LeanObjRef::from_borrowed(ptr).map(|obj| Self { obj }) }
    }

    /// Raw view.
    #[inline]
    pub fn obj(self) -> LeanObjRef<'a> {
        self.obj
    }

    /// Discriminate the variant by tag.
    pub fn kind(self) -> NameKind {
        if self.obj.is_scalar() {
            NameKind::Anonymous
        } else {
            match self.obj.tag() {
                1 => NameKind::Str,
                2 => NameKind::Num,
                t => unreachable!("Name with unexpected ctor tag {t}"),
            }
        }
    }

    /// The parent component of a `.str` or `.num` name, borrowed.
    ///
    /// Returns `None` for `.anonymous`.
    pub fn parent(self) -> Option<NameRef<'a>> {
        match self.kind() {
            NameKind::Anonymous => None,
            NameKind::Str | NameKind::Num => unsafe {
                NameRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
            },
        }
    }

    /// Cached hash via `lean_name_hash` from lean.h.
    pub fn hash(self) -> u64 {
        unsafe { lean_name_hash(self.obj.as_ptr()) }
    }
}

impl<'a> PartialEq for NameRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { lean_name_eq(self.obj.as_ptr(), other.obj.as_ptr()) != 0 }
    }
}

impl<'a> Eq for NameRef<'a> {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NameKind {
    Anonymous,
    Str,
    Num,
}

// Tests that actually instantiate a Name need libleanshared linkage
// (lean_dec_ref, lean_name_eq, ...). That linkage isn't wired in
// lean-runtime-sys yet, so runtime-touching tests live in a future
// integration-test crate or behind a feature gated by libleanshared
// availability.
