use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use lean_corpus::{FORMAT_VERSION, FileHeader, Record, RecordWriter};
use lean_expr::LevelRef;
use lean_runtime_sys::{lean_box, lean_dec, lean_inc, lean_io_result_mk_ok, lean_object};

use crate::level_codec::encode_level;

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
            } else {
                let mut su = String::new();
                let mut sv = String::new();
                unsafe {
                    diag_walk_level(lhs, &mut su);
                    diag_walk_level(rhs, &mut sv);
                }
                eprintln!("RIIR-DIAG mvar-free: u={su} v={sv}");
                fall_through()
            }
        }
    }
}

fn riir_disabled() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| std::env::var_os("LEAN_RIIR_DISABLE").is_some())
}

unsafe fn diag_walk_level(ptr: *mut lean_object, out: &mut String) {
    use lean_runtime_sys::{lean_ctor_get, lean_is_scalar, lean_obj_tag};
    if unsafe { lean_is_scalar(ptr) } != 0 {
        out.push('Z');
        return;
    }
    match unsafe { lean_obj_tag(ptr) } {
        1 => {
            out.push('S');
            unsafe { diag_walk_level(lean_ctor_get(ptr, 0), out) };
        }
        2 => {
            out.push_str("M(");
            unsafe { diag_walk_level(lean_ctor_get(ptr, 0), out) };
            out.push(',');
            unsafe { diag_walk_level(lean_ctor_get(ptr, 1), out) };
            out.push(')');
        }
        3 => {
            out.push_str("I(");
            unsafe { diag_walk_level(lean_ctor_get(ptr, 0), out) };
            out.push(',');
            unsafe { diag_walk_level(lean_ctor_get(ptr, 1), out) };
            out.push(')');
        }
        4 => out.push('P'),
        5 => out.push('V'),
        t => out.push_str(&format!("T{t}!")),
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
