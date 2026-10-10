#include "s3-qt-shell/cpp/spike.h"

#ifdef _WIN32
#include <windows.h>
#endif

#include <clocale>
#include <cstdio>
#include <cstdlib>

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QOpenGLContext>
#include <QtGui/QOpenGLFunctions>
#include <QtGui/QScreen>
#include <QtGui/QWindow>
#include <QtOpenGL/QOpenGLFramebufferObject>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickOpenGLUtils>
#include <QtQuick/QQuickWindow>

#include "s3-qt-shell/src/ffi.cxxqt.h"

namespace {

void writeLine(const QString &line)
{
    const QByteArray bytes = line.toUtf8();
    std::fwrite(bytes.constData(), 1, static_cast<std::size_t>(bytes.size()), stdout);
    std::fputc('\n', stdout);
    std::fflush(stdout);
}

QColor colorFromRgba(std::uint32_t rgba)
{
    return QColor(static_cast<int>((rgba >> 24) & 0xff), static_cast<int>((rgba >> 16) & 0xff),
                  static_cast<int>((rgba >> 8) & 0xff), static_cast<int>(rgba & 0xff));
}

QString fromStr(rust::Str text)
{
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

class MpvRenderer : public QQuickFramebufferObject::Renderer
{
public:
    QOpenGLFramebufferObject *createFramebufferObject(const QSize &size) override
    {
        QOpenGLFramebufferObjectFormat format;
        format.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
        return new QOpenGLFramebufferObject(size, format);
    }

    void render() override
    {
        if (!m_initialized) {
            m_initialized = true;
            if (QOpenGLContext *context = QOpenGLContext::currentContext()) {
                QOpenGLFunctions *gl = context->functions();
                writeLine(QStringLiteral("gl_vendor=%1").arg(
                    QString::fromLatin1(reinterpret_cast<const char *>(gl->glGetString(GL_VENDOR)))));
                writeLine(QStringLiteral("gl_renderer=%1").arg(QString::fromLatin1(
                    reinterpret_cast<const char *>(gl->glGetString(GL_RENDERER)))));
                writeLine(QStringLiteral("gl_version=%1").arg(QString::fromLatin1(
                    reinterpret_cast<const char *>(gl->glGetString(GL_VERSION)))));
            }
            rvp_mpv_init_gl();
        }
        QOpenGLFramebufferObject *fbo = framebufferObject();
        rvp_mpv_render(static_cast<std::int32_t>(fbo->handle()), fbo->width(), fbo->height());
        QQuickOpenGLUtils::resetOpenGLState();
    }

private:
    bool m_initialized = false;
};

} // namespace

MpvItem *MpvItem::instance = nullptr;

MpvItem::MpvItem(QQuickItem *parent)
    : QQuickFramebufferObject(parent)
{
    instance = this;
    setMirrorVertically(true);
}

MpvItem::~MpvItem()
{
    if (instance == this) {
        instance = nullptr;
    }
}

QQuickFramebufferObject::Renderer *MpvItem::createRenderer() const
{
    return new MpvRenderer();
}

void SpikeTools::log(const QString &line) const
{
    writeLine(line);
}

QString SpikeTools::windowsJson() const
{
    QJsonArray windows;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        const QRect geometry = window->geometry();
        const QRect frame = window->frameGeometry();
        windows.append(QJsonObject{
            {QStringLiteral("class"), QString::fromLatin1(window->metaObject()->className())},
            {QStringLiteral("title"), window->title()},
            {QStringLiteral("visible"), window->isVisible()},
            {QStringLiteral("exposed"), window->isExposed()},
            {QStringLiteral("x"), geometry.x()},
            {QStringLiteral("y"), geometry.y()},
            {QStringLiteral("width"), geometry.width()},
            {QStringLiteral("height"), geometry.height()},
            {QStringLiteral("frame_width"), frame.width()},
            {QStringLiteral("frame_height"), frame.height()},
            {QStringLiteral("flags"), static_cast<int>(window->flags().toInt())},
            {QStringLiteral("popup"), window->type() == Qt::Popup || window->type() == Qt::ToolTip},
        });
    }
    return QString::fromUtf8(QJsonDocument(windows).toJson(QJsonDocument::Compact));
}

QString SpikeTools::environmentJson() const
{
    QJsonArray screens;
    for (QScreen *screen : QGuiApplication::screens()) {
        const QRect geometry = screen->geometry();
        screens.append(QJsonObject{
            {QStringLiteral("name"), screen->name()},
            {QStringLiteral("width"), geometry.width()},
            {QStringLiteral("height"), geometry.height()},
            {QStringLiteral("device_pixel_ratio"), screen->devicePixelRatio()},
        });
    }
    return QString::fromUtf8(
        QJsonDocument(QJsonObject{
                          {QStringLiteral("platform"), QGuiApplication::platformName()},
                          {QStringLiteral("qt_version"), QString::fromLatin1(qVersion())},
                          {QStringLiteral("screens"), screens},
                      })
            .toJson(QJsonDocument::Compact));
}

int SpikeTools::grabAll(const QString &directory, const QString &prefix) const
{
    QDir().mkpath(directory);
    int saved = 0;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        auto *quickWindow = qobject_cast<QQuickWindow *>(window);
        if (quickWindow == nullptr || !window->isVisible()) {
            continue;
        }
        const QImage image = quickWindow->grabWindow();
        const QString path = QStringLiteral("%1/%2-%3-%4x%5.png")
                                 .arg(directory, prefix)
                                 .arg(saved)
                                 .arg(image.width())
                                 .arg(image.height());
        if (image.save(path)) {
            writeLine(QStringLiteral("grabbed=%1").arg(path));
            ++saved;
        }
    }
    return saved;
}

