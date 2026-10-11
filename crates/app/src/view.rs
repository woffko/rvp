//! What the skin shows, computed from the player model (docs/PLAN.md
//! section 5, "Skin data binding").
//!
//! Labels get the texts PotPlayer shows for the same command IDs; the info
//! strip's `MappingDB` labels get values that match BlackSpace's tables
//! (`H264`, `NV12`, `0x2000`, `5.1`). Values RVP cannot know (codec tags
//! of codecs that the table does not list) are left empty, so the skin shows
//! nothing, as PotPlayer does for unknown values.

use rvp_engine::{PlayerModel, PropertyId};
use rvp_skin::state::ListScroll;
use rvp_skin::{DataModel, Playlist, PlaylistEntry, SliderValue, UiState};

use crate::playlist::Playlists;

/// `HH:MM:SS`, as PotPlayer's time labels.
#[must_use]
pub fn format_time(seconds: f64) -> String {
    // Durations are far below u64::MAX seconds; negative values show 0.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total = if seconds.is_finite() && seconds > 0.0 {
        seconds.floor() as u64
    } else {
        0
    };
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        total / 60 % 60,
        total % 60
    )
}

/// The last component of a path or URL.
#[must_use]
pub fn file_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed)
}

/// The window title: `[index/count] name` for the entry that plays in an
/// album of several, the name alone otherwise, empty when idle.
#[must_use]
pub fn title(model: &PlayerModel, lists: &Playlists) -> String {
    let Some(path) = model.loaded_path().or_else(|| model.text(PropertyId::Path)) else {
        return String::new();
    };
    let playing = lists.playing.and_then(|album| {
        let album = lists.albums.get(album)?;
        let index = album.current?;
        let entry = album.entries.get(index)?;
        (entry.path == path).then_some((index, album.entries.len(), entry.label()))
    });
    match playing {
        Some((index, count, label)) if count > 1 => format!("[{}/{count}] {label}", index + 1),
        Some((_, _, label)) => label.to_owned(),
        None => file_name(path).to_owned(),
    }
}

/// A number with at most `decimals` decimals and no trailing zeros.
fn format_rate(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

/// Bits per second as whole kilobits, as PotPlayer shows bitrates.
fn kilobits(bits: f64) -> String {
    format!("{:.0}", (bits / 1000.0).round())
}

/// The video codec as a four-character code from BlackSpace's table.
fn video_fourcc(codec: &str) -> String {
    let codec = codec.to_ascii_lowercase();
    let code = match codec.as_str() {
        "h264" => "H264",
        "mpeg4" => "XVID",
        "msmpeg4v3" => "DIV3",
        "msmpeg4v2" => "MP42",
        "msmpeg4v1" => "MP41",
        "wmv1" => "WMV1",
        "wmv2" => "WMV2",
        "wmv3" => "WMV3",
        "vc1" => "WVC1",
        "h263" => "H263",
        "mpeg1video" => "MPG1",
        "mpeg2video" => "MPG2",
        "svq1" => "SVQ1",
        "svq3" => "SVQ3",
        "msvideo1" => "MSVC",
        "flv1" => "FLV1",
        "rv10" => "RV10",
        "rv20" => "RV20",
        "rv30" => "RV30",
        "rv40" => "RV40",
        "theora" => "Theo",
        "vp6" => "VP60",
        "vp6f" => "VP61",
        "dvvideo" => "DVSD",
        _ => return codec.to_ascii_uppercase(),
    };
    code.to_owned()
}

/// The decoder output format from BlackSpace's table: `DXVA` while a
/// hardware decoder runs, as PotPlayer shows it.
fn output_fourcc(pixel_format: &str, hwdec: Option<&str>) -> String {
    if hwdec.is_some_and(|name| !name.is_empty() && name != "no") {
        return "DXVA".to_owned();
    }
    let code = match pixel_format {
        "nv12" => "NV12",
        "yuv420p" => "YV12",
        "yuyv422" => "YUY2",
        "rgb24" | "bgr24" | "rgba" | "bgra" | "bgr0" | "rgb0" => "RGB",
        other => return other.to_ascii_uppercase(),
    };
    code.to_owned()
}

/// The audio codec as a Windows wave format tag from BlackSpace's table.
fn audio_tag(codec: &str) -> &'static str {
    match codec {
        "mp3" | "mp3float" => "0x55",
        "mp1" | "mp2" | "mp2float" => "0x50",
        "ac3" | "eac3" => "0x2000",
        "dts" => "0x2001",
        "wmav1" | "wmav2" | "wmapro" => "0x161",
        "vorbis" => "0x674F",
        "aac" | "aac_latm" => "0xFF",
        "cook" => "0x2004",
        "tta" => "0x77A1",
        "flac" => "0xF1AC",
        "wavpack" => "0x5756",
        pcm if pcm.starts_with("pcm_") => "0x1",
        adpcm if adpcm.starts_with("adpcm_") => "0x2",
        _ => "",
    }
}

