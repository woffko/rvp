#include "rvp-shell/cpp/shell.h"

#include <algorithm>
#include <array>
#include <clocale>
#include <cmath>
#include <cstdio>
#include <string>
#include <utility>

#include <QtCore/QByteArray>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLocale>
#include <QtCore/QMetaObject>
#include <QtCore/QMutex>
#include <QtCore/QPointer>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QTimer>
#include <QtCore/QTranslator>
#include <QtGui/QClipboard>
#include <QtGui/QDesktopServices>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QImageReader>
#include <QtGui/QSurfaceFormat>
#include <QtGui/QOpenGLContext>
#include <QtGui/QOpenGLFunctions>
#include <QtGui/QPainter>
#include <QtGui/QScreen>
#include <QtGui/QPolygon>
#include <QtGui/QRegion>
#include <QtGui/QWindow>
#include <QtNetwork/QHostAddress>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtOpenGL/QOpenGLFramebufferObject>
#include <QtQml/QQmlEngine>
#include <QtQml/QQmlNetworkAccessManagerFactory>
#include <QtQuick/QQuickOpenGLUtils>
#include <QtQuick/QQuickWindow>

#include "rvp-shell/src/ffi.cxxqt.h"

#ifdef Q_OS_WIN
// Without Windows' min and max macros (std::max is used here).
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <shlobj_core.h>
// ClipCursor and the monitor calls.
#pragma comment(lib, "user32.lib")
// Tools::show_file: the shell's folder view and COM.
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "ole32.lib")
#endif

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

QString Tools::write_file(const QString &path, const QString &text) const
{
    const QFileInfo info(path);
    QDir().mkpath(info.absolutePath());
    QFile file(info.absoluteFilePath());
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        return {};
    }
    const QByteArray bytes = text.toUtf8();
    if (file.write(bytes) != bytes.size()) {
        return {};
    }
    file.close();
    return QDir::toNativeSeparators(info.absoluteFilePath());
}

namespace {

// The mean colour of a 5x5 block around (x, y), clamped to the image.
std::array<double, 3> sampleAt(const QImage &image, int x, int y)
{
    std::array<double, 3> sum{0, 0, 0};
    int count = 0;
    for (int dy = -2; dy <= 2; ++dy) {
        for (int dx = -2; dx <= 2; ++dx) {
            const int px = std::clamp(x + dx, 0, image.width() - 1);
            const int py = std::clamp(y + dy, 0, image.height() - 1);
            const QRgb rgb = image.pixel(px, py);
            sum[0] += qRed(rgb);
            sum[1] += qGreen(rgb);
            sum[2] += qBlue(rgb);
            ++count;
        }
    }
    for (double &channel : sum) {
        channel /= count;
    }
    return sum;
}

} // namespace

