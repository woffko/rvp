//! The RVP skin built from a classic Winamp skin (M9): PotPlayer-format
//! markup and pictures in memory, so that RVP's engine lays out, paints
//! and hit-tests the classic windows as it does `.dsf` skins.
//!
//! * `MainSkinArea` is Winamp 5's classic video window around RVP's
//!   video (RVP's main window): `video.bmp`'s frame when the skin has one,
//!   otherwise the playlist frame's pieces of `pledit.bmp` (titled
//!   without "playlist editor"), which almost every skin carries. Its
//!   fullscreen area holds the video alone.
//! * The main window, the equalizer and the playlist are windows of their
//!   own (`RvpWindow='1'` areas: `WinampMainSkinArea`,
//!   `WinampEqualizerSkinArea`, `WinampPlaylistSkinArea`), docked to the
//!   video window as Winamp 5 places them.
//! * Buttons get pictures of their states side by side (normal, hover as
//!   normal, pressed), toggles their lit states in `RvpOnImageLeft`; the
//!   volume, balance and equalizer sliders a picture per value
//!   (`RvpFrames`); the time a 22-cell digit font; the song title, rates
//!   and the playlist's small time `text.bmp`'s cells (`RvpChars`); the
//!   analyzer `viscolor.txt`'s colours per row (`RvpRowColorsX`); the
//!   equalizer's curve `RvpGraph`. `colors.png` holds the skin's colours
//!   as pixels: the playlist's, the info line's and the visualizer's.
//! * Commands are RVP's actions by name (`RVP:play`), the windows'
//!   toggles `RVPWIN:` names, and the texts and states RVP gives these
//!   skins `RVP:` names (`crate::state`'s users fill them).

use super::{WinampSkin, fallback};
use crate::archive::SkinArchive;
use crate::compose::{El, crop, join};
use crate::error::SkinError;
use crate::image::Image;
use crate::skin::Skin;

/// The extra windows' areas.
pub const MAIN_AREA: &str = "WinampMainSkinArea";
pub const EQUALIZER_AREA: &str = "WinampEqualizerSkinArea";
pub const PLAYLIST_AREA: &str = "WinampPlaylistSkinArea";

/// The characters of `text.strip.png`'s cells, in order (space last).
pub const TEXT_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\"@0123456789.:()-'!_+\\/[]^&%,=$#ÅÖÄ?* ";

/// The pictures of the built skin, by file name.
struct Files {
    images: Vec<(String, Image)>,
}

impl Files {
    fn add(&mut self, name: &str, image: Image) -> String {
        self.images.push((name.to_owned(), image));
        name.to_owned()
    }

    /// A button's states side by side from two sprites of one bitmap:
    /// normal, hover (as normal), pressed.
    #[allow(clippy::too_many_arguments)] // Two sprites of one bitmap.
    fn states(
        &mut self,
        name: &str,
        bitmap: &Image,
        normal: (i32, i32),
        pressed: (i32, i32),
        w: i32,
        h: i32,
    ) -> String {
        let normal = crop(bitmap, normal.0, normal.1, w, h);
        let pressed = crop(bitmap, pressed.0, pressed.1, w, h);
        self.add(name, join(&[normal.clone(), normal, pressed], false))
    }
}

impl Files {
    /// A toggle's states: off (normal, hover, pressed) in the first row,
    /// lit below them.
    #[allow(clippy::too_many_arguments)] // Four sprites of one bitmap.
    fn toggle_states(
        &mut self,
        name: &str,
        bitmap: &Image,
        off: [(i32, i32); 2],
        on: [(i32, i32); 2],
        w: i32,
        h: i32,
    ) -> String {
        let row = |[normal, pressed]: [(i32, i32); 2]| {
            let normal = crop(bitmap, normal.0, normal.1, w, h);
            let pressed = crop(bitmap, pressed.0, pressed.1, w, h);
            join(&[normal.clone(), normal, pressed], false)
        };
        self.add(name, join(&[row(off), row(on)], true))
    }
}

/// A button with its states in `file`.
fn button(file: &str, command: &str, tooltip: &str, x: i32, y: i32, w: i32, h: i32) -> El {
    El::new("Button")
        .a("Tooltip", tooltip)
        .a("CmdID", command)
        .rect(x, y, w, h)
        .image(file, 0, 0, w, h)
}

/// A picture that shows while one of the `RVP:` states is set.
fn indicator(file: &str, state: &str, x: i32, y: i32, w: i32, h: i32) -> El {
    El::new("Static")
        .a("State", state)
        .rect(x, y, w, h)
        .image(file, 0, 0, w, h)
}

