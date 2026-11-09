//! Every file built from the art: the shell's build script puts them in
//! its `OUT_DIR` (resources compiled into `rvp`, the Windows icon resource
//! linked into `rvp.exe`, the default skin included in it), and
//! `cargo xtask look DIR` writes them for packaging and review (with the
//! Unified Remote remote's icons, which the shell does not use). None is
//! committed: the sources are.

use rvp_skin::Image;

use crate::brand;

/// PNG sizes of the application icon (window icons and Linux packages).
pub const ICON_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// Height of the idle screen's symbol image; the shell scales it down.
pub const SYMBOL_HEIGHT: u32 = 512;

/// Height of the About box's wordmark images.
pub const WORDMARK_HEIGHT: u32 = 96;

/// The Unified Remote remote's icons (M15): Unified Remote's remotes carry
/// a 64-pixel `icon.png` and a 512-pixel `icon_hires.png`.
pub const REMOTE_ICONS: [(&str, u32); 2] = [("icon.png", 64), ("icon_hires.png", 512)];

/// Where the remote's files go (`remote/unified-remote/RVP` holds the
/// rest).
pub const REMOTE_DIR: &str = "remote/unified-remote/RVP";

/// The remote's button pictures by RVP Default's icons (`art/skin/icons.svg`),
/// where Unified Remote's own icons say too little: its `vmute` is a
/// speaker without a cross (the owner, 2026-11-08).
pub const REMOTE_BUTTONS: [(&str, &str); 1] = [("mute.png", "icon-mute")];

/// The size of a remote's button pictures, as Unified Remote's own
/// remotes draw them.
pub const REMOTE_BUTTON_SIZE: u32 = 48;

/// The built files as `(relative path, bytes)`, in a fixed order.
///
/// # Errors
/// Fails when a source cannot be drawn or encoded.
pub fn outputs() -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut files = Vec::new();
    for size in ICON_SIZES {
        files.push((format!("icons/rvp-{size}.png"), png(&brand::icon(size)?)?));
    }
    let ico = brand::ico()?;
    files.push((
        "icons/rvp.res".to_owned(),
        brand::res(&ico, env!("CARGO_PKG_VERSION"))?,
    ));
    files.push(("icons/rvp.ico".to_owned(), ico));
    files.push((
        "icons/rvp.svg".to_owned(),
        brand::ICON_SVG.as_bytes().to_vec(),
    ));
    files.push((
        "logo/symbol.png".to_owned(),
        png(&brand::symbol(SYMBOL_HEIGHT)?)?,
    ));
    for (name, color) in [
        ("light", brand::WORDMARK_LIGHT),
        ("dark", brand::WORDMARK_DARK),
    ] {
        let image = brand::wordmark(WORDMARK_HEIGHT, color)?;
        files.push((format!("logo/wordmark-{name}.png"), png(&image)?));
    }
    files.push((
        format!("skins/{}.dsf", crate::skin::NAME),
        crate::skin::dsf()?,
    ));
    for (name, size) in REMOTE_ICONS {
        files.push((format!("{REMOTE_DIR}/{name}"), png(&brand::icon(size)?)?));
    }
    for (name, symbol) in REMOTE_BUTTONS {
        files.push((
            format!("{REMOTE_DIR}/{name}"),
            png(&remote_button(symbol, REMOTE_BUTTON_SIZE)?)?,
        ));
    }
    Ok(files)
}

/// RVP Default's icon `symbol`, white on nothing, `size` pixels square, as
/// a button picture of the remote (Unified Remote's buttons are blue).
///
/// # Errors
/// Fails when the icon sheet has no such symbol or it cannot be drawn.
pub fn remote_button(symbol: &str, size: u32) -> Result<Image, String> {
    let sheet = crate::sprites::ICONS_SVG;
    let start = sheet
        .find(&format!("<symbol id=\"{symbol}\""))
        .ok_or_else(|| format!("icons.svg has no {symbol}"))?;
    let end = sheet[start..]
        .find("</symbol>")
        .map(|at| start + at + "</symbol>".len())
        .ok_or_else(|| format!("icons.svg's {symbol} has no end"))?;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" \
         width=\"{size}\" height=\"{size}\"><defs>{}</defs>\
         <g color=\"#FFFFFF\" stroke-width=\"2\"><use href=\"#{symbol}\" x=\"0\" y=\"0\" \
         width=\"24\" height=\"24\"/></g></svg>",
        &sheet[start..end]
    );
    brand::render(&svg, size, size)
}

fn png(image: &Image) -> Result<Vec<u8>, String> {
    image.encode_png().map_err(|error| error.to_string())
}
