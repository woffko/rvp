// Blu-ray menus (M16): see bluray.h. libbluray's own declarations are
// copied here as the few prototypes and structure layouts RVP uses (the
// layouts are libbluray's public ABI, unchanged from 1.0 to 1.5); its
// handle stays opaque.
#include "bluray.h"

#include "rvp-shell/src/bluray_ffi.cxxqt.h"

#include <string>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace {

struct BLURAY;

struct BdEventC {
    uint32_t event;
    uint32_t param;
};

struct PaletteEntry {
    uint8_t y, cr, cb, t;
};

struct RleElement {
    uint16_t len;
    uint16_t color;
};

struct Overlay {
    int64_t pts;
    uint8_t plane;
    uint8_t cmd;
    uint8_t palette_update_flag;
    uint16_t x, y, w, h;
    const PaletteEntry *palette;
    const RleElement *img;
};

struct ArgbOverlay {
    int64_t pts;
    uint8_t plane;
    uint8_t cmd;
    uint16_t x, y, w, h;
    uint16_t stride;
    const uint32_t *argb;
};

struct Title;

// BLURAY_DISC_INFO up to the AACS fields.
struct DiscInfo {
    uint8_t bluray_detected;
    const char *disc_name;
    const char *udf_volume_id;
    uint8_t disc_id[20];
    uint8_t no_menu_support;
    uint8_t first_play_supported;
    uint8_t top_menu_supported;
    uint32_t num_titles;
    const Title *const *titles;
    const Title *first_play;
    const Title *top_menu;
    uint32_t num_hdmv_titles;
    uint32_t num_bdj_titles;
    uint32_t num_unsupported_titles;
    uint8_t bdj_detected;
    uint8_t bdj_supported;
    uint8_t libjvm_detected;
    uint8_t bdj_handled;
    char bdj_org_id[9];
    char bdj_disc_id[33];
    uint8_t video_format;
    uint8_t frame_rate;
    uint8_t content_exist_3D;
    uint8_t initial_output_mode_preference;
    uint8_t provider_data[32];
    uint8_t aacs_detected;
    uint8_t libaacs_detected;
    uint8_t aacs_handled;
};

// bd_player_setting values RVP sets.
constexpr uint32_t SETTING_PERSISTENT_ROOT = 0x200;
constexpr uint32_t SETTING_CACHE_ROOT = 0x201;
// BD_VK_MOUSE_ACTIVATE.
constexpr uint32_t KEY_MOUSE_ACTIVATE = 17;
// bd_overlay_plane_e and bd_overlay_cmd_e values RVP looks at (the ARGB
// commands have the same numbers).
constexpr uint8_t PLANE_IG = 1;
constexpr uint8_t OVERLAY_INIT = 0;
constexpr uint8_t OVERLAY_CLOSE = 1;
constexpr uint8_t OVERLAY_DRAW = 3;
// The largest plane RVP opens (`crate::disc::bluray` in rvp-app keeps the
// same): UHD Blu-ray's video, 3840 x 2160, fits with room.
constexpr uint16_t MAX_PLANE_WIDTH = 4096;
constexpr uint16_t MAX_PLANE_HEIGHT = 2304;

using OverlayProc = void (*)(void *, const Overlay *);
using ArgbOverlayProc = void (*)(void *, const ArgbOverlay *);

struct Api {
    void *library = nullptr;
    void (*version)(int *, int *, int *) = nullptr;
    BLURAY *(*init)() = nullptr;
    int (*open_disc)(BLURAY *, const char *, const char *) = nullptr;
    void (*close)(BLURAY *) = nullptr;
    const DiscInfo *(*info)(BLURAY *) = nullptr;
    int (*setting_str)(BLURAY *, uint32_t, const char *) = nullptr;
    void (*overlay_proc)(BLURAY *, void *, OverlayProc) = nullptr;
    void (*argb_overlay_proc)(BLURAY *, void *, ArgbOverlayProc, void *) = nullptr;
    int (*play)(BLURAY *) = nullptr;
    int (*read_ext)(BLURAY *, unsigned char *, int, BdEventC *) = nullptr;
    int (*get_event)(BLURAY *, BdEventC *) = nullptr;
    int (*skip_still)(BLURAY *) = nullptr;
    int (*user_input)(BLURAY *, int64_t, uint32_t) = nullptr;
    int (*mouse_select)(BLURAY *, int64_t, uint16_t, uint16_t) = nullptr;
    int (*menu_call)(BLURAY *, int64_t) = nullptr;
    int64_t (*seek)(BLURAY *, uint64_t) = nullptr;
    uint64_t (*tell)(BLURAY *) = nullptr;
    uint64_t (*title_size)(BLURAY *) = nullptr;
};

