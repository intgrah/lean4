use lean_expr::LevelRef;
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
    let _u = unsafe { LevelRef::from_borrowed(lhs) };
    let _v = unsafe { LevelRef::from_borrowed(rhs) };
    unsafe { __real_lean_is_level_def_eq(lhs, rhs, meta_ctx, meta_state, core_ctx, core_state) }
}
