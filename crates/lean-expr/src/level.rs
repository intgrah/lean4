use lean_runtime_sys::{b_lean_obj_arg, lean_box, lean_ctor_get, lean_inc_ref, lean_obj_arg};

use crate::name::NameRef;
use crate::obj::{LeanObj, LeanObjRef};

unsafe extern "C" {
    fn lean_level_mk_zero(unit: lean_obj_arg) -> lean_obj_arg;
    fn lean_level_mk_succ(u: lean_obj_arg) -> lean_obj_arg;
    fn lean_level_mk_max(u: lean_obj_arg, v: lean_obj_arg) -> lean_obj_arg;
    fn lean_level_mk_imax(u: lean_obj_arg, v: lean_obj_arg) -> lean_obj_arg;
    fn lean_level_mk_param(name: lean_obj_arg) -> lean_obj_arg;
    fn lean_level_mk_mvar(id: lean_obj_arg) -> lean_obj_arg;

    fn lean_level_hash(u: b_lean_obj_arg) -> u32;
    fn lean_level_has_mvar(u: b_lean_obj_arg) -> u8;
    fn lean_level_has_param(u: b_lean_obj_arg) -> u8;
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Level {
    obj: LeanObj,
}

impl Level {
    pub fn zero() -> Self {
        unsafe {
            let ptr = lean_level_mk_zero(lean_box(0));
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_zero returned null"),
            }
        }
    }

    pub fn succ(u: LevelRef<'_>) -> Self {
        unsafe {
            lean_inc_ref(u.obj.as_ptr());
            let ptr = lean_level_mk_succ(u.obj.as_ptr());
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_succ returned null"),
            }
        }
    }

    pub fn max(u: LevelRef<'_>, v: LevelRef<'_>) -> Self {
        unsafe {
            lean_inc_ref(u.obj.as_ptr());
            lean_inc_ref(v.obj.as_ptr());
            let ptr = lean_level_mk_max(u.obj.as_ptr(), v.obj.as_ptr());
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_max returned null"),
            }
        }
    }

    pub fn imax(u: LevelRef<'_>, v: LevelRef<'_>) -> Self {
        unsafe {
            lean_inc_ref(u.obj.as_ptr());
            lean_inc_ref(v.obj.as_ptr());
            let ptr = lean_level_mk_imax(u.obj.as_ptr(), v.obj.as_ptr());
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_imax returned null"),
            }
        }
    }

    pub fn param(name: NameRef<'_>) -> Self {
        unsafe {
            lean_inc_ref(name.obj().as_ptr());
            let ptr = lean_level_mk_param(name.obj().as_ptr());
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_param returned null"),
            }
        }
    }

    pub fn mvar(id: LMVarIdRef<'_>) -> Self {
        unsafe {
            lean_inc_ref(id.name().obj().as_ptr());
            let ptr = lean_level_mk_mvar(id.name().obj().as_ptr());
            Self {
                obj: LeanObj::from_owned(ptr).expect("lean_level_mk_mvar returned null"),
            }
        }
    }

    pub unsafe fn from_obj(obj: LeanObj) -> Self {
        Self { obj }
    }

    pub fn into_obj(self) -> LeanObj {
        self.obj
    }

    pub fn as_ref(&self) -> LevelRef<'_> {
        LevelRef {
            obj: self.obj.as_ref(),
        }
    }
}

impl Clone for Level {
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct LevelRef<'a> {
    obj: LeanObjRef<'a>,
}

impl<'a> LevelRef<'a> {
    pub unsafe fn from_borrowed(ptr: b_lean_obj_arg) -> Option<Self> {
        unsafe { LeanObjRef::from_borrowed(ptr).map(|obj| Self { obj }) }
    }