/// Builds the RVP skin of a classic skin.
///
/// # Errors
/// Fails only when a picture cannot be encoded or the result not loaded.
pub fn skin(classic: &WinampSkin) -> Result<Skin, SkinError> {
    let mut files = Files { images: Vec::new() };
    let colors = colors(classic);
    files.add("colors.png", colors);
    let text = text_strip(classic);
    files.add("text.strip.png", text);
    let digits = digit_font(classic);
    files.add("digits.png", digits);

    let mut root = El::new("Skin")
        .a("Version", "1.0")
        .a("MinorVersion", "1000");
    let (video, video_osc) = video_window(classic, &mut files);
    root = root.child(video).child(video_osc);
    root = root.child(main_window(classic, &mut files));
    root = root.child(equalizer_window(classic, &mut files));
    root = root.child(playlist_window(classic, &mut files));

    let mut markup = String::from("<?XML version='1.0' ?>\n");
    root.write(&mut markup, 0);
    let mut members: Vec<(String, Vec<u8>)> = Vec::with_capacity(files.images.len() + 1);
    members.push(("VideoSkin.xml".to_owned(), markup.into_bytes()));
    for (name, image) in &files.images {
        members.push((name.clone(), image.encode_png()?));
    }
    Skin::load(
        SkinArchive::from_files(&classic.name, members),
        "VideoSkin.xml",
    )
}

