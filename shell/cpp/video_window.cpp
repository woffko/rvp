// D4's Direct3D 11 renderer: mpv's `gpu-next` draws into a native child
// window of the main window (`wid`) instead of the render API's OpenGL
// item. docs/spikes/m12-studies.md ("Direct3D 11 output") has the study.
//
// - The window belongs to a thread of its own. mpv's window is a child of
//   it, and mpv's threads may send it messages; the window's thread never
//   waits for mpv, so the GUI thread's blocking mpv calls cannot deadlock
//   with them (S3 finding 10).
// - It is disabled: Windows gives the pointer's messages over a disabled
//   child window to its parent, so RVP's window keeps the mouse and the
//   keyboard.
// - RVP's own layers over the video (messages, the seek preview, the
//   skin's bars in fullscreen) cannot draw over a native window: QML
//   passes their rectangles as holes, cut out of the window's region.
// - Only the GUI thread calls these functions; they post the change and
//   return.

#include "rvp-shell/cpp/shell.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtGui/QGuiApplication>
#include <QtGui/QWindow>

#ifdef Q_OS_WIN
#include <algorithm>
#include <chrono>
#include <cmath>
#include <future>
#include <mutex>
#include <thread>
#include <vector>

#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "gdi32.lib")

namespace {

constexpr UINT applyMessage = WM_APP + 1;
constexpr wchar_t className[] = L"RvpVideoWindow";

// Physical pixels: the window in its parent's client area, the holes in
// the window's own coordinates.
struct Place {
    RECT rect{};
    bool visible = false;
    std::vector<RECT> holes;
};

struct State {
    std::mutex lock;
    Place wanted;
    HWND window = nullptr;
    std::thread thread;
};

State &state()
{
    static State instance;
    return instance;
}

void apply(HWND window)
{
    Place place;
    {
        std::lock_guard<std::mutex> guard(state().lock);
        place = state().wanted;
    }
    const int width = place.rect.right - place.rect.left;
    const int height = place.rect.bottom - place.rect.top;
    const bool shown = place.visible && width > 0 && height > 0;
    if (place.holes.empty()) {
        SetWindowRgn(window, nullptr, FALSE);
    } else {
        HRGN region = CreateRectRgn(0, 0, width, height);
        for (const RECT &hole : place.holes) {
            HRGN cut = CreateRectRgn(hole.left, hole.top, hole.right, hole.bottom);
            CombineRgn(region, region, cut, RGN_DIFF);
            DeleteObject(cut);
        }
        // The window owns the region from here.
        SetWindowRgn(window, region, FALSE);
    }
    SetWindowPos(window, HWND_TOP, place.rect.left, place.rect.top, std::max(1, width), std::max(1, height),
                 SWP_NOACTIVATE | SWP_NOOWNERZORDER | (shown ? SWP_SHOWWINDOW : SWP_HIDEWINDOW));
    RedrawWindow(window, nullptr, nullptr, RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN);
}

LRESULT CALLBACK windowProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam)
{
    switch (message) {
    case applyMessage:
        apply(window);
        return 0;
    case WM_ERASEBKGND: {
        // Black until mpv's window covers it.
        RECT rect;
        GetClientRect(window, &rect);
        FillRect(reinterpret_cast<HDC>(wParam), &rect, static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH)));
        return 1;
    }
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    default:
        return DefWindowProcW(window, message, wParam, lParam);
    }
}

void run(HWND parent, std::promise<HWND> created)
{
    const HINSTANCE instance = GetModuleHandleW(nullptr);
    WNDCLASSEXW windowClass{};
    windowClass.cbSize = sizeof(windowClass);
    windowClass.lpfnWndProc = windowProc;
    windowClass.hInstance = instance;
    windowClass.lpszClassName = className;
    windowClass.hbrBackground = static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH));
    RegisterClassExW(&windowClass);
    // No parent notification: it would wait for the GUI thread.
    HWND window = CreateWindowExW(WS_EX_NOPARENTNOTIFY, className, L"RVP video",
                                  WS_CHILD | WS_DISABLED | WS_CLIPCHILDREN | WS_CLIPSIBLINGS, 0, 0, 1, 1, parent,
                                  nullptr, instance, nullptr);
    created.set_value(window);
    if (window == nullptr)
        return;
    MSG message;
    while (GetMessageW(&message, nullptr, 0, 0) > 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

QWindow *mainWindow()
{
    for (QWindow *window : QGuiApplication::topLevelWindows())
        if (window->objectName() == QLatin1String("rvpMain"))
            return window;
    return nullptr;
}

} // namespace
#endif