void SpikeTools::quit(int code) const
{
    // The spike leaks libmpv and its render context; an orderly Qt shutdown
    // can wait on them, so the self-test ends the process directly. On
    // Windows even _Exit runs DLL detach code, so terminate instead.
    std::fflush(stdout);
    std::fflush(stderr);
#ifdef _WIN32
    TerminateProcess(GetCurrentProcess(), static_cast<UINT>(code));
#endif
    std::_Exit(code);
}

int SpikeTools::renderOffscreen() const
{
    // Windows without an interactive desktop (an SSH session) never exposes
    // windows, so the scene graph does not render them; grabbing renders the
    // scene once, which also lets mpv draw its next frame.
    int rendered = 0;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        auto *quickWindow = qobject_cast<QQuickWindow *>(window);
        if (quickWindow != nullptr && window->isVisible() && !window->isExposed()) {
            quickWindow->grabWindow();
            ++rendered;
        }
    }
    return rendered;
}

void rvp_setup_graphics()
{
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
}

void rvp_register_native_types()
{
    qmlRegisterType<MpvItem>("RvpSpikeNative", 1, 0, "MpvItem");
    qmlRegisterSingletonInstance("RvpSpikeNative", 1, 0, "Tools", new SpikeTools());
}

void rvp_set_c_numeric_locale()
{
    // libmpv refuses to start unless LC_NUMERIC is "C"; Qt sets the user
    // locale on Unix.
    std::setlocale(LC_NUMERIC, "C");
}

std::size_t rvp_gl_get_proc_address(rust::Str name)
{
    QOpenGLContext *context = QOpenGLContext::currentContext();
    if (context == nullptr) {
        return 0;
    }
    const QByteArray symbol(name.data(), static_cast<qsizetype>(name.size()));
    return reinterpret_cast<std::size_t>(context->getProcAddress(symbol));
}

void rvp_mpv_request_update()
{
    if (MpvItem::instance != nullptr) {
        QMetaObject::invokeMethod(MpvItem::instance, "update", Qt::QueuedConnection);
    }
}

void rvp_paint_image(QPainter &painter, const QImage &image, std::int32_t sx, std::int32_t sy,
                     std::int32_t sw, std::int32_t sh, std::int32_t dx, std::int32_t dy,
                     std::int32_t dw, std::int32_t dh, std::int32_t cx, std::int32_t cy,
                     std::int32_t cw, std::int32_t ch)
{
    if (sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0) {
        return;
    }
    const QRect target(dx, dy, dw, dh);
    const QRect area = target.intersected(QRect(cx, cy, cw, ch));
    if (area.isEmpty()) {
        return;
    }
    painter.save();
    painter.setClipRect(area);
    const QRect source(sx, sy, sw, sh);
    for (int y = dy; y < dy + dh; y += sh) {
        if (y + sh <= area.top() || y > area.bottom()) {
            continue;
        }
        for (int x = dx; x < dx + dw; x += sw) {
            if (x + sw <= area.left() || x > area.right()) {
                continue;
            }
            painter.drawImage(QRect(x, y, sw, sh), image, source);
        }
    }
    painter.restore();
}

void rvp_paint_fill(QPainter &painter, std::int32_t x, std::int32_t y, std::int32_t w,
                    std::int32_t h, std::uint32_t rgba)
{
    painter.fillRect(QRect(x, y, w, h), colorFromRgba(rgba));
}

void rvp_paint_text(QPainter &painter, rust::Str text, rust::Str trailing, rust::Str family,
                    std::int32_t pixel_size, bool bold, std::uint32_t rgba, std::int32_t x,
                    std::int32_t y, std::int32_t w, std::int32_t h, std::int32_t cx,
                    std::int32_t cy, std::int32_t cw, std::int32_t ch, std::int32_t h_align,
                    std::int32_t v_align, bool elide)
{
    QFont font(fromStr(family));
    font.setPixelSize(pixel_size);
    font.setBold(bold);
    const QFontMetrics metrics(font);
    const QString main = fromStr(text);
    const QString tail = fromStr(trailing);
    const int reserved = tail.isEmpty() ? 0 : metrics.horizontalAdvance(tail) + 4;
    const QRect mainRect(x, y, w - reserved, h);
    const int horizontal = h_align == 1 ? Qt::AlignHCenter : (h_align == 2 ? Qt::AlignRight : Qt::AlignLeft);
    const int vertical = v_align == 1 ? Qt::AlignVCenter : (v_align == 2 ? Qt::AlignBottom : Qt::AlignTop);
    painter.save();
    painter.setClipRect(QRect(cx, cy, cw, ch));
    painter.setFont(font);
    painter.setPen(colorFromRgba(rgba));
    const QString shown = elide ? metrics.elidedText(main, Qt::ElideRight, mainRect.width()) : main;
    painter.drawText(mainRect, horizontal | vertical | Qt::TextSingleLine, shown);
    if (!tail.isEmpty()) {
        painter.drawText(QRect(x, y, w, h), Qt::AlignRight | vertical | Qt::TextSingleLine, tail);
    }
    painter.restore();
}
