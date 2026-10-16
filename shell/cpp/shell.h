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
public:
    using QObject::QObject;
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
};

// Called from Rust (declared in src/ffi.rs).
void rvp_setup_graphics();
void rvp_register_native_types();
bool rvp_has_windows();
bool rvp_instance_send(rust::Str name, rust::Str message);
bool rvp_instance_listen(rust::Str name);
void rvp_set_c_numeric_locale();
void rvp_set_window_icon();
std::size_t rvp_main_window_id();
void rvp_crash_test();
void rvp_set_env(rust::Str name, rust::Str value);
std::size_t rvp_gl_get_proc_address(rust::Str name);
void rvp_video_request_update();
bool rvp_atlas_contains(rust::Str name);
void rvp_atlas_insert(rust::Str name, rust::Slice<const std::uint8_t> rgba, std::int32_t width,
                      std::int32_t height);
void rvp_paint_atlas(QPainter &painter, rust::Str name, std::array<std::int32_t, 4> src,
                     std::array<std::int32_t, 4> dst, std::array<std::int32_t, 4> clip);
void rvp_paint_translate(QPainter &painter, std::int32_t dx, std::int32_t dy);
void rvp_paint_fill(QPainter &painter, std::array<std::int32_t, 4> rect, std::uint32_t rgba);
void rvp_paint_text(QPainter &painter, rust::Str text, rust::Str trailing, rust::Str family,
                    std::int32_t pixel_size, bool bold, std::uint32_t rgba,
                    std::array<std::int32_t, 4> rect, std::array<std::int32_t, 4> clip,
                    std::int32_t align, bool elide);