/// Channel counts as BlackSpace's table names them.
fn channels(count: i64) -> String {
    match count {
        3 => "2.1".to_owned(),
        6 => "5.1".to_owned(),
        7 => "6.1".to_owned(),
        8 => "7.1".to_owned(),
        other => other.to_string(),
    }
}

/// The skin's data model for the current playback state and the albums
/// (the panel shows the shown album).
#[must_use]
pub fn data_model(model: &PlayerModel, lists: &Playlists) -> DataModel {
    let mut data = DataModel::default();
    let loaded = model.media_loaded();
    let time = model.double(PropertyId::TimePos).unwrap_or(0.0);
    let duration = model.double(PropertyId::Duration).filter(|d| *d > 0.0);
    data.set_text("CMD_TITLE_MESSAGE", title(model, lists));
    data.set_text("CMD_CURRENTTIME_MESSAGE", format_time(time));
    data.set_text(
        "CMD_TOTALTIME_MESSAGE",
        format_time(duration.unwrap_or(0.0)),
    );
    data.set_text(
        "CMD_LEFTTIME_MESSAGE",
        format_time(duration.map_or(0.0, |d| d - time)),
    );
    if loaded {
        set_info_strip(&mut data, model, time, duration);
    }
    let fraction = match duration {
        // The slider stores f32; the precision loss is far below a pixel.
        #[allow(clippy::cast_possible_truncation)]
        Some(duration) => (time / duration).clamp(0.0, 1.0) as f32,
        None => 0.0,
    };
    data.sliders.insert(
        "CMD_TIMEBAR_CTRL".to_owned(),
        SliderValue {
            value: fraction,
            enabled: loaded && duration.is_some(),
        },
    );
    // Volume is 0..100 % here; f32 is exact enough for a slider.
    #[allow(clippy::cast_possible_truncation)]
    let volume = (model.double(PropertyId::Volume).unwrap_or(100.0) / 100.0).clamp(0.0, 1.0) as f32;
    data.sliders.insert(
        "CMD_VOLUME_CTRL".to_owned(),
        SliderValue {
            value: volume,
            enabled: true,
        },
    );
    data.playlist = panel(lists);
    data
}

/// The playlist panel's content: the shown album.
#[must_use]
pub fn panel(lists: &Playlists) -> Playlist {
    let shown = &lists.albums[lists.shown.min(lists.albums.len().saturating_sub(1))];
    Playlist {
        albums: lists
            .albums
            .iter()
            .map(|album| album.name.clone())
            .collect(),
        current_album: lists.shown,
        entries: shown
            .entries
            .iter()
            .map(|entry| PlaylistEntry {
                title: entry.label().to_owned(),
                // Whole seconds of at most days.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                duration: entry
                    .duration
                    .map(|seconds| seconds.max(0.0).round() as u32),
                playable: entry.playable,
                played: entry.played,
            })
            .collect(),
        current: shown.current.filter(|_| lists.playing == Some(lists.shown)),
        selected: shown.selected.clone(),
        scroll: ListScroll::Entry(shown.top),
        drop_before: None,
    }
}

