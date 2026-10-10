#!/usr/bin/env python3
"""Checks a frameless window with a real (injected) mouse: dragging the
title bar moves it, dragging the corner resizes it and a right click opens
a menu window. For the S3 spike the window is first shrunk to 440x77 and
the menu must be larger than it; for RVP (`--rvp`) a double click on the
video must also enter fullscreen and a second one leave it. Only mouse
input is injected (it reaches the window under the pointer); keyboard
input could reach the terminal that runs the check.

Wayland clients cannot read their own position, so the check works through
the pointer: the window logs the pointer position in its own coordinates,
and with the pointer at a fixed screen point that position changes by
exactly the distance the window moved. Input is injected with ydotool 1.x
(uinput), which needs ydotoold running as root and a flat pointer
acceleration profile. The ratio between injected and actual pointer motion
is measured first, because it is not always 1 (it is 2 on the Ubuntu VM).

usage: input-check.py S3_BINARY SKIN.dsf SCREEN_WIDTH SCREEN_HEIGHT LOG
       input-check.py --rvp RVP_BINARY SKIN.dsf SCREEN_WIDTH SCREEN_HEIGHT LOG VIDEO
"""

import json
import re
import subprocess
import sys
import time

POINTER = re.compile(r"pointer_local=(-?\d+),(-?\d+) size=(\d+)x(\d+)")
DRAG = (80, 60)  # sign chosen towards the screen centre
RESIZE = (-100, -80)  # the grip is dragged inwards, which always fits on screen
MENU_SIZE = (440, 77)


def ydotool(*args: str) -> None:
    subprocess.run(["ydotool", *args], check=True, capture_output=True)


def move_to(x: int, y: int) -> None:
    """Absolute move in ydotool units (screen pixels divided by the scale)."""
    ydotool("mousemove", "--absolute", "-x", str(x), "-y", str(y))
    time.sleep(0.25)


def log_lines(log_path: str) -> list[str]:
    with open(log_path, encoding="utf-8", errors="replace") as log:
        return log.readlines()


def last_pointer(log_path: str) -> tuple[int, int, int, int] | None:
    found = None
    for line in log_lines(log_path):
        match = POINTER.search(line)
        if match:
            found = tuple(int(group) for group in match.groups())
    return found


def probe(log_path: str, x: int, y: int) -> tuple[int, int, int, int] | None:
    """Moves the pointer to (x, y) in ydotool units and returns what the
    window reported, or None if the pointer is not over the window."""
    before = last_pointer(log_path)
    move_to(x + 3, y + 3)
    move_to(x, y)
    after = last_pointer(log_path)
    return after if after != before else None


class Pointer:
    """Maps screen pixels to ydotool units: screen = scale * units."""

    def __init__(self, log_path: str) -> None:
        self.log_path = log_path
        self.scale = (1.0, 1.0)

    def units(self, x: float, y: float) -> tuple[int, int]:
        return round(x / self.scale[0]), round(y / self.scale[1])

    def probe(self, x: float, y: float) -> tuple[int, int, int, int] | None:
        return probe(self.log_path, *self.units(x, y))

    def calibrate(self, hit_units: tuple[int, int], local: tuple[int, int, int, int]) -> bool:
        """Measures the scale from probes inside the window: first a small
        step, then a step towards the centre sized with the first estimate."""
        lx, ly, width, height = local
        points = [(hit_units, (lx, ly))]
        step = (8 if lx < width / 2 else -8, 8 if ly < height / 2 else -8)
        second = (hit_units[0] + step[0], hit_units[1] + step[1])
        report = probe(self.log_path, *second)
        if report is None:
            return False
        points.append((second, (report[0], report[1])))
        self.scale = (abs(report[0] - lx) / 8 or 1.0, abs(report[1] - ly) / 8 or 1.0)
        centre = (width / 2 - report[0], height / 2 - report[1])
        third = (
            second[0] + round(centre[0] / self.scale[0]),
            second[1] + round(centre[1] / self.scale[1]),
        )
        report = probe(self.log_path, *third)
        if report is not None:
            points.append((third, (report[0], report[1])))
        # The two probes furthest apart give the most precise ratio.
        (u0, l0), (u1, l1) = points[0], points[-1]
        self.scale = tuple(
            (l1[axis] - l0[axis]) / (u1[axis] - u0[axis]) if u1[axis] != u0[axis] else self.scale[axis]
            for axis in (0, 1)
        )
        return self.scale[0] > 0 and self.scale[1] > 0

    def drag(self, start: tuple[float, float], delta: tuple[int, int]) -> None:
        """Left-drags from a screen point by a screen distance. The steps are
        absolute moves: on the Ubuntu VM relative moves were not scaled like
        absolute ones, so only the measured absolute scale is trusted."""
        move_to(*self.units(*start))
        ydotool("click", "0x40")  # left button down
        time.sleep(0.2)
        steps = 8
        for step in range(1, steps + 1):
            move_to(*self.units(start[0] + delta[0] * step / steps, start[1] + delta[1] * step / steps))
        # Each absolute move passes through the screen corner; give the
        # client time to follow the last resize before the button goes up.
        time.sleep(1.0)
        move_to(*self.units(start[0] + delta[0], start[1] + delta[1]))
        time.sleep(1.0)
        ydotool("click", "0x80")  # left button up
        time.sleep(0.6)


