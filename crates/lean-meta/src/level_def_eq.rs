use lean_expr::{LevelRef, LevelView};
use lean_runtime_sys::lean_object;

unsafe extern "C" {
    fn __real_lean_is_level_def_eq(
        lhs: *mut lean_object,
        rhs: *mut lean_object,
        meta_ctx: *mut lean_object,
        meta_state: *mut lean_object,
        core_ctx: *mut lean_object,
        core_state: *mut lean_object,
    ) -> *mut lean_object;
}

pub unsafe fn is_level_def_eq(
    lhs: *mut lean_object,
    rhs: *mut lean_object,
    meta_ctx: *mut lean_object,
    meta_state: *mut lean_object,
    core_ctx: *mut lean_object,
    core_state: *mut lean_object,
) -> *mut lean_object {
    let u = unsafe { LevelRef::from_borrowed(lhs).expect("is_level_def_eq: null lhs") };
    let v = unsafe { LevelRef::from_borrowed(rhs).expect("is_level_def_eq: null rhs") };
    let fall_through = || unsafe {
        __real_lean_is_level_def_eq(lhs, rhs, meta_ctx, meta_state, core_ctx, core_state)
    };
    match (u.view(), v.view()) {
        (LevelView::Zero, LevelView::Zero) => fall_through(),
        (LevelView::Succ(_), LevelView::Succ(_)) => fall_through(),
        (LevelView::Max(_, _), LevelView::Max(_, _)) => fall_through(),
        (LevelView::IMax(_, _), LevelView::IMax(_, _)) => fall_through(),
        (LevelView::Param(_), LevelView::Param(_)) => fall_through(),
        (LevelView::MVar(_), LevelView::MVar(_)) => fall_through(),
        _ => fall_through(),
    }
}
