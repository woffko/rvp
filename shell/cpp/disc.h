// DVD menus (M16): libdvdnav, loaded at run time (libdvdnav-4.dll beside
// rvp.exe on Windows, the system's libdvdnav.so.4 on Linux), so that RVP
// builds and runs without it. Thin calls only: the event data stays in the
// caller's block buffer, and `shell/src/disc` reads it.
// A guard, not #pragma once: the build copies this header, and both
// copies meet in disc.cpp.
#ifndef RVP_DISC_H
#define RVP_DISC_H

#include "rust/cxx.h"

#include <cstdint>
#include <memory>

struct DvdStatus;
struct DvdButton;

// One open disc: libdvdnav's handle, closed with the object.
class DvdNavigator {
public:
    explicit DvdNavigator(void *handle);
    ~DvdNavigator();
    DvdNavigator(const DvdNavigator &) = delete;
    DvdNavigator &operator=(const DvdNavigator &) = delete;

    void *handle;
};

// Loads libdvdnav once; false (and why in `message`) without it.
bool rvp_dvdnav_load(rust::String &message);
// Opens a disc with the machine's preferred languages (two letters each,
// or empty) for the menus, the audio and the sub-pictures.
std::unique_ptr<DvdNavigator> rvp_dvdnav_open(rust::Str path, rust::Str menu, rust::Str audio,
                                              rust::Str spu, rust::String &error);
// The next block or event: its code (libdvdnav's DVDNAV_*), the data in
// `block` (2048 bytes), its length in `length`; -1 on an error.
int32_t rvp_dvdnav_next(DvdNavigator &nav, rust::Slice<uint8_t> block, int32_t &length);
bool rvp_dvdnav_still_skip(DvdNavigator &nav);
bool rvp_dvdnav_wait_skip(DvdNavigator &nav);
DvdStatus rvp_dvdnav_status(DvdNavigator &nav);
// The selected button's rectangle and palette; mode 0 selected, 1 activated.
DvdButton rvp_dvdnav_button(DvdNavigator &nav, int32_t mode);
// 0 up, 1 down, 2 left, 3 right.
bool rvp_dvdnav_select(DvdNavigator &nav, int32_t direction);
bool rvp_dvdnav_activate(DvdNavigator &nav);
bool rvp_dvdnav_mouse(DvdNavigator &nav, int32_t x, int32_t y, bool activate);
// libdvdnav's DVDMenuID_t.
bool rvp_dvdnav_menu(DvdNavigator &nav, int32_t menu);
bool rvp_dvdnav_go_up(DvdNavigator &nav);
bool rvp_dvdnav_sector_search(DvdNavigator &nav, int64_t sector);
rust::String rvp_dvdnav_error(DvdNavigator &nav);

#endif // RVP_DISC_H