/// The skin's colours as pixels of `colors.png`: row 0 the playlist's
/// (normal text, current, background, selected background), the info
/// line's (mbFG, mbBG); row 1 `viscolor.txt`'s 24; row 2 the analyzer's
/// rows from the bottom (colours 17 down to 2).
fn colors(classic: &WinampSkin) -> Image {
    let mut image = Image::new(24, 3);
    let style = &classic.playlist;
    let put = |image: &mut Image, x: usize, y: usize, rgb: [u8; 3]| {
        let at = (y * 24 + x) * 4;
        image.data_mut()[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    };
    for (x, rgb) in [
        style.normal,
        style.current,
        style.normal_bg,
        style.selected_bg,
        style.mb_fg,
        style.mb_bg,
    ]
    .into_iter()
    .enumerate()
    {
        put(&mut image, x, 0, rgb);
    }
    for (x, rgb) in classic.vis_colors.iter().enumerate() {
        put(&mut image, x, 1, *rgb);
    }
    for row in 0..16 {
        put(&mut image, row, 2, classic.vis_colors[17 - row]);
    }
    image
}

/// `text.bmp`'s cells in [`TEXT_CHARS`] order, 5x6 each.
fn text_strip(classic: &WinampSkin) -> Image {
    let text = classic.bitmap("text");
    let cells: Vec<Image> = TEXT_CHARS
        .chars()
        .map(|c| {
            let (row, column) = fallback::text_cell(c).unwrap_or((0, 30));
            crop(text, column * 5, row * 6, 5, 6)
        })
        .collect();
    join(&cells, false)
}

/// The time's digits in the engine's 22-cell order (`%` to `:`), 12
/// pixels a cell (the 9-pixel digit and the gap to the next); `-` the
/// minus sign.
fn digit_font(classic: &WinampSkin) -> Image {
    let numbers = classic.bitmap("numbers");
    let mut cells = vec![Image::new(12, 13); 22];
    for digit in 0..10 {
        cells[11 + digit] = join(
            &[
                crop(numbers, 9 * i32::try_from(digit).unwrap_or(0), 0, 9, 13),
                Image::new(3, 13),
            ],
            false,
        );
    }
    cells[8] = if classic.extended_numbers() {
        join(&[crop(numbers, 99, 0, 9, 13), Image::new(3, 13)], false)
    } else {
        // `numbers.bmp`'s minus: the bar of the 2.
        let mut minus = crop(numbers, 90, 0, 9, 13);
        let bar = crop(numbers, 20, 6, 5, 1);
        for x in 0..5 {
            let pixel = bar.pixel(x, 0);
            let at = (6 * 9 + 2 + usize::try_from(x).unwrap_or(0)) * 4;
            minus.data_mut()[at..at + 4].copy_from_slice(&pixel);
        }
        join(&[minus, Image::new(3, 13)], false)
    };
    join(&cells, false)
}

/// A `PicLabel` in `text.bmp`'s font.
fn text_label(command: &str, x: i32, y: i32, chars: i32, align: i32) -> El {
    El::new("PicLabel")
        .a("CmdID", command)
        .rect(x, y, 5 * chars, 6)
        .a("HoriAlignment", align)
        .image("text.strip.png", 0, 0, 5, 6)
        .a(
            "RvpChars",
            TEXT_CHARS
                .chars()
                .map(|c| u32::from(c).to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
        .a("RvpCellWidth", 5)
}

/// The video window (RVP's main window) and its fullscreen area.
#[allow(clippy::too_many_lines)] // One frame, piece by piece.
fn video_window(classic: &WinampSkin, files: &mut Files) -> (El, El) {
    let own = classic.has_video;
    let source = if own {
        classic.bitmap("video").clone()
    } else {
        classic.bitmap("pledit").clone()
    };
    let frame = files.add("video.frame.png", source.clone());
    // Title bar: corners, fillers, and the title piece (`video.bmp`'s;
    // the playlist's says "playlist editor": a filler there instead).
    let (left_width, right_width, bottom_height) = if own { (11, 8, 38) } else { (12, 7, 38) };
    let title = |y: i32| (y, y + 21);
    let (active, inactive) = title(0);
    // A piece of the title bar at `left` (negative: from the right).
    let piece = |x: i32, w: i32, left: i32| {
        El::new("Static")
            .rect(left, 0, w, 20)
            .image(&frame, x, active, w, 20)
            .a("AltImageLeft", x)
            .a("AltImageTop", inactive)
    };
    let close = files.states(
        "video.close.png",
        &source,
        (167, 3),
        if own { (148, 42) } else { (52, 42) },
        9,
        9,
    );
    let mut title_bar = El::new("Static")
        .a("Align", "Top")
        .a("Height", 20)
        .a("IsTitleBar", 1)
        .image(&frame, 127, active, 25, 20)
        .a("AltImageLeft", 127)
        .a("AltImageTop", inactive)
        .child(piece(0, 25, 0));
    if own {
        title_bar = title_bar.child(piece(26, 100, 0).a("HoriCenter", 1));
    }
    let title_bar = title_bar
        .child(piece(153, 25, -25))
        .child(button(&close, "RVP:exit", "Close", -11, 3, 9, 9));
    let (side_left, side_right) = if own {
        ((127, 42), (139, 42))
    } else {
        ((0, 42), (44, 42))
    };
    let mut bottom = El::new("Static")
        .a("Align", "Bottom")
        .a("Height", bottom_height)
        .image(
            &frame,
            if own { 127 } else { 179 },
            if own { 81 } else { 0 },
            25,
            38,
        );
    if own {
        bottom = bottom
            .child(
                El::new("Static")
                    .rect(0, 0, 125, 38)
                    .image(&frame, 0, 42, 125, 38),
            )
            .child(
                El::new("Static")
                    .rect(-125, 0, 125, 38)
                    .image(&frame, 0, 81, 125, 38),
            );
        let commands = [
            ("RVP:toggle-fullscreen", "Fullscreen"),
            ("RVP:window-normal", "Zoom 100 %"),
            ("RVP:window-double", "Zoom 200 %"),
            ("RVP:toggle-playlist", "Playlist"),
            ("RVP:context-menu", "Menu"),
        ];
        for (index, (command, tooltip)) in commands.iter().enumerate() {
            let i = i32::try_from(index).unwrap_or(0);
            let file = files.states(
                &format!("video.button{i}.png"),
                &source,
                (9 + 15 * i, 51),
                (158 + 15 * i, 42),
                15,
                18,
            );
            bottom = bottom.child(button(&file, command, tooltip, 9 + 15 * i, 9, 15, 18));
        }
        bottom = bottom.child(
            El::new("Label")
                .a("CmdID", "CMD_TITLE_MESSAGE")
                .a("Left", 92)
                .a("Top", 11)
                .a("Width", -25)
                .a("Height", 14)
                .a("VertAlignment", 1)
                .a(
                    "FontName",
                    if classic.playlist.font.is_empty() {
                        "Arial"
                    } else {
                        classic.playlist.font.as_str()
                    },
                )
                .a("FontSize", 10)
                .a("Image", "colors.png")
                .a("ColorX", 4)
                .a("ColorY", 0),
        );
    }
    let video = El::new("MainSkinArea")
        .a("Image", frame.as_str())
        .a("WidthLimit", 275)
        .a("HeightLimit", 116)
        .a("UseWindowFrame", 0)
        .child(
            El::new("Control")
                .child(title_bar)
                .child(bottom)
                .child(
                    El::new("Static")
                        .a("Align", "Left")
                        .a("Width", left_width)
                        .image(&frame, side_left.0, side_left.1, left_width, 29),
                )
                .child(
                    El::new("Static")
                        .a("Align", "Right")
                        .a("Width", right_width)
                        .image(&frame, side_right.0, side_right.1, right_width, 29),
                )
                .child(
                    El::new("Static")
                        .a("Align", "Client")
                        .a("Contain", "CMD_VIDEO_WINDOW"),
                ),
        );
    let fullscreen = El::new("MainSkinAreaOscInside")
        .a("Image", frame.as_str())
        .child(
            El::new("Control").child(
                El::new("Static")
                    .a("Align", "Client")
                    .a("Contain", "CMD_VIDEO_WINDOW"),
            ),
        );
    (video, fullscreen)
}

/// Winamp's main window (275x116), docked above the video window.
#[allow(clippy::too_many_lines)] // Every control at its fixed place.
fn main_window(classic: &WinampSkin, files: &mut Files) -> El {
    let main = files.add("main.png", crop(classic.bitmap("main"), 0, 0, 275, 116));
    let titlebar = classic.bitmap("titlebar").clone();
    let title_file = files.add("titlebar.png", titlebar.clone());
    let mut c = El::new("Static")
        .rect(0, 0, 275, 116)
        .image(&main, 0, 0, 275, 116);
    c = c.child(
        El::new("Static")
            .rect(0, 0, 275, 14)
            .a("IsTitleBar", 1)
            .image(&title_file, 27, 0, 275, 14)
            .a("AltImageLeft", 27)
            .a("AltImageTop", 15),
    );
    for (name, sprite_x, x, command, tooltip, shade) in [
        ("options", 0, 6, "RVP:context-menu", "Main menu", false),
        ("minimize", 9, 244, "RVP:minimize", "Minimize", false),
        ("shade", 0, 254, "", "Windowshade mode", true),
        ("close", 18, 264, "RVP:exit", "Close", false),
    ] {
        let (normal, pressed) = if shade {
            ((sprite_x, 18), (sprite_x + 9, 18))
        } else {
            ((sprite_x, 0), (sprite_x, 9))
        };
        let file = files.states(
            &format!("main.{name}.png"),
            &titlebar,
            normal,
            pressed,
            9,
            9,
        );
        c = c.child(button(&file, command, tooltip, x, 3, 9, 9));
    }
    // The clutter bar: O A I D V.
    c = c.child(
        El::new("Static")
            .rect(10, 22, 8, 43)
            .image(&title_file, 304, 0, 8, 43),
    );
    for (index, (top, height, command, tooltip)) in [
        (2, 8, "RVP:context-menu", "Options"),
        (10, 8, "RVP:on-top-cycle", "Always on top"),
        (18, 8, "RVP:playback-info", "File info"),
        (26, 8, "", "Double size"),
        (34, 7, "RVP:visualization-settings", "Visualization"),
    ]
    .into_iter()
    .enumerate()
    {
        let k = i32::try_from(index).unwrap_or(0);
        let file = files.states(
            &format!("main.clutter{k}.png"),
            &titlebar,
            (304, top),
            (304 + 8 * k, 44 + top),
            8,
            height,
        );
        c = c.child(button(&file, command, tooltip, 10, 22 + top, 8, height));
    }
    // Play state and the sync lamp.
    let playpaus = classic.bitmap("playpaus");
    for (state, x) in [("RVP:playing", 0), ("RVP:paused", 9), ("RVP:stopped", 18)] {
        let file = files.add(&format!("main.state{x}.png"), crop(playpaus, x, 0, 9, 9));
        c = c.child(indicator(&file, state, 26, 28, 9, 9));
    }
    let lamp = files.add("main.lamp.png", crop(playpaus, 36, 0, 3, 9));
    c = c.child(indicator(&lamp, "RVP:playing", 24, 28, 3, 9));
    // The time: the sign column, minutes and seconds.
    for (command, x, width) in [
        ("RVP:time-sign", 36, 12),
        ("RVP:time-minutes", 48, 24),
        ("RVP:time-seconds", 78, 24),
    ] {
        c = c.child(
            El::new("PicLabel")
                .a("CmdID", command)
                .rect(x, 26, width, 13)
                .image("digits.png", 0, 0, 12 * 22, 13),
        );
    }
    // The analyzer over its background (colour 0, grid dots colour 1).
    let mut grid = Image::new(76, 16);
    for y in 0..16_i32 {
        for x in 0..76_i32 {
            let rgb = if x % 2 == 1 && y % 2 == 1 {
                classic.vis_colors[1]
            } else {
                classic.vis_colors[0]
            };
            let at = (usize::try_from(y * 76 + x).unwrap_or(0)) * 4;
            grid.data_mut()[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    let grid = files.add("main.vis.png", grid);
    c = c.child(
        El::new("Static")
            .rect(24, 43, 76, 16)
            .image(&grid, 0, 0, 76, 16),
    );
    c = c.child(
        El::new("Visual")
            .rect(24, 43, 76, 16)
            .a("Image", "colors.png")
            .a("BarWidth", 3)
            .a("BarDistance", 1)
            .a("LineHeight", 1)
            .a("LineDistance", 0)
            .a("RvpRowColorsX", 0)
            .a("RvpRowColorsY", 2)
            .a("PeakHeight", 1)
            .a("PeakColorX", 23)
            .a("PeakColorY", 1),
    );
    c = c.child(text_label("RVP:marquee", 111, 27, 31, 0));
    c = c.child(text_label("RVP:kbps", 111, 43, 3, 2));
    c = c.child(text_label("RVP:khz", 156, 43, 2, 2));
    // Mono and stereo, lit or not.
    let monoster = classic.bitmap("monoster");
    for (state, x, sprite_x, sprite_y, w) in [
        ("RVP:stereo", 239, 0, 0, 29),
        ("RVP:no-stereo", 239, 0, 12, 29),
        ("RVP:mono", 212, 29, 0, 27),
        ("RVP:no-mono", 212, 29, 12, 27),
    ] {
        let file = files.add(
            &format!("main.channels{x}{sprite_y}.png"),
            crop(monoster, sprite_x, sprite_y, w, 12),
        );
        c = c.child(indicator(&file, state, x, 41, w, 12));
    }
    // Volume: 28 pictures and the thumb (normal, pressed) under them.
    let volume = classic.bitmap("volume");
    let mut frames: Vec<Image> = (0..28).map(|f| crop(volume, 0, 15 * f, 68, 13)).collect();
    frames.push(join(
        &[crop(volume, 15, 422, 14, 11), crop(volume, 0, 422, 14, 11)],
        false,
    ));
    let volume_file = files.add("main.volume.png", join(&frames, true));
    c = c.child(slider(
        &volume_file,
        "CMD_VOLUME_CTRL",
        "Volume",
        (107, 57, 68, 13),
        28,
        (14, 11),
    ));
    // Balance: 55 pictures from left to right (frame |pan| * 27 / 127).
    let balance = classic.bitmap("balance");
    let mut frames: Vec<Image> = (0..55)
        .map(|i| {
            let pan: i32 = -127 + i * 254 / 54;
            crop(balance, 9, 15 * (pan.abs() * 27 / 127), 38, 13)
        })
        .collect();
    frames.push(join(
        &[
            crop(balance, 15, 422, 14, 11),
            crop(balance, 0, 422, 14, 11),
        ],
        false,
    ));
    let balance_file = files.add("main.balance.png", join(&frames, true));
    c = c.child(slider(
        &balance_file,
        "RVP:balance",
        "Balance",
        (177, 57, 38, 13),
        55,
        (14, 11),
    ));
    // The equalizer's and the playlist's toggles.
    let shufrep = classic.bitmap("shufrep").clone();
    for (index, (command, tooltip, x)) in [
        ("RVPWIN:equalizer", "Equalizer", 219),
        ("RVPWIN:playlist", "Playlist", 242),
    ]
    .into_iter()
    .enumerate()
    {
        let sx = 23 * i32::try_from(index).unwrap_or(0);
        let file = files.toggle_states(
            &format!("main.toggle{index}.png"),
            &shufrep,
            [(sx, 61), (sx + 46, 61)],
            [(sx, 73), (sx + 46, 73)],
            23,
            12,
        );
        c = c.child(toggle(&file, command, tooltip, (x, 58, 23, 12)));
    }
    // Seek bar.
    let posbar = files.add("main.posbar.png", classic.bitmap("posbar").clone());
    c = c.child(
        El::new("Slider")
            .a("Tooltip", "Seek")
            .a("CmdID", "CMD_TIMEBAR_CTRL")
            .rect(16, 72, 248, 10)
            .a("Image", posbar.as_str())
            .a("ImageWidth", 248)
            .a("ImageHeight", 10)
            .a("BackImageLeft", 0)
            .a("BackImageTop", 0)
            .a("ThumbWidth", 29)
            .a("ThumbHeight", 10)
            .a("ThumbImageLeft", 248)
            .a("ThumbImageTop", 0)
            .a("ThumbPressedImageLeft", 278)
            .a("ThumbPressedImageTop", 0),
    );
    // Transport.
    let cbuttons = classic.bitmap("cbuttons").clone();
    for (name, sx, x, w, command, tooltip) in [
        ("previous", 0, 16, 23, "RVP:previous", "Previous"),
        ("play", 23, 39, 23, "RVP:play", "Play"),
        ("pause", 46, 62, 23, "RVP:play-pause", "Pause"),
        ("stop", 69, 85, 23, "RVP:close-media", "Stop"),
        ("next", 92, 108, 22, "RVP:next", "Next"),
    ] {
        let file = files.states(
            &format!("main.{name}.png"),
            &cbuttons,
            (sx, 0),
            (sx, 18),
            w,
            18,
        );
        c = c.child(button(&file, command, tooltip, x, 88, w, 18));
    }
    let eject = files.states("main.eject.png", &cbuttons, (114, 0), (114, 16), 22, 16);
    c = c.child(button(&eject, "RVP:open", "Open file", 136, 89, 22, 16));
    for (command, tooltip, sx, x, w) in [
        ("RVP:shuffle", "Shuffle", 28, 164, 47),
        ("RVP:repeat-album", "Repeat", 0, 210, 28),
    ] {
        let file = files.toggle_states(
            &format!("main.{sx}.png"),
            &shufrep,
            [(sx, 0), (sx, 15)],
            [(sx, 30), (sx, 45)],
            w,
            15,
        );
        c = c.child(toggle(&file, command, tooltip, (x, 89, w, 15)));
    }
    c = c.child(
        El::new("Button")
            .a("Tooltip", "About")
            .a("CmdID", "RVP:about")
            .rect(253, 91, 13, 15),
    );
    let mut area = El::new(MAIN_AREA)
        .a("Image", main.as_str())
        .a("Width", 275)
        .a("Height", 116)
        .a("RvpWindow", 1)
        .a("RvpDock", "above");
    if let Some(region) = classic.regions.get("Normal") {
        area = area.a("RvpRegion", region_text(region));
    }
    area.child(El::new("Control").child(c))
}

/// A slider drawn a picture per value, its thumb (normal, pressed) below
/// the pictures.
fn slider(
    file: &str,
    command: &str,
    tooltip: &str,
    rect: (i32, i32, i32, i32),
    frames: i32,
    thumb: (i32, i32),
) -> El {
    let (x, y, w, h) = rect;
    El::new("Slider")
        .a("Tooltip", tooltip)
        .a("CmdID", command)
        .rect(x, y, w, h)
        .a("Image", file)
        .a("ImageWidth", w)
        .a("ImageHeight", h)
        .a("BackImageLeft", 0)
        .a("BackImageTop", 0)
        .a("RvpFrames", frames)
        .a("ThumbWidth", thumb.0)
        .a("ThumbHeight", thumb.1)
        .a("ThumbImageLeft", 0)
        .a("ThumbImageTop", frames * h)
        .a("ThumbPressedImageLeft", thumb.0)
        .a("ThumbPressedImageTop", frames * h)
}

/// A toggle with its states in `file` (see [`Files::toggle_states`]).
fn toggle(file: &str, command: &str, tooltip: &str, rect: (i32, i32, i32, i32)) -> El {
    let (x, y, w, h) = rect;
    El::new("Button")
        .a("Tooltip", tooltip)
        .a("CmdID", command)
        .a("DownFlag", 1)
        .rect(x, y, w, h)
        .image(file, 0, 0, w, h)
        .a("RvpOnImageLeft", 0)
        .a("RvpOnImageTop", h)
}

/// `region.txt`'s polygons as `x,y x,y ...;...`.
fn region_text(region: &super::Region) -> String {
    region
        .iter()
        .map(|polygon| {
            polygon
                .iter()
                .map(|(x, y)| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Winamp's equalizer window (275x116), docked below the main window.
#[allow(clippy::too_many_lines)] // Every control at its fixed place.
fn equalizer_window(classic: &WinampSkin, files: &mut Files) -> El {
    let eq = classic.bitmap("eqmain").clone();
    let back = files.add("eq.png", eq.clone());
    let mut c = El::new("Static")
        .rect(0, 0, 275, 116)
        .image(&back, 0, 0, 275, 116);
    c = c.child(
        El::new("Static")
            .rect(0, 0, 275, 14)
            .a("IsTitleBar", 1)
            .image(&back, 0, 134, 275, 14)
            .a("AltImageLeft", 0)
            .a("AltImageTop", 149),
    );
    let close = files.states("eq.close.png", &eq, (0, 116), (0, 125), 9, 9);
    c = c.child(button(&close, "RVPWIN:equalizer", "Close", 264, 3, 9, 9));
    // ON: off and pressed from row 119, lit at +59.
    let on = files.toggle_states(
        "eq.on.png",
        &eq,
        [(10, 119), (128, 119)],
        [(69, 119), (187, 119)],
        25,
        12,
    );
    c = c.child(toggle(
        &on,
        "RVP:toggle-equalizer",
        "Equalizer on/off",
        (14, 18, 25, 12),
    ));
    // AUTO has no counterpart in RVP: shown off.
    c = c.child(
        El::new("Static")
            .rect(39, 18, 33, 12)
            .image(&back, 35, 119, 33, 12),
    );
    let presets = files.states("eq.presets.png", &eq, (224, 164), (224, 176), 44, 12);
    c = c.child(button(
        &presets,
        "CMD_POPUPMENU_AUDIO_EQULIZER",
        "Presets",
        217,
        18,
        44,
        12,
    ));
    // The graph: background, curve, preamp line.
    c = c.child(
        El::new("Static")
            .rect(86, 17, 113, 19)
            .image(&back, 0, 294, 113, 19),
    );
    let mut graph = Image::new(114, 20);
    for row in 0..19 {
        let pixel = eq.pixel(115, 294 + row);
        let at = (usize::try_from(row).unwrap_or(0) * 114 + 113) * 4;
        graph.data_mut()[at..at + 4].copy_from_slice(&pixel);
    }
    let line = crop(&eq, 0, 314, 113, 1);
    for x in 0..113 {
        let at = (19 * 114 + usize::try_from(x).unwrap_or(0)) * 4;
        graph.data_mut()[at..at + 4].copy_from_slice(&line.pixel(x, 0));
    }
    let graph = files.add("eq.graph.png", graph);
    c = c.child(
        El::new("RvpGraph")
            .rect(86, 17, 113, 19)
            .a("Image", graph.as_str())
            .a("ColorsX", 113)
            .a("ColorsY", 0)
            .a("PreampImageLeft", 0)
            .a("PreampImageTop", 19),
    );
    // The sliders: 28 pictures from -12 dB (frame 0) up, the thumb below.
    let mut frames: Vec<Image> = (0..28)
        .map(|n| {
            let (x, y) = if n < 14 {
                (13 + 15 * n, 164)
            } else {
                (13 + 15 * (n - 14), 229)
            };
            crop(&eq, x, y, 14, 63)
        })
        .collect();
    frames.push(join(
        &[crop(&eq, 0, 164, 11, 11), crop(&eq, 0, 176, 11, 11)],
        false,
    ));
    let sliders = files.add("eq.sliders.png", join(&frames, true));
    let vertical = |command: &str, tooltip: &str, x: i32| {
        slider(&sliders, command, tooltip, (x, 38, 14, 63), 28, (11, 11)).a("Vert", 1)
    };
    c = c.child(vertical("RVP:preamp", "Preamp", 21));
    for (band, label) in [
        "60 Hz", "170 Hz", "310 Hz", "600 Hz", "1 kHz", "3 kHz", "6 kHz", "12 kHz", "14 kHz",
        "16 kHz",
    ]
    .iter()
    .enumerate()
    {
        let i = i32::try_from(band).unwrap_or(0);
        c = c.child(vertical(
            &format!("CMD_EQULIZER_{band}"),
            label,
            78 + 18 * i,
        ));
    }
    let mut area = El::new(EQUALIZER_AREA)
        .a("Image", back.as_str())
        .a("Width", 275)
        .a("Height", 116)
        .a("RvpWindow", 1)
        .a("RvpDock", "below")
        .a("RvpToggle", "RVPWIN:equalizer");
    if let Some(region) = classic.regions.get("Equalizer") {
        area = area.a("RvpRegion", region_text(region));
    }
    area.child(El::new("Control").child(c))
}

/// Winamp's playlist window (from 275x116, resizable), docked below.
#[allow(clippy::too_many_lines)] // Every piece of the frame and its buttons.
fn playlist_window(classic: &WinampSkin, files: &mut Files) -> El {
    let pledit = classic.bitmap("pledit").clone();
    let frame = files.add("pl.png", pledit.clone());
    let piece = |x: i32, w: i32, left: i32| {
        El::new("Static")
            .rect(left, 0, w, 20)
            .image(&frame, x, 0, w, 20)
            .a("AltImageLeft", x)
            .a("AltImageTop", 21)
    };
    let close = files.states("pl.close.png", &pledit, (167, 3), (52, 42), 9, 9);
    let title = El::new("Static")
        .a("Align", "Top")
        .a("Height", 20)
        .a("IsTitleBar", 1)
        .image(&frame, 127, 0, 25, 20)
        .a("AltImageLeft", 127)
        .a("AltImageTop", 21)
        .child(piece(0, 25, 0))
        .child(piece(26, 100, 0).a("HoriCenter", 1))
        .child(piece(153, 25, -25))
        .child(button(&close, "RVPWIN:playlist", "Close", -11, 3, 9, 9));
    let mut bottom = El::new("Static")
        .a("Align", "Bottom")
        .a("Height", 38)
        .image(&frame, 179, 0, 25, 38)
        .child(
            El::new("Static")
                .rect(0, 0, 125, 38)
                .image(&frame, 0, 72, 125, 38),
        )
        .child(
            El::new("Static")
                .rect(-150, 0, 150, 38)
                .image(&frame, 126, 72, 150, 38),
        );
    // The menus' buttons: add, remove, select, misc (RVP's playlist
    // actions), and the list menu at the right.
    for (x, command, tooltip) in [
        (14, "CMD_PLAYLIST_ADD", "Add files"),
        (43, "CMD_PLAYLIST_DEL", "Remove the selected files"),
        (72, "RVP:playlist-settings", "Playlist settings"),
        (101, "CMD_PLAYLIST_MENU_SORT", "Sort"),
    ] {
        bottom = bottom.child(
            El::new("Button")
                .a("Tooltip", tooltip)
                .a("CmdID", command)
                .rect(x, 8, 22, 18),
        );
    }
    bottom = bottom.child(
        El::new("Button")
            .a("Tooltip", "Playlist menu")
            .a("CmdID", "RVP:import-playlists")
            .rect(-44, 8, 22, 18),
    );
    // The mini transport over the right corner, and the small time.
    for (index, (command, tooltip)) in [
        ("RVP:previous", "Previous"),
        ("RVP:play", "Play"),
        ("RVP:play-pause", "Pause"),
        ("RVP:close-media", "Stop"),
        ("RVP:next", "Next"),
        ("RVP:open", "Open file"),
    ]
    .iter()
    .enumerate()
    {
        let offsets = [0, 7, 17, 27, 36, 45, 53];
        let x = -144 + offsets[index];
        let w = offsets[index + 1] - offsets[index];
        bottom = bottom.child(
            El::new("Button")
                .a("Tooltip", tooltip)
                .a("CmdID", command)
                .rect(x, 23, w, 8),
        );
    }
    bottom = bottom.child(text_label("RVP:time-short", -86, 23, 6, 0));
    bottom = bottom.child(text_label("RVP:playlist-info", -143, 10, 18, 0));
    let area = El::new(PLAYLIST_AREA)
        .a("Image", frame.as_str())
        .a("Width", 275)
        .a("Height", 232)
        .a("WidthLimit", 275)
        .a("HeightLimit", 116)
        .a("RvpWindow", 1)
        .a("RvpDock", "below")
        .a("RvpResizeStep", "25,29")
        .a("RvpToggle", "RVPWIN:playlist")
        // The list's colours from colors.png, no album tabs.
        .a("AlbumTabH", 0)
        .a("TextColorX", 0)
        .a("TextColorY", 0)
        .a("CurTextColorX", 1)
        .a("CurTextColorY", 0)
        .a("SelTextColorX", 1)
        .a("SelTextColorY", 0)
        .a("PlayedTextColorX", 0)
        .a("PlayedTextColorY", 0)
        .a("BkgndColor1X", 2)
        .a("BkgndColor1Y", 0)
        .a("SelBkgndColorX", 3)
        .a("SelBkgndColorY", 0)
        .a("FrameColorX", 1)
        .a("FrameColorY", 0);
    area.child(
        El::new("Control")
            .child(title)
            .child(bottom)
            .child(
                El::new("Static")
                    .a("Align", "Left")
                    .a("Width", 12)
                    .image(&frame, 0, 42, 12, 29),
            )
            .child(
                El::new("Static")
                    .a("Align", "Right")
                    .a("Width", 20)
                    .image(&frame, 31, 42, 20, 29),
            )
            .child(
                El::new("Static")
                    .a("Align", "Client")
                    .a("Contain", "CMD_PLAY_LIST_ALBUM_TAB")
                    .a("Image", "colors.png"),
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_synthetic_skin_builds() {
        let classic = WinampSkin::from_members("synthetic", fallback::synthetic_skin()).unwrap();
        let built = skin(&classic).expect("builds");
        assert!(built.warnings().is_empty(), "{:?}", built.warnings());
        for area in [
            "MainSkinArea",
            "MainSkinAreaOscInside",
            MAIN_AREA,
            EQUALIZER_AREA,
            PLAYLIST_AREA,
        ] {
            assert!(built.area(area).is_some(), "{area}");
        }
        assert_eq!(built.image("digits.png").map(Image::width), Some(12 * 22));
    }
}
