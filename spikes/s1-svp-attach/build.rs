//! Lets the spike link against a libmpv outside the default library path:
//! `MPV_LIB_DIR` holds `mpv.lib` (Windows) or `libmpv.so` (Linux).
fn main() {
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo::rustc-link-search=native={dir}");
        if std::env::var("CARGO_CFG_UNIX").is_ok() {
            println!("cargo::rustc-link-arg=-Wl,-rpath,{dir}");
        }
    }
}
