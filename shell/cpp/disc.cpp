// DVD menus (M16): see disc.h. libdvdnav's own declarations are copied
// here as the few prototypes RVP calls; its types stay opaque, apart from
// the highlight area, whose layout libdvdnav documents.
#include "disc.h"

#include "rvp-shell/src/disc_ffi.cxxqt.h"

#include <mutex>
#include <string>
#include <vector>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace {

struct dvdnav_t;
struct pci_t;
using dvdnav_status_t = int32_t;
constexpr dvdnav_status_t STATUS_OK = 1;

struct HighlightArea {
    uint32_t palette;
    uint16_t sx, sy, ex, ey;
    uint32_t pts;
    uint32_t buttonN;
};

struct Api {
    void *library = nullptr;
    dvdnav_status_t (*open)(dvdnav_t **, const char *) = nullptr;
    dvdnav_status_t (*close)(dvdnav_t *) = nullptr;
    const char *(*error)(dvdnav_t *) = nullptr;
    const char *(*version)() = nullptr;
    dvdnav_status_t (*readahead)(dvdnav_t *, int32_t) = nullptr;
    dvdnav_status_t (*pgc_positioning)(dvdnav_t *, int32_t) = nullptr;
    dvdnav_status_t (*menu_language)(dvdnav_t *, char *) = nullptr;
    dvdnav_status_t (*audio_language)(dvdnav_t *, char *) = nullptr;
    dvdnav_status_t (*spu_language)(dvdnav_t *, char *) = nullptr;
    dvdnav_status_t (*next)(dvdnav_t *, uint8_t *, int32_t *, int32_t *) = nullptr;
    dvdnav_status_t (*still_skip)(dvdnav_t *) = nullptr;
    dvdnav_status_t (*wait_skip)(dvdnav_t *) = nullptr;
    dvdnav_status_t (*titles)(dvdnav_t *, int32_t *) = nullptr;
    dvdnav_status_t (*title_info)(dvdnav_t *, int32_t *, int32_t *) = nullptr;
    dvdnav_status_t (*position)(dvdnav_t *, uint32_t *, uint32_t *) = nullptr;
    dvdnav_status_t (*highlight)(dvdnav_t *, int32_t *) = nullptr;
    pci_t *(*pci)(dvdnav_t *) = nullptr;
    dvdnav_status_t (*area)(pci_t *, int32_t, int32_t, HighlightArea *) = nullptr;
    dvdnav_status_t (*upper)(dvdnav_t *, pci_t *) = nullptr;
    dvdnav_status_t (*lower)(dvdnav_t *, pci_t *) = nullptr;
    dvdnav_status_t (*left)(dvdnav_t *, pci_t *) = nullptr;
    dvdnav_status_t (*right)(dvdnav_t *, pci_t *) = nullptr;
    dvdnav_status_t (*activate)(dvdnav_t *, pci_t *) = nullptr;
    dvdnav_status_t (*mouse_select)(dvdnav_t *, pci_t *, int32_t, int32_t) = nullptr;
    dvdnav_status_t (*mouse_activate)(dvdnav_t *, pci_t *, int32_t, int32_t) = nullptr;
    dvdnav_status_t (*menu)(dvdnav_t *, int32_t) = nullptr;
    dvdnav_status_t (*go_up)(dvdnav_t *) = nullptr;
    dvdnav_status_t (*sector_search)(dvdnav_t *, int64_t, int32_t) = nullptr;
    dvdnav_status_t (*angle_info)(dvdnav_t *, int32_t *, int32_t *) = nullptr;
    int8_t (*audio)(dvdnav_t *) = nullptr;
    int8_t (*spu)(dvdnav_t *) = nullptr;
    int8_t (*vmgm)(dvdnav_t *) = nullptr;
    int8_t (*vtsm)(dvdnav_t *) = nullptr;
    int8_t (*vts)(dvdnav_t *) = nullptr;
};

