//! RVP's own art for classic skins (M9): every bitmap a classic skin may
//! lack, drawn in code at its standard size with its sprites where Winamp
//! reads them. Winamp falls back to its own base skin, which is Winamp's
//! art; RVP draws plain panels, bevels, symbols and a small font of its
//! own instead (M7's rule: every asset drawn for RVP). The same bitmaps
//! make the synthetic skin of the tests ([`synthetic_skin`]).

use std::fmt::Write as _;

use crate::image::Image;

const PANEL: [u8; 3] = [0x2B, 0x2E, 0x36];
const LIGHT: [u8; 3] = [0x5A, 0x60, 0x6E];
const DARK: [u8; 3] = [0x12, 0x14, 0x18];
const FACE: [u8; 3] = [0x3A, 0x3E, 0x48];
const PRESSED: [u8; 3] = [0x22, 0x25, 0x2C];
const INK: [u8; 3] = [0xC8, 0xCC, 0xD4];
const ACCENT: [u8; 3] = [0xF0, 0x8C, 0x28];
const DISPLAY: [u8; 3] = [0x05, 0x07, 0x0A];
const LIT: [u8; 3] = [0x3C, 0xE6, 0x50];
const UNLIT: [u8; 3] = [0x1E, 0x3C, 0x24];
const TITLE: [u8; 3] = [0x24, 0x3A, 0x6A];
const TITLE_INACTIVE: [u8; 3] = [0x30, 0x33, 0x3A];

/// The standard size of each bitmap (what every sprite Winamp reads needs).
#[must_use]
pub fn size(name: &str) -> (u32, u32) {
    match name {
        "main" | "eqmain_bg" => (275, 116),
        "titlebar" => (344, 87),
        "cbuttons" => (136, 36),
        "shufrep" => (92, 85),
        "posbar" => (307, 10),
        "volume" | "balance" => (68, 433),
        "monoster" => (58, 24),
        "playpaus" => (42, 9),
        "numbers" => (99, 13),
        "nums_ex" => (108, 13),
        "text" => (155, 18),
        "eqmain" => (275, 315),
        "eq_ex" => (275, 82),
        "pledit" => (280, 186),
        "video" => (234, 119),
        "gen" => (194, 109),
        _ => (1, 1),
    }
}

struct Canvas {
    image: Image,
}

