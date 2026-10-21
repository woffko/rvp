// The current user's registry (M14's Association page): RVP's types, verbs
// and AutoPlay handlers under HKEY_CURRENT_USER, as rvp_app::registration
// lists them, and the shell told when they change. Nothing elsewhere.

#include "shell.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <shlobj_core.h>
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "shell32.lib")
#endif

namespace {

#ifdef Q_OS_WIN
std::wstring wide(rust::Str text)
{
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size())).toStdWString();
}
#endif

} // namespace

bool rvp_registry_set(rust::Str key, rust::Str name, rust::Str value)
{
#ifdef Q_OS_WIN
    const std::wstring path = wide(key);
    const std::wstring field = wide(name);
    const std::wstring data = wide(value);
    return RegSetKeyValueW(HKEY_CURRENT_USER, path.c_str(), field.empty() ? nullptr : field.c_str(), REG_SZ,
                           data.c_str(), static_cast<DWORD>((data.size() + 1) * sizeof(wchar_t)))
        == ERROR_SUCCESS;
#else
    (void)key;
    (void)name;
    (void)value;
    return false;
#endif
}

bool rvp_registry_delete_value(rust::Str key, rust::Str name)
{
#ifdef Q_OS_WIN
    const std::wstring path = wide(key);
    const std::wstring field = wide(name);
    const LSTATUS status = RegDeleteKeyValueW(HKEY_CURRENT_USER, path.c_str(), field.c_str());
    return status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND;
#else
    (void)key;
    (void)name;
    return false;
#endif
}

bool rvp_registry_delete_key(rust::Str key)
{
#ifdef Q_OS_WIN
    const std::wstring path = wide(key);
    const LSTATUS status = RegDeleteTreeW(HKEY_CURRENT_USER, path.c_str());
    if (status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND)
        return false;
    // RegDeleteTreeW leaves the key itself.
    const LSTATUS removed = RegDeleteKeyW(HKEY_CURRENT_USER, path.c_str());
    return removed == ERROR_SUCCESS || removed == ERROR_FILE_NOT_FOUND;
#else
    (void)key;
    return false;
#endif
}

bool rvp_registry_has_key(rust::Str key)
{
#ifdef Q_OS_WIN
    const std::wstring path = wide(key);
    HKEY opened = nullptr;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, path.c_str(), 0, KEY_READ, &opened) != ERROR_SUCCESS)
        return false;
    RegCloseKey(opened);
    return true;
#else
    (void)key;
    return false;
#endif
}

bool Tools::registry_has_value(const QString &key, const QString &name) const
{
#ifdef Q_OS_WIN
    const std::wstring path = key.toStdWString();
    const std::wstring field = name.toStdWString();
    return RegGetValueW(HKEY_CURRENT_USER, path.c_str(), field.c_str(), RRF_RT_ANY, nullptr, nullptr,
                        nullptr)
        == ERROR_SUCCESS;
#else
    (void)key;
    (void)name;
    return false;
#endif
}

void rvp_associations_changed()
{
#ifdef Q_OS_WIN
    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, nullptr, nullptr);
#endif
}
