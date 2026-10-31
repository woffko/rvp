//! Loads PotPlayer `.dsf` skins, lays them out and renders them headless.
//!
//! The pipeline is: [`Skin::open`] (archive, preprocessor, tolerant markup
//! parser, atlases) → [`layout::layout`] (element rectangles for a window
//! size and [`UiState`]) → [`paint::paint`] (a [`DisplayList`] of sprite
//! blits, fills and texts) → [`render::rasterize`] (an RGBA [`Image`]).
//! [`hit::hit_test`] maps a point to the control under it.
//!
//! The crate has no GUI or playback dependency: the shell paints the same
//! display list with its own painter.

pub mod archive;
pub mod compare;
pub mod demo;
pub mod error;
pub mod fonts;
pub mod geometry;
pub mod hit;
pub mod image;
pub mod lang;
pub mod layout;
pub mod markup;
pub mod menu;
pub mod paint;
pub mod playlist;
pub mod preprocess;
pub mod render;
pub mod skin;
pub mod state;
pub mod text;
pub mod theme;
pub mod winamp;

pub use archive::SkinArchive;
pub use error::SkinError;
pub use fonts::Fonts;
pub use geometry::Rect;
pub use image::Image;
pub use layout::{Layout, layout};
pub use paint::{DisplayList, paint};
pub use render::rasterize;
pub use skin::{ExtraWindow, LogoSlot, Skin};
pub use state::{DataModel, Mark, MarkKind, Playlist, PlaylistEntry, SliderValue, UiState};

/// Lays out, paints and rasterizes one skin area in a single call.
///
/// # Errors
/// Fails when the skin has no such area.
pub fn render_area(
    skin: &Skin,
    area: &str,
    width: i32,
    height: i32,
    state: &UiState,
    data: &DataModel,
    fonts: &mut Fonts,
) -> Result<Image, SkinError> {
    let placed = layout(skin, area, width, height, state)?;
    let list = paint(skin, &placed, state, data);
    Ok(rasterize(&list, skin, fonts))
}
