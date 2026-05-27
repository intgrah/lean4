//! Owning and borrowing handles around `*mut lean_object`.
//!
//! Lean's runtime is refcounted: every `lean_object*` carries an `RC` that
//! must be `lean_inc`'d before sharing and `lean_dec`'d when ownership is
//! released. The two handles here encode that discipline at the Rust type
//! level:
//!
//! * [`LeanObj`] is an owning handle. `Drop` calls `lean_dec`. Constructed
//!   via [`LeanObj::from_owned`] from a `lean_obj_arg` (transfer of
//!   ownership) or [`LeanObj::clone_borrow`] from a borrowed handle (which
//!   calls `lean_inc`).
//! * [`LeanObjRef`] is a borrowed view. No `Drop`. Construct via
//!   [`LeanObj::as_ref`] or [`LeanObjRef::from_borrowed`] (a raw
//!   `b_lean_obj_arg`). Cheap to copy; lifetime-tied to the underlying
//!   owner so the RC can't drop to zero while the ref is live.

use core::marker::PhantomData;
use core::ptr::NonNull;

use lean_runtime_sys::{lean_dec, lean_inc, lean_object};

/// Owned handle. Drops via `lean_dec`.
#[repr(transparent)]
pub struct LeanObj {
    ptr: NonNull<lean_object>,
}

impl core::fmt::Debug for LeanObj {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LeanObj({:p})", self.ptr.as_ptr())
    }
}

impl LeanObj {
    /// Wrap a `lean_obj_arg` (i.e., a `*mut lean_object` whose RC the caller
    /// has transferred to us).
    ///
    /// # Safety
    /// `ptr` must be a valid `lean_object*` and the caller must be
    /// transferring ownership of one RC count.
    #[inline]
    pub unsafe fn from_owned(ptr: *mut lean_object) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr })
    }

    /// Borrow a handle to the underlying object without affecting the RC.
    #[inline]
    pub fn as_ref(&self) -> LeanObjRef<'_> {
        LeanObjRef {
            ptr: self.ptr,
            _life: PhantomData,
        }
    }

    /// Bump the RC and produce an owned clone.
    #[inline]
    pub fn clone_borrow(other: LeanObjRef<'_>) -> Self {
        unsafe {
            lean_inc(other.ptr.as_ptr());
        }
        Self { ptr: other.ptr }
    }

    /// Release ownership without decrementing the RC. The caller becomes
    /// responsible for the count.
    #[inline]
    pub fn into_raw(self) -> *mut lean_object {
        let ptr = self.ptr.as_ptr();
        core::mem::forget(self);
        ptr
    }

    /// Raw pointer view; the RC is unaffected.
    #[inline]
    pub fn as_ptr(&self) -> *mut lean_object {
        self.ptr.as_ptr()
    }

    /// True iff this object is a small-int scalar (Lean uses tagged
    /// pointers: an `o` with the low bit set is the integer `o >> 1`).
    /// Nullary inductive constructors are encoded as scalars whose value
    /// is the constructor index, so this is how zero-argument variants
    /// are detected.
    #[inline]
    pub fn is_scalar(&self) -> bool {
        unsafe { lean_runtime_sys::lean_is_scalar(self.as_ptr()) != 0 }
    }

    /// Constructor tag for boxed objects, or the scalar value for tagged-
    /// pointer scalars. Callers must inspect [`Self::is_scalar`] first if
    /// the distinction matters.
    #[inline]
    pub fn tag(&self) -> u32 {
        unsafe { lean_runtime_sys::lean_obj_tag(self.as_ptr()) }
    }
}

impl Drop for LeanObj {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            lean_dec(self.ptr.as_ptr());
        }
    }
}

impl Clone for LeanObj {
    #[inline]
    fn clone(&self) -> Self {
        Self::clone_borrow(self.as_ref())
    }
}

/// Borrowed handle. No `Drop`; lifetime-tied to an owner so the RC can't
/// drop to zero while the ref is live.
#[derive(Copy, Clone)]
pub struct LeanObjRef<'a> {
    ptr: NonNull<lean_object>,
    _life: PhantomData<&'a lean_object>,
}

impl<'a> LeanObjRef<'a> {
    /// Wrap a `b_lean_obj_arg` (borrowed pointer; no RC transfer).
    ///
    /// # Safety
    /// `ptr` must be a valid `lean_object*` whose owner outlives `'a`.
    #[inline]
    pub unsafe fn from_borrowed(ptr: *mut lean_object) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _life: PhantomData,
        })
    }

    /// Raw pointer view; the RC is unaffected.
    #[inline]
    pub fn as_ptr(self) -> *mut lean_object {
        self.ptr.as_ptr()
    }

    /// See [`LeanObj::is_scalar`].
    #[inline]
    pub fn is_scalar(self) -> bool {
        unsafe { lean_runtime_sys::lean_is_scalar(self.as_ptr()) != 0 }
    }

    /// See [`LeanObj::tag`].
    #[inline]
    pub fn tag(self) -> u32 {
        unsafe { lean_runtime_sys::lean_obj_tag(self.as_ptr()) }
    }
}
