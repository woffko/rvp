// Text to speech for M14's Accessibility page: Windows' speech (SAPI 5)
// reads RVP's notices, text subtitles and tooltips aloud with the chosen
// voice, output, rate and volume. Each new text replaces the one being
// read. On Linux speech-dispatcher reads them through its client library
// (libspeechd.so.2, loaded at run time: the packages only recommend it),
// on a thread of its own with one connection (opening it may start the
// server, which takes seconds; the window's thread never waits for it):
// its voices by their exact names (spd-say -L's table, cut at the first
// space, lost "English (America)"), no choice of output (the server plays
// on the desktop's sound), and a new text cancels only RVP's own (spd-say
// -C cancelled every client's, a screen reader's too; the Linux audit's
// M4).

#include "shell.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QStringList>

#ifdef Q_OS_LINUX
#include <dlfcn.h>

#include <algorithm>
#include <condition_variable>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <thread>
#endif

#ifdef Q_OS_WIN
#include <windows.h>
#include <sapi.h>
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "sapi.lib")
#endif

namespace {

#ifdef Q_OS_WIN
// The tokens of a SAPI category (voices or audio outputs), with their
// descriptions.
QList<QPair<QString, ISpObjectToken *>> tokens(const wchar_t *category)
{
    QList<QPair<QString, ISpObjectToken *>> found;
    ISpObjectTokenCategory *kind = nullptr;
    if (FAILED(CoCreateInstance(CLSID_SpObjectTokenCategory, nullptr, CLSCTX_ALL, IID_PPV_ARGS(&kind))))
        return found;
    IEnumSpObjectTokens *list = nullptr;
    if (SUCCEEDED(kind->SetId(category, FALSE)) && SUCCEEDED(kind->EnumTokens(nullptr, nullptr, &list))) {
        ISpObjectToken *token = nullptr;
        while (list->Next(1, &token, nullptr) == S_OK) {
            wchar_t *name = nullptr;
            if (SUCCEEDED(token->GetStringValue(nullptr, &name)) && name != nullptr) {
                found.append({QString::fromWCharArray(name), token});
                CoTaskMemFree(name);
            } else {
                token->Release();
            }
        }
        list->Release();
    }
    kind->Release();
    return found;
}

QString namesJson(const wchar_t *category)
{
    QJsonArray names;
    for (auto &[name, token] : tokens(category)) {
        names.append(name);
        token->Release();
    }
    return QString::fromUtf8(QJsonDocument(names).toJson(QJsonDocument::Compact));
}

ISpVoice *voice()
{
    static ISpVoice *made = nullptr;
    if (made == nullptr && FAILED(CoCreateInstance(CLSID_SpVoice, nullptr, CLSCTX_ALL, IID_PPV_ARGS(&made))))
        made = nullptr;
    return made;
}

// Gives `speaker` the token of `category` named `wanted` (empty: SAPI's
// default stays).
template <typename Use>
void choose(const wchar_t *category, const QString &wanted, Use use)
{
    if (wanted.isEmpty())
        return;
    for (auto &[name, token] : tokens(category)) {
        if (name == wanted)
            use(token);
        token->Release();
    }
}
#endif

#ifdef Q_OS_LINUX
// libspeechd's calls RVP makes (its ABI since 0.8; no header needed).
struct SpdVoice {
    char *name;
    char *language;
    char *variant;
};
constexpr int kSpdModeSingle = 0;
constexpr int kSpdText = 3;

struct Speechd {
    void *(*open)(const char *, const char *, const char *, int) = nullptr;
    int (*say)(void *, int, const char *) = nullptr;
    int (*cancel)(void *) = nullptr;
    int (*rate)(void *, int) = nullptr;
    int (*volume)(void *, int) = nullptr;
    int (*voice)(void *, const char *) = nullptr;
    SpdVoice **(*voices)(void *) = nullptr;
    void (*freeVoices)(SpdVoice **) = nullptr;
};

struct Speech {
    std::string text;
    std::string voice;
    int rate = 0;
    int volume = 0;
};

// The speech thread's state, shared with the window's thread under `lock`.
struct Speaker {
    std::mutex lock;
    std::condition_variable wake;
    bool started = false;
    // "loading" until the thread has connected, then "ready" or "none".
    const char *state = "loading";
    std::string voicesJson = "[]";
    // Only the newest text waits: an older one would be cancelled anyway.
    std::optional<Speech> pending;
};

// Never destroyed: the speech thread waits on its condition variable for
// the whole run, and destroying one with a waiter blocks (glibc), which
// held RVP's exit up for good (M14's self-test in the kit).
Speaker &speaker()
{
    static Speaker *state = new Speaker;
    return *state;
}

template <typename T>
bool resolve(void *library, const char *name, T &function)
{
    function = reinterpret_cast<T>(dlsym(library, name));
    return function != nullptr;
}

bool loadSpeechd(Speechd &api)
{
    void *library = dlopen("libspeechd.so.2", RTLD_NOW | RTLD_LOCAL);
    return library != nullptr && resolve(library, "spd_open", api.open)
        && resolve(library, "spd_say", api.say) && resolve(library, "spd_cancel", api.cancel)
        && resolve(library, "spd_set_voice_rate", api.rate)
        && resolve(library, "spd_set_volume", api.volume)
        && resolve(library, "spd_set_synthesis_voice", api.voice)
        && resolve(library, "spd_list_synthesis_voices", api.voices)
        && resolve(library, "free_spd_voices", api.freeVoices);
}

// The voices' names, each once, sorted.
std::string voicesJson(const Speechd &api, void *connection)
{
    std::set<QString> names;
    if (SpdVoice **voices = api.voices(connection)) {
        for (SpdVoice **voice = voices; *voice != nullptr; ++voice) {
            if ((*voice)->name != nullptr && (*voice)->name[0] != '\0')
                names.insert(QString::fromUtf8((*voice)->name));
        }
        api.freeVoices(voices);
    }
    QJsonArray array;
    for (const QString &name : names)
        array.append(name);
    return QJsonDocument(array).toJson(QJsonDocument::Compact).toStdString();
}

// The speech thread: connects, lists the voices, then reads each text.
void speak()
{
    Speaker &state = speaker();
    Speechd api;
    void *connection = loadSpeechd(api) ? api.open("rvp", "main", nullptr, kSpdModeSingle) : nullptr;
    {
        std::lock_guard<std::mutex> guard(state.lock);
        state.state = connection != nullptr ? "ready" : "none";
        if (connection != nullptr)
            state.voicesJson = voicesJson(api, connection);
    }
    if (connection == nullptr)
        return;
    std::string voice;
    for (;;) {
        Speech next;
        {
            std::unique_lock<std::mutex> guard(state.lock);
            state.wake.wait(guard, [&state] { return state.pending.has_value(); });
            next = std::move(*state.pending);
            state.pending.reset();
        }
        api.cancel(connection);
        api.rate(connection, next.rate);
        api.volume(connection, next.volume);
        // The server's default voice stays unless one is chosen.
        if (!next.voice.empty() && next.voice != voice)
            api.voice(connection, next.voice.c_str());
        voice = next.voice;
        if (!next.text.empty())
            api.say(connection, kSpdText, next.text.c_str());
    }
}

void startSpeaker()
{
    Speaker &state = speaker();
    std::lock_guard<std::mutex> guard(state.lock);
    if (state.started)
        return;
    state.started = true;
    // It lives as long as RVP: waiting, it holds nothing the exit needs.
    std::thread(speak).detach();
}
#endif

} // namespace

