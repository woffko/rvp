// Spike S3: the C++ side of the shell experiment.
//
// * MpvItem: a QQuickFramebufferObject whose renderer asks Rust (libmpv's
//   OpenGL render API) to draw into the item's framebuffer.
// * SpikeTools: helpers for the QML self-test (window list, grabs, logging).
// * Painting helpers used by the Rust SkinItem to execute a display list.
#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtGui/QImage>
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

    static MpvItem *instance;
};

class SpikeTools : public QObject
{
    Q_OBJECT
public:
    using QObject::QObject;
    Q_INVOKABLE void log(const QString &line) const;
    Q_INVOKABLE QString windowsJson() const;
    Q_INVOKABLE QString environmentJson() const;
    Q_INVOKABLE int grabAll(const QString &directory, const QString &prefix) const;
    Q_INVOKABLE int renderOffscreen() const;
    // A native child window of the main window, for mpv's `wid` output.
    Q_INVOKABLE qint64 createChildWindow(int x, int y, int width, int height) const;
    Q_INVOKABLE void quit(int code) const;
};

// Called from Rust (declared in src/ffi.rs).
void rvp_setup_graphics();
void rvp_register_native_types();
void rvp_set_c_numeric_locale();
std::size_t rvp_gl_get_proc_address(rust::Str name);
void rvp_mpv_request_update();
void rvp_paint_image(QPainter &painter, const QImage &image, std::int32_t sx, std::int32_t sy,
                     std::int32_t sw, std::int32_t sh, std::int32_t dx, std::int32_t dy,
                     std::int32_t dw, std::int32_t dh, std::int32_t cx, std::int32_t cy,
                     std::int32_t cw, std::int32_t ch);
void rvp_paint_fill(QPainter &painter, std::int32_t x, std::int32_t y, std::int32_t w,
                    std::int32_t h, std::uint32_t rgba);
void rvp_paint_text(QPainter &painter, rust::Str text, rust::Str trailing, rust::Str family,
                    std::int32_t pixel_size, bool bold, std::uint32_t rgba, std::int32_t x,
                    std::int32_t y, std::int32_t w, std::int32_t h, std::int32_t cx,
                    std::int32_t cy, std::int32_t cw, std::int32_t ch, std::int32_t h_align,
                    std::int32_t v_align, bool elide);
