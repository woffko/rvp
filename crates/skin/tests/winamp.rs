//! Classic Winamp skins (M9): RVP's synthetic skin, and, with
//! `RVP_WINAMP_SKINS` naming a folder of `.wsz` files (local only, never
//! committed), every skin there loads without a decoding error.

use std::path::Path;

use rvp_skin::winamp::{WinampSkin, fallback};

#[test]
fn the_synthetic_skin_has_every_bitmap() {
    let skin = WinampSkin::from_members("synthetic", fallback::synthetic_skin()).unwrap();
    assert!(skin.warnings.is_empty(), "{:?}", skin.warnings);
    assert_eq!(skin.missing, vec!["gen"]);
    assert!(skin.has_video);
    assert!(skin.extended_numbers());
}

#[test]
fn the_local_skins_load() {
    // An empty value is unset too: the Linux kit passes the variable on
    // as it found it.
    let Some(folder) = std::env::var_os("RVP_WINAMP_SKINS").filter(|value| !value.is_empty())
    else {
        eprintln!("note: RVP_WINAMP_SKINS is not set; the local skins were skipped");
        return;
    };
    let mut paths = Vec::new();
    collect(Path::new(&folder), &mut paths);
    assert!(
        !paths.is_empty(),
        "no .wsz files in {}",
        Path::new(&folder).display()
    );
    for path in paths {
        let skin =
            WinampSkin::open(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(
            skin.warnings.is_empty(),
            "{}: {:?}",
            path.display(),
            skin.warnings
        );
        eprintln!(
            "{}: missing {:?}, regions {:?}",
            skin.name,
            skin.missing,
            skin.regions.keys().collect::<Vec<_>>()
        );
    }
}

fn collect(folder: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wsz") || ext.eq_ignore_ascii_case("zip"))
        {
            out.push(path);
        }
    }
    out.sort();
}
