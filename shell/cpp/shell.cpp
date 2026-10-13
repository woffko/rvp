#include "rvp-shell/cpp/shell.h"

#include <clocale>
#include <cstdio>
#include <utility>

#include <QtCore/QDir>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QMutex>
#include <QtCore/QPointer>
#include <QtCore/QProcess>
#include <QtCore/QTimer>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtGui/QOpenGLContext>
#include <QtGui/QOpenGLFunctions>
#include <QtGui/QScreen>
#include <QtGui/QWindow>
#include <QtNetwork/QLocalServer>
#include <QtNetwork/QLocalSocket>
#include <QtOpenGL/QOpenGLFramebufferObject>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickOpenGLUtils>
#include <QtQuick/QQuickWindow>

#include "rvp-shell/src/ffi.cxxqt.h"

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

QRect toRect(const std::array<std::int32_t, 4> &r)
{
    return QRect(r[0], r[1], r[2], r[3]);
}

// The single-instance channel: requests (one JSON line each) go to the
// Player once QML attached it; earlier ones wait.
QLocalServer *instanceServer = nullptr;
QPointer<QObject> instanceReceiver;
QStringList instanceQueue;

void deliverRequest(const QString &line)
{
    if (instanceReceiver)
        QMetaObject::invokeMethod(instanceReceiver, "remote_message", Qt::QueuedConnection,
                                  Q_ARG(QString, line));
    else
        instanceQueue.append(line);
}

void acceptInstances()
{
    while (QLocalSocket *socket = instanceServer->nextPendingConnection()) {
        QObject::connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
        QObject::connect(socket, &QLocalSocket::readyRead, socket, [socket] {
            while (socket->canReadLine()) {
                const QString line = QString::fromUtf8(socket->readLine()).trimmed();
                if (line.isEmpty())
                    continue;
                deliverRequest(line);
                socket->write("ok\n");
                socket->flush();
            }
        });
    }
}

// Atlas images by name, shared by every SkinView (render thread).
QMutex atlasMutex;
QHash<QString, QImage> atlases;

class MpvRenderer : public QQuickFramebufferObject::Renderer
{
public:
    ~MpvRenderer() override
    {
        // The scene graph deletes renderers on the render thread with the
        // GL context current: free mpv's render context here.
        rvp_video_release();
    }

    QOpenGLFramebufferObject *createFramebufferObject(const QSize &size) override
    {
        QOpenGLFramebufferObjectFormat format;
        format.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
        return new QOpenGLFramebufferObject(size, format);
    }

    void render() override
    {
        QOpenGLFramebufferObject *fbo = framebufferObject();
        if (!rvp_video_render(static_cast<std::int32_t>(fbo->handle()), fbo->width(),
                              fbo->height())) {
            // Nothing drawn yet (mpv starts after the first frame): black.
            QOpenGLFunctions *gl = QOpenGLContext::currentContext()->functions();
            gl->glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
            gl->glClear(GL_COLOR_BUFFER_BIT);
        }
        QQuickOpenGLUtils::resetOpenGLState();
    }
};

} // namespace

std::atomic<MpvItem *> MpvItem::instance{nullptr};
QList<QPointer<QQuickItem>> MpvItem::pendingChrome;
bool MpvItem::fallbackArmed = false;

MpvItem::MpvItem(QQuickItem *parent)
    : QQuickFramebufferObject(parent)
{
    instance.store(this);
    // No mirroring: mpv renders into the framebuffer object the way Qt
    // samples it (flip off), and mirroring showed every video upside down on
    // Linux and on Windows (the Ubuntu boot run of M3-M5).
    connect(this, &QQuickItem::windowChanged, this, [](QQuickWindow *window) {
        if (window != nullptr) {
            // Emitted on the render thread after each presented frame.
            connect(window, &QQuickWindow::frameSwapped, window, [] { rvp_frame_swapped(); },
                    Qt::DirectConnection);
        }
    });
}

MpvItem::~MpvItem()
{
    MpvItem *self = this;
    instance.compare_exchange_strong(self, nullptr);
}

QQuickFramebufferObject::Renderer *MpvItem::createRenderer() const
{
    return new MpvRenderer();
}

void MpvItem::frameUpdate()
{
    update();
    flushChrome();
}

void MpvItem::flushChrome()
{
    for (const QPointer<QQuickItem> &item : std::as_const(pendingChrome)) {
        if (!item.isNull()) {
            item->update();
        }
    }
    pendingChrome.clear();
    fallbackArmed = false;
}

