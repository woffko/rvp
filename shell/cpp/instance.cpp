// The single-instance channel (M3, crates/app/src/instance.rs): a second
// RVP hands its request, one JSON line, to the running one, which answers
// "ok".
//
// On Unix a QLocalServer and QLocalSocket, on a socket in a folder of the
// user's alone (the Rust side chooses it). On Windows a named pipe, whose
// name any program may take: the server makes the pipe's first instance
// itself (FILE_FLAG_FIRST_PIPE_INSTANCE), for the current user alone and
// local clients only, so it never joins a pipe another user made, and
// does not listen when the name is taken. Qt's QLocalServer cannot adopt
// such a pipe (its listen(qintptr) is not supported on Windows), so the
// server is written here; each connected pipe goes to a QLocalSocket, as
// Qt's own server hands over its pipes. A client sends only to a pipe the
// current user owns, and lets the server identify it but not act as it.
//
// A request is small and comes at once: a connection's buffer is capped,
// and the connection closed after a few seconds.

#include "shell.h"

#include <string>
#include <utility>
#include <vector>

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDeadlineTimer>
#include <QtCore/QElapsedTimer>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QPointer>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtNetwork/QLocalServer>
#include <QtNetwork/QLocalSocket>

#ifdef Q_OS_WIN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <aclapi.h>
#include <QtCore/QWinEventNotifier>
#pragma comment(lib, "advapi32.lib")
#endif

namespace {

QString fromStr(rust::Str text)
{
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

// Requests go to the Player once QML attached it; earlier ones wait.
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

// A request names one command line's files: Windows' command line holds
// 32767 characters, Linux's arguments 2 MiB unless the stack limit is
// raised.
constexpr qint64 requestMax = 4 * 1024 * 1024;
// A second instance writes its request as soon as it connected and waits
// 3 s for the answer.
constexpr int connectionMs = 5000;

// Serves one connection: each line is a request, answered "ok".
void serve(QLocalSocket *socket)
{
    QObject::connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
    socket->setReadBufferSize(requestMax);
    const auto close = [socket] {
        socket->abort();
        socket->deleteLater();
    };
    QTimer::singleShot(connectionMs, socket, close);
    QObject::connect(socket, &QLocalSocket::readyRead, socket, [socket, close] {
        while (socket->canReadLine()) {
            const QString line = QString::fromUtf8(socket->readLine()).trimmed();
            if (line.isEmpty())
                continue;
            deliverRequest(line);
            socket->write("ok\n");
            socket->flush();
        }
        // The buffer is full and holds no line: longer than any request.
        if (socket->bytesAvailable() >= requestMax)
            close();
    });
}

#ifdef Q_OS_WIN
std::wstring pipePath(const QString &name)
{
    const QString prefix = QStringLiteral("\\\\.\\pipe\\");
    return (name.startsWith(prefix) ? name : prefix + name).toStdWString();
}

// The current user's TOKEN_USER, its SID inside; empty when it cannot be
// read.
std::vector<BYTE> currentUser()
{
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token))
        return {};
    DWORD size = 0;
    GetTokenInformation(token, TokenUser, nullptr, 0, &size);
    std::vector<BYTE> user(size);
    if (size == 0 || !GetTokenInformation(token, TokenUser, user.data(), size, &size))
        user.clear();
    CloseHandle(token);
    return user;
}

PSID sidOf(std::vector<BYTE> &user)
{
    return reinterpret_cast<TOKEN_USER *>(user.data())->User.Sid;
}

enum class Pipe { Own, None, Foreign };

// The channel whose pipe was last found in another owner's hands: what
// rvp_instance_foreign answers for it without asking again (the other
// program may be between two instances of its pipe just then).
QString foreignChannel;

// Opens the channel's pipe as a client and keeps it only when the current
// user owns it; another owner, or a pipe the user may not open, is
// Foreign. The server may identify this process, not act as it. Waits up
// to `ms` while every instance of the pipe is busy.
Pipe openPipe(const QString &name, int ms, HANDLE *opened)
{
    *opened = INVALID_HANDLE_VALUE;
    std::vector<BYTE> user = currentUser();
    if (user.empty())
        return Pipe::None;
    const std::wstring path = pipePath(name);
    const QDeadlineTimer deadline(ms);
    for (;;) {
        const HANDLE pipe = CreateFileW(path.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
                                        FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                                        nullptr);
        if (pipe != INVALID_HANDLE_VALUE) {
            PSID owner = nullptr;
            PSECURITY_DESCRIPTOR descriptor = nullptr;
            const bool own = GetSecurityInfo(pipe, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION, &owner,
                                             nullptr, nullptr, nullptr, &descriptor)
                    == ERROR_SUCCESS
                && owner != nullptr && EqualSid(owner, sidOf(user));
            LocalFree(descriptor);
            if (own) {
                *opened = pipe;
                return Pipe::Own;
            }
            CloseHandle(pipe);
            foreignChannel = name;
            return Pipe::Foreign;
        }
        const DWORD error = GetLastError();
        if (error == ERROR_ACCESS_DENIED) {
            foreignChannel = name;
            return Pipe::Foreign;
        }
        const qint64 left = deadline.remainingTime();
        if (error != ERROR_PIPE_BUSY || left <= 0 || !WaitNamedPipeW(path.c_str(), static_cast<DWORD>(left)))
            return Pipe::None;
    }
}

