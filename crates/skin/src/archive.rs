//! Access to the files of a skin: a `.dsf` ZIP archive or an unpacked
//! directory. PotPlayer looks names up case-insensitively, so this does too.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::SkinError;

/// All files of one skin, held in memory. Skins are small (well under 10 MB).
#[derive(Debug, Clone)]
pub struct SkinArchive {
    name: String,
    files: BTreeMap<String, Vec<u8>>,
}

fn key(name: &str) -> String {
    name.trim().replace('\\', "/").to_ascii_lowercase()
}

impl SkinArchive {
    /// Opens a `.dsf` file or a directory holding the unpacked skin.
    ///
    /// # Errors
    /// Fails when the path cannot be read or is not a valid ZIP archive.
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let io_error = |source| SkinError::Io {
            path: path.display().to_string(),
            source,
        };
        if path.is_dir() {
            let mut files = BTreeMap::new();
            for entry in std::fs::read_dir(path).map_err(io_error)? {
                let entry = entry.map_err(io_error)?;
                if entry.file_type().map_err(io_error)?.is_file() {
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    let bytes = std::fs::read(entry.path()).map_err(io_error)?;
                    files.insert(key(&file_name), bytes);
                }
            }
            return Ok(Self { name, files });
        }
        let bytes = std::fs::read(path).map_err(io_error)?;
        Self::from_zip_bytes(&name, bytes)
    }

    /// Reads a skin from the bytes of a `.dsf` archive.
    ///
    /// # Errors
    /// Fails when the bytes are not a readable ZIP archive.
    pub fn from_zip_bytes(name: &str, bytes: Vec<u8>) -> Result<Self, SkinError> {
        let archive_error = |error: zip::result::ZipError| SkinError::Archive(error.to_string());
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(archive_error)?;
        let mut files = BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(archive_error)?;
            if file.is_dir() {
                continue;
            }
            let file_name = file.name().map_err(archive_error)?.into_owned();
            let mut content = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
            file.read_to_end(&mut content)
                .map_err(|error| SkinError::Archive(format!("{file_name}: {error}")))?;
            files.insert(key(&file_name), content);
        }
        Ok(Self {
            name: name.to_owned(),
            files,
        })
    }

    /// Builds a skin from in-memory files; used by tests and fixtures.
    #[must_use]
    pub fn from_files<I, N>(name: &str, files: I) -> Self
    where
        I: IntoIterator<Item = (N, Vec<u8>)>,
        N: AsRef<str>,
    {
        Self {
            name: name.to_owned(),
            files: files
                .into_iter()
                .map(|(file_name, bytes)| (key(file_name.as_ref()), bytes))
                .collect(),
        }
    }

    /// The skin name: the archive file name without its extension.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a file by name, ignoring case and path separator style.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(&key(name)).map(Vec::as_slice)
    }

    /// Lists the (lower-cased) file names in the skin.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }
}
