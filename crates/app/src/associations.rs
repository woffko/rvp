//! The video and audio types RVP registers for (M8): the list in
//! `packaging/associations.tsv`, which the packages read for the Windows
//! installer's registry entries and the Linux `.desktop` file, and MPRIS for
//! its `SupportedMimeTypes`. RVP only registers itself as a handler of these
//! types and never makes itself the default (owner decision 2026-10-16).

const LIST: &str = include_str!("../../../packaging/associations.tsv");

/// What a type holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Video,
    Audio,
}

/// One registered type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Association {
    /// Lower case, without the dot.
    pub extension: &'static str,
    pub kind: Kind,
    /// shared-mime-info's names, newest first; empty when it has none.
    pub mime_types: Vec<&'static str>,
}

/// Every registered type, in the list's order. Lines that do not read as
/// `extension<TAB>kind<TAB>mime types` are left out (the tests keep the
/// list free of them).
#[must_use]
pub fn associations() -> Vec<Association> {
    LIST.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut cells = line.split('\t');
            let extension = cells.next()?;
            let kind = match cells.next()? {
                "video" => Kind::Video,
                "audio" => Kind::Audio,
                _ => return None,
            };
            let mime = cells.next()?;
            if cells.next().is_some() {
                return None;
            }
            let mime_types = if mime == "-" {
                Vec::new()
            } else {
                mime.split(';').collect()
            };
            Some(Association {
                extension,
                kind,
                mime_types,
            })
        })
        .collect()
}

/// The MIME types of all registered types, each once, in the list's order.
#[must_use]
pub fn mime_types() -> Vec<&'static str> {
    let mut types: Vec<&'static str> = Vec::new();
    for association in associations() {
        for mime in association.mime_types {
            if !types.contains(&mime) {
                types.push(mime);
            }
        }
    }
    types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_reads() {
        let lines = LIST
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .count();
        assert_eq!(associations().len(), lines);
        assert!(lines > 70, "{lines} types");
    }

    #[test]
    fn extensions_are_unique_lower_case_names() {
        let list = associations();
        for (index, association) in list.iter().enumerate() {
            let extension = association.extension;
            assert!(
                !extension.is_empty()
                    && extension
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{extension}"
            );
            assert!(
                list[..index]
                    .iter()
                    .all(|other| other.extension != extension),
                "{extension} twice"
            );
        }
    }

    #[test]
    fn mime_types_are_type_slash_subtype() {
        for mime in mime_types() {
            let (kind, subtype) = mime.split_once('/').expect(mime);
            assert!(["video", "audio", "application"].contains(&kind), "{mime}");
            assert!(
                !subtype.is_empty()
                    && subtype
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)),
                "{mime}"
            );
        }
    }

    #[test]
    fn the_list_holds_media_and_nothing_else() {
        let list = associations();
        let find = |extension: &str| list.iter().find(|a| a.extension == extension);
        assert_eq!(find("mkv").map(|a| a.kind), Some(Kind::Video));
        assert_eq!(find("mp3").map(|a| a.kind), Some(Kind::Audio));
        assert_eq!(
            find("avi").map(|a| a.mime_types.clone()),
            Some(vec!["video/vnd.avi", "video/x-msvideo"])
        );
        // Subtitles, playlists, PotPlayer's skins and bookmarks, discs.
        for other in [
            "srt", "ass", "m3u", "pls", "dsf", "pbf", "ifo", "cda", "swf",
        ] {
            assert!(find(other).is_none(), "{other}");
        }
    }
}
