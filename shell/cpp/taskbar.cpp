// Windows' taskbar and session (M14's General page): the taskbar button's
// progress and its thumbnail toolbar (previous, play/pause, next), the
// jump list's recent files (Windows lists what SHAddToRecentDocs names for
// the types RVP is registered for), the screen lock and unlock, and the
// start with the user's session (the Run key), and the media keys taken
// while another window has the keyboard. Elsewhere these do nothing
// (Linux's start with the session is src/startup.rs's).

#include "shell.h"

#include <QtCore/QAbstractNativeEventFilter>
#include <QtCore/QCoreApplication>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

#ifdef Q_OS_WIN
#include <windows.h>
#include <shlobj_core.h>
#include <shobjidl_core.h>
#include <wtsapi32.h>
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "wtsapi32.lib")
#endif

namespace {

#ifdef Q_OS_WIN
// The thumbnail buttons' command IDs: previous, play/pause, next.
constexpr UINT firstButton = 100;
// The media keys' hotkey IDs, in Tools::mediaKey's order.
constexpr int firstHotKey = 0x5256;
constexpr UINT mediaKeys[4] = {VK_MEDIA_PLAY_PAUSE, VK_MEDIA_STOP, VK_MEDIA_NEXT_TRACK,
                               VK_MEDIA_PREV_TRACK};

struct Taskbar
{
    UINT created = 0;
    ITaskbarList3 *list = nullptr;
    bool buttons = false;
    bool enabled = false;
    bool playing = false;
    HICON icons[4] = {};
};

Taskbar &taskbar()
{
    static Taskbar state;
    return state;
}

// RVP's own small symbols, drawn here: previous, play, pause, next.
HICON drawIcon(int kind)
{
    QImage image(32, 32, QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);
    QPainter painter(&image);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(240, 240, 240));
    auto triangle = [&](qreal left, qreal right) {
        QPainterPath path;
        path.moveTo(left, 7);
        path.lineTo(right, 16);
        path.lineTo(left, 25);
        path.closeSubpath();
        painter.drawPath(path);
    };
    switch (kind) {
    case 0:
        painter.drawRect(QRectF(7, 7, 3, 18));
        painter.translate(32, 0);
        painter.scale(-1, 1);
        triangle(7, 22);
        break;
    case 1:
        triangle(9, 25);
        break;
    case 2:
        painter.drawRect(QRectF(9, 7, 5, 18));
        painter.drawRect(QRectF(18, 7, 5, 18));
        break;
    default:
        painter.drawRect(QRectF(22, 7, 3, 18));
        triangle(7, 22);
        break;
    }
    painter.end();
    return image.toHICON();
}

HWND mainWindow()
{
    return reinterpret_cast<HWND>(rvp_main_window_id());
}

void addButtons()
{
    Taskbar &state = taskbar();
    const HWND window = mainWindow();
    if (state.buttons || state.list == nullptr || window == nullptr)
        return;
    const wchar_t *tips[3] = {L"Previous", L"Play/Pause", L"Next"};
    THUMBBUTTON buttons[3] = {};
    for (int i = 0; i < 3; ++i) {
        buttons[i].dwMask = THB_ICON | THB_TOOLTIP | THB_FLAGS;
        buttons[i].iId = firstButton + i;
        buttons[i].hIcon = state.icons[i == 0 ? 0 : i == 1 ? 1 : 3];
        buttons[i].dwFlags = state.enabled ? THBF_ENABLED : THBF_HIDDEN;
        wcsncpy_s(buttons[i].szTip, tips[i], _TRUNCATE);
    }
    state.buttons = SUCCEEDED(state.list->ThumbBarAddButtons(window, 3, buttons));
}

void updateButtons()
{
    Taskbar &state = taskbar();
    const HWND window = mainWindow();
    if (!state.buttons || window == nullptr)
        return;
    THUMBBUTTON buttons[3] = {};
    for (int i = 0; i < 3; ++i) {
        buttons[i].dwMask = THB_ICON | THB_FLAGS;
        buttons[i].iId = firstButton + i;
        buttons[i].hIcon = state.icons[i == 0 ? 0 : i == 2 ? 3 : (state.playing ? 2 : 1)];
        buttons[i].dwFlags = state.enabled ? THBF_ENABLED : THBF_HIDDEN;
    }
    state.list->ThumbBarUpdateButtons(window, 3, buttons);
}