void *openLibrary()
{
#ifdef _WIN32
    // RVP's folder (and System32) only, as the process's search is.
    return LoadLibraryExW(L"libbluray-4.dll", nullptr,
                          LOAD_LIBRARY_SEARCH_APPLICATION_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
#else
    // libbluray 1.4 and later are .so.4 (Ubuntu 26.04 and later; the
    // Windows build's 1.5.1 is libbluray-4.dll), 1.3 is .so.2 (Ubuntu 22.04
    // and 24.04); the bare name only comes with the development package.
    // The one libmpv already loaded (the package's own, or the system's)
    // comes first, so that one libbluray serves the process.
    for (const char *name : {"libbluray.so.4", "libbluray.so.2"}) {
        if (void *library = dlopen(name, RTLD_NOW | RTLD_LOCAL | RTLD_NOLOAD))
            return library;
    }
    for (const char *name : {"libbluray.so.4", "libbluray.so.2", "libbluray.so"}) {
        if (void *library = dlopen(name, RTLD_NOW | RTLD_LOCAL))
            return library;
    }
    return nullptr;
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
            reason = "libbluray is not installed";
            return;
        }
        loaded = find(a.library, "bd_get_version", a.version) && find(a.library, "bd_init", a.init)
            && find(a.library, "bd_open_disc", a.open_disc) && find(a.library, "bd_close", a.close)
            && find(a.library, "bd_get_disc_info", a.info)
            && find(a.library, "bd_set_player_setting_str", a.setting_str)
            && find(a.library, "bd_register_overlay_proc", a.overlay_proc)
            && find(a.library, "bd_register_argb_overlay_proc", a.argb_overlay_proc)
            && find(a.library, "bd_play", a.play) && find(a.library, "bd_read_ext", a.read_ext)
            && find(a.library, "bd_get_event", a.get_event)
            && find(a.library, "bd_read_skip_still", a.skip_still)
            && find(a.library, "bd_user_input", a.user_input)
            && find(a.library, "bd_mouse_select", a.mouse_select)
            && find(a.library, "bd_menu_call", a.menu_call) && find(a.library, "bd_seek", a.seek)
            && find(a.library, "bd_tell", a.tell) && find(a.library, "bd_get_title_size", a.title_size);
        if (!loaded)
            reason = "libbluray lacks a function RVP calls";
    });
    why = reason;
    return loaded;
}

BLURAY *h(BlurayNavigator &nav)
{
    return static_cast<BLURAY *>(nav.handle);
}

BdOverlay closing(uint8_t plane)
{
    BdOverlay overlay{};
    overlay.plane = plane;
    overlay.command = OVERLAY_CLOSE;
    return overlay;
}

// The interactive plane's size after an INIT or a CLOSE of it (one too
// large stays closed, as in `crate::disc::bluray`).
void notePlane(BlurayNavigator &nav, uint8_t plane, uint8_t command, uint16_t w, uint16_t h)
{
    if (plane != PLANE_IG)
        return;
    const bool opens = command == OVERLAY_INIT && w <= MAX_PLANE_WIDTH && h <= MAX_PLANE_HEIGHT;
    if (command == OVERLAY_INIT || command == OVERLAY_CLOSE) {
        nav.planeWidth = opens ? w : 0;
        nav.planeHeight = opens ? h : 0;
    }
}

// How many of a DRAW's runs make its w x h image: row by row, a row
// ending at an end-of-line run (length 0) or once it holds w pixels (with
// the end-of-line run right after it), for h rows, and never past w x h
// pixels in all. libbluray gives no length with the runs, so these bounds
// are what keeps the walk inside them, and they rest on libbluray's own
// rules: its decoder keeps an object only when its runs hold exactly its
// width times its height pixels (so a walk that has seen fewer has runs
// left to read), and its crop writes each row as runs of at most the
// crop's width followed by an end-of-line run. libbluray 1.5.1 passes a
// crop taller than its object on unchecked, as the object's own runs
// with the crop's height (RVP's Windows build clamps the crop to the
// object, tools/discs/libbluray-crop.patch; on Linux the system's library
// is used): there the walk reads at most w + 1 runs for each row the
// object lacks.
size_t runCount(const RleElement *img, uint16_t w, uint16_t h)
{
    const uint64_t pixels = uint64_t(w) * h;
    uint64_t seen = 0;
    size_t count = 0;
    for (uint32_t row = 0; row < h && seen < pixels; ++row) {
        uint32_t x = 0;
        while (seen < pixels) {
            const uint16_t length = img[count++].len;
            if (length == 0)
                break;
            x += length;
            seen += length;
            if (x >= w) {
                if (seen < pixels && img[count].len == 0)
                    ++count;
                break;
            }
        }
    }
    return count;
}