rust::String rvp_tts_voices()
{
#ifdef Q_OS_WIN
    const QByteArray json = namesJson(SPCAT_VOICES).toUtf8();
    return rust::String(json.constData(), static_cast<std::size_t>(json.size()));
#elif defined(Q_OS_LINUX)
    // The thread lists them once it has connected; until then none.
    startSpeaker();
    Speaker &state = speaker();
    std::lock_guard<std::mutex> guard(state.lock);
    return rust::String(state.voicesJson);
#else
    return rust::String("[]");
#endif
}

rust::String rvp_tts_state()
{
#ifdef Q_OS_LINUX
    startSpeaker();
    Speaker &state = speaker();
    std::lock_guard<std::mutex> guard(state.lock);
    return rust::String(state.state);
#else
    return rust::String("ready");
#endif
}

rust::String rvp_tts_outputs()
{
#ifdef Q_OS_WIN
    const QByteArray json = namesJson(SPCAT_AUDIOOUT).toUtf8();
    return rust::String(json.constData(), static_cast<std::size_t>(json.size()));
#else
    return rust::String("[]");
#endif
}

bool rvp_tts_speak(rust::Str text, rust::Str voiceName, rust::Str output, std::int32_t rate,
                   std::int32_t volume)
{
#ifdef Q_OS_WIN
    ISpVoice *speaker = voice();
    if (speaker == nullptr)
        return false;
    const auto qt = [](rust::Str value) {
        return QString::fromUtf8(value.data(), static_cast<qsizetype>(value.size()));
    };
    choose(SPCAT_VOICES, qt(voiceName), [&](ISpObjectToken *token) { speaker->SetVoice(token); });
    choose(SPCAT_AUDIOOUT, qt(output), [&](ISpObjectToken *token) { speaker->SetOutput(token, TRUE); });
    speaker->SetRate(static_cast<long>(rate));
    speaker->SetVolume(static_cast<USHORT>(volume));
    const std::wstring wide = qt(text).toStdWString();
    return SUCCEEDED(speaker->Speak(wide.c_str(), SPF_ASYNC | SPF_PURGEBEFORESPEAK | SPF_IS_NOT_XML, nullptr));
#elif defined(Q_OS_LINUX)
    (void)output;
    startSpeaker();
    Speaker &state = speaker();
    {
        std::lock_guard<std::mutex> guard(state.lock);
        if (std::string(state.state) == "none")
            return false;
        // SAPI's rate (-10 to 10) and volume (0 to 100) in
        // speech-dispatcher's -100 to 100.
        state.pending = Speech{std::string(text.data(), text.size()),
                               std::string(voiceName.data(), voiceName.size()),
                               std::clamp(rate, -10, 10) * 10, std::clamp(volume, 0, 100) * 2 - 100};
    }
    state.wake.notify_one();
    return true;
#else
    (void)text;
    (void)voiceName;
    (void)output;
    (void)rate;
    (void)volume;
    return false;
#endif
}
