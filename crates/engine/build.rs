//! libmpv2-sys links `mpv` without a search path. `MPV_LIB_DIR` points the
//! linker at a libmpv that is not installed system-wide (the Linux kit's
//! build, a sysroot, or the Windows import library); unset, the system
//! library is used.

fn main() {
    println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    }
}
