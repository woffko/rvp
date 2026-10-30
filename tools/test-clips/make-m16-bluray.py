#!/usr/bin/env python3
"""Builds M16's synthetic Blu-ray with an HDMV menu (all synthetic: colour
fields, tones and frames drawn here), for the Blu-ray menu tests
(shell/src/disc/bluray_tests.rs, `rvp --selftest=m16`). Never committed:
only this script is.

  First play and the top menu: movie object 0 plays playlist 00000, a 2 s
    menu clip whose last frame stays (an endless still), with an always-on
    menu in its interactive graphics stream (PID 0x1400): three buttons in
    a row, 280x160 at x = 320, 820, 1320 and y = 700 (1920x1080 plane).
    Each has a grey frame; the selected one's frame is yellow, the
    activated one's orange. Left and right move along the row.
      button 1: JumpTitle 1 (title 1 from its start)
      button 2: JumpTitle 2 (title 2: title 1's playlist from its second
                chapter)
      button 3: SetStream audio 2, then JumpTitle 1
  Titles 1 and 2: movie objects 1 and 2 play playlist 00001 (an 8 s clip
    with chapters at 0 and 4 s and two AC-3 streams, 440 Hz and 880 Hz),
    from its start or from its second mark, then jump to object 0 (the
    menu again).

The menu's buttons are written to OUT/buttons.txt for the tests.

usage: make-m16-bluray.py OUT [FFMPEG]
"""

import struct
import subprocess
import sys
import tempfile
from pathlib import Path

WIDTH, HEIGHT = 1920, 1080
# BD's MPEG-2 1080p at 23.976 (format 6, rate 1).
RATE = "24000/1001"
VIDEO_PID, AUDIO_PID, IG_PID, PMT_PID = 0x1011, 0x1100, 0x1400, 0x0100
PACKET = 192
UNIT = 32  # source packets in an aligned unit (6144 bytes)
BUTTONS = [(320, 700), (820, 700), (1320, 700)]
BUTTON_W, BUTTON_H, BORDER = 280, 160, 8
MENU_SECONDS, TITLE_SECONDS, CHAPTER_2 = 2, 8, 4


def ffmpeg(binary, *args):
    subprocess.run([binary, "-v", "error", "-y", *args], check=True)


def make_clips(binary, work):
    """The menu clip (video with the buttons' places, silence) and the
    title clip (video, two tones), as m2ts with BD's PIDs."""
    boxes = ",".join(
        f"drawbox=x={x}:y={y}:w={BUTTON_W}:h={BUTTON_H}:color=0x304050:t=fill" for x, y in BUTTONS
    )
    common = ["-c:v", "mpeg2video", "-pix_fmt", "yuv420p", "-b:v", "4M", "-g", "12", "-bf", "0",
              "-c:a", "ac3", "-b:a", "192k", "-ar", "48000", "-ac", "2",
              "-f", "mpegts", "-mpegts_m2ts_mode", "1", "-fflags", "+bitexact",
              "-flags:v", "+bitexact", "-flags:a", "+bitexact"]
    menu = work / "menu.m2ts"
    ffmpeg(binary,
           "-f", "lavfi", "-i", f"color=c=0x102040:s={WIDTH}x{HEIGHT}:r={RATE}:d={MENU_SECONDS},{boxes}",
           "-f", "lavfi", "-i", f"anullsrc=r=48000:cl=stereo:d={MENU_SECONDS}",
           "-map", "0:v", "-map", "1:a", *common, str(menu))
    title = work / "title.m2ts"
    ffmpeg(binary,
           "-f", "lavfi", "-i", f"color=c=0x204010:s={WIDTH}x{HEIGHT}:r={RATE}:d={TITLE_SECONDS}",
           "-f", "lavfi", "-i", f"sine=frequency=440:sample_rate=48000:duration={TITLE_SECONDS}",
           "-f", "lavfi", "-i", f"sine=frequency=880:sample_rate=48000:duration={TITLE_SECONDS}",
           "-map", "0:v", "-map", "1:a", "-map", "2:a", *common, str(title))
    return menu.read_bytes(), title.read_bytes()


# --- MPEG-2 transport stream ---------------------------------------------

def crc32_mpeg(data):
    crc = 0xFFFFFFFF
    for byte in data:
        crc ^= byte << 24
        for _ in range(8):
            crc = ((crc << 1) ^ 0x04C11DB7) & 0xFFFFFFFF if crc & 0x80000000 else (crc << 1) & 0xFFFFFFFF
    return crc


def packets(stream):
    return [stream[i:i + PACKET] for i in range(0, len(stream), PACKET)]


