//! Access to the files of a skin: a `.dsf` ZIP archive or an unpacked
//! directory. PotPlayer looks names up case-insensitively, so this does too.
//!
//! Every skin format's archive is read here ([`zip_members`],
//! [`tar_members`], [`folder_members`]), within [`crate::limits`]: the
//! file's size, the number of members, one member's size and all of them
//! together are checked from the archive's own records before a member is
//! unpacked, and a ZIP archive whose members share their data is refused.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::SkinError;
use crate::limits;

/// All files of one skin, held in memory. Skins are small (well under 10 MB).
#[derive(Debug, Clone)]
pub struct SkinArchive {
    name: String,
    files: BTreeMap<String, Vec<u8>>,
}

fn key(name: &str) -> String {
    name.trim().replace('\\', "/").to_ascii_lowercase()
}

/// The members of an archive, each a name and its bytes.
pub type Members = Vec<(String, Vec<u8>)>;

/// Reads a skin's file.
///
/// # Errors
/// Fails when it cannot be read or is larger than
/// [`limits::ARCHIVE_BYTES`].
pub fn read_file(path: &Path) -> Result<Vec<u8>, SkinError> {
    let io_error = |source| SkinError::Io {
        path: path.display().to_string(),
        source,
    };
    let file = std::fs::File::open(path).map_err(io_error)?;
    let mut bytes = Vec::new();
    // One byte past the limit shows a larger file, whatever its size
    // says.
    file.take(limits::ARCHIVE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    file_size(bytes.len())?;
    Ok(bytes)
}

/// Checks an archive file's size.
fn file_size(bytes: usize) -> Result<(), SkinError> {
    if u64::try_from(bytes).unwrap_or(u64::MAX) > limits::ARCHIVE_BYTES {
        return Err(limits::exceeded(
            "a skin file of more bytes",
            limits::ARCHIVE_BYTES,
        ));
    }
    Ok(())
}

/// What has been unpacked of an archive so far.
#[derive(Default)]
struct Unpacked {
    members: usize,
    bytes: u64,
}

impl Unpacked {
    /// Counts one more member of `size` bytes, before it is read.
    fn member(&mut self, name: &str, size: u64) -> Result<(), SkinError> {
        self.members += 1;
        if self.members > limits::ARCHIVE_ENTRIES {
            return Err(limits::exceeded(
                "more files in the skin",
                limits::ARCHIVE_ENTRIES,
            ));
        }
        if size > limits::ENTRY_BYTES {
            return Err(limits::exceeded(
                &format!("{name} of {size} bytes"),
                limits::ENTRY_BYTES,
            ));
        }
        self.bytes = self.bytes.saturating_add(size);
        if self.bytes > limits::UNPACKED_BYTES {
            return Err(limits::exceeded(
                &format!("more bytes unpacked, at {name}"),
                limits::UNPACKED_BYTES,
            ));
        }
        Ok(())
    }
}

/// The files of a ZIP archive in archive order (folders left out).
///
/// # Errors
/// Fails when the bytes are not a readable ZIP archive, when members
/// share their data, and when the archive is over a limit.
pub fn zip_members(bytes: &[u8]) -> Result<Members, SkinError> {
    let archive_error = |error: zip::result::ZipError| SkinError::Archive(error.to_string());
    file_size(bytes.len())?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(archive_error)?;
    if archive.len() > limits::ARCHIVE_ENTRIES {
        return Err(limits::exceeded(
            &format!("{} files in the skin", archive.len()),
            limits::ARCHIVE_ENTRIES,
        ));
    }
    // Members that share one block of data unpack to many times the
    // archive's size.
    if archive.has_overlapping_files().map_err(archive_error)? {
        return Err(SkinError::Archive(
            "members of the archive share their data".to_owned(),
        ));
    }
    let mut unpacked = Unpacked::default();
    let mut members = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(archive_error)?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().map_err(archive_error)?.into_owned();
        // The size the archive records, which the reader does not let a
        // member exceed: nothing is unpacked, or reserved, before it is
        // within the limits.
        unpacked.member(&name, file.size())?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|error| SkinError::Archive(format!("{name}: {error}")))?;
        members.push((name, content));
    }
    Ok(members)
}