QString Tools::compare_frame(QQuickItem *item, double aspect, const QString &reference) const
{
    QJsonObject result;
    const QImage expected = QImage(reference).convertToFormat(QImage::Format_RGB32);
    QQuickWindow *window = item != nullptr ? item->window() : nullptr;
    if (window == nullptr || expected.isNull() || !(aspect > 0)) {
        result.insert(QStringLiteral("error"), QStringLiteral("no item, reference or aspect"));
        return QString::fromUtf8(QJsonDocument(result).toJson(QJsonDocument::Compact));
    }
    const QImage frame = window->grabWindow().convertToFormat(QImage::Format_RGB32);
    const qreal ratio = frame.devicePixelRatio();
    QRectF area = item->mapRectToScene(QRectF(0, 0, item->width(), item->height()));
    area = QRectF(area.x() * ratio, area.y() * ratio, area.width() * ratio, area.height() * ratio);
    // mpv keeps the aspect ratio: the picture is centred with bars.
    QRectF picture = area;
    if (area.width() / area.height() > aspect) {
        picture.setWidth(area.height() * aspect);
        picture.moveLeft(area.x() + (area.width() - picture.width()) / 2);
    } else {
        picture.setHeight(area.width() / aspect);
        picture.moveTop(area.y() + (area.height() - picture.height()) / 2);
    }
    const std::array<double, 4> grid{0.125, 0.375, 0.625, 0.875};
    // Upright, flipped (upside down), mirrored, turned.
    std::array<double, 4> distance{0, 0, 0, 0};
    bool dark = true;
    int points = 0;
    for (const double fy : grid) {
        for (const double fx : grid) {
            const auto shown = sampleAt(frame, int(picture.x() + fx * picture.width()),
                                        int(picture.y() + fy * picture.height()));
            if (std::max({shown[0], shown[1], shown[2]}) >= 24) {
                dark = false;
            }
            const std::array<std::array<double, 2>, 4> places{{
                {fx, fy}, {fx, 1 - fy}, {1 - fx, fy}, {1 - fx, 1 - fy}}};
            for (std::size_t h = 0; h < places.size(); ++h) {
                const auto wanted = sampleAt(expected, int(places[h][0] * expected.width()),
                                             int(places[h][1] * expected.height()));
                distance[h] += (std::abs(shown[0] - wanted[0]) + std::abs(shown[1] - wanted[1])
                                + std::abs(shown[2] - wanted[2])) / 3;
            }
            ++points;
        }
    }
    const char *names[] = {"upright", "flipped", "mirrored", "rotated"};
    for (std::size_t h = 0; h < distance.size(); ++h) {
        result.insert(QString::fromLatin1(names[h]), distance[h] / points);
    }
    result.insert(QStringLiteral("points"), points);
    result.insert(QStringLiteral("dark"), dark);
    result.insert(QStringLiteral("picture"),
                  QJsonArray{int(picture.x()), int(picture.y()), int(picture.width()), int(picture.height())});
    return QString::fromUtf8(QJsonDocument(result).toJson(QJsonDocument::Compact));
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

bool Tools::run_self(const QStringList &arguments) const
{
    return QProcess::startDetached(QCoreApplication::applicationFilePath(), arguments);
}

QString Tools::clipboard_text() const
{
    return QGuiApplication::clipboard()->text();
}

void Tools::set_clipboard_text(const QString &text) const
{
    QGuiApplication::clipboard()->setText(text);
}

QString Tools::image_info(const QString &path) const
{
    QImageReader reader(path);
    const QSize size = reader.size();
    const QJsonObject object{
        {QStringLiteral("exists"), QFileInfo::exists(path)},
        {QStringLiteral("format"), QString::fromLatin1(reader.format())},
        {QStringLiteral("width"), size.width()},
        {QStringLiteral("height"), size.height()},
    };
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QString Tools::clipboard_image_info() const
{
    const QImage image = QGuiApplication::clipboard()->image();
    const QJsonObject object{
        {QStringLiteral("width"), image.width()},
        {QStringLiteral("height"), image.height()},
    };
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QString Tools::read_file(const QString &path, qint64 from) const
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || !file.seek(std::max<qint64>(0, from)))
        return {};
    return QString::fromUtf8(file.readAll());
}

namespace {

// Where shell/build.rs puts the compiled translations, beside Main.qml.
const QString i18nFolder = QStringLiteral(":/qt/qml/Rvp/qml/i18n");

// The codes of the built-in translations (qml_<code>.qm), sorted.
QStringList languageCodes()
{
    QStringList codes;
    const QStringList files = QDir(i18nFolder).entryList({QStringLiteral("qml_*.qm")}, QDir::Files, QDir::Name);
    for (const QString &file : files)
        codes.append(QFileInfo(file).completeBaseName().mid(4));
    return codes;
}

} // namespace

QString Tools::interface_languages() const
{
    QList<QPair<QString, QString>> found;
    for (const QString &code : languageCodes()) {
        QTranslator translator;
        QString name;
        if (translator.load(i18nFolder + QStringLiteral("/qml_") + code + QStringLiteral(".qm")))
            // The translation's own name: its translation of "English".
            name = translator.translate("Language", "English");
        found.append({name.isEmpty() ? code : name, code});
    }
    std::sort(found.begin(), found.end());
    QJsonArray languages;
    for (const auto &[name, code] : found)
        languages.append(QJsonObject{{QStringLiteral("code"), code}, {QStringLiteral("name"), name}});
    return QString::fromUtf8(QJsonDocument(languages).toJson(QJsonDocument::Compact));
}

QString Tools::system_language() const
{
    const QStringList codes = languageCodes();
    const auto known = [&codes](const QString &candidate) -> QString {
        for (const QString &code : codes)
            if (code.compare(candidate, Qt::CaseInsensitive) == 0)
                return code;
        return {};
    };
    const QStringList wanted = QLocale::system().uiLanguages();
    // QTranslator's rule: each UI language in turn, then the same with its
    // last part dropped (zh_Hant_TW, zh_Hant, zh); English, the source
    // text, counts as present.
    for (const QString &language : wanted) {
        if (QLocale(language).language() == QLocale::English)
            return QStringLiteral("en");
        QString candidate = QString(language).replace(QLatin1Char('-'), QLatin1Char('_'));
        while (!candidate.isEmpty()) {
            const QString code = known(candidate);
            if (!code.isEmpty())
                return code;
            const qsizetype cut = candidate.lastIndexOf(QLatin1Char('_'));
            candidate = cut > 0 ? candidate.left(cut) : QString();
        }
    }
    // Then a translation of the same language for another region, of the
    // same script first (pt_PT finds pt_BR; zh_SG the Simplified one).
    for (const bool sameScript : {true, false}) {
        for (const QString &language : wanted) {
            const QLocale locale(language);
            for (const QString &code : codes) {
                const QLocale offered(code);
                if (offered.language() == locale.language() && (!sameScript || offered.script() == locale.script()))
                    return code;
            }
        }
    }
    return QStringLiteral("en");
}

int Tools::selftest_listen()
{
    if (listener == nullptr) {
        listener = new QTcpServer(this);
        if (!listener->listen(QHostAddress::LocalHost, 0))
            return 0;
        connect(listener, &QTcpServer::newConnection, this, [this] {
            while (QTcpSocket *socket = listener->nextPendingConnection()) {
                const qsizetype index = requests.size();
                requests.append(QString());
                connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
                connect(socket, &QTcpSocket::readyRead, this, [this, socket, index] {
                    if (!socket->canReadLine())
                        return;
                    requests[index] = QString::fromUtf8(socket->readLine()).trimmed();
                    socket->write("HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    socket->disconnectFromHost();
                });
            }
        });
    }
    return listener->serverPort();
}

QString Tools::selftest_requests() const
{
    return QString::fromUtf8(QJsonDocument(QJsonArray::fromStringList(requests)).toJson(QJsonDocument::Compact));
}

QString Tools::list_files(const QString &folder) const
{
    const QStringList names = QDir(folder).entryList(QDir::Files, QDir::Name);
    return QString::fromUtf8(QJsonDocument(QJsonArray::fromStringList(names)).toJson(QJsonDocument::Compact));
}

void Tools::set_window_mask(QWindow *window, const QString &polygons, double scale) const
{
    if (window == nullptr)
        return;
    QRegion region;
    const QStringList shapes = polygons.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &shape : shapes) {
        QPolygon polygon;
        for (const QString &point : shape.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
            const QStringList xy = point.split(QLatin1Char(','));
            if (xy.size() == 2)
                polygon << QPoint(qRound(xy[0].toDouble() * scale), qRound(xy[1].toDouble() * scale));
        }
        if (polygon.size() >= 3)
            region = region.united(QRegion(polygon, Qt::WindingFill));
    }
    window->setMask(region);
}

void Tools::set_window_mask_rects(QWindow *window, const QString &rects, double scale) const
{
    if (window == nullptr)
        return;
    QRegion region;
    for (const QString &rect : rects.split(QLatin1Char(';'), Qt::SkipEmptyParts)) {
        const QStringList parts = rect.split(QLatin1Char(','));
        if (parts.size() != 4)
            continue;
        region += QRect(qRound(parts[0].toDouble() * scale), qRound(parts[1].toDouble() * scale),
                        qRound(parts[2].toDouble() * scale), qRound(parts[3].toDouble() * scale));
    }
    window->setMask(region);
}

QString Tools::screen_json(QWindow *window) const
{
    QScreen *screen = window != nullptr ? window->screen() : QGuiApplication::primaryScreen();
    if (screen == nullptr)
        return QStringLiteral("{}");
    const qreal ratio = screen->devicePixelRatio();
    const QSize size = screen->geometry().size();
    const QSizeF physical = screen->physicalSize();
    const QRect available = screen->availableGeometry();
    const QJsonObject object{
        {QStringLiteral("width"), qRound(size.width() * ratio)},
        {QStringLiteral("height"), qRound(size.height() * ratio)},
        {QStringLiteral("width_mm"), physical.width()},
        {QStringLiteral("height_mm"), physical.height()},
        {QStringLiteral("refresh"), screen->refreshRate()},
        {QStringLiteral("name"), screen->name()},
        // The free area (without task bars) in the window's coordinates.
        {QStringLiteral("x"), available.x()},
        {QStringLiteral("y"), available.y()},
        {QStringLiteral("available_width"), available.width()},
        {QStringLiteral("available_height"), available.height()},
    };
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

bool Tools::show_file(const QString &path) const
{
#ifdef Q_OS_WIN
    // The shell's own call opens the folder with the file selected: no
    // program is started by name, and the path is never part of a
    // command line. COM is joined in the apartment Qt chose (balanced
    // below).
    const std::wstring native = QDir::toNativeSeparators(QFileInfo(path).absoluteFilePath()).toStdWString();
    const HRESULT joined = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    bool shown = false;
    PIDLIST_ABSOLUTE item = nullptr;
    if (SUCCEEDED(SHParseDisplayName(native.c_str(), nullptr, &item, 0, nullptr))) {
        shown = SUCCEEDED(SHOpenFolderAndSelectItems(item, 0, nullptr, 0));
        ILFree(item);
    }
    if (SUCCEEDED(joined))
        CoUninitialize();
    if (shown)
        return true;
    // An item the shell cannot make: its folder, as elsewhere.
#endif
    return open_url(QUrl::fromLocalFile(QFileInfo(path).absolutePath()).toString());
}

QString Tools::tr_menu(const QString &label) const
{
    static const QRegularExpression number(QStringLiteral("\\d+(?:[.,]\\d+)?"));
    QStringList numbers;
    QString key;
    qsizetype last = 0;
    for (auto match = number.globalMatch(label); match.hasNext();) {
        const QRegularExpressionMatch found = match.next();
        key += label.mid(last, found.capturedStart() - last);
        numbers << found.captured();
        key += QStringLiteral("%%1").arg(numbers.size());
        last = found.capturedEnd();
    }
    key += label.mid(last);
    const QByteArray source = key.toUtf8();
    QString text = QCoreApplication::translate("Menu", source.constData());
    if (text == key)
        return label;
    for (qsizetype i = 0; i < numbers.size(); ++i)
        text.replace(QStringLiteral("%%1").arg(i + 1), numbers[i]);
    return text;
}

void Tools::set_app_font(const QString &family, int points) const
{
    // The first call keeps Qt's own font to go back to.
    static const QFont original = QGuiApplication::font();
    QFont font = original;
    if (!family.isEmpty())
        font.setFamily(family);
    if (points > 0)
        font.setPointSize(points);
    QGuiApplication::setFont(font);
}

void rvp_setup_graphics()
{
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
#ifdef Q_OS_LINUX
    // Desktop OpenGL, as GLX (X11, XWayland) and WGL give it anyway: on
    // Wayland Qt's EGL takes the driver's first API unless the format asks,
    // and NVIDIA's gave OpenGL ES 3.2, where libplacebo has no rgba32f and
    // NVScaler's passes failed (the Ubuntu boot run, the Linux audit's H2).
    // RVP_GL_ES=1 leaves the choice to the driver, for an EGL without
    // desktop OpenGL.
    if (qEnvironmentVariableIntValue("RVP_GL_ES") != 1) {
        QSurfaceFormat format = QSurfaceFormat::defaultFormat();
        format.setRenderableType(QSurfaceFormat::OpenGL);
        QSurfaceFormat::setDefaultFormat(format);
    }
#endif
}

Tools *Tools::instance = nullptr;

namespace {

// QML's network: RVP's QML loads its own files from the resources and
// pictures from local files, neither through a network manager, and
// nothing it shows comes from the network. Its manager answers every
// request at once with an error (an empty address), so a text that is
// taken for HTML somewhere (a title's <img>) still loads nothing.
// Text from outside RVP is shown as plain text in the first place.
// The one caller is a self-test's: SelfTestM15.qml asks RVP's own control
// API on this computer (`/api/...`), which passes while a self-test runs.
bool selfTestRuns = false;

bool ownApi(const QUrl &url)
{
    return url.scheme() == QLatin1String("http") && url.path().startsWith(QLatin1String("/api/"))
        && QHostAddress(url.host()).isLoopback();
}

class NoNetworkManager : public QNetworkAccessManager
{
public:
    using QNetworkAccessManager::QNetworkAccessManager;

protected:
    QNetworkReply *createRequest(Operation operation, const QNetworkRequest &request, QIODevice *data) override
    {
        if (selfTestRuns && ownApi(request.url()))
            return QNetworkAccessManager::createRequest(operation, request, data);
        return QNetworkAccessManager::createRequest(operation, QNetworkRequest(QUrl()), data);
    }
};

class NoNetworkFactory : public QQmlNetworkAccessManagerFactory
{
public:
    // Called on any of the engine's threads; keeps no state.
    QNetworkAccessManager *create(QObject *parent) override { return new NoNetworkManager(parent); }
};

NoNetworkFactory noNetwork;

}  // namespace

void rvp_register_native_types()
{
    qmlRegisterType<MpvItem>("RvpNative", 1, 0, "MpvItem");
    Tools::instance = new Tools();
    // Set before any of the engine's threads asks for a manager.
    for (const QString &argument : QCoreApplication::arguments())
        selfTestRuns = selfTestRuns || argument.startsWith(QLatin1String("--selftest"));
    // The engine asks for Tools first while Main.qml loads, before any
    // QML could have asked the network for anything: it gets the
    // network guard then.
    qmlRegisterSingletonType<Tools>("RvpNative", 1, 0, "Tools", [](QQmlEngine *engine, QJSEngine *) -> QObject * {
        engine->setNetworkAccessManagerFactory(&noNetwork);
        QJSEngine::setObjectOwnership(Tools::instance, QJSEngine::CppOwnership);
        return Tools::instance;
    });
    rvp_taskbar_install();
}

bool rvp_has_windows()
{
    return !QGuiApplication::topLevelWindows().isEmpty();
}

rust::String rvp_interface_languages()
{
    const QByteArray text = Tools().interface_languages().toUtf8();
    return rust::String(text.constData(), static_cast<std::size_t>(text.size()));
}

void rvp_set_c_numeric_locale()
{
    // libmpv refuses to start unless LC_NUMERIC is "C"; Qt sets the user
    // locale on Unix.
    std::setlocale(LC_NUMERIC, "C");
}

void rvp_set_window_icon()
{
    // RVP's icon in every size the build drew (shell/build.rs); the window
    // system picks the one that fits.
    QIcon icon;
    for (const int size : {16, 24, 32, 48, 64, 128, 256})
        icon.addFile(QStringLiteral(":/qt/qml/Rvp/look/icons/rvp-%1.png").arg(size), QSize(size, size));
    QGuiApplication::setWindowIcon(icon);
}

void rvp_language_fonts(rust::Str code)
{
#ifdef Q_OS_WIN
    // Han characters take their shapes from a language. Windows' own
    // fallback for them follows the system's language: on an English
    // Windows Yu Gothic UI came first, so Chinese showed the characters
    // Japanese shares in Japanese shapes and weight and the others in
    // Microsoft YaHei UI (goal V-W). A Chinese, Japanese or Korean
    // interface puts that language's own UI font first; characters it
    // lacks still go to Windows' fallback.
    const QString language = QString::fromUtf8(code.data(), static_cast<qsizetype>(code.size()));
    QString family;
    if (language == QLatin1String("zh_CN"))
        family = QStringLiteral("Microsoft YaHei UI");
    else if (language == QLatin1String("zh_TW"))
        family = QStringLiteral("Microsoft JhengHei UI");
    else if (language == QLatin1String("ja"))
        family = QStringLiteral("Yu Gothic UI");
    else if (language == QLatin1String("ko"))
        family = QStringLiteral("Malgun Gothic");
    if (family.isEmpty() || !QFontDatabase::hasFamily(family))
        return;
    for (const QChar::Script script : {QChar::Script_Han, QChar::Script_Hiragana, QChar::Script_Katakana,
                                       QChar::Script_Bopomofo, QChar::Script_Hangul})
        QFontDatabase::addApplicationFallbackFontFamily(script, family);
#else
    // fontconfig orders the fonts by FC_LANG (player::font_language).
    static_cast<void>(code);
#endif
}

std::size_t rvp_main_window_id()
{
    for (QWindow *window : QGuiApplication::topLevelWindows())
        if (window->objectName() == QLatin1String("rvpMain"))
            return static_cast<std::size_t>(window->winId());
    return 0;
}

void rvp_confine_cursor(bool on)
{
#ifdef Q_OS_WIN
    if (!on) {
        ClipCursor(nullptr);
        return;
    }
    const auto window = reinterpret_cast<HWND>(rvp_main_window_id());
    if (window == nullptr)
        return;
    MONITORINFO info{};
    info.cbSize = sizeof(info);
    if (GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &info))
        ClipCursor(&info.rcMonitor);
#else
    (void)on;
#endif
}

void rvp_crash_test()
{
    // Undefined behaviour on purpose: the crash the handler must report.
    volatile int *nowhere = nullptr;
    *nowhere = 1;
}

// The variables RVP set for itself before Qt started (its platform, its
// scaling, VapourSynth's runtime...): programs it opens for the user do
// not get them (the Linux audit's L8: xdg-open handed QT_QPA_PLATFORM=xcb
// to a Qt file manager).
QStringList &ownVariables()
{
    static QStringList names;
    return names;
}

void rvp_set_env(rust::Str name, rust::Str value)
{
    const std::string key(name.data(), name.size());
    qputenv(key.c_str(), QString::fromUtf8(value.data(), qsizetype(value.size())).toLocal8Bit());
    ownVariables().append(QString::fromStdString(key));
}

bool Tools::open_url(const QString &url) const
{
#ifdef Q_OS_LINUX
    if (!ownVariables().isEmpty()) {
        QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
        for (const QString &name : ownVariables())
            environment.remove(name);
        QProcess opener;
        opener.setProgram(QStringLiteral("xdg-open"));
        opener.setArguments({url});
        opener.setProcessEnvironment(environment);
        opener.setStandardInputFile(QProcess::nullDevice());
        if (opener.startDetached())
            return true;
    }
#endif
    return QDesktopServices::openUrl(QUrl(url));
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

void rvp_atlas_remove_prefix(rust::Str prefix)
{
    const QString start = fromStr(prefix);
    const QMutexLocker lock(&atlasMutex);
    for (auto it = atlases.begin(); it != atlases.end();) {
        if (it.key().startsWith(start))
            it = atlases.erase(it);
        else
            ++it;
    }
}

namespace {

// An RGBA picture from Rust as a QImage of its own (the slice belongs to
// Rust); a null image when the sizes do not fit.
QImage imageFromRgba(rust::Slice<const std::uint8_t> rgba, std::int32_t width, std::int32_t height)
{
    if (width <= 0 || height <= 0
        || rgba.size() < static_cast<std::size_t>(width) * static_cast<std::size_t>(height) * 4) {
        return {};
    }
    // The row's bytes in qsizetype, as QImage takes them: no int product.
    return QImage(rgba.data(), width, height, static_cast<qsizetype>(width) * 4,
                  QImage::Format_RGBA8888)
        .copy();
}

} // namespace

bool rvp_save_image(rust::Slice<const std::uint8_t> rgba, std::int32_t width, std::int32_t height,
                    rust::Str path, std::int32_t quality, rust::Str texts)
{
    // Any thread: QImage and QPainter on a QImage need no GUI thread.
    QImage image = imageFromRgba(rgba, width, height);
    if (image.isNull())
        return false;
    const QJsonArray lines = QJsonDocument::fromJson(fromStr(texts).toUtf8()).array();
    if (!lines.isEmpty()) {
        QPainter painter(&image);
        painter.setRenderHint(QPainter::TextAntialiasing);
        for (const QJsonValue &value : lines) {
            const QJsonObject line = value.toObject();
            QFont font;
            font.setPixelSize(std::max(6, line.value(QStringLiteral("size")).toInt()));
            painter.setFont(font);
            const QRect rect(line.value(QStringLiteral("x")).toInt(),
                             line.value(QStringLiteral("y")).toInt(),
                             line.value(QStringLiteral("width")).toInt(),
                             line.value(QStringLiteral("height")).toInt());
            const int align = (line.value(QStringLiteral("align")).toString() == QStringLiteral("right")
                                   ? Qt::AlignRight
                                   : Qt::AlignLeft)
                              | Qt::AlignVCenter;
            const QString text = line.value(QStringLiteral("text")).toString();
            // A dark outline keeps the text readable over the pictures.
            painter.setPen(QColor(0, 0, 0, 200));
            painter.drawText(rect.translated(1, 1), align, text);
            painter.setPen(QColor(235, 235, 235));
            painter.drawText(rect, align, text);
        }
    }
    const QString file = fromStr(path);
    QDir().mkpath(QFileInfo(file).absolutePath());
    return image.convertToFormat(QImage::Format_RGB888).save(file, nullptr, quality);
}

bool rvp_copy_image(rust::Slice<const std::uint8_t> rgba, std::int32_t width, std::int32_t height)
{
    // The window's thread: the clipboard belongs to it.
    const QImage image = imageFromRgba(rgba, width, height);
    if (image.isNull())
        return false;
    QGuiApplication::clipboard()->setImage(image.convertToFormat(QImage::Format_RGB888));
    return true;
}

rust::String rvp_local_time()
{
    const QByteArray text =
        QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-dd HH.mm.ss")).toUtf8();
    return rust::String(text.constData(), static_cast<std::size_t>(text.size()));
}

std::int32_t rvp_utc_offset()
{
    return QDateTime::currentDateTime().offsetFromUtc();
}

std::int32_t rvp_text_width(rust::Str text, rust::Str family, std::int32_t pixel_size, bool bold)
{
    // The font rvp_paint_text draws with.
    QFont font(fromStr(family));
    font.setPixelSize(pixel_size > 0 ? pixel_size : 12);
    font.setBold(bold);
    return QFontMetrics(font).horizontalAdvance(fromStr(text));
}

void rvp_atlas_insert(rust::Str name, rust::Slice<const std::uint8_t> rgba, std::int32_t width,
                      std::int32_t height)
{
    if (width <= 0 || height <= 0
        || rgba.size() < static_cast<std::size_t>(width) * static_cast<std::size_t>(height) * 4) {
        return;
    }
    // Copy the pixels: the slice belongs to Rust. The row's bytes in
    // qsizetype: no int product (a skin's pictures are within
    // rvp_skin::limits::PICTURE_SIDE anyway).
    const QImage image = QImage(rgba.data(), width, height, static_cast<qsizetype>(width) * 4,
                                QImage::Format_RGBA8888)
                             .convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QMutexLocker lock(&atlasMutex);
    atlases.insert(fromStr(name), image);
}

bool rvp_add_font(rust::Slice<const std::uint8_t> data)
{
    const QByteArray bytes(reinterpret_cast<const char *>(data.data()), static_cast<qsizetype>(data.size()));
    return QFontDatabase::addApplicationFontFromData(bytes) >= 0;
}

void rvp_paint_stretch(QPainter &painter, rust::Str name, std::array<std::int32_t, 4> src,
                       std::array<std::int32_t, 4> dst, std::array<std::int32_t, 4> clip)
{
    QImage image;
    {
        const QMutexLocker lock(&atlasMutex);
        image = atlases.value(fromStr(name));
    }
    const QRect source = toRect(src);
    const QRect target = toRect(dst);
    const QRect area = target.intersected(toRect(clip));
    if (image.isNull() || source.isEmpty() || target.isEmpty() || area.isEmpty()) {
        return;
    }
    painter.save();
    painter.setClipRect(area);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    painter.drawImage(QRectF(target), image, QRectF(source));
    painter.restore();
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
    // Sprites are tiled, never scaled (rvp_skin::paint). The tiles lie on a
    // grid from the target's corner, but only those that meet the clip are
    // visited: from the first one that reaches into `area` to `area`'s end,
    // so a target far larger than the window costs what shows of it. The
    // skin engine keeps layout coordinates within +-2^20 of the window
    // (rvp_skin::limits::COORDINATE), and the sums here are 64-bit.
    const std::int64_t stepX = source.width();
    const std::int64_t stepY = source.height();
    const std::int64_t left = area.x();
    const std::int64_t top = area.y();
    const std::int64_t right = left + area.width();
    const std::int64_t bottom = top + area.height();
    // `area` lies inside `target`: the offsets are not negative.
    const std::int64_t firstX = target.x() + (left - target.x()) / stepX * stepX;
    const std::int64_t firstY = target.y() + (top - target.y()) / stepY * stepY;
    for (std::int64_t y = firstY; y < bottom; y += stepY) {
        for (std::int64_t x = firstX; x < right; x += stepX) {
            // Within `area`'s numbers, which are ints.
            painter.drawImage(QRect(static_cast<int>(x), static_cast<int>(y), source.width(),
                                    source.height()),
                              image, source);
        }
    }
    painter.restore();
}

void rvp_paint_translate(QPainter &painter, std::int32_t dx, std::int32_t dy)
{
    painter.translate(dx, dy);
}

void rvp_paint_scale(QPainter &painter, double factor)
{
    painter.scale(factor, factor);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, factor != 1.0);
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