def pid_of(packet):
    return ((packet[5] & 0x1F) << 8) | packet[6]


def payload_of(packet):
    ts = packet[4:]
    control = (ts[3] >> 4) & 3
    start = 4
    if control & 2:
        start += 1 + ts[4]
    return ts[start:] if control & 1 else b""


def pes_pts(packet):
    """The PTS of a PES that starts in this packet, if any."""
    ts = packet[4:]
    if not ts[1] & 0x40:
        return None
    pes = payload_of(packet)
    if len(pes) < 14 or pes[:3] != b"\x00\x00\x01" or not pes[7] & 0x80:
        return None
    p = pes[9:14]
    return ((p[0] >> 1) & 7) << 30 | p[1] << 22 | (p[2] >> 1) << 15 | p[3] << 7 | p[4] >> 1


def with_ig_in_pmt(packet):
    """FFmpeg's PMT with the IG stream (stream_type 0x91) added."""
    ts = bytearray(packet[4:])
    assert ts[3] & 0x30 == 0x10, "a PMT packet with an adaptation field"
    start = 5 + ts[4]  # after the pointer field
    length = ((ts[start + 1] & 0x0F) << 8) | ts[start + 2]
    section = bytes(ts[start:start + 3 + length - 4])
    entry = bytes([0x91, 0xE0 | (IG_PID >> 8), IG_PID & 0xFF, 0xF0, 0x00])
    body = section[3:] + entry
    header = bytes([section[0], 0xB0 | ((len(body) + 4) >> 8), (len(body) + 4) & 0xFF])
    new = header + body
    new += struct.pack(">I", crc32_mpeg(new))
    out = ts[:start] + new
    out += b"\xff" * (188 - len(out))
    assert len(out) == 188, "the PMT does not fit one packet"
    return packet[:4] + bytes(out)


def ts_packets(pid, pes, counter, extra):
    """`pes` as 192-byte source packets of `pid` with the arrival stamp
    `extra`; the continuity counter goes on from `counter`."""
    out = []
    first = True
    while pes:
        chunk = pes[:184]
        pes = pes[184:]
        header = bytearray([0x47, (0x40 if first else 0) | (pid >> 8), pid & 0xFF, 0x10 | (counter & 0xF)])
        if len(chunk) < 184:
            stuffing = 184 - len(chunk) - 1
            header[3] |= 0x20
            field = bytes([stuffing]) + (bytes([0x00]) + b"\xff" * (stuffing - 1) if stuffing else b"")
            body = bytes(header) + field + chunk
        else:
            body = bytes(header) + chunk
        assert len(body) == 188
        out.append(extra + body)
        counter += 1
        first = False
    return out, counter


def pes_packet(payload, pts):
    """A private stream 1 PES with PTS and DTS (both `pts`)."""
    def stamp(prefix, value):
        return bytes([
            (prefix << 4) | (((value >> 30) & 7) << 1) | 1,
            (value >> 22) & 0xFF, (((value >> 15) & 0x7F) << 1) | 1,
            (value >> 7) & 0xFF, ((value & 0x7F) << 1) | 1,
        ])
    header = b"\x80\xc0\x0a" + stamp(3, pts) + stamp(1, pts)
    length = len(header) + len(payload)
    return b"\x00\x00\x01\xbd" + struct.pack(">H", length) + header + payload


def null_packet(extra):
    return extra + bytes([0x47, 0x1F, 0xFF, 0x10]) + b"\xff" * 184


def aligned(source):
    """Null packets up to a whole aligned unit."""
    extra = source[-1][:4]
    while len(source) % UNIT:
        source.append(null_packet(extra))
    return source


# --- Interactive graphics (the menu) --------------------------------------

def rle(width, height, pixel):
    """PGS run-length coding of a width x height bitmap."""
    out = bytearray()
    for y in range(height):
        x = 0
        while x < width:
            colour = pixel(x, y)
            run = 1
            while x + run < width and pixel(x + run, y) == colour and run < 16383:
                run += 1
            if colour != 0 and run < 3:
                out += bytes([colour]) * run
            elif colour == 0:
                out += b"\x00" + (bytes([run]) if run < 64 else struct.pack(">H", 0x4000 | run))
            else:
                out += b"\x00" + (bytes([0x80 | run]) if run < 64 else struct.pack(">H", 0xC000 | run)) + bytes([colour])
            x += run
        out += b"\x00\x00"
    return bytes(out)


def frame(colour):
    def pixel(x, y):
        inside = BORDER <= x < BUTTON_W - BORDER and BORDER <= y < BUTTON_H - BORDER
        return 0 if inside else colour
    return pixel