// The server: one instance of the pipe waits for a client at a time. A
// connected one goes to a QLocalSocket once the next instance is made, so
// the name stays this process's. Lives as long as the process.
class PipeServer
{
public:
    bool listen(const QString &name);

private:
    bool wait(bool first);
    void accept();

    std::wstring path;
    std::vector<BYTE> user;
    std::vector<BYTE> acl;
    SECURITY_DESCRIPTOR descriptor{};
    SECURITY_ATTRIBUTES attributes{};
    HANDLE pipe = INVALID_HANDLE_VALUE;
    OVERLAPPED overlapped{};
    bool connectedAtOnce = false;
    HANDLE event = nullptr;
    QWinEventNotifier *notifier = nullptr;
};

bool PipeServer::listen(const QString &name)
{
    path = pipePath(name);
    user = currentUser();
    if (user.empty())
        return false;
    // The current user owns the pipe and alone may open it or add
    // instances to it.
    const PSID sid = sidOf(user);
    const DWORD size = sizeof(ACL) + sizeof(ACCESS_ALLOWED_ACE) - sizeof(DWORD) + GetLengthSid(sid);
    acl.assign(size, 0);
    auto *list = reinterpret_cast<ACL *>(acl.data());
    if (!InitializeAcl(list, size, ACL_REVISION) || !AddAccessAllowedAce(list, ACL_REVISION, FILE_ALL_ACCESS, sid)
        || !InitializeSecurityDescriptor(&descriptor, SECURITY_DESCRIPTOR_REVISION)
        || !SetSecurityDescriptorOwner(&descriptor, sid, FALSE)
        || !SetSecurityDescriptorDacl(&descriptor, TRUE, list, FALSE))
        return false;
    attributes.nLength = sizeof(attributes);
    attributes.lpSecurityDescriptor = &descriptor;
    attributes.bInheritHandle = FALSE;
    event = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (event == nullptr)
        return false;
    if (!wait(true)) {
        CloseHandle(event);
        event = nullptr;
        return false;
    }
    notifier = new QWinEventNotifier(event, QCoreApplication::instance());
    QObject::connect(notifier, &QWinEventNotifier::activated, notifier, [this] { accept(); });
    return true;
}

// Makes an instance of the pipe (the first one: only when the name is
// free) and waits for a client on it.
bool PipeServer::wait(bool first)
{
    const DWORD open = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | (first ? FILE_FLAG_FIRST_PIPE_INSTANCE : 0);
    const HANDLE made = CreateNamedPipeW(path.c_str(), open,
                                         PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT
                                             | PIPE_REJECT_REMOTE_CLIENTS,
                                         PIPE_UNLIMITED_INSTANCES, 0, 0, 0, &attributes);
    if (made == INVALID_HANDLE_VALUE)
        return false;
    overlapped = {};
    overlapped.hEvent = event;
    connectedAtOnce = ConnectNamedPipe(made, &overlapped) != FALSE;
    if (!connectedAtOnce) {
        const DWORD error = GetLastError();
        // A client that came between the two calls.
        if (error == ERROR_PIPE_CONNECTED) {
            connectedAtOnce = true;
        } else if (error != ERROR_IO_PENDING) {
            CloseHandle(made);
            return false;
        }
    }
    if (connectedAtOnce)
        SetEvent(event);
    pipe = made;
    return true;
}

void PipeServer::accept()
{
    ResetEvent(event);
    const HANDLE connected = pipe;
    if (connected == INVALID_HANDLE_VALUE)
        return;
    DWORD unused = 0;
    const bool ok = connectedAtOnce || GetOverlappedResult(connected, &overlapped, &unused, FALSE);
    if (!ok && GetLastError() == ERROR_IO_INCOMPLETE)
        return;
    pipe = INVALID_HANDLE_VALUE;
    if (!wait(false))
        qWarning("rvp: the instance channel stopped listening (error %lu)", GetLastError());
    if (!ok) {
        CloseHandle(connected);
        return;
    }
    auto *socket = new QLocalSocket(notifier);
    socket->setSocketDescriptor(reinterpret_cast<qintptr>(connected));
    serve(socket);
}

PipeServer *pipeServer = nullptr;
#else
QLocalServer *localServer = nullptr;
#endif

