//! M14's check of other `.dsf` skins with the user's files: for each skin
//! given, the video skin loads, `MainSkinArea` and
//! `MainSkinAreaOscInside` (the bars over the video) lay out and draw at
//! 1280x720, the audio skin (`AudioSkin.xml`) does the same when the skin
//! has one, and every command the skin names is sorted as
//! `docs/coverage/` sorts BlackSpace's. Third-party skins stay local: the
//! reports and renders go to OUT, and one summary line per skin to stdout.
//!
//! `cargo run --release -p rvp-app --example skin_coverage -- OUT SKIN.dsf...`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rvp_app::coverage::{skin_covered, skin_reason, skin_report};
use rvp_skin::{DataModel, Fonts, Skin, SkinArchive, UiState, render_area};

const INTRO: &str = "Made by `cargo run -p rvp-app --example skin_coverage` from \
     the skin's live elements: every `CmdID`, `AltCmdID` and menu item, where it \
     appears, and what RVP does with it. Kept local: the skin is third-party.";

/// Lays out and draws `area` at 1280x720 into OUT; "ok", "none" when the
/// skin has no such area, or the error.
fn draw(skin: &Skin, area: &str, out: &Path, stem: &str, fonts: &mut Fonts) -> String {
    match render_area(
        skin,
        area,
        1280,
        720,
        &UiState::default(),
        &DataModel::default(),
        fonts,
    ) {
        Ok(image) => {
            let file = out.join(format!("{stem}-{area}.png"));
            let written = image
                .encode_png()
                .is_ok_and(|bytes| std::fs::write(&file, bytes).is_ok());
            if written { "ok" } else { "drawn, not written" }.to_owned()
        }
        Err(error) if error.to_string().contains(area) => "none".to_owned(),
        Err(error) => format!("error: {error}"),
    }
}

fn check(path: &Path, out: &Path, fonts: &mut Fonts) -> Result<String, String> {
    let stem = path.file_stem().map_or_else(
        || "skin".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let skin = Skin::open(path).map_err(|error| error.to_string())?;
    let commands = skin.commands();
    std::fs::write(
        out.join(format!("{stem}.md")),
        skin_report(&stem, INTRO, &commands),
    )
    .map_err(|error| error.to_string())?;
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for command in commands.keys().filter(|command| !skin_covered(command)) {
        *reasons.entry(skin_reason(command)).or_default() += 1;
    }
    let missing: usize = reasons.values().sum();
    let video = draw(&skin, "MainSkinArea", out, &stem, fonts);
    let osc = draw(&skin, "MainSkinAreaOscInside", out, &stem, fonts);
    let audio = match SkinArchive::open(path)
        .map_err(|error| error.to_string())
        .and_then(|archive| {
            if archive.get("AudioSkin.xml").is_none() {
                return Ok(None);
            }
            Skin::load(archive, "AudioSkin.xml")
                .map(Some)
                .map_err(|error| error.to_string())
        }) {
        Ok(Some(audio)) => draw(&audio, "MainSkinArea", out, &format!("{stem}-audio"), fonts),
        Ok(None) => "none".to_owned(),
        Err(error) => format!("error: {error}"),
    };
    let reasons: Vec<String> = reasons
        .iter()
        .map(|(reason, count)| format!("{count}: {reason}"))
        .collect();
    Ok(format!(
        "skin={stem} commands={} carried_out={} not={missing} warnings={} video={video} osc={osc} audio={audio}\n  {}",
        commands.len(),
        commands.len() - missing,
        skin.warnings().len(),
        reasons.join("\n  ")
    ))
}

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let Some(out) = args.next() else {
        eprintln!("usage: skin_coverage OUT SKIN.dsf...");
        return ExitCode::FAILURE;
    };
    if let Err(error) = std::fs::create_dir_all(&out) {
        eprintln!("{}: {error}", out.display());
        return ExitCode::FAILURE;
    }
    let mut fonts = Fonts::new(Vec::new());
    let mut failed = false;
    for path in args {
        match check(&path, &out, &mut fonts) {
            Ok(line) => println!("{line}"),
            Err(error) => {
                failed = true;
                println!("skin={} error={error}", path.display());
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
