// The C++ side of the RVP shell (D3: a small shim next to cxx-qt):
//
// * MpvItem: a QQuickFramebufferObject whose renderer asks Rust (libmpv's
//   OpenGL render API) to draw into the item's framebuffer.
// * Tools: helpers for QML (the frame-synced chrome repaint, file URLs,
//   window grabs for the self-test).
// * The atlas cache and painting helpers the Rust SkinView draws with.
// * The single-instance channel (QLocalServer; crates/app/src/instance.rs).
#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>

#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QUrl>
#include <QtGui/QWindow>
#include <QtGui/QPainter>
#include <QtQuick/QQuickFramebufferObject>

#include "rust/cxx.h"

class MpvItem : public QQuickFramebufferObject
{
    Q_OBJECT
public:
    explicit MpvItem(QQuickItem *parent = nullptr);
    ~MpvItem() override;
    Renderer *createRenderer() const override;

    // Queued when mpv has a new frame: repaints the video and every chrome
    // item waiting for it in the same event-loop pass, so Qt presents them
    // together (goal C rule 1).
    Q_INVOKABLE void frameUpdate();

    // Repaints the waiting chrome items now (no video frame came in time).
    static void flushChrome();

    static std::atomic<MpvItem *> instance;
    static QList<QPointer<QQuickItem>> pendingChrome;
    static bool fallbackArmed;
};

class Tools : public QObject
{
    Q_OBJECT
    // True when mpv draws into the Direct3D 11 renderer's native window
    // (set at start, before any dialog loads).
    Q_PROPERTY(bool native_video READ native_video CONSTANT)
public:
    using QObject::QObject;
    // The QML singleton, for the native event filter's signals.
    static Tools *instance;
    bool native_video() const;
    // Repaints `item` now, or with the next video frame when `synced`.
    Q_INVOKABLE void repaint(QQuickItem *item, bool synced) const;
    // A local path for a file URL; other URLs as text.
    Q_INVOKABLE QString local_file(const QUrl &url) const;
    Q_INVOKABLE void log(const QString &line) const;
    // Writes a UTF-8 text file (the self-test's shader); returns its
    // absolute path, empty on failure.
    Q_INVOKABLE QString write_file(const QString &path, const QString &text) const;
    // The orientation check: grabs the window, finds the picture of a
    // video of `aspect` (width / height) inside `item` and compares a 4x4
    // grid of points with the reference image as it is, flipped, mirrored
    // and turned 180 degrees. JSON: the mean colour distance of each, and
    // `dark` when every point is black (a grab without video).
    Q_INVOKABLE QString compare_frame(QQuickItem *item, double aspect, const QString &reference) const;
    // Grabs the visible Quick windows to `directory/prefix-N.png`.
    Q_INVOKABLE int grab(const QString &directory, const QString &prefix) const;
    // Renders windows that are visible but not exposed (Windows session 0).
    Q_INVOKABLE int render_offscreen() const;
    Q_INVOKABLE QString environment_json() const;
    // The top-level windows (class, title, geometry, popup) for the input
    // check.
    Q_INVOKABLE QString windows_json() const;
    // Hands requests from other instances to `receiver`'s
    // remote_message(QString), the ones that came before it first.
    Q_INVOKABLE void instance_receiver(QObject *receiver) const;
    // Starts this program again (the self-test's second instance).
    Q_INVOKABLE bool run_self(const QStringList &arguments) const;
    // The clipboard's text, and putting text on it (M11).
    Q_INVOKABLE QString clipboard_text() const;
    // The self-test's view of captured images: an image file's format and
    // size, the clipboard's picture size, a folder's files (JSON).
    Q_INVOKABLE QString image_info(const QString &path) const;
    Q_INVOKABLE QString clipboard_image_info() const;
    Q_INVOKABLE QString list_files(const QString &folder) const;
    // A text file's contents from `from` bytes on (the self-test reads
    // mpv's log); empty when it cannot be read.
    Q_INVOKABLE QString read_file(const QString &path, qint64 from) const;
    Q_INVOKABLE void set_clipboard_text(const QString &text) const;
    // The screen `window` is on: its size in device pixels and in
    // millimetres (QScreen's physical size, which QML does not show).
    Q_INVOKABLE QString screen_json(QWindow *window) const;
    // A window's shape (M9: classic skins' region.txt): polygons as
    // "x,y x,y ...;..." in its pixels, scaled by `scale` (the skin's
    // zoom), united; empty: rectangular.
    Q_INVOKABLE void set_window_mask(QWindow *window, const QString &polygons, double scale) const;
    // A window's shape from rectangles "x,y,w,h;..." (M9: VLC's windows,
    // their drawn pixels), scaled; empty: rectangular.
    Q_INVOKABLE void set_window_mask_rects(QWindow *window, const QString &rects, double scale) const;
    // Shows a file in the system's file manager: selected in Explorer on
    // Windows, its folder elsewhere.
    Q_INVOKABLE bool show_file(const QString &path) const;
    // RVP's windows' font (M14's Dialog Box page): `family` (empty: Qt's)
    // at `points` (0: Qt's size).
    Q_INVOKABLE void set_app_font(const QString &family, int points) const;
    // A menu or action label in the interface's language (M14): its
    // numbers stand aside as %1, %2... so that "30 Sec Backward" and "5 Sec
    // Backward" share the "Menu" context's one translation; a label without
    // one stays.
    Q_INVOKABLE QString tr_menu(const QString &label) const;
    // Whether the current user's registry has a value (Windows; false
    // elsewhere): the self-test's look at the Run key (M14).
    Q_INVOKABLE bool registry_has_value(const QString &key, const QString &name) const;
    // D4's Direct3D 11 renderer (cpp/video_window.cpp): places mpv's
    // window. `place` is JSON in the main window's coordinates:
    // {"x", "y", "width", "height", "visible", "holes": [[x, y, w, h]...]}.
    Q_INVOKABLE void place_video_window(const QString &place) const;
    // The video window as Windows has it (JSON; `exists` false without
    // one), for the self-test.
    Q_INVOKABLE QString video_window_json() const;

signals:
    // A taskbar thumbnail button (M14, cpp/taskbar.cpp): 0 previous, 1
    // play/pause, 2 next.
    void taskbarButton(int button);
    // The user's session was locked (`true`) or unlocked (Windows).
    void sessionLocked(bool locked);
    // An Xbox controller's button (M14, cpp/xinput.cpp).
    void gamepadButton(int button);
    // A media key taken while another window has the keyboard (M14,
    // cpp/taskbar.cpp): 0 play/pause, 1 stop, 2 next, 3 previous.
    void mediaKey(int key);
};

