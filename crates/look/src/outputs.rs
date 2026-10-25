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
    files.push(("icons/rvp.res".to_owned(), brand::res(&ico)?));
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
    Ok(files)
}

fn png(image: &Image) -> Result<Vec<u8>, String> {
    image.encode_png().map_err(|error| error.to_string())
}
