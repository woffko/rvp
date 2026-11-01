// The Xbox controller for M14's Accessibility > XInput page (Windows): its
// buttons reach QML as Tools.gamepadButton (0 A, 1 B, 2 X, 3 Y, 4 LB, 5 RB,
// 6 Back, 7 Start, 8 up, 9 down, 10 left, 11 right), and with the mouse
// emulation a stick moves the pointer and A clicks. Polled 60 times a
// second while it is on. On Linux the kernel's joystick API
// (/dev/input/js*, with the xpad driver's numbers for an Xbox controller)
// gives the buttons as they change; the stick moves the pointer where the
// window system lets a program do so (X11; Wayland does not), and A clicks
// inside RVP's own windows only.

#include "shell.h"

#include <cstdlib>

#include <QtCore/QTimer>
#include <QtGui/QCursor>
#include <QtGui/QGuiApplication>

#ifdef Q_OS_LINUX
#include <QtCore/QDir>
#include <QtCore/QSocketNotifier>
#include <QtGui/QMouseEvent>
#include <QtGui/QWindow>

#include <cerrno>

#include <fcntl.h>
#include <linux/joystick.h>
#include <unistd.h>
#endif

#ifdef Q_OS_WIN
#include <windows.h>
#include <xinput.h>
#pragma comment(lib, "xinput.lib")
#pragma comment(lib, "user32.lib")
#endif

namespace {

#ifdef Q_OS_WIN
struct Pad
{
    bool focusOnly = true;
    int stick = 0; // 0 none, 1 left, 2 right
    unsigned short buttons = 0;
    QTimer *timer = nullptr;
};

Pad &pad()
{
    static Pad state;
    return state;
}

void click(bool down)
{
    INPUT input{};
    input.type = INPUT_MOUSE;
    input.mi.dwFlags = down ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP;
    SendInput(1, &input, sizeof(input));
}

void poll()
{
    Pad &state = pad();
    if (state.focusOnly && QGuiApplication::applicationState() != Qt::ApplicationActive)
        return;
    XINPUT_STATE now{};
    if (XInputGetState(0, &now) != ERROR_SUCCESS) {
        state.buttons = 0;
        return;
    }
    const WORD buttons = now.Gamepad.wButtons;
    const WORD pressed = buttons & ~state.buttons;
    const WORD released = state.buttons & ~buttons;
    state.buttons = buttons;
    if (state.stick != 0) {
        const SHORT x = state.stick == 1 ? now.Gamepad.sThumbLX : now.Gamepad.sThumbRX;
        const SHORT y = state.stick == 1 ? now.Gamepad.sThumbLY : now.Gamepad.sThumbRY;
        const auto step = [](SHORT value) {
            return std::abs(static_cast<int>(value)) < XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE ? 0 : value / 2048;
        };
        if (step(x) != 0 || step(y) != 0)
            QCursor::setPos(QCursor::pos() + QPoint(step(x), -step(y)));
        // A is the mouse's left button while a stick moves the pointer.
        if (pressed & XINPUT_GAMEPAD_A)
            click(true);
        if (released & XINPUT_GAMEPAD_A)
            click(false);
    }
    const WORD order[12] = {XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_X,
                            XINPUT_GAMEPAD_Y, XINPUT_GAMEPAD_LEFT_SHOULDER,
                            XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_BACK,
                            XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_DPAD_UP,
                            XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT,
                            XINPUT_GAMEPAD_DPAD_RIGHT};
    for (int i = 0; i < 12; ++i) {
        const bool clicks = i == 0 && state.stick != 0;
        if ((pressed & order[i]) != 0 && !clicks && Tools::instance != nullptr)
            emit Tools::instance->gamepadButton(i);
    }
}
#endif

#ifdef Q_OS_LINUX
// The xpad driver's numbers: buttons A, B, X, Y, LB, RB, Back, Start;
// axes 0 and 1 the left stick, 3 and 4 the right, 6 and 7 the pad.
constexpr int kAxisPadX = 6;
constexpr int kAxisPadY = 7;
constexpr int kDeadZone = 7849;

struct Pad
{
    bool on = false;
    bool focusOnly = true;
    int stick = 0; // 0 none, 1 left, 2 right
    int fd = -1;
    int axes[8] = {};
    QSocketNotifier *notifier = nullptr;
    QTimer *timer = nullptr; // the pointer's moves, and finding a controller
};

Pad &pad()
{
    static Pad state;
    return state;
}

void emitButton(int button)
{
    Pad &state = pad();
    if (state.focusOnly && QGuiApplication::applicationState() != Qt::ApplicationActive)
        return;
    if (Tools::instance != nullptr)
        emit Tools::instance->gamepadButton(button);
}

// A press or release of the left button where the pointer is, in RVP's
// window there (the system takes no injected input from a program).
void click(bool down)
{
    const QPoint global = QCursor::pos();
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        if (!window->isVisible() || !window->geometry().contains(global))
            continue;
        const QPointF local = window->mapFromGlobal(global);
        QMouseEvent event(down ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease, local,
                          QPointF(global), Qt::LeftButton, down ? Qt::LeftButton : Qt::NoButton,
                          Qt::NoModifier);
        QCoreApplication::sendEvent(window, &event);
        return;
    }
}

