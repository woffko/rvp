//! Builds the QML module, the cxx-qt bridges and the C++ video item, and
//! draws RVP's own art (crates/look): the icons and logo images become Qt
//! resources under `qrc:/qt/qml/Rvp/look/`, and on Windows the icon
//! resource is linked into `rvp.exe`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QmlModule};

/// Writes the files built from the art to `OUT_DIR/look` and a `.qrc` file
/// listing the images; returns the `.qrc` path.
fn build_look(out: &Path) -> PathBuf {
    println!("cargo::rerun-if-changed=../crates/look/art");
    let look = out.join("look");
    let mut qrc = String::from("<RCC>\n  <qresource prefix=\"/qt/qml/Rvp\">\n");
    let outputs = rvp_look::outputs::outputs().expect("draw RVP's art");
    for (name, bytes) in outputs {
        let path = look.join(&name);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("create look folder");
        std::fs::write(&path, bytes).expect("write built art");
        let extension = Path::new(&name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        if extension.as_deref() == Some("png") {
            let _ = writeln!(
                qrc,
                "    <file alias=\"look/{name}\">{}</file>",
                path.display()
            );
        }
        if extension.as_deref() == Some("res")
            && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        {
            // The exe's icon in Explorer and the taskbar (link.exe and
            // lld-link take a .res file as an input).
            println!("cargo::rustc-link-arg-bins={}", path.display());
        }
    }
    qrc.push_str("  </qresource>\n</RCC>\n");
    let qrc_path = out.join("look.qrc");
    std::fs::write(&qrc_path, qrc).expect("write look.qrc");
    qrc_path
}

fn main() {
    // cxx-qt-build 0.10 compiles the QML ahead of time (qmlcachegen) before
    // it writes this build's type information, so an incremental build used
    // the previous build's `plugin.qmltypes`: once a bridge gained a
    // method, the compiled bindings called methods by stale indexes and the
    // window crashed at start. Without the old file every build compiles the
    // QML the same way a clean build does.
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let _ = std::fs::remove_file(out.join("qt-build-utils/qml_modules/Rvp/plugin.qmltypes"));
    let look_qrc = build_look(&out);
    let module = QmlModule::new("Rvp")
        .qml_file("qml/Main.qml")
        .qml_file("qml/Menus.qml")
        .qml_file("qml/OpenDialog.qml")
        .qml_file("qml/Tip.qml")
        .qml_file("qml/AlbumName.qml")
        .qml_file("qml/FolderDialog.qml")
        .qml_file("qml/KeysAndMouse.qml")
        .qml_file("qml/Settings.qml")
        .qml_file("qml/UrlDialog.qml")
        .qml_file("qml/About.qml")
        .qml_file("qml/AudioSync.qml")
        .qml_file("qml/SubtitleSync.qml")
        .qml_file("qml/ControlPanel.qml")
        .qml_file("qml/PlaybackDialogs.qml")
        .qml_file("qml/VideoDialogs.qml")
        .qml_file("qml/SelfTestM12.qml")
        .qml_file("qml/Slideshow.qml")
        .qml_file("qml/SceneBrowser.qml")
        .qml_file("qml/SelfTestM11.qml")
        .qml_file("qml/SelfTestGpu.qml")
        .qml_file("qml/SelfTestSmoothing.qml");
    CxxQtBuilder::new_qml_module(module)
        .qt_module("Gui")
        .qt_module("Qml")
        .qt_module("Quick")
        .qt_module("OpenGL")
        .qt_module("Network")
        .files([
            "src/ffi.rs",
            "src/player_qobject.rs",
            "src/skin_view_qobject.rs",
            "src/thumb_view_qobject.rs",
        ])
        .cpp_file("cpp/shell.h")
        .cpp_file("cpp/shell.cpp")
        .cpp_file("cpp/video_window.cpp")
        .cpp_file("cpp/gpu_power.cpp")
        .qrc(&look_qrc)
        .build();

    // libmpv outside the default library path: `MPV_LIB_DIR` holds mpv.lib
    // (Windows) or libmpv.so (Linux).
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo::rustc-link-search=native={dir}");
    }
}
