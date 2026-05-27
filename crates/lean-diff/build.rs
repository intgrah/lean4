use std::env;

fn main() {
    if let Ok(lib_dir) = env::var("DEP_LEAN_RUNTIME_SYS_LEAN_LIB_DIR")
        && !lib_dir.is_empty()
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }
}
