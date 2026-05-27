use lean_runtime_sys::{
    b_lean_obj_arg, lean_box, lean_ctor_get, lean_inc_ref, lean_is_scalar, lean_name_eq,
    lean_name_hash, lean_obj_arg, lean_string_byte_size, lean_string_cstr, lean_unbox,
};

use crate::obj::{LeanObj, LeanObjRef};

unsafe extern "C" {
    fn lean_name_mk_string(parent: lean_obj_arg, sym: lean_obj_arg) -> lean_obj_arg;
    fn lean_name_mk_numeral(parent: lean_obj_arg, idx: lean_obj_arg) -> lean_obj_arg;
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Name {
    obj: LeanObj,
}

impl Name {
    #[inline]
    pub fn anonymous() -> Self {
        unsafe {
            let ptr = lean_box(0);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_box(0) returned null"),
            }
        }
    }

    pub unsafe fn str_unchecked(parent: LeanObjRef<'_>, sym: lean_obj_arg) -> Self {
        unsafe {
            lean_inc_ref(parent.as_ptr());
            let ptr = lean_name_mk_string(parent.as_ptr(), sym);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_name_mk_string returned null"),
            }
        }
    }

    pub unsafe fn num_unchecked(parent: LeanObjRef<'_>, idx: lean_obj_arg) -> Self {
        unsafe {
            lean_inc_ref(parent.as_ptr());
            let ptr = lean_name_mk_numeral(parent.as_ptr(), idx);
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_name_mk_numeral returned null"),
            }
        }
    }

    #[inline]
    pub unsafe fn from_obj(obj: LeanObj) -> Self {
        Self { obj }
    }

    #[inline]
    pub fn into_obj(self) -> LeanObj {
        self.obj
    }

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

#[derive(Copy, Clone)]
pub struct NameRef<'a> {
    obj: LeanObjRef<'a>,
}

impl<'a> NameRef<'a> {
    #[inline]
    pub unsafe fn from_borrowed(ptr: b_lean_obj_arg) -> Option<Self> {
        unsafe { LeanObjRef::from_borrowed(ptr).map(|obj| Self { obj }) }
    }

    #[inline]
    pub fn obj(self) -> LeanObjRef<'a> {
        self.obj
    }

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

    pub fn parent(self) -> Option<NameRef<'a>> {
        match self.kind() {
            NameKind::Anonymous => None,
            NameKind::Str | NameKind::Num => unsafe {
                NameRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
            },
        }
    }

    pub fn hash(self) -> u64 {
        unsafe { lean_name_hash(self.obj.as_ptr()) }
    }

    pub fn str_value(self) -> Option<&'a [u8]> {
        if self.kind() != NameKind::Str {
            return None;
        }
        unsafe {
            let s = lean_ctor_get(self.obj.as_ptr(), 1);
            let bytes = lean_string_byte_size(s);
            let len = bytes.saturating_sub(1);
            let ptr = lean_string_cstr(s) as *const u8;
            Some(core::slice::from_raw_parts(ptr, len))
        }
    }

    pub fn num_value(self) -> Option<u64> {
        if self.kind() != NameKind::Num {
            return None;
        }
        unsafe {
            let n = lean_ctor_get(self.obj.as_ptr(), 1);
            if lean_is_scalar(n) != 0 {
                Some(lean_unbox(n) as u64)
            } else {
                None
            }
        }
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

#[cfg(all(test, has_lean_runtime))]
mod tests {
    use super::*;

    #[test]
    fn anonymous_is_scalar() {
        let n = Name::anonymous();
        assert_eq!(n.as_ref().kind(), NameKind::Anonymous);
        assert!(n.as_ref().obj().is_scalar());
        assert_eq!(n, Name::anonymous());
        assert!(n.as_ref().parent().is_none());
    }
}
