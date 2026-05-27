use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let primary_include = env::var("LEAN_INCLUDE_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let cmake = repo_root.join("build/release/stage0/include");
            cmake.join("lean/lean.h").exists().then_some(cmake)
        })
        .unwrap_or_else(|| repo_root.join("src/include"));

    println!("cargo:rerun-if-env-changed=LEAN_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
    println!(
        "cargo:rerun-if-changed={}/lean/lean.h",
        primary_include.display()
    );
    println!("cargo:rerun-if-changed=build.rs");

    let lean_h = primary_include.join("lean/lean.h");
    assert!(
        lean_h.exists(),
        "lean.h not found at {}; set LEAN_INCLUDE_DIR or check src/include/lean/lean.h",
        lean_h.display()
    );

    let stub_include = out_dir.join("stub-include");
    let mut clang_args: Vec<String> = vec![format!("-I{}", primary_include.display())];
    if !primary_include.join("lean/config.h").exists()
        || !primary_include.join("lean/version.h").exists()
    {
        let stub_lean = stub_include.join("lean");
        fs::create_dir_all(&stub_lean).expect("create stub include dir");
        if !primary_include.join("lean/version.h").exists() {
            fs::write(
                stub_lean.join("version.h"),
                "#pragma once\n\
                 #define LEAN_VERSION_MAJOR 4\n\
                 #define LEAN_VERSION_MINOR 31\n\
                 #define LEAN_VERSION_PATCH 0\n\
                 #define LEAN_VERSION_IS_RELEASE 0\n\
                 #define LEAN_SPECIAL_VERSION_DESC \"\"\n\
                 #define LEAN_VERSION_STRING \"4.31.0-pre\"\n\
                 #define LEAN_PLATFORM_TARGET \"\"\n\
                 #define LEAN_MANUAL_ROOT \"\"\n",
            )
            .expect("write stub version.h");
        }
        if !primary_include.join("lean/config.h").exists() {
            fs::write(
                stub_lean.join("config.h"),
                "#pragma once\n#include <lean/version.h>\n#define LEAN_IS_STAGE0 1\n",
            )
            .expect("write stub config.h");
        }
        clang_args.insert(0, format!("-I{}", stub_include.display()));
    }

    ensure_libclang();

    let extern_c = out_dir.join("extern.c");
    let mut builder = bindgen::Builder::default()
        .header(lean_h.to_string_lossy())
        .allowlist_function("lean_.*")
        .allowlist_type("lean_.*")
        .allowlist_var("LEAN_.*")
        .allowlist_var("lean_.*")
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .prepend_enum_name(false)
        .layout_tests(false)
        .generate_comments(false)
        .wrap_static_fns(true)
        .wrap_static_fns_path(&extern_c);
    for arg in &clang_args {
        builder = builder.clang_arg(arg);
    }
    let bindings = builder.generate().expect("bindgen failed for lean.h");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    let mut cc = cc::Build::new();
    cc.file(&extern_c)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-incompatible-pointer-types");
    for arg in &clang_args {
        if let Some(dir) = arg.strip_prefix("-I") {
            cc.include(dir);
        }
    }
    cc.compile("lean_inline_wrappers");

    println!("cargo:rustc-check-cfg=cfg(has_lean_runtime)");
    if let Some(lib_dir) = find_libleanshared(repo_root) {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=dylib=leanshared");
        for sibling in ["Init_shared", "leanshared_1", "leanshared_2"] {
            if lib_dir.join(format!("lib{sibling}.so")).exists() {
                println!("cargo:rustc-link-lib=dylib={sibling}");
            }
        }
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        println!("cargo:lean_lib_dir={}", lib_dir.display());
        println!("cargo:rustc-cfg=has_lean_runtime");
    }
}

fn find_libleanshared(repo_root: &Path) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed=LEAN_LIB_DIR");
    if let Ok(dir) = env::var("LEAN_LIB_DIR")
        && !dir.is_empty()
    {
        let p = PathBuf::from(&dir);
        if p.join("libleanshared.so").exists() {
            return Some(p);
        }
    }
    for candidate in [
        "build/release/stage1/lib/lean",
        "build/release/stage2/lib/lean",
    ] {
        let p = repo_root.join(candidate);
        if p.join("libleanshared.so").exists() {
            return Some(p);
        }
    }
    None
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
        let Some(rest) = line.strip_prefix("programs:") else {
            continue;
        };
        let rest = rest.trim_start().trim_start_matches('=');
        for path in rest.split(':') {
            let dir = Path::new(path.trim());
            if has_libclang(dir) {
                unsafe {
                    env::set_var("LIBCLANG_PATH", dir);
                }
                return;
            }
        }
    }
}

fn has_libclang(dir: &Path) -> bool {
    let Ok(entries) = dir.read_dir() else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name().to_str().is_some_and(|n| {
            n == "libclang.so" || n.starts_with("libclang.so.") || n.starts_with("libclang-")
        })
    })
}
