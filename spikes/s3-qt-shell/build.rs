use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("RvpSpike").qml_file("qml/Main.qml"))
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .qt_module("OpenGL")
        .files(["src/ffi.rs", "src/skin_item.rs", "src/mpv.rs"])
        .cpp_file("cpp/spike.h")
        .cpp_file("cpp/spike.cpp")
        .build();

    // libmpv outside the default library path: `MPV_LIB_DIR` holds mpv.lib
    // (Windows) or libmpv.so (Linux).
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo::rustc-link-search=native={dir}");
        if std::env::var("CARGO_CFG_UNIX").is_ok() {
            println!("cargo::rustc-link-arg=-Wl,-rpath,{dir}");
        }
    }
}
