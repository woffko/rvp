// Blu-ray menus (M16): libbluray, loaded at run time (libbluray-4.dll
// beside rvp.exe on Windows, the system's libbluray.so.4 or .so.2 on Linux), so that
// RVP builds and runs without it. Thin calls: libbluray's overlay events
// are copied into a queue the caller drains; `shell/src/disc/bluray.rs`
// holds the logic.
// A guard, not #pragma once: the build copies this header, and both
// copies meet in bluray.cpp.
#ifndef RVP_BLURAY_H
#define RVP_BLURAY_H

#include "rust/cxx.h"

#include <cstdint>
#include <memory>
#include <mutex>
#include <vector>

struct BdEvent;
struct BdInfo;
struct BdOverlay;

// One open disc: libbluray's handle, closed with the object, the overlay
// events its callbacks queued, and the size of the interactive plane its
// last INIT opened (0 x 0 while none is open, or when it was too large).
class BlurayNavigator {
public:
    explicit BlurayNavigator(void *handle);
    ~BlurayNavigator();
    BlurayNavigator(const BlurayNavigator &) = delete;
    BlurayNavigator &operator=(const BlurayNavigator &) = delete;

    void *handle;
    std::mutex lock;
    std::vector<BdOverlay> overlays;
    uint16_t planeWidth = 0;
    uint16_t planeHeight = 0;
};

// Loads libbluray once; false (and why in `message`) without it.
bool rvp_bluray_load(rust::String &message);
// Opens a disc, a folder holding BDMV or an image; BD-J keeps its files in
// `storage`. The overlay callbacks are registered; `bd_play` is not called.
std::unique_ptr<BlurayNavigator> rvp_bluray_open(rust::Str path, rust::Str storage, rust::String &error);
BdInfo rvp_bluray_info(BlurayNavigator &nav);
// Starts the disc's first play (its menus).
bool rvp_bluray_play(BlurayNavigator &nav);
// Reads up to block.size() bytes of the stream (0 when an event comes
// instead); the events of the call (and those queued) are appended. -1 on
// an error.
int32_t rvp_bluray_read(BlurayNavigator &nav, rust::Slice<uint8_t> block, rust::Vec<BdEvent> &events);
bool rvp_bluray_skip_still(BlurayNavigator &nav);
// One of libbluray's keys (bd_vk_key_e).
bool rvp_bluray_key(BlurayNavigator &nav, uint32_t key);
// The pointer over the plane; `activate` clicks.
bool rvp_bluray_mouse(BlurayNavigator &nav, uint16_t x, uint16_t y, bool activate);
// The top menu.
bool rvp_bluray_menu(BlurayNavigator &nav);
// Byte positions in the current title.
int64_t rvp_bluray_seek(BlurayNavigator &nav, uint64_t position);
uint64_t rvp_bluray_tell(BlurayNavigator &nav);
uint64_t rvp_bluray_title_size(BlurayNavigator &nav);
// The overlay events queued since the last call.
rust::Vec<BdOverlay> rvp_bluray_overlays(BlurayNavigator &nav);

#endif // RVP_BLURAY_H