// libbluray's callbacks, on the thread of the call that drew (a read or an
// input, under the caller's lock): a copy of the event for the queue. They
// run inside libbluray's C code, so nothing may leave them as an
// exception: an event that cannot be queued (no memory) is dropped.
void onOverlay(void *handle, const Overlay *overlay) noexcept
{
    auto *nav = static_cast<BlurayNavigator *>(handle);
    try {
        std::lock_guard<std::mutex> guard(nav->lock);
        if (overlay == nullptr) {
            // Every plane closes.
            notePlane(*nav, PLANE_IG, OVERLAY_CLOSE, 0, 0);
            nav->overlays.push_back(closing(0));
            nav->overlays.push_back(closing(1));
            return;
        }
        notePlane(*nav, overlay->plane, overlay->cmd, overlay->w, overlay->h);
        BdOverlay copy{};
        copy.argb = false;
        copy.plane = overlay->plane;
        copy.command = overlay->cmd;
        copy.x = overlay->x;
        copy.y = overlay->y;
        copy.width = overlay->w;
        copy.height = overlay->h;
        // RVP draws the interactive plane only (mpv the subtitles).
        if (overlay->plane == PLANE_IG && overlay->cmd == OVERLAY_DRAW) {
            // An image larger than the plane is not drawn.
            if (overlay->w > nav->planeWidth || overlay->h > nav->planeHeight)
                return;
            if (overlay->palette != nullptr) {
                copy.palette.reserve(256);
                for (int i = 0; i < 256; ++i) {
                    const PaletteEntry &e = overlay->palette[i];
                    copy.palette.push_back(uint32_t(e.y) << 24 | uint32_t(e.cr) << 16 | uint32_t(e.cb) << 8 | e.t);
                }
            }
            if (overlay->img != nullptr) {
                const size_t count = runCount(overlay->img, overlay->w, overlay->h);
                copy.data.reserve(count);
                for (size_t i = 0; i < count; ++i) {
                    const RleElement &run = overlay->img[i];
                    copy.data.push_back(uint32_t(run.len) << 16 | run.color);
                }
            }
        }
        nav->overlays.push_back(std::move(copy));
    } catch (...) {
    }
}

void onArgbOverlay(void *handle, const ArgbOverlay *overlay) noexcept
{
    auto *nav = static_cast<BlurayNavigator *>(handle);
    try {
        std::lock_guard<std::mutex> guard(nav->lock);
        if (overlay == nullptr) {
            notePlane(*nav, PLANE_IG, OVERLAY_CLOSE, 0, 0);
            nav->overlays.push_back(closing(0));
            nav->overlays.push_back(closing(1));
            return;
        }
        notePlane(*nav, overlay->plane, overlay->cmd, overlay->w, overlay->h);
        BdOverlay copy{};
        copy.argb = true;
        copy.plane = overlay->plane;
        copy.command = overlay->cmd;
        copy.x = overlay->x;
        copy.y = overlay->y;
        copy.width = overlay->w;
        copy.height = overlay->h;
        // The pixels come with the DRAW; the FLUSH after it repeats them.
        if (overlay->plane == PLANE_IG && overlay->cmd == OVERLAY_DRAW && overlay->argb != nullptr) {
            // BD-J's pixels are the plane's own: the rectangle lies in it.
            if (uint32_t(overlay->x) + overlay->w > nav->planeWidth
                || uint32_t(overlay->y) + overlay->h > nav->planeHeight)
                return;
            copy.data.reserve(size_t(overlay->w) * overlay->h);
            for (uint16_t y = 0; y < overlay->h; ++y) {
                const uint32_t *line = overlay->argb + size_t(y) * overlay->stride;
                for (uint16_t x = 0; x < overlay->w; ++x)
                    copy.data.push_back(line[x]);
            }
        }
        nav->overlays.push_back(std::move(copy));
    } catch (...) {
    }
}

} // namespace

BlurayNavigator::BlurayNavigator(void *handle) : handle(handle) {}