void Tools::repaint(QQuickItem *item, bool synced) const
{
    if (item == nullptr) {
        return;
    }
    if (!synced || MpvItem::instance.load() == nullptr) {
        item->update();
        return;
    }
    if (!MpvItem::pendingChrome.contains(item)) {
        MpvItem::pendingChrome.append(item);
    }
    if (!MpvItem::fallbackArmed) {
        // A stalled video must not hold the chrome back for long.
        MpvItem::fallbackArmed = true;
        QTimer::singleShot(150, [] { MpvItem::flushChrome(); });
    }
}

QString Tools::local_file(const QUrl &url) const
{
    return url.isLocalFile() ? QDir::toNativeSeparators(url.toLocalFile()) : url.toString();
}

void Tools::log(const QString &line) const
{
    writeLine(line);
}

int Tools::grab(const QString &directory, const QString &prefix) const
{
    QDir().mkpath(directory);
    int saved = 0;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        auto *quickWindow = qobject_cast<QQuickWindow *>(window);
        if (quickWindow == nullptr || !window->isVisible()) {
            continue;
        }
        const QImage image = quickWindow->grabWindow();
        const QString path = QStringLiteral("%1/%2-%3.png").arg(directory, prefix).arg(saved);
        if (image.save(path)) {
            writeLine(QStringLiteral("grabbed=%1 %2x%3").arg(path).arg(image.width()).arg(image.height()));
            ++saved;
        }
    }
    return saved;
}

