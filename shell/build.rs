//! Builds the QML module, the cxx-qt bridges and the C++ video item.

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // cxx-qt-build 0.10 compiles the QML ahead of time (qmlcachegen) before
    // it writes this build's type information, so an incremental build used
    // the previous build's `plugin.qmltypes`: once a bridge gained a
    // method, the compiled bindings called methods by stale indexes and the
    // window crashed at start. Without the old file every build compiles the
    // QML the same way a clean build does.
    if let Some(out) = std::env::var_os("OUT_DIR") {
        let stale =
            std::path::Path::new(&out).join("qt-build-utils/qml_modules/Rvp/plugin.qmltypes");
        let _ = std::fs::remove_file(stale);
    }
    let module = QmlModule::new("Rvp")
        .qml_file("qml/Main.qml")
        .qml_file("qml/ContextMenu.qml")
        .qml_file("qml/OpenDialog.qml")
        .qml_file("qml/Tip.qml");
    CxxQtBuilder::new_qml_module(module)
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .qt_module("OpenGL")
        .files([
            "src/ffi.rs",
            "src/player_qobject.rs",
            "src/skin_view_qobject.rs",
        ])
        .cpp_file("cpp/shell.h")
        .cpp_file("cpp/shell.cpp")
        .build();

    // libmpv outside the default library path: `MPV_LIB_DIR` holds mpv.lib
    // (Windows) or libmpv.so (Linux).
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo::rustc-link-search=native={dir}");
    }
}