def segment(kind, body):
    return bytes([kind]) + struct.pack(">H", len(body)) + body


def palette():
    # Entry: id, Y, Cr, Cb, T (BT.709 limited range).
    entries = [(0, 16, 128, 128, 0), (1, 126, 128, 128, 255), (2, 219, 138, 16, 255), (3, 151, 192, 39, 255)]
    return segment(0x14, bytes([0, 0]) + b"".join(bytes(e) for e in entries))


def bitmap(object_id, colour):
    data = struct.pack(">HH", BUTTON_W, BUTTON_H) + rle(BUTTON_W, BUTTON_H, frame(colour))
    body = struct.pack(">HB", object_id, 0) + bytes([0xC0]) + len(data).to_bytes(3, "big") + data
    return segment(0x15, body)


def command(op_cnt, group, sub, branch=0, set_opt=0, dst=0, src=0, imm1=1, imm2=0):
    word = (op_cnt << 29) | (group << 27) | (sub << 24) | (imm1 << 23) | (imm2 << 22) | (branch << 16) | set_opt
    return struct.pack(">III", word, dst, src)


def jump_title(title):
    return command(1, 0, 1, branch=1, dst=title)


def jump_object(number):
    return command(1, 0, 1, branch=0, dst=number)


def play_pl(playlist):
    return command(1, 0, 2, branch=0, dst=playlist)


def play_pl_mark(playlist, mark):
    return command(2, 0, 2, branch=2, dst=playlist, src=mark, imm2=1)


def set_audio(number):
    return command(2, 2, 1, set_opt=1, dst=0x80000000 | (number << 16), imm2=1)


def interactive_composition(commands):
    buttons = b""
    count = len(BUTTONS)
    for index, ((x, y), cmds) in enumerate(zip(BUTTONS, commands)):
        number = index + 1
        left = count if number == 1 else number - 1
        right = 1 if number == count else number + 1
        button = struct.pack(">HHB", number, 0xFFFF, 0)
        button += struct.pack(">HH", x, y)
        button += struct.pack(">HHHH", number, number, left, right)
        button += struct.pack(">HHB", 0, 0, 0)          # normal: object 0
        button += struct.pack(">BHHB", 0xFF, 1, 1, 0)   # selected: object 1
        button += struct.pack(">BHH", 0xFF, 2, 2)       # activated: object 2
        button += struct.pack(">H", len(cmds)) + b"".join(cmds)
        buttons += struct.pack(">HB", number, 1) + button
    page = bytes([0, 0]) + bytes(8)                     # id, version, UO mask
    page += bytes([0, 0]) + bytes([0, 0])               # in and out effects: none
    page += bytes([0]) + struct.pack(">HH", 1, 0xFFFF)  # frame rate, default selected, activated
    page += bytes([0, count]) + buttons                 # palette 0, button groups
    composition = bytes([0x00])                         # multiplexed, always on
    composition += bytes(5) + bytes(5)                  # composition and selection time-outs
    composition += (0).to_bytes(3, "big") + bytes([1]) + page
    body = struct.pack(">HHB", WIDTH, HEIGHT, 0x10)     # video: 1080p, 23.976
    body += struct.pack(">HB", 0, 0x80)                 # composition 0, epoch start
    body += bytes([0xC0])                               # first and last in sequence
    body += len(composition).to_bytes(3, "big") + composition
    return segment(0x18, body)


def ig_stream(pts):
    commands = [
        [jump_title(1)],
        [jump_title(2)],
        [set_audio(2), jump_title(1)],
    ]
    segments = [
        interactive_composition(commands),
        palette(),
        bitmap(0, 1),
        bitmap(1, 2),
        bitmap(2, 3),
        segment(0x80, b""),
    ]
    return [pes_packet(s, pts) for s in segments]


def add_menu(stream):
    """The menu clip with its interactive graphics after the first PMT."""
    source = packets(stream)
    pts = next(p for p in (pes_pts(s) for s in source if pid_of(s) == VIDEO_PID) if p is not None)
    out, counter, added = [], 0, False
    for s in source:
        if pid_of(s) == PMT_PID:
            s = with_ig_in_pmt(s)
            out.append(s)
            if not added:
                for pes in ig_stream(pts):
                    ts, counter = ts_packets(IG_PID, pes, counter, s[:4])
                    out += ts
                added = True
            continue
        out.append(s)
    return aligned(out)


# --- Navigation files ----------------------------------------------------