fn set_info_strip(data: &mut DataModel, model: &PlayerModel, time: f64, duration: Option<f64>) {
    if let Some(path) = model.loaded_path() {
        let name = file_name(path);
        if let Some((_, extension)) = name.rsplit_once('.') {
            data.set_text("CMD_PLAY_FILE_TYPE", extension.to_ascii_uppercase());
        }
    }
    if let Some(codec) = model.text(PropertyId::VideoFormat) {
        data.set_text("CMD_VIDEO_CODEC_FOURCC_MESSAGE", video_fourcc(codec));
    }
    if let Some(format) = model.text(PropertyId::VideoOutFormat) {
        data.set_text(
            "CMD_VIDEO_OUT_FOURCC_MESSAGE",
            output_fourcc(format, model.text(PropertyId::HwdecCurrent)),
        );
    }
    if let Some(bits) = model.double(PropertyId::VideoBitrate) {
        data.set_text("CMD_VIDEO_BIT_RATE_MESSAGE", kilobits(bits));
    }
    if let Some(codec) = model.text(PropertyId::AudioCodecName) {
        data.set_text("CMD_AUDIO_CODEC_TAG_MESSAGE", audio_tag(codec));
    }
    if let Some(count) = model.int(PropertyId::AudioChannels) {
        data.set_text("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE", channels(count));
    }
    if let Some(rate) = model.int(PropertyId::AudioSampleRate) {
        data.set_text("CMD_AUDIO_OUT_SAMPLE_RATE_MESSAGE", rate.to_string());
    }
    if let Some(bits) = model.double(PropertyId::AudioBitrate) {
        data.set_text("CMD_AUDIO_BIT_RATE_MESSAGE", kilobits(bits));
    }
    if let Some(fps) = model.double(PropertyId::ContainerFps) {
        data.set_text("CMD_VIDEO_CODEC_FPS_MESSAGE", format_rate(fps, 3));
    }
    if let Some(fps) = model.double(PropertyId::EstimatedVfFps) {
        data.set_text("CMD_VIDEO_OUT_FPS_MESSAGE", format_rate(fps, 2));
    }
    if let (Some(width), Some(height)) = (
        model.int(PropertyId::VideoWidth),
        model.int(PropertyId::VideoHeight),
    ) {
        data.set_text("CMD_VIDEO_CODEC_SIZE_MESSAGE", format!("{width} {height}"));
    }
    if let Some(duration) = duration {
        let percent = (time / duration * 100.0).clamp(0.0, 100.0);
        data.set_text("CMD_PERCENTTIME_MESSAGE", format!("{percent:.0}"));
    }
}