int Tools::render_offscreen() const
{
    // Windows without an interactive desktop (an SSH session) are never
    // exposed, so the scene graph does not render them; grabbing renders
    // the scene once, which also lets mpv draw its next frame.
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

QString Tools::environment_json() const
{
    QJsonArray screens;
    for (QScreen *screen : QGuiApplication::screens()) {
        const QRect geometry = screen->geometry();
        screens.append(QJsonObject{
            {QStringLiteral("name"), screen->name()},
            {QStringLiteral("width"), geometry.width()},
            {QStringLiteral("height"), geometry.height()},
            {QStringLiteral("device_pixel_ratio"), screen->devicePixelRatio()},
            {QStringLiteral("refresh_rate"), screen->refreshRate()},
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

QString Tools::windows_json() const
{
    QJsonArray windows;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        const QRect geometry = window->geometry();
        windows.append(QJsonObject{
            {QStringLiteral("class"), QString::fromLatin1(window->metaObject()->className())},
            {QStringLiteral("title"), window->title()},
            {QStringLiteral("visible"), window->isVisible()},
            {QStringLiteral("exposed"), window->isExposed()},
            {QStringLiteral("width"), geometry.width()},
            {QStringLiteral("height"), geometry.height()},
            {QStringLiteral("popup"), window->type() == Qt::Popup || window->type() == Qt::ToolTip},
        });
    }
    return QString::fromUtf8(QJsonDocument(windows).toJson(QJsonDocument::Compact));
}

void Tools::instance_receiver(QObject *receiver) const
{
    instanceReceiver = receiver;
    const QStringList waiting = std::exchange(instanceQueue, {});
    for (const QString &line : waiting)
        deliverRequest(line);
}

bool Tools::run_self(const QStringList &arguments) const
{
    return QProcess::startDetached(QCoreApplication::applicationFilePath(), arguments);
}

bool rvp_instance_send(rust::Str name, rust::Str message)
{
    QLocalSocket socket;
    socket.connectToServer(fromStr(name));
    if (!socket.waitForConnected(1000))
        return false;
    QByteArray line(message.data(), static_cast<qsizetype>(message.size()));
    line.append('\n');
    socket.write(line);
    if (!socket.waitForBytesWritten(1000))
        return false;
    while (!socket.canReadLine()) {
        if (!socket.waitForReadyRead(3000))
            return false;
    }
    return socket.readLine().trimmed() == "ok";
}

bool rvp_instance_listen(rust::Str name)
{
    if (instanceServer)
        return true;
    auto *server = new QLocalServer(QCoreApplication::instance());
    server->setSocketOptions(QLocalServer::UserAccessOption);
    const QString channel = fromStr(name);
    bool listening = server->listen(channel);
    if (!listening && server->serverError() == QAbstractSocket::AddressInUseError) {
        // Nobody answered on the channel just before: a socket file left
        // by a process that died (Unix).
        QLocalServer::removeServer(channel);
        listening = server->listen(channel);
    }
    if (!listening) {
        delete server;
        return false;
    }
    instanceServer = server;
    QObject::connect(server, &QLocalServer::newConnection, server, acceptInstances);
    return true;
}

void rvp_setup_graphics()
{
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
}

void rvp_register_native_types()
{
    qmlRegisterType<MpvItem>("RvpNative", 1, 0, "MpvItem");
    qmlRegisterSingletonInstance("RvpNative", 1, 0, "Tools", new Tools());
}

bool rvp_has_windows()
{
    return !QGuiApplication::topLevelWindows().isEmpty();
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

void rvp_video_request_update()
{
    // Any thread: only queue.
    if (MpvItem *item = MpvItem::instance.load()) {
        QMetaObject::invokeMethod(item, "frameUpdate", Qt::QueuedConnection);
    }
}

bool rvp_atlas_contains(rust::Str name)
{
    const QMutexLocker lock(&atlasMutex);
    return atlases.contains(fromStr(name));
}

void rvp_atlas_insert(rust::Str name, rust::Slice<const std::uint8_t> rgba, std::int32_t width,
                      std::int32_t height)
{
    if (width <= 0 || height <= 0
        || rgba.size() < static_cast<std::size_t>(width) * static_cast<std::size_t>(height) * 4) {
        return;
    }
    // Copy the pixels: the slice belongs to Rust.
    const QImage image = QImage(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888)
                             .convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QMutexLocker lock(&atlasMutex);
    atlases.insert(fromStr(name), image);
}

void rvp_paint_atlas(QPainter &painter, rust::Str name, std::array<std::int32_t, 4> src,
                     std::array<std::int32_t, 4> dst, std::array<std::int32_t, 4> clip)
{
    QImage image;
    {
        const QMutexLocker lock(&atlasMutex);
        image = atlases.value(fromStr(name));
    }
    const QRect source = toRect(src);
    const QRect target = toRect(dst);
    if (image.isNull() || source.isEmpty() || target.isEmpty()) {
        return;
    }
    const QRect area = target.intersected(toRect(clip));
    if (area.isEmpty()) {
        return;
    }
    painter.save();
    painter.setClipRect(area);
    // Sprites are tiled, never scaled (rvp_skin::paint).
    for (int y = target.y(); y < target.y() + target.height(); y += source.height()) {
        if (y + source.height() <= area.top() || y > area.bottom()) {
            continue;
        }
        for (int x = target.x(); x < target.x() + target.width(); x += source.width()) {
            if (x + source.width() <= area.left() || x > area.right()) {
                continue;
            }
            painter.drawImage(QRect(x, y, source.width(), source.height()), image, source);
        }
    }
    painter.restore();
}

void rvp_paint_translate(QPainter &painter, std::int32_t dx, std::int32_t dy)
{
    painter.translate(dx, dy);
}

void rvp_paint_fill(QPainter &painter, std::array<std::int32_t, 4> rect, std::uint32_t rgba)
{
    painter.fillRect(toRect(rect), colorFromRgba(rgba));
}

void rvp_paint_text(QPainter &painter, rust::Str text, rust::Str trailing, rust::Str family,
                    std::int32_t pixel_size, bool bold, std::uint32_t rgba,
                    std::array<std::int32_t, 4> rect, std::array<std::int32_t, 4> clip,
                    std::int32_t align, bool elide)
{
    QFont font(fromStr(family));
    font.setPixelSize(pixel_size > 0 ? pixel_size : 12);
    font.setBold(bold);
    const QFontMetrics metrics(font);
    const QString main = fromStr(text);
    const QString tail = fromStr(trailing);
    const QRect box = toRect(rect);
    const int reserved = tail.isEmpty() ? 0 : metrics.horizontalAdvance(tail) + 4;
    const QRect mainRect(box.x(), box.y(), box.width() - reserved, box.height());
    const int h = align % 3;
    const int v = align / 3;
    const int horizontal = h == 1 ? Qt::AlignHCenter : (h == 2 ? Qt::AlignRight : Qt::AlignLeft);
    const int vertical = v == 1 ? Qt::AlignVCenter : (v == 2 ? Qt::AlignBottom : Qt::AlignTop);
    painter.save();
    painter.setClipRect(toRect(clip));
    painter.setFont(font);
    painter.setPen(colorFromRgba(rgba));
    const QString shown = elide ? metrics.elidedText(main, Qt::ElideRight, mainRect.width()) : main;
    painter.drawText(mainRect, horizontal | vertical | Qt::TextSingleLine, shown);
    if (!tail.isEmpty()) {
        painter.drawText(box, Qt::AlignRight | vertical | Qt::TextSingleLine, tail);
    }
    painter.restore();
}