void *openLibrary()
{
#ifdef _WIN32
    // RVP's folder (and System32) only, as the process's search is.
    return LoadLibraryExW(L"libdvdnav-4.dll", nullptr,
                          LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
#else
    void *library = dlopen("libdvdnav.so.4", RTLD_NOW | RTLD_LOCAL);
    return library != nullptr ? library : dlopen("libdvdnav.so", RTLD_NOW | RTLD_LOCAL);
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

Api &api()
{
    static Api state;
    return state;
}

bool loadApi(std::string &why)
{
    static std::once_flag once;
    static bool loaded = false;
    static std::string reason;
    std::call_once(once, [] {
        Api &a = api();
        a.library = openLibrary();
        if (a.library == nullptr) {
            reason = "libdvdnav is not installed";
            return;
        }
        loaded = find(a.library, "dvdnav_open", a.open) && find(a.library, "dvdnav_close", a.close)
            && find(a.library, "dvdnav_err_to_string", a.error)
            && find(a.library, "dvdnav_version", a.version)
            && find(a.library, "dvdnav_set_readahead_flag", a.readahead)
            && find(a.library, "dvdnav_set_PGC_positioning_flag", a.pgc_positioning)
            && find(a.library, "dvdnav_menu_language_select", a.menu_language)
            && find(a.library, "dvdnav_audio_language_select", a.audio_language)
            && find(a.library, "dvdnav_spu_language_select", a.spu_language)
            && find(a.library, "dvdnav_get_next_block", a.next)
            && find(a.library, "dvdnav_still_skip", a.still_skip)
            && find(a.library, "dvdnav_wait_skip", a.wait_skip)
            && find(a.library, "dvdnav_get_number_of_titles", a.titles)
            && find(a.library, "dvdnav_current_title_info", a.title_info)
            && find(a.library, "dvdnav_get_position", a.position)
            && find(a.library, "dvdnav_get_current_highlight", a.highlight)
            && find(a.library, "dvdnav_get_current_nav_pci", a.pci)
            && find(a.library, "dvdnav_get_highlight_area", a.area)
            && find(a.library, "dvdnav_upper_button_select", a.upper)
            && find(a.library, "dvdnav_lower_button_select", a.lower)
            && find(a.library, "dvdnav_left_button_select", a.left)
            && find(a.library, "dvdnav_right_button_select", a.right)
            && find(a.library, "dvdnav_button_activate", a.activate)
            && find(a.library, "dvdnav_mouse_select", a.mouse_select)
            && find(a.library, "dvdnav_mouse_activate", a.mouse_activate)
            && find(a.library, "dvdnav_menu_call", a.menu) && find(a.library, "dvdnav_go_up", a.go_up)
            && find(a.library, "dvdnav_sector_search", a.sector_search)
            && find(a.library, "dvdnav_get_angle_info", a.angle_info)
            && find(a.library, "dvdnav_get_active_audio_stream", a.audio)
            && find(a.library, "dvdnav_get_active_spu_stream", a.spu)
            && find(a.library, "dvdnav_is_domain_vmgm", a.vmgm)
            && find(a.library, "dvdnav_is_domain_vtsm", a.vtsm)
            && find(a.library, "dvdnav_is_domain_vts", a.vts);
        if (!loaded)
            reason = "libdvdnav lacks a function RVP calls";
    });
    why = reason;
    return loaded;
}

bool ok(dvdnav_status_t status)
{
    return status == STATUS_OK;
}

} // namespace

namespace {

dvdnav_t *h(DvdNavigator &nav)
{
    return static_cast<dvdnav_t *>(nav.handle);
}

} // namespace

DvdNavigator::DvdNavigator(void *handle) : handle(handle) {}

DvdNavigator::~DvdNavigator()
{
    if (handle != nullptr)
        api().close(static_cast<dvdnav_t *>(handle));
}

bool rvp_dvdnav_load(rust::String &message)
{
    std::string why;
    const bool loaded = loadApi(why);
    message = loaded ? rust::String(api().version()) : rust::String(why);
    return loaded;
}

std::unique_ptr<DvdNavigator> rvp_dvdnav_open(rust::Str path, rust::Str menu, rust::Str audio,
                                              rust::Str spu, rust::String &error)
{
    std::string why;
    if (!loadApi(why)) {
        error = why;
        return nullptr;
    }
    const std::string name(path.data(), path.size());
    dvdnav_t *nav = nullptr;
    if (!ok(api().open(&nav, name.c_str()))) {
        error = nav != nullptr ? std::string(api().error(nav)) : std::string("cannot open the disc");
        if (nav != nullptr)
            api().close(nav);
        return nullptr;
    }
    // The menus' timing follows what shows: libdvdnav must not read ahead.
    api().readahead(nav, 0);
    // Positions and sector searches count from the title's start, not the
    // chapter's.
    api().pgc_positioning(nav, 1);
    // The machine's language preferences, before it starts.
    const auto prefer = [nav](rust::Str code, dvdnav_status_t (*select)(dvdnav_t *, char *)) {
        if (code.size() != 2)
            return;
        char text[3] = {code.data()[0], code.data()[1], '\0'};
        select(nav, text);
    };
    prefer(menu, api().menu_language);
    prefer(audio, api().audio_language);
    prefer(spu, api().spu_language);
    return std::make_unique<DvdNavigator>(nav);
}

int32_t rvp_dvdnav_next(DvdNavigator &nav, rust::Slice<uint8_t> block, int32_t &length)
{
    if (block.size() < 2048)
        return -1;
    int32_t event = 0;
    length = 0;
    if (!ok(api().next(h(nav), block.data(), &event, &length)))
        return -1;
    return event;
}

bool rvp_dvdnav_still_skip(DvdNavigator &nav)
{
    return ok(api().still_skip(h(nav)));
}

bool rvp_dvdnav_wait_skip(DvdNavigator &nav)
{
    return ok(api().wait_skip(h(nav)));
}

DvdStatus rvp_dvdnav_status(DvdNavigator &nav)
{
    DvdStatus status{};
    const Api &a = api();
    if (a.vmgm(h(nav)) != 0)
        status.domain = 4;
    else if (a.vtsm(h(nav)) != 0)
        status.domain = 8;
    else if (a.vts(h(nav)) != 0)
        status.domain = 2;
    else
        status.domain = 1;
    a.titles(h(nav), &status.titles);
    a.title_info(h(nav), &status.title, &status.part);
    a.highlight(h(nav), &status.button);
    a.angle_info(h(nav), &status.angle, &status.angles);
    status.audio = a.audio(h(nav));
    status.spu = a.spu(h(nav));
    uint32_t position = 0, length = 0;
    if (ok(a.position(h(nav), &position, &length))) {
        status.position = position;
        status.length = length;
    }
    return status;
}

DvdButton rvp_dvdnav_button(DvdNavigator &nav, int32_t mode)
{
    DvdButton button{};
    const Api &a = api();
    int32_t current = 0;
    pci_t *pci = a.pci(h(nav));
    if (pci == nullptr || !ok(a.highlight(h(nav), &current)) || current <= 0)
        return button;
    HighlightArea area{};
    if (!ok(a.area(pci, current, mode, &area)))
        return button;
    button.valid = true;
    button.button = current;
    button.palette = area.palette;
    button.x0 = area.sx;
    button.y0 = area.sy;
    button.x1 = area.ex;
    button.y1 = area.ey;
    return button;
}

bool rvp_dvdnav_select(DvdNavigator &nav, int32_t direction)
{
    const Api &a = api();
    pci_t *pci = a.pci(h(nav));
    if (pci == nullptr)
        return false;
    switch (direction) {
    case 0:
        return ok(a.upper(h(nav), pci));
    case 1:
        return ok(a.lower(h(nav), pci));
    case 2:
        return ok(a.left(h(nav), pci));
    case 3:
        return ok(a.right(h(nav), pci));
    default:
        return false;
    }
}

bool rvp_dvdnav_activate(DvdNavigator &nav)
{
    pci_t *pci = api().pci(h(nav));
    return pci != nullptr && ok(api().activate(h(nav), pci));
}

bool rvp_dvdnav_mouse(DvdNavigator &nav, int32_t x, int32_t y, bool activate)
{
    pci_t *pci = api().pci(h(nav));
    if (pci == nullptr)
        return false;
    return ok(activate ? api().mouse_activate(h(nav), pci, x, y) : api().mouse_select(h(nav), pci, x, y));
}

bool rvp_dvdnav_menu(DvdNavigator &nav, int32_t menu)
{
    return ok(api().menu(h(nav), menu));
}

bool rvp_dvdnav_go_up(DvdNavigator &nav)
{
    return ok(api().go_up(h(nav)));
}

bool rvp_dvdnav_sector_search(DvdNavigator &nav, int64_t sector)
{
    // SEEK_SET.
    return ok(api().sector_search(h(nav), sector, 0));
}

rust::String rvp_dvdnav_error(DvdNavigator &nav)
{
    const char *text = api().error(h(nav));
    return rust::String(text != nullptr ? text : "");
}
