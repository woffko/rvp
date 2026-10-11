//! Import of real PotPlayer playlists, read-only (goal F). Set
//! `RVP_POTPLAYER_PLAYLISTS` to a `Playlist` folder (on WSL, for example
//! `/mnt/c/Users/<user>/AppData/Roaming/PotPlayerMini64/Playlist`); the
//! test passes with a note when it is unset or missing. It prints counts
//! only, never paths or titles: the owner's playlists stay private.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rvp_app::Settings;
use rvp_app::dpl::{DriveMap, import_dir};

#[test]
fn owner_playlists_import() {
    let Some(folder) = std::env::var_os("RVP_POTPLAYER_PLAYLISTS").map(PathBuf::from) else {
        eprintln!("note: RVP_POTPLAYER_PLAYLISTS is not set; skipped");
        return;
    };
    if !folder.is_dir() {
        eprintln!("note: {} is missing; skipped", folder.display());
        return;
    }
    let map = DriveMap::new(&Settings::default().drive_map);
    let albums = import_dir(&folder, &map).expect("the folder can be listed");
    assert!(!albums.is_empty(), "no .dpl files");
    assert_eq!(albums[0].album.name, "Default", "Default comes first");

    // What PotPlayer wrote: the entry lines of each file, by album name.
    let mut written: BTreeMap<String, usize> = BTreeMap::new();
    for entry in std::fs::read_dir(&folder)
        .expect("list")
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dpl"))
        {
            continue;
        }
        let text = String::from_utf8_lossy(&std::fs::read(&path).expect("read")).into_owned();
        let files = text
            .lines()
            .filter(|line| {
                let mut parts = line.splitn(3, '*');
                parts.next().is_some_and(|n| n.parse::<u32>().is_ok())
                    && parts.next() == Some("file")
            })
            .count();
        let stem = path
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .into_owned();
        let name = if stem.eq_ignore_ascii_case("PotPlayerMini64") {
            "Default".to_owned()
        } else {
            stem
        };
        written.insert(name, files);
    }

    let mut total = 0;
    for (number, imported) in albums.iter().enumerate() {
        let album = &imported.album;
        assert_eq!(
            Some(&album.entries.len()),
            written.get(&album.name),
            "album {number}: every entry imports"
        );
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in &album.entries {
            let kind = if entry.path.starts_with("/mnt/media/") {
                "mapped V:"
            } else if entry.path.contains("://") {
                "URL"
            } else if entry.path.get(1..3) == Some(":\\") {
                "other drive"
            } else {
                "other"
            };
            *kinds.entry(kind).or_default() += 1;
            // No Windows separators survive a mapping.
            if entry.path.starts_with("/mnt/media/") {
                assert!(!entry.path.contains('\\'), "album {number}: mapped path");
            }
        }
        let played = album.entries.iter().filter(|e| e.played).count();
        let unplayable = album.entries.iter().filter(|e| !e.playable).count();
        eprintln!(
            "album {number}: {} entries {kinds:?}, {played} played, {unplayable} unplayable, \
             current {}, {} positions",
            album.entries.len(),
            album.current.is_some(),
            imported.positions.len()
        );
        total += album.entries.len();
    }
    eprintln!("{} albums, {total} entries", albums.len());
}