// Called from Rust (declared in src/ffi.rs).
void rvp_setup_graphics();
void rvp_register_native_types();
bool rvp_has_windows();
bool rvp_instance_send(rust::Str name, rust::Str message);
bool rvp_instance_listen(rust::Str name);
bool rvp_instance_running(rust::Str name);
void rvp_set_c_numeric_locale();
void rvp_set_window_icon();
std::size_t rvp_main_window_id();
std::size_t rvp_video_window_create();
void rvp_video_window_close();
void rvp_confine_cursor(bool on);
void rvp_crash_test();
void rvp_set_env(rust::Str name, rust::Str value);
void rvp_taskbar_install();
void rvp_taskbar_start(bool enabled);
void rvp_global_media_keys(bool on);
void rvp_taskbar_state(std::int32_t state, double progress);
void rvp_taskbar_recent(rust::Str path);
bool rvp_run_at_startup(bool on, rust::Str command);
bool rvp_registry_set(rust::Str key, rust::Str name, rust::Str value);
bool rvp_registry_delete_value(rust::Str key, rust::Str name);
bool rvp_registry_delete_key(rust::Str key);
bool rvp_registry_has_key(rust::Str key);
void rvp_associations_changed();
rust::String rvp_tts_voices();
rust::String rvp_tts_outputs();
bool rvp_tts_speak(rust::Str text, rust::Str voiceName, rust::Str output, std::int32_t rate,
                   std::int32_t volume);
void rvp_xinput_start(bool enabled, bool focus_only, std::int32_t stick);
std::size_t rvp_gl_get_proc_address(rust::Str name);
void rvp_video_request_update();
rust::String rvp_gpu_power_read(bool nvidia);
void rvp_gpu_power_close();
bool rvp_atlas_contains(rust::Str name);
void rvp_atlas_remove_prefix(rust::Str prefix);
void rvp_atlas_insert(rust::Str name, rust::Slice<const std::uint8_t> rgba, std::int32_t width,
                      std::int32_t height);
bool rvp_save_image(rust::Slice<const std::uint8_t> rgba, std::int32_t width, std::int32_t height,
                    rust::Str path, std::int32_t quality, rust::Str texts);
bool rvp_copy_image(rust::Slice<const std::uint8_t> rgba, std::int32_t width, std::int32_t height);
rust::String rvp_local_time();
void rvp_paint_stretch(QPainter &painter, rust::Str name, std::array<std::int32_t, 4> src,
                       std::array<std::int32_t, 4> dst, std::array<std::int32_t, 4> clip);
void rvp_paint_atlas(QPainter &painter, rust::Str name, std::array<std::int32_t, 4> src,
                     std::array<std::int32_t, 4> dst, std::array<std::int32_t, 4> clip);
void rvp_paint_translate(QPainter &painter, std::int32_t dx, std::int32_t dy);
bool rvp_add_font(rust::Slice<const std::uint8_t> data);
void rvp_paint_scale(QPainter &painter, double factor);
void rvp_paint_fill(QPainter &painter, std::array<std::int32_t, 4> rect, std::uint32_t rgba);
void rvp_paint_text(QPainter &painter, rust::Str text, rust::Str trailing, rust::Str family,
                    std::int32_t pixel_size, bool bold, std::uint32_t rgba,
                    std::array<std::int32_t, 4> rect, std::array<std::int32_t, 4> clip,
                    std::int32_t align, bool elide);