impl Canvas {
    fn new(name: &str) -> Self {
        let (width, height) = size(name);
        let mut image = Image::new(width, height);
        image.fill([PANEL[0], PANEL[1], PANEL[2], 255]);
        Self { image }
    }

    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
        let width = i32::try_from(self.image.width()).unwrap_or(0);
        let height = i32::try_from(self.image.height()).unwrap_or(0);
        for py in y.max(0)..(y + h).min(height) {
            for px in x.max(0)..(x + w).min(width) {
                self.put(px, py, rgb);
            }
        }
    }

    fn put(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        let width = self.image.width() as usize;
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return;
        };
        if x >= width || y >= self.image.height() as usize {
            return;
        }
        let at = (y * width + x) * 4;
        self.image.data_mut()[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }

    /// A raised (or, `pressed`, sunken) box.
    fn bevel(&mut self, x: i32, y: i32, w: i32, h: i32, pressed: bool) {
        let (face, top, bottom) = if pressed {
            (PRESSED, DARK, LIGHT)
        } else {
            (FACE, LIGHT, DARK)
        };
        self.fill(x, y, w, h, face);
        self.fill(x, y, w, 1, top);
        self.fill(x, y, 1, h, top);
        self.fill(x, y + h - 1, w, 1, bottom);
        self.fill(x + w - 1, y, 1, h, bottom);
    }

    /// A sunken black display.
    fn display(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.fill(x, y, w, h, DISPLAY);
        self.fill(x, y, w, 1, DARK);
        self.fill(x, y + h - 1, w, 1, LIGHT);
    }

    /// Rows of a small picture, `#` set, centred in a box.
    fn picture(&mut self, x: i32, y: i32, w: i32, h: i32, rows: &[&str], rgb: [u8; 3]) {
        let ph = i32::try_from(rows.len()).unwrap_or(0);
        let pw = rows
            .iter()
            .map(|row| i32::try_from(row.len()).unwrap_or(0))
            .max()
            .unwrap_or(0);
        let (ox, oy) = (x + (w - pw) / 2, y + (h - ph) / 2);
        for (row, line) in rows.iter().enumerate() {
            for (column, cell) in line.chars().enumerate() {
                if cell == '#' {
                    self.put(
                        ox + i32::try_from(column).unwrap_or(0),
                        oy + i32::try_from(row).unwrap_or(0),
                        rgb,
                    );
                }
            }
        }
    }

    /// Text in RVP's 4x5 font at a point.
    fn text(&mut self, x: i32, y: i32, text: &str, rgb: [u8; 3]) {
        for (index, c) in text.chars().enumerate() {
            if let Some(rows) = glyph(c) {
                let gx = x + 5 * i32::try_from(index).unwrap_or(0);
                for (row, line) in rows.iter().enumerate() {
                    for (column, cell) in line.chars().enumerate() {
                        if cell == '#' {
                            self.put(
                                gx + i32::try_from(column).unwrap_or(0),
                                y + i32::try_from(row).unwrap_or(0),
                                rgb,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// RVP's 4x5 font for `text.bmp`'s 5x6 cells.
fn glyph(c: char) -> Option<[&'static str; 5]> {
    Some(match c.to_ascii_uppercase() {
        'A' => [".##.", "#..#", "####", "#..#", "#..#"],
        'B' => ["###.", "#..#", "###.", "#..#", "###."],
        'C' => [".###", "#...", "#...", "#...", ".###"],
        'D' => ["###.", "#..#", "#..#", "#..#", "###."],
        'E' => ["####", "#...", "###.", "#...", "####"],
        'F' => ["####", "#...", "###.", "#...", "#..."],
        'G' => [".###", "#...", "#.##", "#..#", ".###"],
        'H' => ["#..#", "#..#", "####", "#..#", "#..#"],
        'I' => [".###", "..#.", "..#.", "..#.", ".###"],
        'J' => ["...#", "...#", "...#", "#..#", ".##."],
        'K' => ["#..#", "#.#.", "##..", "#.#.", "#..#"],
        'L' => ["#...", "#...", "#...", "#...", "####"],
        'M' => ["#..#", "####", "####", "#..#", "#..#"],
        'N' => ["#..#", "##.#", "#.##", "#..#", "#..#"],
        'O' => [".##.", "#..#", "#..#", "#..#", ".##."],
        'P' => ["###.", "#..#", "###.", "#...", "#..."],
        'Q' => [".##.", "#..#", "#..#", "#.#.", ".#.#"],
        'R' => ["###.", "#..#", "###.", "#.#.", "#..#"],
        'S' => [".###", "#...", ".##.", "...#", "###."],
        'T' => ["####", ".#..", ".#..", ".#..", ".#.."],
        'U' => ["#..#", "#..#", "#..#", "#..#", ".##."],
        'V' => ["#..#", "#..#", "#..#", ".##.", ".##."],
        'W' => ["#..#", "#..#", "####", "####", "#..#"],
        'X' => ["#..#", ".##.", ".##.", ".##.", "#..#"],
        'Y' => ["#..#", "#..#", ".###", "...#", ".##."],
        'Z' => ["####", "..#.", ".#..", "#...", "####"],
        '0' => [".##.", "#.##", "##.#", "#..#", ".##."],
        '1' => [".#..", "##..", ".#..", ".#..", "###."],
        '2' => ["###.", "...#", ".##.", "#...", "####"],
        '3' => ["###.", "...#", ".##.", "...#", "###."],
        '4' => ["#..#", "#..#", "####", "...#", "...#"],
        '5' => ["####", "#...", "###.", "...#", "###."],
        '6' => [".##.", "#...", "###.", "#..#", ".##."],
        '7' => ["####", "...#", "..#.", ".#..", ".#.."],
        '8' => [".##.", "#..#", ".##.", "#..#", ".##."],
        '9' => [".##.", "#..#", ".###", "...#", ".##."],
        '.' => ["....", "....", "....", "....", ".#.."],
        ':' => ["....", ".#..", "....", ".#..", "...."],
        '(' => ["..#.", ".#..", ".#..", ".#..", "..#."],
        ')' => [".#..", "..#.", "..#.", "..#.", ".#.."],
        '-' => ["....", "....", "###.", "....", "...."],
        '\'' => [".#..", ".#..", "....", "....", "...."],
        '!' => [".#..", ".#..", ".#..", "....", ".#.."],
        '_' => ["....", "....", "....", "....", "####"],
        '+' => ["....", ".#..", "###.", ".#..", "...."],
        '\\' => ["#...", "#...", ".#..", "..#.", "..#."],
        '/' => ["..#.", "..#.", ".#..", "#...", "#..."],
        '[' => [".##.", ".#..", ".#..", ".#..", ".##."],
        ']' => [".##.", "..#.", "..#.", "..#.", ".##."],
        '^' => [".#..", "#.#.", "....", "....", "...."],
        '&' => [".#..", "#.#.", ".#..", "#.#.", ".#.#"],
        '%' => ["#..#", "..#.", ".#..", "#...", "#..#"],
        ',' => ["....", "....", "....", ".#..", "#..."],
        '=' => ["....", "###.", "....", "###.", "...."],
        '$' => [".###", "##..", ".##.", "..##", "###."],
        '#' => [".#.#", "####", ".#.#", "####", ".#.#"],
        '"' => ["#.#.", "#.#.", "....", "....", "...."],
        '@' => [".##.", "#.##", "#.##", "#...", ".###"],
        '?' => ["###.", "...#", ".##.", "....", ".#.."],
        '*' => ["....", "#.#.", ".#..", "#.#.", "...."],
        _ => return None,
    })
}

/// `text.bmp`'s cells: row, column of each character (Winamp's grid of
/// 31 by 3 cells of 5x6).
#[must_use]
pub fn text_cell(c: char) -> Option<(i32, i32)> {
    let c = c.to_ascii_uppercase();
    if c.is_ascii_uppercase() {
        return Some((0, i32::try_from(u32::from(c) - u32::from('A')).unwrap_or(0)));
    }
    if c.is_ascii_digit() {
        return Some((1, i32::try_from(u32::from(c) - u32::from('0')).unwrap_or(0)));
    }
    Some(match c {
        '"' => (0, 26),
        '@' => (0, 27),
        ' ' => (0, 30),
        '\u{2026}' => (1, 10),
        '.' => (1, 11),
        ':' => (1, 12),
        '(' => (1, 13),
        ')' => (1, 14),
        '-' => (1, 15),
        '\'' | '`' => (1, 16),
        '!' => (1, 17),
        '_' => (1, 18),
        '+' => (1, 19),
        '\\' => (1, 20),
        '/' => (1, 21),
        '[' | '{' | '<' => (1, 22),
        ']' | '}' | '>' => (1, 23),
        '^' | '~' => (1, 24),
        '&' => (1, 25),
        '%' => (1, 26),
        ',' => (1, 27),
        '=' => (1, 28),
        '$' => (1, 29),
        '#' => (1, 30),
        'Å' | 'å' => (2, 0),
        'Ö' | 'ö' => (2, 1),
        'Ä' | 'ä' => (2, 2),
        '?' => (2, 3),
        '*' => (2, 4),
        _ => return None,
    })
}

const PREVIOUS: [&str; 7] = [
    "#....#", "#...##", "#..###", "#.####", "#..###", "#...##", "#....#",
];
const PLAY: [&str; 7] = ["#...", "##..", "###.", "####", "###.", "##..", "#..."];
const PAUSE: [&str; 7] = [
    "##.##", "##.##", "##.##", "##.##", "##.##", "##.##", "##.##",
];
const STOP: [&str; 6] = ["######", "######", "######", "######", "######", "######"];
const NEXT: [&str; 7] = [
    "#....#", "##...#", "###..#", "####.#", "###..#", "##...#", "#....#",
];
const EJECT: [&str; 6] = [
    "...#...", "..###..", ".#####.", "#######", ".......", "#######",
];
const CLOSE: [&str; 5] = ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"];
const MINIMIZE: [&str; 5] = [".....", ".....", ".....", ".....", "#####"];
const SHADE: [&str; 5] = ["#####", ".....", ".....", ".....", "....."];
const OPTIONS: [&str; 5] = ["#####", ".....", "#####", ".....", "#####"];
const THUMB: [&str; 1] = [""];

/// Seven segments of a 9x13 digit: top, upper left, upper right, middle,
/// lower left, lower right, bottom.
fn segments(digit: u8) -> [bool; 7] {
    match digit {
        0 => [true, true, true, false, true, true, true],
        1 => [false, false, true, false, false, true, false],
        2 => [true, false, true, true, true, false, true],
        3 => [true, false, true, true, false, true, true],
        4 => [false, true, true, true, false, true, false],
        5 => [true, true, false, true, false, true, true],
        6 => [true, true, false, true, true, true, true],
        7 => [true, false, true, false, false, true, false],
        8 => [true; 7],
        9 => [true, true, true, true, false, true, true],
        _ => [false; 7],
    }
}

fn digit(canvas: &mut Canvas, x: i32, value: Option<u8>) {
    canvas.fill(x, 0, 9, 13, DISPLAY);
    let Some(value) = value else {
        return;
    };
    let on = segments(value);
    let parts = [
        (1, 0, 7, 2),
        (0, 1, 2, 6),
        (7, 1, 2, 6),
        (1, 6, 7, 1),
        (0, 6, 2, 6),
        (7, 6, 2, 6),
        (1, 11, 7, 2),
    ];
    for (lit, (px, py, w, h)) in on.iter().zip(parts) {
        canvas.fill(x + px, py, w, h, if *lit { LIT } else { UNLIT });
    }
}

/// The bitmap `name` as RVP draws it.
#[must_use]
#[allow(clippy::too_many_lines)] // One drawing per bitmap of the format.
pub fn bitmap(name: &str) -> Image {
    let mut c = Canvas::new(name);
    match name {
        "main" => {
            c.bevel(0, 0, 275, 116, false);
            // Time, visualizer, title and the rates.
            c.display(9, 22, 98, 44);
            c.display(108, 23, 159, 14);
            c.display(108, 41, 30, 10);
            c.display(153, 41, 18, 10);
            c.text(140, 43, "KBPS", INK);
            c.text(174, 43, "KHZ", INK);
            c.fill(70, 28, 2, 2, LIT);
            c.fill(70, 34, 2, 2, LIT);
            c.display(16, 71, 249, 12);
            c.text(233, 99, "RVP", ACCENT);
        }
        "titlebar" => {
            for (y, color) in [
                (0, TITLE),
                (15, TITLE_INACTIVE),
                (29, TITLE),
                (42, TITLE_INACTIVE),
                (57, TITLE),
                (72, TITLE_INACTIVE),
            ] {
                let height = 14;
                c.fill(27, y, 275, height, color);
                c.text(27 + 100, y + 5, "RVP CLASSIC", INK);
            }
            for (x, symbol) in [(0, &OPTIONS[..]), (9, &MINIMIZE[..]), (18, &CLOSE[..])] {
                for (y, pressed) in [(0, false), (9, true)] {
                    c.bevel(x, y, 9, 9, pressed);
                    c.picture(x, y, 9, 9, symbol, INK);
                }
            }
            for (x, y, pressed) in [(0, 18, false), (9, 18, true), (0, 27, false), (9, 27, true)] {
                c.bevel(x, y, 9, 9, pressed);
                c.picture(x, y, 9, 9, &SHADE, INK);
            }
            c.display(0, 36, 17, 7);
            for x in [17, 20, 23] {
                c.fill(x, 36, 3, 7, ACCENT);
            }
            for (x, enabled) in [(304, true), (312, false)] {
                c.fill(x, 0, 8, 43, if enabled { FACE } else { PRESSED });
                for (row, letter) in ["O", "A", "I", "D", "V"].iter().enumerate() {
                    c.text(x + 2, 2 + 8 * i32::try_from(row).unwrap_or(0), letter, INK);
                }
            }
            for k in 0..5 {
                c.fill(304 + 8 * k, 44, 8, 43, PRESSED);
            }
        }
        "cbuttons" => {
            let buttons: [(i32, i32, &[&str]); 5] = [
                (0, 23, &PREVIOUS),
                (23, 23, &PLAY),
                (46, 23, &PAUSE),
                (69, 23, &STOP),
                (92, 22, &NEXT),
            ];
            for (x, w, symbol) in buttons {
                for (y, pressed) in [(0, false), (18, true)] {
                    c.bevel(x, y, w, 18, pressed);
                    c.picture(x, y, w, 18, symbol, INK);
                }
            }
            for (y, pressed) in [(0, false), (16, true)] {
                c.bevel(114, y, 22, 16, pressed);
                c.picture(114, y, 22, 16, &EJECT, INK);
            }
        }
        "shufrep" => {
            for (state, y) in [0, 15, 30, 45].into_iter().enumerate() {
                let (on, pressed) = (state >= 2, state % 2 == 1);
                c.bevel(0, y, 28, 15, pressed);
                c.text(5, y + 5, "REP", if on { ACCENT } else { INK });
                c.bevel(28, y, 47, 15, pressed);
                c.text(36, y + 5, "SHUF", if on { ACCENT } else { INK });
            }
            for (on, y) in [(false, 61), (true, 73)] {
                for (pressed, dx) in [(false, 0), (true, 46)] {
                    for (x, label) in [(0, "EQ"), (23, "PL")] {
                        c.bevel(x + dx, y, 23, 12, pressed);
                        c.text(x + dx + 7, y + 4, label, if on { ACCENT } else { INK });
                    }
                }
            }
        }
        "posbar" => {
            c.display(0, 0, 248, 10);
            c.fill(2, 4, 244, 2, FACE);
            c.bevel(248, 0, 29, 10, false);
            c.bevel(278, 0, 29, 10, true);
        }
        "volume" | "balance" => {
            for frame in 0..28 {
                let y = 15 * frame;
                c.display(0, y, 68, 13);
                let level = 2 + frame * 64 / 27;
                c.fill(2, y + 5, level, 3, if frame == 0 { UNLIT } else { LIT });
            }
            c.bevel(15, 422, 14, 11, false);
            c.bevel(0, 422, 14, 11, true);
        }
        "monoster" => {
            for (y, lit) in [(0, true), (12, false)] {
                let color = if lit { LIT } else { UNLIT };
                c.display(0, y, 29, 12);
                c.text(2, y + 4, "STEREO", color);
                c.display(29, y, 28, 12);
                c.text(33, y + 4, "MONO", color);
            }
        }
        "playpaus" => {
            c.fill(0, 0, 42, 9, DISPLAY);
            c.picture(0, 0, 9, 9, &["#..", "##.", "###", "##.", "#.."], LIT);
            c.picture(9, 0, 9, 9, &["#.#", "#.#", "#.#", "#.#", "#.#"], LIT);
            c.picture(18, 0, 9, 9, &["###", "###", "###"], LIT);
            c.fill(36, 2, 2, 5, LIT);
            c.fill(39, 2, 2, 5, UNLIT);
        }
        "numbers" | "nums_ex" => {
            for value in 0..10_u8 {
                digit(&mut c, 9 * i32::from(value), Some(value));
            }
            digit(&mut c, 90, None);
            if name == "nums_ex" {
                c.fill(99, 0, 9, 13, DISPLAY);
                c.fill(100, 6, 7, 1, LIT);
            }
        }
        "text" => {
            c.fill(0, 0, 155, 18, DISPLAY);
            let all = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\"@0123456789.:()-'!_+\\/[]^&%,=$#?*";
            for ch in all.chars() {
                if let Some((row, column)) = text_cell(ch) {
                    c.text(column * 5, row * 6, &ch.to_string(), LIT);
                }
            }
        }
        "eqmain" => {
            c.bevel(0, 0, 275, 116, false);
            c.display(86, 17, 113, 19);
            c.text(20, 104, "PRE", INK);
            c.text(84, 104, "60    310   1K    6K    14K", INK);
            for (y, color) in [(134, TITLE), (149, TITLE_INACTIVE)] {
                c.fill(0, y, 275, 14, color);
                c.text(110, y + 5, "EQUALIZER", INK);
            }
            for (y, pressed) in [(116, false), (125, true)] {
                c.bevel(0, y, 9, 9, pressed);
                c.picture(0, y, 9, 9, &CLOSE, INK);
            }
            for pressed in [false, true] {
                for on in [false, true] {
                    let dx = if pressed { 118 } else { 0 } + if on { 59 } else { 0 };
                    let ink = if on { ACCENT } else { INK };
                    c.bevel(10 + dx, 119, 25, 12, pressed);
                    c.text(15 + dx, 123, "ON", ink);
                    c.bevel(35 + dx, 119, 33, 12, pressed);
                    c.text(40 + dx, 123, "AUTO", ink);
                }
            }
            for (y, pressed) in [(164, false), (176, true)] {
                c.bevel(224, y, 44, 12, pressed);
                c.text(229, y + 4, "PRESETS", INK);
            }
            for n in 0..28 {
                let (x, y) = if n < 14 {
                    (13 + 15 * n, 164)
                } else {
                    (13 + 15 * (n - 14), 229)
                };
                c.display(x, y, 14, 63);
                let filled = 63 - (27 - n) * 63 / 27;
                c.fill(
                    x + 5,
                    y + 63 - filled,
                    4,
                    filled,
                    if n < 4 { UNLIT } else { LIT },
                );
            }
            c.bevel(0, 164, 11, 11, false);
            c.bevel(0, 176, 11, 11, true);
            c.display(0, 294, 113, 19);
            c.fill(0, 314, 113, 1, INK);
            for row in 0..19 {
                let t = u8::try_from(row * 255 / 18).unwrap_or(u8::MAX);
                c.fill(115, 294 + row, 1, 1, [u8::MAX - t / 2, t, 0]);
            }
        }
        "eq_ex" => {
            for (y, color) in [(0, TITLE), (15, TITLE_INACTIVE)] {
                c.fill(0, y, 275, 14, color);
            }
            for x in [1, 4, 7, 11, 14, 17] {
                c.fill(x, 30, 3, 7, ACCENT);
            }
            c.bevel(1, 38, 9, 9, true);
            c.bevel(11, 38, 9, 9, false);
            c.bevel(11, 47, 9, 9, true);
        }
        "pledit" => {
            for (y, color) in [(0, TITLE), (21, TITLE_INACTIVE)] {
                for x in [0, 26, 127, 153] {
                    c.fill(x, y, 25, 20, color);
                }
                c.fill(26, y, 100, 20, color);
                c.text(44, y + 8, "PLAYLIST", INK);
                c.bevel(153 + 14, y + 3, 9, 9, false);
                c.picture(153 + 14, y + 3, 9, 9, &CLOSE, INK);
                c.bevel(153 + 5, y + 3, 9, 9, false);
                c.picture(153 + 5, y + 3, 9, 9, &SHADE, INK);
            }
            c.fill(0, 42, 12, 29, FACE);
            c.fill(31, 42, 5, 29, FACE);
            c.fill(36, 42, 8, 29, DISPLAY);
            c.fill(44, 42, 7, 29, FACE);
            c.bevel(52, 53, 8, 18, false);
            c.bevel(61, 53, 8, 18, true);
            c.bevel(52, 42, 9, 9, true);
            c.bevel(62, 42, 9, 9, true);
            c.fill(0, 72, 125, 38, FACE);
            c.fill(179, 0, 25, 38, FACE);
            c.fill(205, 0, 75, 38, FACE);
            c.fill(126, 72, 150, 38, FACE);
            c.display(126 + 4, 72 + 10, 60, 8);
            for (index, label) in ["ADD", "REM", "SEL", "MISC"].iter().enumerate() {
                let x = 11 + 29 * i32::try_from(index).unwrap_or(0);
                c.bevel(x, 72 + 14, 25, 18, false);
                c.text(x + 3, 72 + 20, label, INK);
            }
            c.fill(72, 42, 25, 14, TITLE);
            c.fill(72, 57, 25, 14, TITLE);
            c.fill(99, 42, 50, 14, TITLE);
            c.fill(99, 57, 50, 14, TITLE_INACTIVE);
            for (x, w) in [(0, 25), (54, 25), (104, 25), (154, 25), (204, 25)] {
                for y in [111, 130, 149, 168] {
                    c.bevel(x, y, 22, 18, false);
                    c.bevel(x + 23, y, 22, 18, true);
                }
                c.fill(x + w + 23, 111, 3, 54, ACCENT);
            }
        }
        "video" => {
            for (y, color) in [(0, TITLE), (21, TITLE_INACTIVE)] {
                for x in [0, 127, 153] {
                    c.fill(x, y, 25, 20, color);
                }
                c.fill(26, y, 100, 20, color);
                c.text(56, y + 8, "VIDEO", INK);
                c.bevel(167, y + 3, 9, 9, false);
                c.picture(167, y + 3, 9, 9, &CLOSE, INK);
            }
            c.fill(127, 42, 11, 29, FACE);
            c.fill(139, 42, 8, 29, FACE);
            c.fill(0, 42, 125, 38, FACE);
            c.fill(0, 81, 125, 38, FACE);
            c.fill(127, 81, 25, 38, FACE);
            let labels = ["F", "1", "2", "L", "M"];
            for (index, label) in labels.iter().enumerate() {
                let i = i32::try_from(index).unwrap_or(0);
                c.bevel(9 + 15 * i, 51, 15, 18, false);
                c.text(9 + 15 * i + 5, 51 + 7, label, INK);
                c.bevel(158 + 15 * i, 42, 15, 18, true);
                c.text(158 + 15 * i + 5, 42 + 7, label, INK);
            }
            c.bevel(148, 42, 9, 9, true);
            c.picture(148, 42, 9, 9, &CLOSE, INK);
        }
        _ => {
            c.picture(0, 0, 1, 1, &THUMB, INK);
        }
    }
    c.image
}

/// RVP's synthetic classic skin: every bitmap of [`super::BITMAPS`] as
/// RVP draws it, with a `pledit.txt` and `viscolor.txt`, as the members of
/// a `.wsz` (the tests' fixture: no third-party art).
#[must_use]
pub fn synthetic_skin() -> Vec<(String, Vec<u8>)> {
    let mut members: Vec<(String, Vec<u8>)> = super::BITMAPS
        .iter()
        .filter(|name| **name != "gen")
        .map(|name| (format!("{name}.bmp"), bitmap(name).encode_bmp()))
        .collect();
    members.push((
        "pledit.txt".to_owned(),
        b"[Text]\r\nNormal=#00FF00\r\nCurrent=#FFFFFF\r\nNormalBG=#000000\r\nSelectedBG=#0000C6\r\nFont=Arial\r\n".to_vec(),
    ));
    let mut vis = String::new();
    for color in super::default_vis_colors() {
        let _ = write!(vis, "{},{},{}\r\n", color[0], color[1], color[2]);
    }
    members.push(("viscolor.txt".to_owned(), vis.into_bytes()));
    members
}