BlurayNavigator::~BlurayNavigator()
{
    if (handle != nullptr)
        api().close(static_cast<BLURAY *>(handle));
}

bool rvp_bluray_load(rust::String &message)
{
    std::string why;
    if (!loadApi(why)) {
        message = why;
        return false;
    }
    int major = 0, minor = 0, micro = 0;
    api().version(&major, &minor, &micro);
    message = std::to_string(major) + "." + std::to_string(minor) + "." + std::to_string(micro);
    return true;
}

std::unique_ptr<BlurayNavigator> rvp_bluray_open(rust::Str path, rust::Str storage, rust::String &error)
{
    std::string why;
    if (!loadApi(why)) {
        error = why;
        return nullptr;
    }
    const Api &a = api();
    BLURAY *bd = a.init();
    if (bd == nullptr) {
        error = "libbluray cannot start";
        return nullptr;
    }
    auto nav = std::make_unique<BlurayNavigator>(bd);
    if (storage.size() > 0) {
        const std::string root(storage.data(), storage.size());
        a.setting_str(bd, SETTING_PERSISTENT_ROOT, (root + "/persistent").c_str());
        a.setting_str(bd, SETTING_CACHE_ROOT, (root + "/cache").c_str());
    }
    const std::string name(path.data(), path.size());
    if (!a.open_disc(bd, name.c_str(), nullptr)) {
        error = "libbluray cannot open the disc";
        return nullptr;
    }
    a.overlay_proc(bd, nav.get(), onOverlay);
    a.argb_overlay_proc(bd, nav.get(), onArgbOverlay, nullptr);
    return nav;
}

BdInfo rvp_bluray_info(BlurayNavigator &nav)
{
    BdInfo info{};
    const DiscInfo *d = api().info(h(nav));
    if (d == nullptr)
        return info;
    info.bluray = d->bluray_detected != 0;
    info.menus = d->no_menu_support == 0;
    info.first_play = d->first_play_supported != 0;
    info.top_menu = d->top_menu_supported != 0;
    info.hdmv_titles = d->num_hdmv_titles;
    info.bdj_titles = d->num_bdj_titles;
    info.bdj_handled = d->bdj_handled != 0;
    info.java = d->libjvm_detected != 0;
    info.aacs = d->aacs_detected != 0;
    info.aacs_handled = d->aacs_handled != 0;
    return info;
}

bool rvp_bluray_play(BlurayNavigator &nav)
{
    return api().play(h(nav)) != 0;
}

int32_t rvp_bluray_read(BlurayNavigator &nav, rust::Slice<uint8_t> block, rust::Vec<BdEvent> &events)
{
    BdEventC event{};
    const int length = api().read_ext(h(nav), block.data(), int(block.size()), &event);
    while (event.event != 0) {
        events.push_back(BdEvent{event.event, event.param});
        if (!api().get_event(h(nav), &event))
            break;
    }
    return length;
}

bool rvp_bluray_skip_still(BlurayNavigator &nav)
{
    return api().skip_still(h(nav)) != 0;
}

bool rvp_bluray_key(BlurayNavigator &nav, uint32_t key)
{
    // -1: the last position libbluray knows.
    return api().user_input(h(nav), -1, key) >= 0;
}

bool rvp_bluray_mouse(BlurayNavigator &nav, uint16_t x, uint16_t y, bool activate)
{
    const bool over = api().mouse_select(h(nav), -1, x, y) > 0;
    if (activate && over)
        return api().user_input(h(nav), -1, KEY_MOUSE_ACTIVATE) >= 0;
    return over;
}

bool rvp_bluray_menu(BlurayNavigator &nav)
{
    return api().menu_call(h(nav), -1) != 0;
}

int64_t rvp_bluray_seek(BlurayNavigator &nav, uint64_t position)
{
    return api().seek(h(nav), position);
}

uint64_t rvp_bluray_tell(BlurayNavigator &nav)
{
    return api().tell(h(nav));
}

uint64_t rvp_bluray_title_size(BlurayNavigator &nav)
{
    return api().title_size(h(nav));
}

rust::Vec<BdOverlay> rvp_bluray_overlays(BlurayNavigator &nav)
{
    std::vector<BdOverlay> taken;
    {
        std::lock_guard<std::mutex> guard(nav.lock);
        taken.swap(nav.overlays);
    }
    rust::Vec<BdOverlay> out;
    out.reserve(taken.size());
    for (auto &overlay : taken)
        out.push_back(std::move(overlay));
    return out;
}
