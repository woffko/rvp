//! VLC's skins (M9): with `RVP_VLC_SKINS` naming a folder of `.vlt`
//! files (local only, never committed), every theme there builds into an
//! RVP skin whose files all load.

use std::path::Path;

use rvp_skin::vlc::{Theme, build};
use rvp_skin::{Skin, SkinArchive};

#[test]
fn the_local_themes_build() {
    let Some(folder) = std::env::var_os("RVP_VLC_SKINS") else {
        eprintln!("note: RVP_VLC_SKINS is not set; the local themes were skipped");
        return;
    };
    let mut paths = Vec::new();
    collect(Path::new(&folder), &mut paths);
    assert!(
        !paths.is_empty(),
        "no .vlt files in {}",
        Path::new(&folder).display()
    );
    for path in paths {
        let theme =
            Theme::open(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let members =
            build::members(&theme).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let files: Vec<String> = members
            .iter()
            .map(|(name, _)| name.clone())
            .filter(|name| {
                std::path::Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            })
            .collect();
        let archive = SkinArchive::from_files(&theme.name, members);
        for file in &files {
            let skin = Skin::load(archive.clone(), file)
                .unwrap_or_else(|error| panic!("{}: {file}: {error}", path.display()));
            assert!(
                skin.area("MainSkinArea").is_some(),
                "{}: {file}",
                path.display()
            );
        }
        eprintln!(
            "{}: windows {}, files {}, warnings {:?}",
            theme.name,
            theme.windows.len(),
            files.len(),
            theme.warnings
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
            .is_some_and(|ext| ext.eq_ignore_ascii_case("vlt"))
        {
            out.push(path);
        }
    }
    out.sort();
}
