// Text to speech for M14's Accessibility page: Windows' speech (SAPI 5)
// reads RVP's notices, text subtitles and tooltips aloud with the chosen
// voice, output, rate and volume. Each new text replaces the one being
// read. Elsewhere there are no voices (Linux's speech is goal S's).

#include "shell.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>

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

} // namespace

rust::String rvp_tts_voices()
{
#ifdef Q_OS_WIN
    const QByteArray json = namesJson(SPCAT_VOICES).toUtf8();
    return rust::String(json.constData(), static_cast<std::size_t>(json.size()));
#else
    return rust::String("[]");
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
#else
    (void)text;
    (void)voiceName;
    (void)output;
    (void)rate;
    (void)volume;
    return false;
#endif
}
