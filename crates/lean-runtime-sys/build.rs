use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();

    let include_dir = env::var("LEAN_INCLUDE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("build/release/stage0/include"));

    println!("cargo:rerun-if-env-changed=LEAN_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
    println!("cargo:rerun-if-changed={}/lean/lean.h", include_dir.display());
    println!("cargo:rerun-if-changed={}/lean/config.h", include_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let lean_h = include_dir.join("lean/lean.h");
    assert!(
        lean_h.exists(),
        "lean.h not found at {}; set LEAN_INCLUDE_DIR or run CMake configure to populate build/release/stage0/include",
        lean_h.display()
    );

    ensure_libclang();

    let bindings = bindgen::Builder::default()
        .header(lean_h.to_string_lossy())
        .clang_arg(format!("-I{}", include_dir.display()))
        .clang_arg("-DLEAN_MIMALLOC")
        .allowlist_function("lean_.*")
        .allowlist_type("lean_.*")
        .allowlist_var("LEAN_.*")
        .allowlist_var("lean_.*")
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .prepend_enum_name(false)
        .layout_tests(false)
        .generate_comments(false)
        .generate()
        .expect("bindgen failed for lean.h");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings
        .write_to_file(&out_path)
        .expect("failed to write bindings.rs");
}

fn ensure_libclang() {
    if env::var_os("LIBCLANG_PATH").is_some() {
        return;
    }
    let Ok(out) = Command::new("clang").arg("--print-search-dirs").output() else {
        return;
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("programs:") else { continue };
        let rest = rest.trim_start().trim_start_matches('=');
        for path in rest.split(':') {
            let dir = Path::new(path.trim());
            if has_libclang(dir) {
                unsafe { env::set_var("LIBCLANG_PATH", dir); }
                return;
            }
        }
    }
}

fn has_libclang(dir: &Path) -> bool {
    let Ok(entries) = dir.read_dir() else { return false };
    entries.flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| n == "libclang.so" || n.starts_with("libclang.so.") || n.starts_with("libclang-"))
    })
}