class Filter : public QAbstractNativeEventFilter
{
public:
    bool nativeEventFilter(const QByteArray &type, void *message, qintptr *) override
    {
        if (type != "windows_generic_MSG" || Tools::instance == nullptr)
            return false;
        const MSG *msg = static_cast<const MSG *>(message);
        if (msg->message == taskbar().created && msg->hwnd == mainWindow()) {
            addButtons();
        } else if (msg->message == WM_COMMAND && HIWORD(msg->wParam) == THBN_CLICKED) {
            const int button = static_cast<int>(LOWORD(msg->wParam)) - static_cast<int>(firstButton);
            if (button >= 0 && button < 3)
                emit Tools::instance->taskbarButton(button);
        } else if (msg->message == WM_HOTKEY && msg->hwnd == mainWindow()) {
            const int key = static_cast<int>(msg->wParam) - firstHotKey;
            if (key >= 0 && key < 4) {
                emit Tools::instance->mediaKey(key);
                return true;
            }
        } else if (msg->message == WM_WTSSESSION_CHANGE) {
            if (msg->wParam == WTS_SESSION_LOCK)
                emit Tools::instance->sessionLocked(true);
            else if (msg->wParam == WTS_SESSION_UNLOCK)
                emit Tools::instance->sessionLocked(false);
        }
        return false;
    }
};
#endif

} // namespace

void rvp_taskbar_install()
{
#ifdef Q_OS_WIN
    // Before the main window exists: Windows says when its taskbar button
    // is made.
    taskbar().created = RegisterWindowMessageW(L"TaskbarButtonCreated");
    static Filter filter;
    QCoreApplication::instance()->installNativeEventFilter(&filter);
#endif
}

void rvp_taskbar_start(bool enabled)
{
#ifdef Q_OS_WIN
    Taskbar &state = taskbar();
    state.enabled = enabled;
    const HWND window = mainWindow();
    if (window == nullptr)
        return;
    static bool started = false;
    if (!started) {
        started = true;
        WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION);
        for (int i = 0; i < 4; ++i)
            state.icons[i] = drawIcon(i);
        if (FAILED(CoCreateInstance(CLSID_TaskbarList, nullptr, CLSCTX_INPROC_SERVER,
                                    IID_PPV_ARGS(&state.list)))
            || FAILED(state.list->HrInit())) {
            state.list = nullptr;
            return;
        }
        addButtons();
    }
    updateButtons();
    if (!enabled && state.list != nullptr)
        state.list->SetProgressState(window, TBPF_NOPROGRESS);
#else
    (void)enabled;
#endif
}

void rvp_global_media_keys(bool on)
{
#ifdef Q_OS_WIN
    static bool registered = false;
    const HWND window = mainWindow();
    if (window == nullptr || on == registered)
        return;
    registered = on;
    for (int i = 0; i < 4; ++i) {
        // Another program may hold a key already: that key stays its.
        if (on)
            RegisterHotKey(window, firstHotKey + i, MOD_NOREPEAT, mediaKeys[i]);
        else
            UnregisterHotKey(window, firstHotKey + i);
    }
#else
    (void)on;
#endif
}

void rvp_taskbar_state(std::int32_t state, double progress)
{
#ifdef Q_OS_WIN
    Taskbar &bar = taskbar();
    const HWND window = mainWindow();
    if (bar.list == nullptr || window == nullptr || !bar.enabled)
        return;
    const bool playing = state == 1;
    if (playing != bar.playing) {
        bar.playing = playing;
        updateButtons();
    }
    if (state == 0 || progress < 0.0) {
        bar.list->SetProgressState(window, TBPF_NOPROGRESS);
        return;
    }
    bar.list->SetProgressState(window, state == 2 ? TBPF_PAUSED : TBPF_NORMAL);
    const double clamped = progress < 0.0 ? 0.0 : progress > 1.0 ? 1.0 : progress;
    bar.list->SetProgressValue(window, static_cast<ULONGLONG>(clamped * 10000.0), 10000);
#else
    (void)state;
    (void)progress;
#endif
}

void rvp_taskbar_recent(rust::Str path)
{
#ifdef Q_OS_WIN
    const std::wstring wide = QString::fromUtf8(path.data(), static_cast<qsizetype>(path.size()))
                                  .toStdWString();
    SHAddToRecentDocs(SHARD_PATHW, wide.c_str());
#else
    (void)path;
#endif
}

bool rvp_run_at_startup(bool on, rust::Str command)
{
#ifdef Q_OS_WIN
    const wchar_t *key = L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    if (!on) {
        const LSTATUS status = RegDeleteKeyValueW(HKEY_CURRENT_USER, key, L"RVP");
        return status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND;
    }
    const std::wstring value = QString::fromUtf8(command.data(), static_cast<qsizetype>(command.size()))
                                   .toStdWString();
    return RegSetKeyValueW(HKEY_CURRENT_USER, key, L"RVP", REG_SZ, value.c_str(),
                           static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)))
        == ERROR_SUCCESS;
#else
    (void)on;
    (void)command;
    return false;
#endif
}
