#![allow(clippy::missing_safety_doc)]

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __wrap_lean_is_level_def_eq(
    lhs: *mut lean_object,
    rhs: *mut lean_object,
    meta_ctx: *mut lean_object,
    meta_state: *mut lean_object,
    core_ctx: *mut lean_object,
    core_state: *mut lean_object,
) -> *mut lean_object {
    unsafe { __real_lean_is_level_def_eq(lhs, rhs, meta_ctx, meta_state, core_ctx, core_state) }
}
