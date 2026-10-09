use std::fmt;

/// Errors raised while loading a skin. Layout and rendering never fail: they
/// work on an already loaded skin and degrade instead.
#[derive(Debug)]
pub enum SkinError {
    /// Reading a skin file or directory failed.
    Io {
        path: String,
        source: std::io::Error,
    },
    /// The `.dsf` archive could not be read.
    Archive(String),
    /// A file the skin refers to is not in the archive.
    MissingFile(String),
    /// An image could not be decoded.
    Image { name: String, reason: String },
    /// The requested skin area (for example `MainSkinArea`) does not exist.
    MissingArea(String),
    /// A font file could not be parsed.
    Font { path: String, reason: String },
}

impl fmt::Display for SkinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {path}: {source}"),
            Self::Archive(reason) => write!(f, "cannot read skin archive: {reason}"),
            Self::MissingFile(name) => write!(f, "skin file {name} is missing"),
            Self::Image { name, reason } => write!(f, "cannot decode image {name}: {reason}"),
            Self::MissingArea(name) => write!(f, "skin has no {name} section"),
            Self::Font { path, reason } => write!(f, "cannot use font {path}: {reason}"),
        }
    }
}

impl std::error::Error for SkinError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