std::size_t rvp_video_window_create()
{
#ifdef Q_OS_WIN
    State &video = state();
    if (video.window != nullptr)
        return reinterpret_cast<std::size_t>(video.window);
    const auto parent = reinterpret_cast<HWND>(rvp_main_window_id());
    if (parent == nullptr)
        return 0;
    std::promise<HWND> promise;
    std::future<HWND> created = promise.get_future();
    video.thread = std::thread(run, parent, std::move(promise));
    if (created.wait_for(std::chrono::seconds(5)) != std::future_status::ready) {
        // The thread lives on with the process; the render API is used.
        video.thread.detach();
        return 0;
    }
    video.window = created.get();
    if (video.window == nullptr) {
        video.thread.join();
        return 0;
    }
    return reinterpret_cast<std::size_t>(video.window);
#else
    return 0;
#endif
}

void rvp_video_window_close()
{
#ifdef Q_OS_WIN
    State &video = state();
    if (video.window != nullptr)
        PostMessageW(video.window, WM_CLOSE, 0, 0);
    if (video.thread.joinable())
        video.thread.join();
    video.window = nullptr;
#endif
}

bool Tools::native_video() const
{
#ifdef Q_OS_WIN
    return state().window != nullptr;
#else
    return false;
#endif
}

void Tools::place_video_window(const QString &place) const
{
#ifdef Q_OS_WIN
    State &video = state();
    if (video.window == nullptr)
        return;
    const QJsonObject object = QJsonDocument::fromJson(place.toUtf8()).object();
    const QWindow *window = mainWindow();
    const double ratio = window != nullptr ? window->devicePixelRatio() : 1.0;
    const auto scaled = [ratio](double value) { return static_cast<LONG>(std::lround(value * ratio)); };
    Place wanted;
    const double x = object.value(QStringLiteral("x")).toDouble();
    const double y = object.value(QStringLiteral("y")).toDouble();
    wanted.rect = {scaled(x), scaled(y), scaled(x + object.value(QStringLiteral("width")).toDouble()),
                   scaled(y + object.value(QStringLiteral("height")).toDouble())};
    wanted.visible = object.value(QStringLiteral("visible")).toBool();
    for (const QJsonValue &value : object.value(QStringLiteral("holes")).toArray()) {
        const QJsonArray hole = value.toArray();
        if (hole.size() != 4)
            continue;
        const double left = hole.at(0).toDouble() - x;
        const double top = hole.at(1).toDouble() - y;
        RECT cut{scaled(left), scaled(top), scaled(left + hole.at(2).toDouble()),
                 scaled(top + hole.at(3).toDouble())};
        if (cut.right > cut.left && cut.bottom > cut.top)
            wanted.holes.push_back(cut);
    }
    {
        std::lock_guard<std::mutex> guard(video.lock);
        video.wanted = std::move(wanted);
    }
    PostMessageW(video.window, applyMessage, 0, 0);
#else
    (void)place;
#endif
}

QString Tools::video_window_json() const
{
    QJsonObject object;
#ifdef Q_OS_WIN
    const HWND window = state().window;
    object.insert(QStringLiteral("exists"), window != nullptr && IsWindow(window) != FALSE);
    if (window != nullptr && IsWindow(window) != FALSE) {
        RECT rect;
        GetWindowRect(window, &rect);
        MapWindowPoints(HWND_DESKTOP, GetParent(window), reinterpret_cast<POINT *>(&rect), 2);
        object.insert(QStringLiteral("x"), static_cast<int>(rect.left));
        object.insert(QStringLiteral("y"), static_cast<int>(rect.top));
        object.insert(QStringLiteral("width"), static_cast<int>(rect.right - rect.left));
        object.insert(QStringLiteral("height"), static_cast<int>(rect.bottom - rect.top));
        object.insert(QStringLiteral("visible"), IsWindowVisible(window) != FALSE);
        object.insert(QStringLiteral("enabled"), IsWindowEnabled(window) != FALSE);
        // The region's rectangles: 0 without a region.
        int pieces = 0;
        HRGN region = CreateRectRgn(0, 0, 0, 0);
        if (GetWindowRgn(window, region) != ERROR) {
            const DWORD size = GetRegionData(region, 0, nullptr);
            std::vector<char> data(size);
            if (size > 0 && GetRegionData(region, size, reinterpret_cast<RGNDATA *>(data.data())) == size)
                pieces = static_cast<int>(reinterpret_cast<RGNDATA *>(data.data())->rdh.nCount);
        }
        DeleteObject(region);
        object.insert(QStringLiteral("region_pieces"), pieces);
        // mpv's own window inside it.
        wchar_t name[64] = {};
        const HWND child = FindWindowExW(window, nullptr, nullptr, nullptr);
        if (child != nullptr)
            GetClassNameW(child, name, 64);
        object.insert(QStringLiteral("child"), QString::fromWCharArray(name));
    }
#else
    object.insert(QStringLiteral("exists"), false);
#endif
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}