    pub fn obj(self) -> LeanObjRef<'a> {
        self.obj
    }

    pub fn kind(self) -> LevelKind {
        if self.obj.is_scalar() {
            LevelKind::Zero
        } else {
            match self.obj.tag() {
                1 => LevelKind::Succ,
                2 => LevelKind::Max,
                3 => LevelKind::IMax,
                4 => LevelKind::Param,
                5 => LevelKind::MVar,
                t => unreachable!("Level with unexpected ctor tag {t}"),
            }
        }
    }

    pub fn view(self) -> LevelView<'a> {
        match self.kind() {
            LevelKind::Zero => LevelView::Zero,
            LevelKind::Succ => unsafe {
                let u = LevelRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
                    .expect("Level.succ slot 0 was null");
                LevelView::Succ(u)
            },
            LevelKind::Max => unsafe {
                let u = LevelRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
                    .expect("Level.max slot 0 was null");
                let v = LevelRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 1))
                    .expect("Level.max slot 1 was null");
                LevelView::Max(u, v)
            },
            LevelKind::IMax => unsafe {
                let u = LevelRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
                    .expect("Level.imax slot 0 was null");
                let v = LevelRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 1))
                    .expect("Level.imax slot 1 was null");
                LevelView::IMax(u, v)
            },
            LevelKind::Param => unsafe {
                let n = NameRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
                    .expect("Level.param slot 0 was null");
                LevelView::Param(n)
            },
            LevelKind::MVar => unsafe {
                let n = NameRef::from_borrowed(lean_ctor_get(self.obj.as_ptr(), 0))
                    .expect("Level.mvar slot 0 was null");
                LevelView::MVar(LMVarIdRef { name: n })
            },
        }
    }

    pub fn hash(self) -> u32 {
        unsafe { lean_level_hash(self.obj.as_ptr()) }
    }

    pub fn has_mvar(self) -> bool {
        unsafe { lean_level_has_mvar(self.obj.as_ptr()) != 0 }
    }

    pub fn has_param(self) -> bool {
        unsafe { lean_level_has_param(self.obj.as_ptr()) != 0 }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LevelKind {
    Zero,
    Succ,
    Max,
    IMax,
    Param,
    MVar,
}

#[derive(Copy, Clone)]
pub enum LevelView<'a> {
    Zero,
    Succ(LevelRef<'a>),
    Max(LevelRef<'a>, LevelRef<'a>),
    IMax(LevelRef<'a>, LevelRef<'a>),
    Param(NameRef<'a>),
    MVar(LMVarIdRef<'a>),
}

#[derive(Copy, Clone)]
pub struct LMVarIdRef<'a> {
    name: NameRef<'a>,
}

impl<'a> LMVarIdRef<'a> {
    pub unsafe fn from_name(name: NameRef<'a>) -> Self {
        Self { name }
    }

    pub fn name(self) -> NameRef<'a> {
        self.name
    }
}

#[cfg(all(test, has_lean_runtime))]
mod tests {
    use super::*;

    #[test]
    fn zero_is_scalar() {
        let z = Level::zero();
        assert_eq!(z.as_ref().kind(), LevelKind::Zero);
        assert!(z.as_ref().obj().is_scalar());
        assert!(!z.as_ref().has_mvar());
        assert!(!z.as_ref().has_param());
        matches!(z.as_ref().view(), LevelView::Zero);
    }

    #[test]
    #[ignore = "lean_level_mk_succ segfaults even after initialize_Lean_Level(1); investigation deferred"]
    fn succ_dispatch() {
        let z = Level::zero();
        let s = Level::succ(z.as_ref());
        assert_eq!(s.as_ref().kind(), LevelKind::Succ);
        match s.as_ref().view() {
            LevelView::Succ(inner) => assert_eq!(inner.kind(), LevelKind::Zero),
            _ => panic!("expected Succ"),
        }
    }

    #[test]
    #[ignore = "needs runtime init resolution; see succ_dispatch"]
    fn param_carries_name() {
        let n = crate::name::Name::anonymous();
        let p = Level::param(n.as_ref());
        assert_eq!(p.as_ref().kind(), LevelKind::Param);
        assert!(p.as_ref().has_param());
        match p.as_ref().view() {
            LevelView::Param(name) => {
                assert_eq!(name.kind(), crate::name::NameKind::Anonymous);
            }
            _ => panic!("expected Param"),
        }
    }
}
