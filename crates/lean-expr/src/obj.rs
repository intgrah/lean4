use core::marker::PhantomData;
use core::ptr::NonNull;

use lean_runtime_sys::{lean_dec, lean_inc, lean_object};

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
    #[inline]
    pub unsafe fn from_owned(ptr: *mut lean_object) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr })
    }

    #[inline]
    pub fn as_ref(&self) -> LeanObjRef<'_> {
        LeanObjRef {
            ptr: self.ptr,
            _life: PhantomData,
        }
    }

    #[inline]
    pub fn clone_borrow(other: LeanObjRef<'_>) -> Self {
        unsafe {
            lean_inc(other.ptr.as_ptr());
        }
        Self { ptr: other.ptr }
    }

    #[inline]
    pub fn into_raw(self) -> *mut lean_object {
        let ptr = self.ptr.as_ptr();
        core::mem::forget(self);
        ptr
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut lean_object {
        self.ptr.as_ptr()
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        unsafe { lean_runtime_sys::lean_is_scalar(self.as_ptr()) != 0 }
    }

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

#[derive(Copy, Clone)]
pub struct LeanObjRef<'a> {
    ptr: NonNull<lean_object>,
    _life: PhantomData<&'a lean_object>,
}

impl<'a> LeanObjRef<'a> {
    #[inline]
    pub unsafe fn from_borrowed(ptr: *mut lean_object) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            _life: PhantomData,
        })
    }

    #[inline]
    pub fn as_ptr(self) -> *mut lean_object {
        self.ptr.as_ptr()
    }

    #[inline]
    pub fn is_scalar(self) -> bool {
        unsafe { lean_runtime_sys::lean_is_scalar(self.as_ptr()) != 0 }
    }

    #[inline]
    pub fn tag(self) -> u32 {
        unsafe { lean_runtime_sys::lean_obj_tag(self.as_ptr()) }
    }
}
