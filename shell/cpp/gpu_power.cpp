// The graphics cards' power for the GPU power budget (D17,
// crates/app/src/gpu_power.rs), read by src/gpu_power.rs's thread:
//
// - NVIDIA's cards through NVML: nvml.dll from System32 on Windows (older
//   drivers kept it in NVSMI), libnvidia-ml.so.1 on Linux. The draw is
//   nvmlDeviceGetPowerUsage (milliwatts; a one-second average on Ampere
//   and later), the rated power the default power limit, else the one in
//   force. None of these calls needs administrator rights. NVML is
//   loaded on the first reading, never linked: RVP runs without it. The
//   load, clocks, temperature and performance state go to the log at
//   Debug; a driver's NVML without one of those calls leaves it out.
// - On Windows, AMD's cards through Windows' own counter: D3DKMT's adapter
//   performance data (WDDM 2.4) gives the draw in tenths of a percent of
//   the card's power; a driver that does not fill it reports 0. AMD's
//   ADLX SDK would give watts, but its licence forbids using it in
//   software under a free licence such as RVP's GPL. With the environment
//   variable RVP_GPU_POWER_ADAPTERS=all every adapter's counter is read
//   (a check against NVML's figures; the VM's adapters have no AMD card).
//
// Only reader threads call these functions; a mutex keeps a reader that
// is ending and a new one apart.

#include "rvp-shell/cpp/shell.h"

#include <cstdio>
#include <mutex>
#include <string>

#ifdef _WIN32
#include <vector>

#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
// NTSTATUS, which d3dkmthk.h needs.
#include <winternl.h>

#include <d3dkmthk.h>
#include <dxgi.h>
#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "gdi32.lib")
#pragma comment(lib, "advapi32.lib")
#else
#include <dlfcn.h>
#endif