def intra_frames(source):
    """(SPN, PTS) of each MPEG-2 intra picture: its PES starts in the
    packet, and its picture header says I."""
    entries = []
    for spn, packet in enumerate(source):
        if pid_of(packet) != VIDEO_PID:
            continue
        pts = pes_pts(packet)
        if pts is None:
            continue
        data = payload_of(packet)
        at = data.find(b"\x00\x00\x01\x00")
        if at >= 0 and at + 5 < len(data) and (data[at + 5] >> 3) & 7 == 1:
            entries.append((spn, pts))
    return entries


def ep_map(source):
    """The video's entry points: one coarse entry for each fine one."""
    entries = intra_frames(source)
    # The first entry point is the clip's start (its PAT and PMT before
    # the picture), as authoring tools write it.
    entries[0] = (0, entries[0][1])
    coarse = b""
    fine = b""
    for index, (spn, pts) in enumerate(entries):
        coarse += struct.pack(">II", (index << 14) | ((pts >> 19) & 0x3FFF), spn)
        fine += struct.pack(">I", (1 << 28) | (((pts >> 9) & 0x7FF) << 17) | (spn & 0x1FFFF))
    stream = struct.pack(">I", 4 + len(coarse)) + coarse + fine
    header = bytes([0, 1])
    header += struct.pack(">H", VIDEO_PID)
    # Reserved 10 bits, stream type 1 (video), coarse and fine counts, and
    # where the stream's table starts (from the EP map's start).
    count = len(entries)
    header += struct.pack(">IHI", (1 << 20) | (count << 2) | (count >> 16), count & 0xFFFF, 2 + 12)
    assert len(header) == 14
    body = bytes([0x00, 0x01]) + header + stream  # CPI type 1: an EP map
    return struct.pack(">I", len(body)) + body


def stream_times(source):
    """First and last video PTS (90 kHz)."""
    stamps = [p for p in (pes_pts(s) for s in source if pid_of(s) == VIDEO_PID) if p is not None]
    return min(stamps), max(stamps)


def video_attributes():
    return bytes([0x02, 0x61, 0x30]) + bytes(2)  # MPEG-2, 1080p, 23.976, 16:9


def audio_attributes(language):
    return bytes([0x81, 0x31]) + language.encode()  # AC-3, stereo, 48 kHz


def ig_attributes():
    return bytes([0x91]) + b"eng" + bytes(1)


