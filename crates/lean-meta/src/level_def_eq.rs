use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use lean_corpus::{FORMAT_VERSION, FileHeader, Record, RecordWriter};
use lean_expr::LevelRef;
use lean_runtime_sys::{
    lean_alloc_ctor, lean_box, lean_ctor_get, lean_ctor_set, lean_dec, lean_dec_ref, lean_inc,
    lean_inc_ref, lean_io_result_mk_ok, lean_is_exclusive, lean_object, lean_st_ref_get,
    lean_st_ref_set,
};

use crate::level_build::build_level;
use crate::level_codec::{decoded_level_of_ref, encode_level};

unsafe extern "C" {
    fn __real_lean_is_level_def_eq(
        lhs: *mut lean_object,
        rhs: *mut lean_object,
        meta_ctx: *mut lean_object,
        meta_state: *mut lean_object,
        core_ctx: *mut lean_object,
        core_state: *mut lean_object,
    ) -> *mut lean_object;
    fn lean_st_ref_take(r: *mut lean_object) -> *mut lean_object;
    fn lean_instantiate_level_mvars(
        mctx: *mut lean_object,
        level: *mut lean_object,
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
    if riir_disabled() {
        return unsafe {
            __real_lean_is_level_def_eq(lhs, rhs, meta_ctx, meta_state, core_ctx, core_state)
        };
    }
    let u = unsafe { LevelRef::from_borrowed(lhs).expect("is_level_def_eq: null lhs") };
    let v = unsafe { LevelRef::from_borrowed(rhs).expect("is_level_def_eq: null rhs") };
    capture_inputs(u, v);
    let fall_through = || unsafe {
        __real_lean_is_level_def_eq(lhs, rhs, meta_ctx, meta_state, core_ctx, core_state)
    };
    match (u.view(), v.view()) {
        (lean_expr::LevelView::Succ(a), lean_expr::LevelView::Succ(b)) => unsafe {
            let a_ptr = a.obj().as_ptr();
            let b_ptr = b.obj().as_ptr();
            lean_inc(a_ptr);
            lean_inc(b_ptr);
            lean_dec(lhs);
            lean_dec(rhs);
            is_level_def_eq(a_ptr, b_ptr, meta_ctx, meta_state, core_ctx, core_state)
        },
        _ => {
            if u.get_level_offset().structurally_eq(v.get_level_offset()) {
                let eq = u.get_offset() == v.get_offset();
                unsafe {
                    lean_dec(lhs);
                    lean_dec(rhs);
                    lean_io_result_mk_ok(lean_box(eq as usize))
                }
            } else if u.has_mvar() || v.has_mvar() {
                let du = decoded_level_of_ref(u);
                let dv = decoded_level_of_ref(v);
                unsafe {
                    lean_inc(lhs);
                    let l1 = instantiate_level_mvars_in_state(lhs, meta_state);
                    lean_inc(rhs);
                    let l2 = instantiate_level_mvars_in_state(rhs, meta_state);
                    let n1 = decoded_level_of_ref(
                        LevelRef::from_borrowed(l1).expect("instantiate lhs returned null"),
                    )
                    .normalize();
                    let n2 = decoded_level_of_ref(
                        LevelRef::from_borrowed(l2).expect("instantiate rhs returned null"),
                    )
                    .normalize();
                    if du != n1 || dv != n2 {
                        let b1 = build_level(&n1);
                        let b2 = build_level(&n2);
                        lean_dec(lhs);
                        lean_dec(rhs);
                        lean_dec(l1);
                        lean_dec(l2);
                        is_level_def_eq(
                            b1.into_obj().into_raw(),
                            b2.into_obj().into_raw(),
                            meta_ctx,
                            meta_state,
                            core_ctx,
                            core_state,
                        )
                    } else {
                        lean_dec(l1);
                        lean_dec(l2);
                        fall_through()
                    }
                }
            } else {
                let un = decoded_level_of_ref(u).normalize();
                let vn = decoded_level_of_ref(v).normalize();
                if un == vn {
                    unsafe {
                        lean_dec(lhs);
                        lean_dec(rhs);
                        lean_io_result_mk_ok(lean_box(1))
                    }
                } else {
                    fall_through()
                }
            }
        }
    }
}

fn riir_disabled() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| std::env::var_os("LEAN_RIIR_DISABLE").is_some())
}

unsafe fn instantiate_level_mvars_in_state(
    level: *mut lean_object,
    meta_state: *mut lean_object,
) -> *mut lean_object {
    unsafe {
        let st_read = lean_st_ref_get(meta_state);
        let mctx = lean_ctor_get(st_read, 0);
        lean_inc_ref(mctx);
        lean_dec(st_read);
        let pair = lean_instantiate_level_mvars(mctx, level);
        let new_mctx = lean_ctor_get(pair, 0);
        lean_inc(new_mctx);
        let inst = lean_ctor_get(pair, 1);
        lean_inc(inst);
        lean_dec_ref(pair);
        let st = lean_st_ref_take(meta_state);
        let cache = lean_ctor_get(st, 1);
        let zeta_delta = lean_ctor_get(st, 2);
        let postponed = lean_ctor_get(st, 3);
        let diag = lean_ctor_get(st, 4);
        let new_state = if lean_is_exclusive(st) {
            let old_mctx = lean_ctor_get(st, 0);
            lean_dec(old_mctx);
            lean_ctor_set(st, 0, new_mctx);
            st
        } else {
            lean_inc(diag);
            lean_inc(postponed);
            lean_inc(zeta_delta);
            lean_inc(cache);
            lean_dec(st);
            let ns = lean_alloc_ctor(0, 5, 0);
            lean_ctor_set(ns, 0, new_mctx);
            lean_ctor_set(ns, 1, cache);
            lean_ctor_set(ns, 2, zeta_delta);
            lean_ctor_set(ns, 3, postponed);
            lean_ctor_set(ns, 4, diag);
            ns
        };
        let _ = lean_st_ref_set(meta_state, new_state);
        inst
    }
}

struct Capture {
    writer: Mutex<RecordWriter<BufWriter<File>>>,
}

static CAPTURE: OnceLock<Option<Capture>> = OnceLock::new();

fn capture() -> Option<&'static Capture> {
    CAPTURE.get_or_init(open_capture).as_ref()
}

fn open_capture() -> Option<Capture> {
    let dir = std::env::var("LEAN_RIIR_CAPTURE_DIR").ok()?;
    if dir.is_empty() {
        return None;
    }
    let mut path = PathBuf::from(dir);
    std::fs::create_dir_all(&path).ok()?;
    path.push("lean_is_level_def_eq.bin");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    let header = FileHeader {
        version: FORMAT_VERSION,
        flags: 0,
        pin_sha: [0u8; 20],
    };
    let needs_header = file.metadata().ok().map(|m| m.len() == 0).unwrap_or(true);
    let mut buf = BufWriter::new(file);
    if needs_header {
        use std::io::Write;
        buf.write_all(&header.encode()).ok()?;
    }
    Some(Capture {
        writer: Mutex::new(RecordWriter::append(buf)),
    })
}

fn capture_inputs(u: LevelRef<'_>, v: LevelRef<'_>) {
    let Some(cap) = capture() else { return };
    let mut input = Vec::new();
    if encode_level(u, &mut input).is_err() {
        return;
    }
    if encode_level(v, &mut input).is_err() {
        return;
    }
    let record = Record {
        input,
        state_in: Vec::new(),
        output: Vec::new(),
        state_out: Vec::new(),
        message_delta: Vec::new(),
    };
    if let Ok(mut w) = cap.writer.lock() {
        let _ = w.push(&record);
    }
}