// The self-test's look at the limits (Tools::probe_instance): when the
// server closed each probe, in ms from the start, -1 while it is open.
// The channel this process listens on (Tools::probe_instance).
QString listenedChannel;

struct Probe
{
    QElapsedTimer clock;
    qint64 flood = -1;
    qint64 idle = -1;
};
Probe probe;

// One connection of the self-test's probe to this process's own channel:
// `flood` writes more than a request may hold, without a line; the other
// sends nothing. Each is timed from its own start to its end. The idle
// one starts when the flood's has ended: this thread is the server's too,
// and on Windows a second client that finds every pipe instance taken
// waits for one in a call that keeps this thread from making it.
void startProbe(bool flood)
{
    probe.clock.start();
    auto *socket = new QLocalSocket(QCoreApplication::instance());
    QObject::connect(socket, &QLocalSocket::disconnected, socket, [socket, flood] {
        (flood ? probe.flood : probe.idle) = probe.clock.elapsed();
        socket->deleteLater();
        if (flood)
            startProbe(false);
    });
    if (flood)
        QObject::connect(socket, &QLocalSocket::connected, socket,
                         [socket] { socket->write(QByteArray(requestMax + 4096, 'x')); });
    socket->connectToServer(listenedChannel);
}

} // namespace

void Tools::instance_receiver(QObject *receiver) const
{
    instanceReceiver = receiver;
    const QStringList waiting = std::exchange(instanceQueue, {});
    for (const QString &line : waiting)
        deliverRequest(line);
}

bool Tools::probe_instance() const
{
    if (listenedChannel.isEmpty())
        return false;
    probe = Probe();
    startProbe(true);
    return true;
}

QString Tools::instance_probe() const
{
    return QString::fromUtf8(
        QJsonDocument(QJsonObject{{QStringLiteral("flood"), probe.flood}, {QStringLiteral("idle"), probe.idle}})
            .toJson(QJsonDocument::Compact));
}

bool rvp_instance_send(rust::Str name, rust::Str message)
{
    QLocalSocket socket;
#ifdef Q_OS_WIN
    HANDLE pipe = INVALID_HANDLE_VALUE;
    if (openPipe(fromStr(name), 1000, &pipe) != Pipe::Own)
        return false;
    // The socket takes the pipe (overlapped, duplex) and closes it.
    if (!socket.setSocketDescriptor(reinterpret_cast<qintptr>(pipe))) {
        CloseHandle(pipe);
        return false;
    }
#else
    socket.connectToServer(fromStr(name));
    if (!socket.waitForConnected(1000))
        return false;
#endif
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

bool rvp_instance_running(rust::Str name)
{
#ifdef Q_OS_WIN
    HANDLE pipe = INVALID_HANDLE_VALUE;
    if (openPipe(fromStr(name), 500, &pipe) != Pipe::Own)
        return false;
    CloseHandle(pipe);
    return true;
#else
    QLocalSocket socket;
    socket.connectToServer(fromStr(name));
    const bool running = socket.waitForConnected(500);
    socket.abort();
    return running;
#endif
}

bool rvp_instance_foreign(rust::Str name)
{
#ifdef Q_OS_WIN
    const QString channel = fromStr(name);
    if (foreignChannel == channel)
        return true;
    HANDLE pipe = INVALID_HANDLE_VALUE;
    const Pipe found = openPipe(channel, 500, &pipe);
    if (found == Pipe::Own)
        CloseHandle(pipe);
    return found == Pipe::Foreign;
#else
    static_cast<void>(name);
    return false;
#endif
}

bool rvp_instance_listen(rust::Str name)
{
    const QString channel = fromStr(name);
#ifdef Q_OS_WIN
    if (pipeServer)
        return true;
    auto *server = new PipeServer();
    if (!server->listen(channel)) {
        delete server;
        return false;
    }
    pipeServer = server;
    listenedChannel = channel;
    return true;
#else
    if (localServer)
        return true;
    auto *server = new QLocalServer(QCoreApplication::instance());
    server->setSocketOptions(QLocalServer::UserAccessOption);
    bool listening = server->listen(channel);
    // A socket file nobody accepts on: left by a process that died. One
    // that accepts belongs to a running RVP that did not answer in time
    // (its window thread was busy): it keeps its channel, and this one
    // runs on its own.
    if (!listening && server->serverError() == QAbstractSocket::AddressInUseError
        && !rvp_instance_running(name)) {
        QLocalServer::removeServer(channel);
        listening = server->listen(channel);
    }
    if (!listening) {
        delete server;
        return false;
    }
    localServer = server;
    listenedChannel = channel;
    QObject::connect(server, &QLocalServer::newConnection, server, [server] {
        while (QLocalSocket *socket = server->nextPendingConnection())
            serve(socket);
    });
    return true;
#endif
}
