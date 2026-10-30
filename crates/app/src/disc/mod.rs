//! DVD and Blu-ray menus (M16, `docs/spikes/s7-disc-menus.md`): what RVP
//! does with a disc's navigation data that needs no library. The shell
//! drives libdvdnav and libbluray (`shell/src/disc`) and hands their data
//! to these parts.

pub mod bluray;
pub mod overlay;
pub mod play;
pub mod spu;
