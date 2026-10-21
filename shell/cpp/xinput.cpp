// The Xbox controller for M14's Accessibility > XInput page (Windows): its
// buttons reach QML as Tools.gamepadButton (0 A, 1 B, 2 X, 3 Y, 4 LB, 5 RB,
// 6 Back, 7 Start, 8 up, 9 down, 10 left, 11 right), and with the mouse
// emulation a stick moves the pointer and A clicks. Polled 60 times a
// second while it is on. Elsewhere it does nothing (Linux's controllers are
// goal S's).

#include "shell.h"

#include <cstdlib>

#include <QtCore/QTimer>
#include <QtGui/QCursor>
#include <QtGui/QGuiApplication>

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
#else
    (void)enabled;
    (void)focus_only;
    (void)stick;
#endif
}
