//! RVP's own look: the logo, the icons and the default skin "RVP Default".
//!
//! The art is vector sources in `art/` (SVG and the skin's markup); this
//! crate draws them ([`svg`]) and builds the files RVP ships. Nothing here
//! comes from PotPlayer's skins: see `art/SOURCES.md`.

pub mod brand;
pub mod font;
pub mod outputs;
pub mod skin;
pub mod sprites;
pub mod svg;
pub mod xml;