/// The regular files of a tar archive, gzip'd or not, in archive order
/// (ustar and GNU long names).
///
/// # Errors
/// Fails when the bytes are no tar archive or the archive is over a
/// limit.
pub fn tar_members(bytes: &[u8]) -> Result<Members, SkinError> {
    file_size(bytes.len())?;
    let inflated;
    let tar = if bytes.starts_with(&[0x1F, 0x8B]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(limits::UNPACKED_BYTES.saturating_add(1))
            .read_to_end(&mut out)
            .map_err(|error| SkinError::Archive(error.to_string()))?;
        if u64::try_from(out.len()).unwrap_or(u64::MAX) > limits::UNPACKED_BYTES {
            return Err(limits::exceeded(
                "more bytes unpacked",
                limits::UNPACKED_BYTES,
            ));
        }
        inflated = out;
        inflated.as_slice()
    } else {
        bytes
    };
    read_tar(tar)
}

fn read_tar(bytes: &[u8]) -> Result<Members, SkinError> {
    let broken = || SkinError::Archive("not a tar archive".to_owned());
    let mut unpacked = Unpacked::default();
    let mut out = Vec::new();
    let mut at = 0;
    let mut long_name: Option<String> = None;
    while at + 512 <= bytes.len() {
        let header = &bytes[at..at + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        let field = |from: usize, to: usize| {
            let raw = &header[from..to];
            let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
            String::from_utf8_lossy(&raw[..end]).into_owned()
        };
        let size = usize::from_str_radix(field(124, 136).trim(), 8).map_err(|_| broken())?;
        let kind = header[156];
        let mut name = field(0, 100);
        if &header[257..262] == b"ustar" {
            let prefix = field(345, 500);
            if !prefix.is_empty() {
                name = format!("{prefix}/{name}");
            }
        }
        let start = at + 512;
        let end = start.checked_add(size).ok_or_else(broken)?;
        let data = bytes.get(start..end).ok_or_else(broken)?;
        match kind {
            b'L' => {
                let raw = data.split(|byte| *byte == 0).next().unwrap_or(data);
                long_name = Some(String::from_utf8_lossy(raw).into_owned());
            }
            b'0' | 0 => {
                let name = long_name.take().unwrap_or(name);
                unpacked.member(&name, u64::try_from(size).unwrap_or(u64::MAX))?;
                out.push((name, data.to_vec()));
            }
            _ => long_name = None,
        }
        at = start + size.div_ceil(512) * 512;
    }
    Ok(out)
}

/// The files of a folder that holds an unpacked skin, sorted by name
/// (its own files only, not those of folders inside).
///
/// # Errors
/// Fails when the folder cannot be read or is over a limit.
pub fn folder_members(path: &Path) -> Result<Members, SkinError> {
    let io_error = |source| SkinError::Io {
        path: path.display().to_string(),
        source,
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if entry.file_type().map_err(io_error)?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    let mut unpacked = Unpacked::default();
    let mut members = Vec::with_capacity(files.len().min(limits::ARCHIVE_ENTRIES));
    for file in files {
        let name = file
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let size = std::fs::metadata(&file).map_err(io_error)?.len();
        unpacked.member(&name, size)?;
        members.push((name, read_file(&file)?));
    }
    Ok(members)
}

impl SkinArchive {
    /// Opens a `.dsf` file or a directory holding the unpacked skin.
    ///
    /// # Errors
    /// Fails when the path cannot be read, is not a valid ZIP archive, or
    /// is over one of [`crate::limits`].
    pub fn open(path: &Path) -> Result<Self, SkinError> {
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let members = if path.is_dir() {
            folder_members(path)?
        } else {
            zip_members(&read_file(path)?)?
        };
        Ok(Self::from_files(&name, members))
    }

    /// Reads a skin from the bytes of a `.dsf` archive.
    ///
    /// # Errors
    /// Fails when the bytes are not a readable ZIP archive, or the archive
    /// is over one of [`crate::limits`].
    // By value, as before: the callers hand their buffer over.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_zip_bytes(name: &str, bytes: Vec<u8>) -> Result<Self, SkinError> {
        Ok(Self::from_files(name, zip_members(&bytes)?))
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

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    /// One member of [`forged_zip`]: its name, the bytes kept in the
    /// archive and the size the archive says they unpack to.
    struct Forged<'a> {
        name: &'a str,
        stored: &'a [u8],
        unpacked: u32,
    }

    /// A ZIP archive written by hand, whose records may say what its
    /// data does not hold: every member is marked as deflated. `shared`
    /// gives each member the first one's data.
    fn forged_zip(members: &[Forged<'_>], shared: bool) -> Vec<u8> {
        let u16_of = |value: usize| u16::try_from(value).unwrap().to_le_bytes();
        let u32_of = |value: usize| u32::try_from(value).unwrap().to_le_bytes();
        let mut out = Vec::new();
        let mut directory = Vec::new();
        let mut first = 0;
        for (index, member) in members.iter().enumerate() {
            let offset = if shared && index > 0 {
                first
            } else {
                let offset = out.len();
                out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
                out.extend_from_slice(&[0; 4]);
                out.extend_from_slice(&u32_of(member.stored.len()));
                out.extend_from_slice(&member.unpacked.to_le_bytes());
                out.extend_from_slice(&u16_of(member.name.len()));
                out.extend_from_slice(&[0, 0]);
                out.extend_from_slice(member.name.as_bytes());
                out.extend_from_slice(member.stored);
                offset
            };
            if index == 0 {
                first = offset;
            }
            directory.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 8, 0]);
            directory.extend_from_slice(&[0; 8]);
            directory.extend_from_slice(&u32_of(member.stored.len()));
            directory.extend_from_slice(&member.unpacked.to_le_bytes());
            directory.extend_from_slice(&u16_of(member.name.len()));
            directory.extend_from_slice(&[0; 12]);
            directory.extend_from_slice(&u32_of(offset));
            directory.extend_from_slice(member.name.as_bytes());
        }
        let start = out.len();
        out.extend_from_slice(&directory);
        out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend_from_slice(&u16_of(members.len()));
        out.extend_from_slice(&u16_of(members.len()));
        out.extend_from_slice(&u32_of(directory.len()));
        out.extend_from_slice(&u32_of(start));
        out.extend_from_slice(&[0, 0]);
        out
    }

    /// A ZIP archive of `count` members of `size` zero bytes each.
    fn honest_zip(count: usize, size: usize, method: CompressionMethod) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(method);
        let content = vec![0_u8; size];
        for index in 0..count {
            writer.start_file(format!("{index}.txt"), options).unwrap();
            writer.write_all(&content).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn limit(result: Result<Members, SkinError>) -> String {
        match result {
            Err(SkinError::Limit(what)) => what,
            Err(other) => panic!("not a limit: {other}"),
            Ok(members) => panic!("{} members were read", members.len()),
        }
    }

    #[test]
    fn a_zip_archive_reads_in_order_without_its_folders() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("b.txt", options).unwrap();
        writer.write_all(b"second name, first member").unwrap();
        writer.add_directory("folder", options).unwrap();
        writer.start_file("folder/a.txt", options).unwrap();
        writer.write_all(b"a").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let members = zip_members(&bytes).unwrap();
        let names: Vec<&str> = members.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["b.txt", "folder/a.txt"]);
        assert_eq!(members[1].1, b"a");
        let archive = SkinArchive::from_zip_bytes("test", bytes).unwrap();
        assert_eq!(archive.get("Folder\\A.TXT"), Some(&b"a"[..]));
    }

    #[test]
    fn a_member_is_refused_by_the_size_the_archive_records() {
        // A few bytes that claim to unpack to more than a member may be:
        // refused from the record, without unpacking.
        let over = u32::try_from(limits::ENTRY_BYTES + 1).unwrap();
        let bytes = forged_zip(
            &[Forged {
                name: "big.png",
                stored: &[3, 0],
                unpacked: over,
            }],
            false,
        );
        assert!(bytes.len() < 200);
        assert!(limit(zip_members(&bytes)).contains("big.png"));
        // The same record within the limit is read: its data (an empty
        // deflate stream) holds less than it says, never more.
        let bytes = forged_zip(
            &[Forged {
                name: "big.png",
                stored: &[3, 0],
                unpacked: over - 1,
            }],
            false,
        );
        assert_eq!(
            zip_members(&bytes).unwrap(),
            [("big.png".to_owned(), Vec::new())]
        );
    }

    #[test]
    fn the_members_together_are_bounded() {
        // Each member is within its own limit; together they are not.
        let size = usize::try_from(limits::ENTRY_BYTES).unwrap() - 1;
        let count = usize::try_from(limits::UNPACKED_BYTES / limits::ENTRY_BYTES).unwrap() + 1;
        let bytes = honest_zip(count, size, CompressionMethod::Deflated);
        // The archive itself is small.
        assert!(bytes.len() < 1 << 20, "{}", bytes.len());
        assert!(limit(zip_members(&bytes)).contains("unpacked"));
    }

    #[test]
    fn the_number_of_members_is_bounded() {
        let bytes = honest_zip(limits::ARCHIVE_ENTRIES + 1, 0, CompressionMethod::Stored);
        assert!(limit(zip_members(&bytes)).contains("files"));
        let bytes = honest_zip(16, 0, CompressionMethod::Stored);
        assert_eq!(zip_members(&bytes).unwrap().len(), 16);
    }

    #[test]
    fn members_that_share_their_data_are_refused() {
        // Many names for one block of data: each would unpack it again.
        let stored = [
            0x63, 0x60, 0x18, 0x05, 0xA3, 0x60, 0x14, 0x0C, 0x77, 0x00, 0x00,
        ];
        let members: Vec<Forged<'_>> = ["a", "b", "c", "d"]
            .iter()
            .map(|name| Forged {
                name,
                stored: &stored,
                unpacked: 1 << 20,
            })
            .collect();
        match zip_members(&forged_zip(&members, true)) {
            Err(SkinError::Archive(reason)) => assert!(reason.contains("share"), "{reason}"),
            other => panic!("{:?}", other.map(|members| members.len())),
        }
    }

    #[test]
    fn a_skin_file_is_bounded() {
        let size = usize::try_from(limits::ARCHIVE_BYTES).unwrap() + 1;
        assert!(limit(zip_members(&vec![0; size])).contains("skin file"));
        assert!(limit(tar_members(&vec![0; size])).contains("skin file"));
        // On disk: a file of that size with nothing written to it.
        let path = std::env::temp_dir().join(format!("rvp-skin-limit-{}.dsf", std::process::id()));
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(limits::ARCHIVE_BYTES + 1).unwrap();
        drop(file);
        let read = read_file(&path);
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(read, Err(SkinError::Limit(_))));
        assert!(matches!(
            SkinArchive::open(Path::new("/nowhere/skin.dsf")),
            Err(SkinError::Io { .. })
        ));
    }

    /// A tar header for a regular file of `size` bytes.
    fn tar_header(name: &str, size: usize) -> Vec<u8> {
        let mut header = vec![0_u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
        header[156] = b'0';
        header
    }

    #[test]
    fn a_tar_archive_is_bounded_as_a_zip_is() {
        // Too many members.
        let mut tar = Vec::new();
        for index in 0..=limits::ARCHIVE_ENTRIES {
            tar.extend(tar_header(&format!("{index}"), 0));
        }
        assert!(limit(tar_members(&tar)).contains("files"));
        // One member too large.
        let size = usize::try_from(limits::ENTRY_BYTES).unwrap() + 1;
        let mut tar = tar_header("big.png", size);
        tar.resize(512 + size.div_ceil(512) * 512, 0);
        assert!(limit(tar_members(&tar)).contains("big.png"));
        // A gzip'd tar that unpacks to more than all members may be: the
        // unpacking stops at the limit.
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let block = vec![0_u8; 1 << 20];
        for _ in 0..=(limits::UNPACKED_BYTES >> 20) {
            encoder.write_all(&block).unwrap();
        }
        let packed = encoder.finish().unwrap();
        assert!(packed.len() < 1 << 20, "{}", packed.len());
        assert!(limit(tar_members(&packed)).contains("unpacked"));
        // A tar whose size field is no number is no tar.
        let mut broken = tar_header("x", 0);
        broken[124..136].copy_from_slice(b"not a number");
        assert!(matches!(tar_members(&broken), Err(SkinError::Archive(_))));
    }
}
