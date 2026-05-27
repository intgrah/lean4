use std::env;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_lean_runtime)");
    if let Ok(lib_dir) = env::var("DEP_LEAN_RUNTIME_SYS_LEAN_LIB_DIR")
        && !lib_dir.is_empty()
    {
        println!("cargo:rustc-cfg=has_lean_runtime");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }
}