/// Sets the playback part of the interface state: loaded, playing, and the
/// mute flag (shown by skins that mark the mute button with `DownFlag`;
/// BlackSpace does not). Window state, hover and pressed are the shell's.
pub fn apply_playback_state(state: &mut UiState, model: &PlayerModel) {
    state.media_loaded = model.media_loaded();
    state.playing = model.playing();
    if model.flag(PropertyId::Mute) == Some(true) {
        state.flags.insert("ID_AUDIO_MUTE".to_owned());
    } else {
        state.flags.remove("ID_AUDIO_MUTE");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{Album, Entry};
    use rvp_engine::Value;

    /// `count` entries in one album; entry `index` (path `path`) plays.
    fn lists_playing(path: &str, index: usize, count: usize) -> Playlists {
        let mut album = Album::new("Default");
        album.entries = (0..count)
            .map(|n| {
                if n == index {
                    // As mpv reports it (`playing_model` sets that form).
                    Entry {
                        path: path.to_owned(),
                        ..Entry::default()
                    }
                } else {
                    Entry::new(&format!("/films/other {n}.mkv"))
                }
            })
            .collect();
        album.current = Some(index);
        Playlists {
            albums: vec![album],
            playing: Some(0),
            ..Playlists::default()
        }
    }

    fn playing_model() -> PlayerModel {
        let mut model = PlayerModel::new();
        let path = "/films/Sample Film.mkv";
        model.set(PropertyId::Path, Some(Value::Text(path.to_owned())));
        model.set_loaded_path(Some(path.to_owned()));
        model.set(PropertyId::IdleActive, Some(Value::Flag(false)));
        model.set(PropertyId::Pause, Some(Value::Flag(false)));
        model.set(PropertyId::TimePos, Some(Value::Double(3725.4)));
        model.set(PropertyId::Duration, Some(Value::Double(7729.0)));
        model.set(PropertyId::PlaylistPos, Some(Value::Int(11)));
        model.set(PropertyId::PlaylistCount, Some(Value::Int(12)));
        model.set(PropertyId::VideoFormat, Some(Value::Text("h264".into())));
        model.set(PropertyId::VideoOutFormat, Some(Value::Text("nv12".into())));
        model.set(PropertyId::AudioCodecName, Some(Value::Text("ac3".into())));
        model.set(PropertyId::AudioChannels, Some(Value::Int(6)));
        model.set(PropertyId::AudioSampleRate, Some(Value::Int(48000)));
        model.set(PropertyId::AudioBitrate, Some(Value::Double(448_000.0)));
        model.set(PropertyId::VideoBitrate, Some(Value::Double(5_621_400.0)));
        model.set(PropertyId::ContainerFps, Some(Value::Double(23.976_023)));
        model.set(PropertyId::EstimatedVfFps, Some(Value::Double(59.0632)));
        model.set(PropertyId::VideoWidth, Some(Value::Int(1920)));
        model.set(PropertyId::VideoHeight, Some(Value::Int(800)));
        model.set(PropertyId::Volume, Some(Value::Double(22.0)));
        model
    }

    #[test]
    fn times_and_names() {
        assert_eq!(format_time(0.0), "00:00:00");
        assert_eq!(format_time(7729.9), "02:08:49");
        assert_eq!(format_time(-3.0), "00:00:00");
        assert_eq!(format_time(f64::NAN), "00:00:00");
        assert_eq!(file_name("/a/b/c.mkv"), "c.mkv");
        assert_eq!(file_name(r"V:\films\c.mkv"), "c.mkv");
        assert_eq!(file_name("https://host/dir/"), "dir");
        assert_eq!(format_rate(23.976_023, 3), "23.976");
        assert_eq!(format_rate(24.0, 3), "24");
        assert_eq!(format_rate(59.0632, 2), "59.06");
    }

    #[test]
    fn info_strip_matches_blackspace_tables() {
        let data = data_model(
            &playing_model(),
            &lists_playing("/films/Sample Film.mkv", 11, 12),
        );
        for (command, text) in [
            ("CMD_TITLE_MESSAGE", "[12/12] Sample Film.mkv"),
            ("CMD_CURRENTTIME_MESSAGE", "01:02:05"),
            ("CMD_TOTALTIME_MESSAGE", "02:08:49"),
            ("CMD_LEFTTIME_MESSAGE", "01:06:43"),
            ("CMD_PLAY_FILE_TYPE", "MKV"),
            ("CMD_VIDEO_CODEC_FOURCC_MESSAGE", "H264"),
            ("CMD_VIDEO_OUT_FOURCC_MESSAGE", "NV12"),
            ("CMD_VIDEO_BIT_RATE_MESSAGE", "5621"),
            ("CMD_AUDIO_CODEC_TAG_MESSAGE", "0x2000"),
            ("CMD_AUDIO_IN_NUM_CHANNEL_MESSAGE", "5.1"),
            ("CMD_AUDIO_OUT_SAMPLE_RATE_MESSAGE", "48000"),
            ("CMD_AUDIO_BIT_RATE_MESSAGE", "448"),
            ("CMD_VIDEO_CODEC_FPS_MESSAGE", "23.976"),
            ("CMD_VIDEO_OUT_FPS_MESSAGE", "59.06"),
            ("CMD_VIDEO_CODEC_SIZE_MESSAGE", "1920 800"),
            ("CMD_PERCENTTIME_MESSAGE", "48"),
        ] {
            assert_eq!(data.text(command), Some(text), "{command}");
        }
        let seek = data.sliders["CMD_TIMEBAR_CTRL"];
        assert!(seek.enabled);
        assert!((seek.value - 0.482).abs() < 0.001);
        assert!((data.sliders["CMD_VOLUME_CTRL"].value - 0.22).abs() < 1e-6);
    }

    #[test]
    fn idle_shows_zero_times_and_a_disabled_seek_bar() {
        let data = data_model(&PlayerModel::new(), &Playlists::default());
        assert_eq!(data.text("CMD_TITLE_MESSAGE"), Some(""));
        assert_eq!(data.text("CMD_CURRENTTIME_MESSAGE"), Some("00:00:00"));
        assert_eq!(data.text("CMD_PLAY_FILE_TYPE"), None);
        assert!(!data.sliders["CMD_TIMEBAR_CTRL"].enabled);
    }

    #[test]
    fn hardware_decoding_shows_dxva_and_playlist_marks_current() {
        let mut model = playing_model();
        model.set(
            PropertyId::HwdecCurrent,
            Some(Value::Text("nvdec-copy".into())),
        );
        model.set(PropertyId::PlaylistPos, Some(Value::Int(1)));
        model.set(PropertyId::PlaylistCount, Some(Value::Int(2)));
        let mut lists = lists_playing("/films/Sample Film.mkv", 1, 2);
        lists.albums[0].entries[0] = Entry::new("/a/one.mkv");
        lists.albums[0].entries[1].played = true;
        let data = data_model(&model, &lists);
        assert_eq!(data.text("CMD_VIDEO_OUT_FOURCC_MESSAGE"), Some("DXVA"));
        assert_eq!(data.playlist.entries[0].title, "one.mkv");
        assert_eq!(data.playlist.current, Some(1));
        assert!(data.playlist.entries[1].played);
        assert_eq!(
            data.text("CMD_TITLE_MESSAGE"),
            Some("[2/2] Sample Film.mkv")
        );
        let mut state = UiState::default();
        model.set(PropertyId::Mute, Some(Value::Flag(true)));
        apply_playback_state(&mut state, &model);
        assert!(state.playing && state.media_loaded && state.flag("ID_AUDIO_MUTE"));
    }
}