def clpi(source, streams):
    """Clip information: one ATC and STC sequence, the program's streams
    and the video's entry points."""
    first, last = stream_times(source)
    clip_info = struct.pack(">HBB", 0, 1, 1) + struct.pack(">I", 0)
    clip_info += struct.pack(">II", 6_000_000, len(source)) + bytes(128)
    clip_info += struct.pack(">H", 30) + bytes([0x80]) + b"HDMV" + bytes(25)
    clip_info = struct.pack(">I", len(clip_info)) + clip_info
    sequence = bytes([0, 1]) + struct.pack(">IBB", 0, 1, 0)
    sequence += struct.pack(">HIII", VIDEO_PID, 0, first // 2, (last + 3754) // 2)
    sequence = struct.pack(">I", len(sequence)) + sequence
    program = bytes([0, 1]) + struct.pack(">IHBB", 0, PMT_PID, len(streams), 0)
    for pid, attributes in streams:
        attributes = attributes + bytes(12)  # ISRC
        program += struct.pack(">HB", pid, len(attributes)) + attributes
    program = struct.pack(">I", len(program)) + program
    cpi = ep_map(source)
    marks = struct.pack(">I", 0)
    start = 40
    seq_at = start + len(clip_info)
    prog_at = seq_at + len(sequence)
    cpi_at = prog_at + len(program)
    mark_at = cpi_at + len(cpi)
    header = b"HDMV0200" + struct.pack(">IIIII", seq_at, prog_at, cpi_at, mark_at, 0)
    header += bytes(start - len(header))
    return header + clip_info + sequence + program + cpi + marks


def stream_entry(pid):
    return bytes([9, 1]) + struct.pack(">H", pid) + bytes(6)


def stn(video, audio, ig):
    body = bytes(2) + bytes([len(video), len(audio), 0, len(ig), 0, 0, 0, 0]) + bytes(4)
    for pid, attributes in video + audio + ig:
        body += stream_entry(pid) + bytes([len(attributes)]) + attributes
    return struct.pack(">H", len(body)) + body


def mpls(clip, first, last, still, streams, marks):
    item = clip.encode() + b"M2TS" + struct.pack(">HB", 0x0001, 0)  # connection 1, STC 0
    item += struct.pack(">II", first // 2, last // 2) + bytes(8)
    item += bytes([0]) + bytes([still]) + struct.pack(">H", 0)
    item += stn(*streams)
    item = struct.pack(">H", len(item)) + item
    playlist = bytes(2) + struct.pack(">HH", 1, 0) + item
    playlist = struct.pack(">I", len(playlist)) + playlist
    mark_body = struct.pack(">H", len(marks))
    for time in marks:
        mark_body += bytes([0, 1]) + struct.pack(">HIHI", 0, time // 2, 0xFFFF, 0)
    mark = struct.pack(">I", len(mark_body)) + mark_body
    app_info = bytes([0, 1]) + bytes(2) + bytes(8) + bytes([0x40, 0])
    app_info = struct.pack(">I", len(app_info)) + app_info
    list_at = 40 + len(app_info)
    mark_at = list_at + len(playlist)
    header = b"MPLS0200" + struct.pack(">III", list_at, mark_at, 0) + bytes(20)
    return header + app_info + playlist + mark


def movie_objects(objects):
    body = bytes(4) + struct.pack(">H", len(objects))
    for cmds in objects:
        body += struct.pack(">HH", 0, len(cmds)) + b"".join(cmds)
    body = struct.pack(">I", len(body)) + body
    header = b"MOBJ0200" + struct.pack(">I", 0) + bytes(28)
    return header + body


def index(titles):
    def hdmv(object_id, playback=0):
        return struct.pack(">HHI", playback << 14, object_id, 0)
    first_play = struct.pack(">I", 1 << 30) + hdmv(0, 1)
    top_menu = struct.pack(">I", 1 << 30) + hdmv(0, 1)
    body = first_play + top_menu + struct.pack(">H", len(titles))
    for object_id in titles:
        body += struct.pack(">I", 1 << 30) + hdmv(object_id)
    body = struct.pack(">I", len(body)) + body
    app_info = struct.pack(">I", 34) + bytes(34)
    header = b"INDX0200" + struct.pack(">II", 40 + len(app_info), 0) + bytes(24)
    return header + app_info + body


def main():
    out = Path(sys.argv[1])
    binary = sys.argv[2] if len(sys.argv) > 2 else "ffmpeg"
    bdmv = out / "BDMV"
    for folder in ("PLAYLIST", "CLIPINF", "STREAM", "BACKUP/PLAYLIST", "BACKUP/CLIPINF"):
        (bdmv / folder).mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temporary:
        menu_stream, title_stream = make_clips(binary, Path(temporary))
    menu = add_menu(menu_stream)
    title = aligned(packets(title_stream))
    files = {}
    files["STREAM/00000.m2ts"] = b"".join(menu)
    files["STREAM/00001.m2ts"] = b"".join(title)
    menu_streams = [(VIDEO_PID, video_attributes()), (AUDIO_PID, audio_attributes("eng")), (IG_PID, ig_attributes())]
    title_streams = [(VIDEO_PID, video_attributes()), (AUDIO_PID, audio_attributes("eng")),
                     (AUDIO_PID + 1, audio_attributes("deu"))]
    files["CLIPINF/00000.clpi"] = clpi(menu, menu_streams)
    files["CLIPINF/00001.clpi"] = clpi(title, title_streams)
    menu_first, menu_last = stream_times(menu)
    title_first, title_last = stream_times(title)
    # The menu's play item ends in an endless still (still mode 2).
    files["PLAYLIST/00000.mpls"] = mpls("00000", menu_first, menu_last + 3754, 2,
                                        ([menu_streams[0]], [menu_streams[1]], [menu_streams[2]]), [menu_first])
    files["PLAYLIST/00001.mpls"] = mpls("00001", title_first, title_last + 3754, 0,
                                        ([title_streams[0]], title_streams[1:], []),
                                        [title_first, title_first + CHAPTER_2 * 90000])
    files["MovieObject.bdmv"] = movie_objects([
        [play_pl(0)],
        [play_pl(1), jump_object(0)],
        [play_pl_mark(1, 1), jump_object(0)],
    ])
    files["index.bdmv"] = index([1, 2])
    for name, data in files.items():
        (bdmv / name).write_bytes(data)
        if not name.startswith("STREAM/"):
            (bdmv / "BACKUP" / name).write_bytes(data)
    with open(out / "buttons.txt", "w") as listing:
        for number, (x, y) in enumerate(BUTTONS, 1):
            listing.write(f"button {number} {x} {y} {BUTTON_W} {BUTTON_H}\n")
    print(f"Blu-ray in {out}: menu {len(menu)} packets, title {len(title)} packets")


if __name__ == "__main__":
    main()