def menu_windows(log_path: str) -> list[dict] | None:
    found = None
    for line in log_lines(log_path):
        if line.startswith("menu_windows="):
            found = json.loads(line.split("=", 1)[1])
    return found


def wait_for(log_path: str, text: str, seconds: float) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if any(text in line for line in log_lines(log_path)):
            return True
        time.sleep(0.2)
    return False


def find_window(pointer: Pointer, screen: tuple[int, int]):
    for gy in range(1, 8):
        for gx in range(1, 8):
            units = (screen[0] * gx // 8, screen[1] * gy // 8)
            report = probe(pointer.log_path, *units)
            if report:
                return units, report
    return None


def check_move(pointer: Pointer, origin, size, drag) -> bool:
    reference = (origin[0] + size[0] / 2, origin[1] + size[1] / 2)
    before = pointer.probe(*reference)
    pointer.drag((origin[0] + size[0] / 3, origin[1] + 12), drag)
    after = pointer.probe(*reference)
    if before is None or after is None:
        print("input_check_move=FAIL no pointer report")
        return False
    moved = (before[0] - after[0], before[1] - after[1])
    ok = abs(moved[0] - drag[0]) <= 3 and abs(moved[1] - drag[1]) <= 3
    print(f"input_check_move={'PASS' if ok else 'FAIL'} moved={moved[0]},{moved[1]} expected={drag[0]},{drag[1]}")
    return ok


def check_resize(pointer: Pointer, origin, size, inset: int = 6) -> bool:
    """`inset`: where the corner is grabbed (S3 has a 16 px grip, RVP's
    resize border is 5 px wide)."""
    pointer.drag((origin[0] + size[0] - inset, origin[1] + size[1] - inset), RESIZE)
    report = pointer.probe(origin[0] + 20, origin[1] + 40) or last_pointer(pointer.log_path)
    if report is None:
        print("input_check_resize=FAIL no pointer report")
        return False
    resized = (report[2] - size[0], report[3] - size[1])
    ok = abs(resized[0] - RESIZE[0]) <= 3 and abs(resized[1] - RESIZE[1]) <= 3
    print(
        f"input_check_resize={'PASS' if ok else 'FAIL'} resized={resized[0]},{resized[1]} "
        f"expected={RESIZE[0]},{RESIZE[1]}"
    )
    return ok


def check_menu(pointer: Pointer, origin, size) -> bool:
    # Only mouse input: it goes to the surface under the pointer. Keys would
    # go to the focused window, which may be the terminal running this check.
    # A middle click below the title area shrinks the window to 440x77.
    move_to(*pointer.units(origin[0] + size[0] / 2, origin[1] + size[1] / 2))
    ydotool("click", "0xC2")  # middle button down and up
    if not wait_for(pointer.log_path, f"window_size={MENU_SIZE[0]}x{MENU_SIZE[1]}", 3):
        print("input_check_menu=FAIL the window did not shrink to 440x77")
        return False
    time.sleep(0.5)
    move_to(*pointer.units(origin[0] + 200, origin[1] + 50))
    ydotool("click", "0xC1")  # right button down and up
    time.sleep(1.2)
    windows = menu_windows(pointer.log_path)
    # The menu stays open; terminating the program closes it.
    if windows is None:
        print("input_check_menu=FAIL the menu did not open")
        return False
    main = next((w for w in windows if w["title"] == "RVP spike S3"), None)
    popups = [w for w in windows if w["popup"] and w["visible"] and w["exposed"]]
    if main is None or not popups:
        print(f"input_check_menu=FAIL no visible menu window: {json.dumps(windows)}")
        return False
    menu = popups[0]
    ok = menu["height"] > main["height"] and main["height"] == MENU_SIZE[1]
    print(
        f"input_check_menu={'PASS' if ok else 'FAIL'} menu {menu['width']}x{menu['height']} "
        f"main {main['width']}x{main['height']} class {menu['class']}"
    )
    return ok


def double_click(pointer: Pointer, x: float, y: float) -> None:
    move_to(*pointer.units(x, y))
    ydotool("click", "--repeat", "2", "--next-delay", "120", "0xC0")  # left, twice
    time.sleep(1.5)


def last_size(log_path: str) -> tuple[int, int] | None:
    found = None
    for line in log_lines(log_path):
        match = re.search(r"window_size=(\d+)x(\d+)", line)
        if match:
            found = (int(match.group(1)), int(match.group(2)))
    return found


def check_fullscreen(pointer: Pointer, origin, size, screen) -> bool:
    """RVP: a double click on the video enters fullscreen, another leaves."""
    centre = (origin[0] + size[0] / 2, origin[1] + size[1] / 2)
    double_click(pointer, *centre)
    entered = last_size(pointer.log_path)
    double_click(pointer, screen[0] / 2, screen[1] / 2)
    time.sleep(1.0)
    left = last_size(pointer.log_path)
    ok = entered == screen and left == size
    print(
        f"input_check_fullscreen={'PASS' if ok else 'FAIL'} entered={entered} left={left} "
        f"screen={screen} window={size}"
    )
    return ok


def check_rvp_menu(pointer: Pointer, origin, size) -> bool:
    """RVP: a right click on the video opens the context menu window."""
    move_to(*pointer.units(origin[0] + size[0] / 2, origin[1] + size[1] / 2))
    ydotool("click", "0xC1")  # right button down and up
    time.sleep(1.5)
    windows = menu_windows(pointer.log_path)
    if windows is None:
        print("input_check_menu=FAIL the menu did not open")
        return False
    popups = [w for w in windows if w["popup"] and w["visible"] and w["exposed"]]
    ok = bool(popups)
    detail = f"menu {popups[0]['width']}x{popups[0]['height']}" if popups else json.dumps(windows)
    print(f"input_check_menu={'PASS' if ok else 'FAIL'} {detail}")
    return ok


def main() -> int:
    rvp = len(sys.argv) > 1 and sys.argv[1] == "--rvp"
    args = sys.argv[2:] if rvp else sys.argv[1:]
    if len(args) != (6 if rvp else 5):
        print(__doc__)
        return 2
    binary, skin, log_path = args[0], args[1], args[4]
    screen = (int(args[2]), int(args[3]))
    if rvp:
        # A video plays: a click on an empty video area would open the file
        # dialog.
        command = [binary, f"--skin={skin}", "--log-pointer", "--log-geometry", args[5]]
    else:
        command = [binary, "--skin", skin, "--log-pointer", "--log-geometry"]
    with open(log_path, "w", encoding="utf-8") as log:
        app = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    pointer = Pointer(log_path)
    failures = 0
    try:
        time.sleep(3)
        hit = find_window(pointer, screen)
        if hit is None:
            print("input_check=FAIL window not found under the pointer")
            return 1
        hit_units, report = hit
        if not pointer.calibrate(hit_units, report):
            print("input_check=FAIL pointer scale could not be measured")
            return 1
        print(f"pointer_scale={pointer.scale[0]:.3f},{pointer.scale[1]:.3f}")
        report = probe(log_path, *hit_units) or report
        lx, ly, width, height = report
        origin = (hit_units[0] * pointer.scale[0] - lx, hit_units[1] * pointer.scale[1] - ly)
        print(f"window_origin={origin[0]:.0f},{origin[1]:.0f} size={width}x{height}")
        drag = tuple(
            DRAG[axis] if origin[axis] + (width, height)[axis] / 2 < screen[axis] / 2 else -DRAG[axis]
            for axis in (0, 1)
        )
        failures += 0 if check_move(pointer, origin, (width, height), drag) else 1
        origin = (origin[0] + drag[0], origin[1] + drag[1])
        failures += 0 if check_resize(pointer, origin, (width, height), 2 if rvp else 6) else 1
        resized = (width + RESIZE[0], height + RESIZE[1])
        if rvp:
            failures += 0 if check_fullscreen(pointer, origin, resized, screen) else 1
            failures += 0 if check_rvp_menu(pointer, origin, resized) else 1
        else:
            failures += 0 if check_menu(pointer, origin, resized) else 1
    finally:
        app.terminate()
        try:
            app.wait(timeout=5)
        except subprocess.TimeoutExpired:
            app.kill()
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