void closePad()
{
    Pad &state = pad();
    if (state.notifier != nullptr) {
        delete state.notifier;
        state.notifier = nullptr;
    }
    if (state.fd >= 0)
        ::close(state.fd);
    state.fd = -1;
    for (int &axis : state.axes)
        axis = 0;
}

void readPad()
{
    Pad &state = pad();
    js_event event{};
    for (;;) {
        const ssize_t read = ::read(state.fd, &event, sizeof(event));
        if (read != static_cast<ssize_t>(sizeof(event))) {
            // EAGAIN: all read; anything else: the controller is gone.
            if (read < 0 && errno != EAGAIN)
                closePad();
            return;
        }
        const bool initial = (event.type & JS_EVENT_INIT) != 0;
        const int kind = event.type & ~JS_EVENT_INIT;
        if (kind == JS_EVENT_AXIS && event.number < 8) {
            const int before = state.axes[event.number];
            state.axes[event.number] = event.value;
            // The pad is two axes: a move off the middle is a press.
            if (!initial && (event.number == kAxisPadX || event.number == kAxisPadY) && before == 0
                && event.value != 0) {
                const bool vertical = event.number == kAxisPadY;
                emitButton(vertical ? (event.value < 0 ? 8 : 9) : (event.value < 0 ? 10 : 11));
            }
        } else if (kind == JS_EVENT_BUTTON && !initial && event.number < 8) {
            const bool down = event.value != 0;
            if (event.number == 0 && state.stick != 0) {
                click(down);
                continue;
            }
            if (down)
                emitButton(event.number);
        }
    }
}

// Opens the first joystick device when none is open.
void findPad()
{
    Pad &state = pad();
    if (state.fd >= 0)
        return;
    const QStringList nodes = QDir(QStringLiteral("/dev/input")).entryList({QStringLiteral("js*")},
                                                                           QDir::System, QDir::Name);
    for (const QString &node : nodes) {
        const QByteArray path = QStringLiteral("/dev/input/%1").arg(node).toLocal8Bit();
        const int fd = ::open(path.constData(), O_RDONLY | O_NONBLOCK | O_CLOEXEC);
        if (fd < 0)
            continue;
        state.fd = fd;
        state.notifier = new QSocketNotifier(fd, QSocketNotifier::Read);
        QObject::connect(state.notifier, &QSocketNotifier::activated, readPad);
        return;
    }
}

// Every 16 ms: the stick moves the pointer; about every 2 s a controller
// that is not open yet is looked for.
void tick()
{
    Pad &state = pad();
    static int calls = 0;
    if (state.fd < 0 && ++calls % 125 == 1)
        findPad();
    if (state.fd < 0 || state.stick == 0)
        return;
    if (state.focusOnly && QGuiApplication::applicationState() != Qt::ApplicationActive)
        return;
    const int x = state.axes[state.stick == 1 ? 0 : 3];
    const int y = state.axes[state.stick == 1 ? 1 : 4];
    const auto step = [](int value) { return std::abs(value) < kDeadZone ? 0 : value / 2048; };
    // The joystick API's y grows downwards, as the screen's.
    if (step(x) != 0 || step(y) != 0)
        QCursor::setPos(QCursor::pos() + QPoint(step(x), step(y)));
}
#endif

} // namespace

void rvp_xinput_start(bool enabled, bool focus_only, std::int32_t stick)
{
#ifdef Q_OS_WIN
    Pad &state = pad();
    state.focusOnly = focus_only;
    state.stick = stick;
    if (!enabled) {
        if (state.timer != nullptr)
            state.timer->stop();
        return;
    }
    if (state.timer == nullptr) {
        state.timer = new QTimer(QCoreApplication::instance());
        state.timer->setInterval(16);
        QObject::connect(state.timer, &QTimer::timeout, poll);
    }
    state.timer->start();
#elif defined(Q_OS_LINUX)
    Pad &state = pad();
    state.focusOnly = focus_only;
    state.stick = stick;
    state.on = enabled;
    if (!enabled) {
        if (state.timer != nullptr)
            state.timer->stop();
        closePad();
        return;
    }
    if (state.timer == nullptr) {
        state.timer = new QTimer(QCoreApplication::instance());
        state.timer->setInterval(16);
        QObject::connect(state.timer, &QTimer::timeout, tick);
    }
    findPad();
    state.timer->start();
#else
    (void)enabled;
    (void)focus_only;
    (void)stick;
#endif
}