namespace {

std::mutex &lock()
{
    static std::mutex mutex;
    return mutex;
}

// A card's load (percent), graphics and memory clocks (MHz), temperature
// (degrees Celsius) and performance state; -1 where the reader has none.
struct CardState {
    long long load = -1;
    long long graphics = -1;
    long long memory = -1;
    long long temperature = -1;
    long long pstate = -1;
};

// One reading as src/gpu_power.rs parses it: `unit<TAB>draw<TAB>rated<TAB>
// load<TAB>graphics<TAB>memory<TAB>temperature<TAB>pstate<TAB>name`, the
// unit `W` or `%`.
void append(std::string &lines, char unit, double draw, double rated, const CardState &state,
            const char *name)
{
    char numbers[160];
    std::snprintf(numbers, sizeof numbers, "%c\t%.3f\t%.3f\t%lld\t%lld\t%lld\t%lld\t%lld\t", unit,
                  draw, rated, state.load, state.graphics, state.memory, state.temperature,
                  state.pstate);
    lines += numbers;
    for (const char *c = name; *c != '\0'; ++c)
        lines += (*c == '\t' || *c == '\n' || *c == '\r') ? ' ' : *c;
    lines += '\n';
}

// NVML's types and the calls RVP makes (NVML API reference).
using NvmlReturn = int;
using NvmlDevice = struct NvmlDeviceSt *;
constexpr NvmlReturn nvmlSuccess = 0;
struct NvmlUtilization {
    unsigned int gpu;
    unsigned int memory;
};
constexpr int nvmlClockGraphics = 0;
constexpr int nvmlClockMemory = 2;
constexpr int nvmlTemperatureGpu = 0;

struct Nvml {
    void *library = nullptr;
    // Loading failed, or NVML would not start: not tried again until the
    // reader ends.
    bool failed = false;
    bool started = false;
    NvmlReturn (*init)() = nullptr;
    NvmlReturn (*shutdown)() = nullptr;
    NvmlReturn (*count)(unsigned int *) = nullptr;
    NvmlReturn (*handle)(unsigned int, NvmlDevice *) = nullptr;
    NvmlReturn (*power)(NvmlDevice, unsigned int *) = nullptr;
    NvmlReturn (*enforced)(NvmlDevice, unsigned int *) = nullptr;
    NvmlReturn (*rated)(NvmlDevice, unsigned int *) = nullptr;
    NvmlReturn (*name)(NvmlDevice, char *, unsigned int) = nullptr;
    // For the log; any of them may be missing.
    NvmlReturn (*utilization)(NvmlDevice, NvmlUtilization *) = nullptr;
    NvmlReturn (*clock)(NvmlDevice, int, unsigned int *) = nullptr;
    NvmlReturn (*temperature)(NvmlDevice, int, unsigned int *) = nullptr;
    NvmlReturn (*pstate)(NvmlDevice, int *) = nullptr;
};

Nvml &nvml()
{
    static Nvml state;
    return state;
}

void *openNvml()
{
#ifdef _WIN32
    // System32 only: never a DLL of that name found on the PATH.
    HMODULE module = LoadLibraryExW(L"nvml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (module == nullptr) {
        // Older drivers' NVSMI folder, by its full path; the libraries it
        // needs come from that folder and System32 alone.
        wchar_t path[MAX_PATH];
        const DWORD length = ExpandEnvironmentStringsW(
            L"%ProgramW6432%\\NVIDIA Corporation\\NVSMI\\nvml.dll", path, MAX_PATH);
        if (length > 0 && length <= MAX_PATH && path[0] != L'%')
            module = LoadLibraryExW(path, nullptr,
                                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
    return module;
#else
    return dlopen("libnvidia-ml.so.1", RTLD_NOW | RTLD_LOCAL);
#endif
}

template <typename Function> bool find(void *library, const char *name, Function &function)
{
#ifdef _WIN32
    const FARPROC address = GetProcAddress(static_cast<HMODULE>(library), name);
    function = reinterpret_cast<Function>(reinterpret_cast<void *>(address));
#else
    function = reinterpret_cast<Function>(dlsym(library, name));
#endif
    return function != nullptr;
}

bool startNvml(Nvml &state)
{
    if (state.library == nullptr)
        state.library = openNvml();
    void *library = state.library;
    const bool ready = library != nullptr && find(library, "nvmlInit_v2", state.init)
        && find(library, "nvmlShutdown", state.shutdown)
        && find(library, "nvmlDeviceGetCount_v2", state.count)
        && find(library, "nvmlDeviceGetHandleByIndex_v2", state.handle)
        && find(library, "nvmlDeviceGetPowerUsage", state.power)
        && find(library, "nvmlDeviceGetEnforcedPowerLimit", state.enforced)
        && find(library, "nvmlDeviceGetPowerManagementDefaultLimit", state.rated)
        && find(library, "nvmlDeviceGetName", state.name) && state.init() == nvmlSuccess;
    if (ready) {
        // For the log: a missing one leaves its part out.
        find(library, "nvmlDeviceGetUtilizationRates", state.utilization);
        find(library, "nvmlDeviceGetClockInfo", state.clock);
        find(library, "nvmlDeviceGetTemperature", state.temperature);
        find(library, "nvmlDeviceGetPerformanceState", state.pstate);
    }
    return ready;
}

// The card's state for the log, where NVML has the calls.
CardState nvmlState(const Nvml &library, NvmlDevice device)
{
    CardState state;
    NvmlUtilization utilization{};
    if (library.utilization != nullptr && library.utilization(device, &utilization) == nvmlSuccess)
        state.load = utilization.gpu;
    unsigned int value = 0;
    if (library.clock != nullptr && library.clock(device, nvmlClockGraphics, &value) == nvmlSuccess)
        state.graphics = value;
    if (library.clock != nullptr && library.clock(device, nvmlClockMemory, &value) == nvmlSuccess)
        state.memory = value;
    if (library.temperature != nullptr
        && library.temperature(device, nvmlTemperatureGpu, &value) == nvmlSuccess)
        state.temperature = value;
    int pstate = 0;
    if (library.pstate != nullptr && library.pstate(device, &pstate) == nvmlSuccess && pstate >= 0
        && pstate <= 15)
        state.pstate = pstate;
    return state;
}

void readNvml(std::string &lines)
{
    Nvml &state = nvml();
    if (!state.started) {
        if (state.failed)
            return;
        state.started = startNvml(state);
        state.failed = !state.started;
        if (!state.started)
            return;
    }
    unsigned int count = 0;
    if (state.count(&count) != nvmlSuccess)
        return;
    for (unsigned int index = 0; index < count; ++index) {
        NvmlDevice device = nullptr;
        unsigned int draw = 0;
        unsigned int rated = 0;
        if (state.handle(index, &device) != nvmlSuccess
            || state.power(device, &draw) != nvmlSuccess)
            continue;
        if (state.rated(device, &rated) != nvmlSuccess || rated == 0) {
            if (state.enforced(device, &rated) != nvmlSuccess || rated == 0)
                continue;
        }
        char name[96] = {};
        if (state.name(device, name, sizeof name) != nvmlSuccess)
            std::snprintf(name, sizeof name, "NVIDIA GPU %u", index);
        append(lines, 'W', draw / 1000.0, rated / 1000.0, nvmlState(state, device), name);
    }
}

void closeNvml()
{
    Nvml &state = nvml();
    if (state.started)
        state.shutdown();
    state.started = false;
    state.failed = false;
    // The library stays loaded: unloading a driver's library while the
    // process runs gains nothing.
}

#ifdef _WIN32
constexpr UINT amdVendor = 0x1002;

// AMD's cards as Windows sees them, opened on the first reading.
struct Adapter {
    D3DKMT_HANDLE handle = 0;
    std::string name;
};

struct Adapters {
    bool listed = false;
    std::vector<Adapter> list;
};

Adapters &adapters()
{
    static Adapters state;
    return state;
}

bool allAdapters()
{
    char value[8] = {};
    const DWORD length = GetEnvironmentVariableA("RVP_GPU_POWER_ADAPTERS", value, sizeof value);
    return length == 3 && std::string(value) == "all";
}

void listAdapters(Adapters &state)
{
    state.listed = true;
    const bool all = allAdapters();
    IDXGIFactory1 *factory = nullptr;
    if (FAILED(CreateDXGIFactory1(__uuidof(IDXGIFactory1), reinterpret_cast<void **>(&factory))))
        return;
    for (UINT index = 0;; ++index) {
        IDXGIAdapter1 *adapter = nullptr;
        if (FAILED(factory->EnumAdapters1(index, &adapter)) || adapter == nullptr)
            break;
        DXGI_ADAPTER_DESC1 desc{};
        const bool wanted = SUCCEEDED(adapter->GetDesc1(&desc))
            && (all || (desc.VendorId == amdVendor && (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) == 0));
        adapter->Release();
        if (!wanted)
            continue;
        D3DKMT_OPENADAPTERFROMLUID open{};
        open.AdapterLuid = desc.AdapterLuid;
        if (D3DKMTOpenAdapterFromLuid(&open) != 0)
            continue;
        char name[256] = {};
        WideCharToMultiByte(CP_UTF8, 0, desc.Description, -1, name, sizeof name - 1, nullptr,
                            nullptr);
        state.list.push_back({ open.hAdapter, name });
    }
    factory->Release();
}

void readAdapters(std::string &lines)
{
    Adapters &state = adapters();
    if (!state.listed)
        listAdapters(state);
    for (const Adapter &adapter : state.list) {
        D3DKMT_ADAPTER_PERFDATA perf{};
        D3DKMT_QUERYADAPTERINFO query{};
        query.hAdapter = adapter.handle;
        query.Type = KMTQAITYPE_ADAPTERPERFDATA;
        query.pPrivateDriverData = &perf;
        query.PrivateDriverDataSize = sizeof perf;
        if (D3DKMTQueryAdapterInfo(&query) == 0)
            append(lines, '%', perf.Power / 10.0, 100.0, CardState{}, adapter.name.c_str());
    }
}

void closeAdapters()
{
    Adapters &state = adapters();
    for (const Adapter &adapter : state.list) {
        D3DKMT_CLOSEADAPTER close{};
        close.hAdapter = adapter.handle;
        D3DKMTCloseAdapter(&close);
    }
    state.list.clear();
    state.listed = false;
}
#endif

} // namespace

rust::String rvp_gpu_power_read(bool nvidia)
{
    const std::lock_guard<std::mutex> guard(lock());
    std::string lines;
    if (nvidia)
        readNvml(lines);
    else
        // Its handles would keep a card the system put to sleep awake.
        closeNvml();
#ifdef _WIN32
    readAdapters(lines);
#endif
    return rust::String(lines);
}

void rvp_gpu_power_close()
{
    const std::lock_guard<std::mutex> guard(lock());
    closeNvml();
#ifdef _WIN32
    closeAdapters();
#endif
}

rust::String rvp_hardware()
{
    std::string text;
#ifdef _WIN32
    wchar_t cpu[256] = {};
    DWORD size = sizeof cpu;
    if (RegGetValueW(HKEY_LOCAL_MACHINE, L"HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0",
                     L"ProcessorNameString", RRF_RT_REG_SZ, nullptr, cpu, &size)
        == ERROR_SUCCESS) {
        char name[512] = {};
        WideCharToMultiByte(CP_UTF8, 0, cpu, -1, name, sizeof name - 1, nullptr, nullptr);
        text += name;
    }
    IDXGIFactory1 *factory = nullptr;
    if (SUCCEEDED(CreateDXGIFactory1(__uuidof(IDXGIFactory1), reinterpret_cast<void **>(&factory)))) {
        for (UINT index = 0;; ++index) {
            IDXGIAdapter1 *adapter = nullptr;
            if (FAILED(factory->EnumAdapters1(index, &adapter)) || adapter == nullptr)
                break;
            DXGI_ADAPTER_DESC1 desc{};
            LARGE_INTEGER version{};
            const bool listed = SUCCEEDED(adapter->GetDesc1(&desc))
                && (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) == 0;
            // The user-mode driver's version (the only interface DXGI
            // answers this for).
            const bool known = listed
                && SUCCEEDED(adapter->CheckInterfaceSupport(__uuidof(IDXGIDevice), &version));
            adapter->Release();
            if (!listed)
                continue;
            char card[256] = {};
            WideCharToMultiByte(CP_UTF8, 0, desc.Description, -1, card, sizeof card - 1, nullptr,
                                nullptr);
            const auto part = [&](LONG high, unsigned shift) {
                return known ? static_cast<unsigned>((static_cast<unsigned long>(high) >> shift) & 0xFFFFu)
                             : 0u;
            };
            char line[400] = {};
            std::snprintf(line, sizeof line, "; %s %04x:%04x driver %u.%u.%u.%u", card,
                          static_cast<unsigned>(desc.VendorId), static_cast<unsigned>(desc.DeviceId),
                          part(version.HighPart, 16), part(version.HighPart, 0),
                          part(static_cast<LONG>(version.LowPart), 16),
                          part(static_cast<LONG>(version.LowPart), 0));
            text += line;
        }
        factory->Release();
    }
#endif
    return rust::String(text);
}
