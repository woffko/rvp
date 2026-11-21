//! M13: PotPlayer's Create Subtitles from Audio (goal O), offline with
//! whisper.cpp; `docs/spikes/m13-whisper.md` has the study.
//!
//! * The engine is whisper.cpp's command line (`whisper-cli`), started as a
//!   process of its own with no console: for the whole file at normal
//!   priority on all cores but one, in real time at a lower priority on
//!   half of them, so playback keeps the CPU first. RVP's Windows
//!   package has one build for the CPU and Vulkan (`whisper/` next to
//!   `rvp.exe`, `tools/whisper/build.sh`): ggml loads its Vulkan backend
//!   when a driver offers one, and `-ng` keeps a job on the CPU. The CUDA
//!   and BLAS builds are whisper.cpp's own releases, which the user puts
//!   into `whisper/cuda` or `whisper/blas` of RVP's settings folder (that
//!   folder's `whisper/cpu` and `whisper/vulkan` take the place of the
//!   package's, and Linux also finds `whisper-cli` on the PATH).
//! * Models are whisper.cpp's `ggml-<model>.bin` files the user downloads
//!   into `whisper-models` of RVP's settings folder; RVP downloads nothing.
//! * The audio goes to the engine as a 16 kHz mono WAV file that a libmpv
//!   core of its own writes (`rvp_engine::encode`, the playing audio
//!   track), with a filter for speech when the user asks for it.
//! * Ahead of time ("Start", or automatically when a file starts) the whole
//!   file is made into an SRT file next to the video
//!   (`<name>.<language>.srt`, or `<name>.<language>.whisper.srt` when the
//!   first exists) or, when that folder is not writable or the user chose
//!   so, in RVP's state folder; it is added as a subtitle track. A
//!   stream's go to the state folder, named by its title or the last part
//!   of its address, in characters every file system takes.
//! * In real time RVP makes 30 s pieces from the playback position on, two
//!   ahead, and again from where a seek lands; their subtitles show as one
//!   track (`subtitles/realtime-<pid>.srt` in the state folder, each RVP's
//!   own).
//! * While the GPU power budget's step is taken (D17, `crate::gpu_power`)
//!   the next pieces and jobs run on the CPU.
//!
//! The work runs on threads of its own (extraction and the engine); the
//! window's thread takes their events in [`App::speech_tick`].

use std::collections::BTreeSet;
use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use rvp_engine::Engine;
use rvp_engine::encode::{EncodeJob, encode};
use rvp_subtitle::{Cue, Document, Format};
use serde::{Deserialize, Serialize};

use crate::app::{App, UiRequest};

/// RVP's engines: settings name and PotPlayer's menu text.
pub const ENGINES: &[(&str, &str)] = &[
    ("cpu", "whisper.cpp CPU"),
    ("blas", "whisper.cpp BLAS"),
    ("vulkan", "whisper.cpp Vulkan"),
    ("cuda", "whisper.cpp CUDA"),
];

/// whisper.cpp's models (`ggml-<name>.bin`), as PotPlayer lists them.
pub const MODELS: &[&str] = &[
    "tiny",
    "tiny.en",
    "base",
    "base.en",
    "small",
    "small.en",
    "medium",
    "medium.en",
    "large",
    "large-v1",
    "large-v2",
    "large-v3",
    "large-v3-turbo",
];

/// whisper's languages in PotPlayer's order: the code whisper.cpp takes
/// (`auto`: it finds out) and the name PotPlayer's menu shows.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("auto", "Auto"),
    ("ko", "korean"),
    ("en", "english"),
    ("ja", "japanese"),
    ("zh", "chinese"),
    ("de", "german"),
    ("es", "spanish"),
    ("ru", "russian"),
    ("fr", "french"),
    ("pt", "portuguese"),
    ("tr", "turkish"),
    ("pl", "polish"),
    ("ca", "catalan"),
    ("nl", "dutch"),
    ("ar", "arabic"),
    ("sv", "swedish"),
    ("it", "italian"),
    ("id", "indonesian"),
    ("hi", "hindi"),
    ("fi", "finnish"),
    ("vi", "vietnamese"),
    ("he", "hebrew"),
    ("uk", "ukrainian"),
    ("el", "greek"),
    ("ms", "malay"),
    ("cs", "czech"),
    ("ro", "romanian"),
    ("da", "danish"),
    ("hu", "hungarian"),
    ("ta", "tamil"),
    ("no", "norwegian"),
    ("th", "thai"),
    ("ur", "urdu"),
    ("hr", "croatian"),
    ("bg", "bulgarian"),
    ("yue", "cantonese"),
    ("lt", "lithuanian"),
    ("la", "latin"),
    ("mi", "maori"),
    ("ml", "malayalam"),
    ("cy", "welsh"),
    ("sk", "slovak"),
    ("te", "telugu"),
    ("fa", "persian"),
    ("lv", "latvian"),
    ("bn", "bengali"),
    ("sr", "serbian"),
    ("az", "azerbaijani"),
    ("sl", "slovenian"),
    ("kn", "kannada"),
    ("et", "estonian"),
    ("mk", "macedonian"),
    ("br", "breton"),
    ("eu", "basque"),
    ("is", "icelandic"),
    ("hy", "armenian"),
    ("ne", "nepali"),
    ("mn", "mongolian"),
    ("bs", "bosnian"),
    ("kk", "kazakh"),
    ("sq", "albanian"),
    ("sw", "swahili"),
    ("gl", "galician"),
    ("mr", "marathi"),
    ("pa", "punjabi"),
    ("si", "sinhala"),
    ("km", "khmer"),
    ("sn", "shona"),
    ("yo", "yoruba"),
    ("so", "somali"),
    ("af", "afrikaans"),
    ("oc", "occitan"),
    ("ka", "georgian"),
    ("be", "belarusian"),
    ("tg", "tajik"),
    ("sd", "sindhi"),
    ("gu", "gujarati"),
    ("am", "amharic"),
    ("yi", "yiddish"),
    ("lo", "lao"),
    ("uz", "uzbek"),
    ("fo", "faroese"),
    ("ht", "haitian creole"),
    ("ps", "pashto"),
    ("tk", "turkmen"),
    ("nn", "nynorsk"),
    ("mt", "maltese"),
    ("sa", "sanskrit"),
    ("lb", "luxembourgish"),
    ("my", "myanmar"),
    ("bo", "tibetan"),
    ("tl", "tagalog"),
    ("mg", "malagasy"),
    ("as", "assamese"),
    ("tt", "tatar"),
    ("haw", "hawaiian"),
    ("ln", "lingala"),
    ("ha", "hausa"),
    ("ba", "bashkir"),
    ("jw", "javanese"),
    ("su", "sundanese"),
];

/// When subtitles are made ahead of time by themselves (PotPlayer's three
/// radio items): never, for every file, or for files without subtitles.
pub const AUTO_MODES: &[&str] = &["off", "always", "no-subtitles"];
/// When real-time subtitles are made: never, always, for files without
/// subtitles, or for sources that are not local files.
pub const REALTIME_MODES: &[&str] = &["off", "always", "no-subtitles", "streams"];

/// Where whisper.cpp's models come from (the settings page says it).
pub const MODELS_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp";
/// Where whisper.cpp's CUDA and BLAS builds come from.
pub const ENGINES_URL: &str = "https://github.com/ggml-org/whisper.cpp/releases";

/// A real-time piece's length and how many pieces are made ahead.
const PIECE_SECONDS: f64 = 30.0;
const PIECES_AHEAD: u64 = 2;
/// The longest an extraction may take (a whole film's audio).
const EXTRACT_TIMEOUT: Duration = Duration::from_hours(1);
/// How often the position goes to the real-time thread.
const POSITION_INTERVAL: Duration = Duration::from_secs(1);

/// One mode's engine, model and language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Recognizer {
    /// One of [`ENGINES`].
    pub engine: String,
    /// One of [`MODELS`].
    pub model: String,
    /// A code of [`LANGUAGES`].
    pub language: String,
}

impl Default for Recognizer {
    fn default() -> Self {
        Self {
            engine: "vulkan".to_owned(),
            model: "small".to_owned(),
            language: "auto".to_owned(),
        }
    }
}

/// Create Subtitles from Audio's settings (`speech` in settings.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SpeechSettings {
    /// Ahead of time.
    pub file: Recognizer,
    /// A filter that brings speech forward when the audio is extracted.
    pub voice_filter: bool,
    /// The SRT file goes next to the video when its folder can take it.
    pub beside_video: bool,
    /// One of [`AUTO_MODES`].
    pub auto: String,
    /// Made subtitles show when they arrive (PotPlayer's "Show
    /// auto-generated subtitles").
    pub show: bool,
    /// One of [`REALTIME_MODES`].
    pub realtime: String,
    pub realtime_recognizer: Recognizer,
    /// The models' folder (PotPlayer's Whisper Model home folder);
    /// unset: `whisper-models` beside the settings.
    pub model_folder: Option<PathBuf>,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        Self {
            file: Recognizer::default(),
            voice_filter: false,
            beside_video: true,
            auto: "off".to_owned(),
            show: true,
            realtime: "off".to_owned(),
            realtime_recognizer: Recognizer {
                model: "base".to_owned(),
                ..Recognizer::default()
            },
            model_folder: None,
        }
    }
}

impl SpeechSettings {
    pub(crate) fn validate(&mut self, notes: &mut Vec<String>) {
        let choose = |value: &mut String,
                      allowed: &[&str],
                      fallback: &str,
                      name: &str,
                      notes: &mut Vec<String>| {
            if !allowed.contains(&value.as_str()) {
                notes.push(format!(
                    "speech.{name} {value:?} is none of {allowed:?}: {fallback}"
                ));
                fallback.clone_into(value);
            }
        };
        let engines: Vec<&str> = ENGINES.iter().map(|(name, _)| *name).collect();
        let languages: Vec<&str> = LANGUAGES.iter().map(|(code, _)| *code).collect();
        for (place, recognizer) in [
            ("file", &mut self.file),
            ("realtime-recognizer", &mut self.realtime_recognizer),
        ] {
            choose(
                &mut recognizer.engine,
                &engines,
                "vulkan",
                &format!("{place}.engine"),
                notes,
            );
            choose(
                &mut recognizer.model,
                MODELS,
                "base",
                &format!("{place}.model"),
                notes,
            );
            choose(
                &mut recognizer.language,
                &languages,
                "auto",
                &format!("{place}.language"),
                notes,
            );
        }
        choose(&mut self.auto, AUTO_MODES, "off", "auto", notes);
        choose(&mut self.realtime, REALTIME_MODES, "off", "realtime", notes);
        if let Some(folder) = &self.model_folder
            && !folder.is_absolute()
        {
            notes.push(format!(
                "speech.model-folder {} is not a full path: the folder beside the settings",
                folder.display()
            ));
            self.model_folder = None;
        }
    }
}

/// A command of the Create Subtitles from Audio menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpeechCommand {
    /// Start, or stop a running job.
    Start,
    VoiceFilter,
    BesideVideo,
    /// One of [`AUTO_MODES`].
    Auto(u8),
    /// One of [`REALTIME_MODES`].
    Realtime(u8),
    /// The folders of the engines and of the models.
    EngineFolder,
    ModelFolder,
}

/// A choice of the run-time lists (engine, model, language) for the file
/// (`realtime` false) or real time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechChoice {
    Engine(bool, usize),
    Model(bool, usize),
    Language(bool, usize),
}

impl SpeechChoice {
    /// The menu's ID: the list (`e`, `m`, `l`), `f` (the file) or `r`
    /// (real time), and the index: `ef2`.
    #[must_use]
    pub fn id(self) -> String {
        let (kind, realtime, index) = match self {
            Self::Engine(realtime, index) => ('e', realtime, index),
            Self::Model(realtime, index) => ('m', realtime, index),
            Self::Language(realtime, index) => ('l', realtime, index),
        };
        format!("{kind}{}{index}", if realtime { 'r' } else { 'f' })
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut chars = text.chars();
        let kind = chars.next()?;
        let realtime = match chars.next()? {
            'r' => true,
            'f' => false,
            _ => return None,
        };
        let index: usize = chars.as_str().parse().ok()?;
        Some(match kind {
            'e' => Self::Engine(realtime, index),
            'm' => Self::Model(realtime, index),
            'l' => Self::Language(realtime, index),
            _ => return None,
        })
    }
}

/// What a job's thread reports.
#[derive(Debug)]
enum Event {
    /// The engine's progress in percent.
    Progress(u8),
    /// The whole file's subtitles and the language (code) whisper used.
    Done(Document, String),
    /// A real-time piece: its index and subtitles (times of the file).
    Piece(u64, Vec<Cue>),
    Failed(String),
}

/// What the window's thread tells a job's thread.
#[derive(Debug)]
enum Control {
    Stop,
    /// The playback position (real time), in seconds.
    Position(f64),
    /// Run the engine on the CPU (the GPU budget's step).
    Cpu(bool),
}

/// A whisper.cpp run: the program, its model and how to run it.
#[derive(Debug, Clone)]
struct Recognition {
    program: PathBuf,
    model: PathBuf,
    language: String,
    /// The engine may use the GPU (`-ng` otherwise).
    gpu: bool,
    threads: usize,
    /// Below normal priority (real time: playback keeps the CPU first).
    background: bool,
}

/// The audio a job reads.
#[derive(Debug, Clone)]
struct Source {
    path: String,
    audio_track: Option<i64>,
    duration: f64,
    voice_filter: bool,
    /// The job's own folder for its WAV and SRT files.
    work: PathBuf,
}

/// A job running on its thread.
#[derive(Debug)]
struct Running {
    control: Sender<Control>,
    events: Receiver<Event>,
    /// The video it is for.
    video: String,
    /// The stream's own title, when mpv knows one (not the address's name
    /// it shows for a stream without).
    title: Option<String>,
    gpu: bool,
    progress: u8,
}

/// Real time's subtitles so far.
#[derive(Debug, Default)]
struct Pieces {
    cues: Vec<Cue>,
    done: BTreeSet<u64>,
    /// mpv's track that shows them.
    track: Option<i64>,
}

/// RVP's speech recognition state, part of [`App`].
#[derive(Debug, Default)]
pub struct Speech {
    file: Option<Running>,
    realtime: Option<Running>,
    pieces: Pieces,
    position_sent: Option<Instant>,
    /// The GPU budget's step is taken: the engine runs on the CPU.
    cpu_only: bool,
}

/// The engine's program for `engine`: the user's build in
/// `config/whisper/<engine>`, else (CPU and Vulkan) the package's
/// `whisper/`, else `whisper-cli` on the PATH (Linux). `RVP_WHISPER_CLI`
/// names one for every engine (tests).
#[must_use]
pub fn engine_program(
    engine: &str,
    config: Option<&Path>,
    exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(program) = std::env::var_os("RVP_WHISPER_CLI").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(program));
    }
    let name = if cfg!(windows) {
        "whisper-cli.exe"
    } else {
        "whisper-cli"
    };
    let mut candidates = Vec::new();
    if let Some(config) = config {
        candidates.push(config.join("whisper").join(engine).join(name));
    }
    if matches!(engine, "cpu" | "vulkan")
        && let Some(dir) = exe_dir
    {
        candidates.push(dir.join("whisper").join(name));
    }
    if !cfg!(windows)
        && let Some(path) = std::env::var_os("PATH")
    {
        candidates.extend(std::env::split_paths(&path).map(|dir| dir.join(name)));
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// The model file of `model` in `folder`.
#[must_use]
pub fn model_file(folder: &Path, model: &str) -> PathBuf {
    folder.join(format!("ggml-{model}.bin"))
}

/// The longest name, in bytes, of the subtitles made for a stream (before
/// `.<language>.srt`): well inside the 255 bytes file systems take.
const STREAM_NAME_BYTES: usize = 120;

/// The name of the subtitles made for `video`, before `.<language>.srt`.
/// A local file's own name without its extension, so that players find
/// them beside it. For an address, the stream's `title` or else the last
/// part of the address's path, without its query and fragment (a stream's
/// address can be long and full of characters no file system takes):
/// those characters become `_`, and the name is cut at
/// [`STREAM_NAME_BYTES`]. Never empty.
fn subtitle_stem(video: &str, title: Option<&str>) -> String {
    if !video.contains("://") {
        return Path::new(video).file_stem().map_or_else(
            || "subtitles".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        );
    }
    let name = title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map_or_else(|| address_name(video), str::to_owned);
    file_name_of(&name)
}

/// The last part of an address's path as mpv shows it (`%XX` decoded),
/// without the query, the fragment and the extension; empty when the
/// address has no path (its host, and any credentials there, are no name).
fn address_name(address: &str) -> String {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let rest = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    let path = rest.find('/').map_or("", |slash| &rest[slash..]);
    let segment = percent_decoded(path.rsplit('/').find(|part| !part.is_empty()).unwrap_or(""));
    match segment.rfind('.') {
        Some(dot) if dot > 0 => segment[..dot].to_owned(),
        _ => segment,
    }
}

/// `text` with its `%XX` escapes decoded, when that makes UTF-8.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escape = bytes
            .get(at + 1..at + 3)
            .filter(|hex| bytes[at] == b'%' && hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        if let Some(byte) = escape {
            out.push(byte);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// `name` as a file name Windows and Linux both take: the characters
/// Windows refuses and control characters as `_`, white space at the ends
/// left out, at most [`STREAM_NAME_BYTES`] (cut between characters), `_`
/// before a name Windows keeps for a device (`CON`, `NUL.x`), and
/// `subtitles` for nothing.
fn file_name_of(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut text = replaced.trim();
    if text.len() > STREAM_NAME_BYTES {
        let mut end = STREAM_NAME_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text = text[..end].trim_end();
    }
    if text.is_empty() {
        return "subtitles".to_owned();
    }
    let base = text.split('.').next().unwrap_or(text).trim_end();
    let device = ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|device| base.eq_ignore_ascii_case(device))
        || (base.len() == 4
            && base.get(..3).is_some_and(|prefix| {
                prefix.eq_ignore_ascii_case("COM") || prefix.eq_ignore_ascii_case("LPT")
            })
            && matches!(base.as_bytes()[3], b'1'..=b'9'));
    if device {
        format!("_{text}")
    } else {
        text.to_owned()
    }
}

/// The engine's progress in a line of its output (`-pp`).
fn progress_of(line: &str) -> Option<u8> {
    let rest = line.split("progress =").nth(1)?;
    rest.trim().trim_end_matches('%').trim().parse().ok()
}

/// The language whisper found (`auto-detected language: en (p = 0.97)`).
fn detected_language(line: &str) -> Option<String> {
    let rest = line.split("auto-detected language:").nth(1)?;
    rest.split_whitespace().next().map(str::to_owned)
}

/// Writes the audio of `source` from `start` to `end` (seconds) as a 16 kHz
/// mono WAV file.
fn extract(source: &Source, wav: &Path, start: f64, end: f64) -> Result<(), String> {
    let pair = |name: &str, value: &str| (name.to_owned(), value.to_owned());
    let mut extra = vec![
        pair("vid", "no"),
        pair("sid", "no"),
        pair("of", "wav"),
        pair("oac", "pcm_s16le"),
        pair("audio-samplerate", "16000"),
        pair("audio-channels", "mono"),
    ];
    if let Some(track) = source.audio_track {
        extra.push(pair("aid", &track.to_string()));
    }
    if source.voice_filter {
        // Speech's band, evened out.
        extra.push(pair(
            "af",
            "lavfi=[highpass=f=80,lowpass=f=7600,dynaudnorm=f=250:g=15]",
        ));
    }
    let job = EncodeJob {
        source: source.path.clone(),
        output: wav.to_path_buf(),
        start,
        end,
        vf: String::new(),
        extra,
    };
    encode(&job, EXTRACT_TIMEOUT).map_err(|error| format!("cannot extract the audio: {}", error.0))
}

/// Starts the engine on `wav`, writing `<base>.srt`.
fn start_engine(recognition: &Recognition, wav: &Path, base: &Path) -> Result<Child, String> {
    // Linux: coreutils' nice for the lower priority.
    let mut command = if recognition.background && !cfg!(windows) {
        let mut nice = Command::new("nice");
        nice.args(["-n", "10"]).arg(&recognition.program);
        nice
    } else {
        Command::new(&recognition.program)
    };
    command
        .arg("-m")
        .arg(&recognition.model)
        .arg("-f")
        .arg(wav)
        .args(["-l", &recognition.language, "-osrt", "-pp", "-t"])
        .arg(recognition.threads.to_string())
        .arg("-of")
        .arg(base)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if !recognition.gpu {
        command.arg("-ng");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: no console over the video; with
        // BELOW_NORMAL_PRIORITY_CLASS, playback keeps the CPU first.
        let priority = if recognition.background {
            0x0000_4000
        } else {
            0
        };
        command.creation_flags(0x0800_0000 | priority);
    }
    command
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", recognition.program.display()))
}

/// Runs the engine on `wav` until it is done; the subtitles and the
/// language whisper used. A `Stop` ends it; positions go to `position`.
fn recognize(
    recognition: &Recognition,
    wav: &Path,
    base: &Path,
    control: &Receiver<Control>,
    position: &mut f64,
    mut progress: impl FnMut(u8),
) -> Result<Option<(Document, String)>, String> {
    let mut child = start_engine(recognition, wav, base)?;
    let (lines, read) = mpsc::channel();
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                if lines.send(line).is_err() {
                    break;
                }
            }
        });
    }
    let mut language = recognition.language.clone();
    let mut last_error = String::new();
    loop {
        loop {
            match control.try_recv() {
                Ok(Control::Stop) | Err(mpsc::TryRecvError::Disconnected) => {
                    drop(child.kill());
                    drop(child.wait());
                    return Ok(None);
                }
                Ok(Control::Position(seconds)) => *position = seconds,
                Ok(Control::Cpu(_)) => {}
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        match read.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if let Some(percent) = progress_of(&line) {
                    progress(percent);
                } else if let Some(found) = detected_language(&line) {
                    language = found;
                } else if line.contains("error") {
                    last_error = line;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("whisper.cpp failed ({status}): {last_error}"));
    }
    let srt = base.with_extension("srt");
    let bytes =
        std::fs::read(&srt).map_err(|error| format!("whisper.cpp wrote no subtitles: {error}"))?;
    // No speech: whisper writes an empty file.
    let mut document = if String::from_utf8_lossy(&bytes).trim().is_empty() {
        Document::new(Format::Srt)
    } else {
        Document::parse(&bytes, Some("whisper.srt")).map_err(|error| error.to_string())?
    };
    // whisper starts each line with a space and marks silence.
    for cue in &mut document.cues {
        cue.text = cue.text.trim().to_owned();
    }
    document
        .cues
        .retain(|cue| !cue.text.is_empty() && cue.text != "[BLANK_AUDIO]");
    Ok(Some((document, language)))
}

/// The whole file, ahead of time.
fn run_file(
    source: &Source,
    recognition: &Recognition,
    control: &Receiver<Control>,
    events: &Sender<Event>,
) {
    let result = (|| {
        std::fs::create_dir_all(&source.work).map_err(|error| error.to_string())?;
        let wav = source.work.join("audio.wav");
        extract(source, &wav, 0.0, source.duration)?;
        let mut position = 0.0;
        let found = recognize(
            recognition,
            &wav,
            &source.work.join("audio"),
            control,
            &mut position,
            |percent| {
                drop(events.send(Event::Progress(percent)));
            },
        );
        drop(std::fs::remove_file(&wav));
        found
    })();
    match result {
        Ok(Some((document, language))) => drop(events.send(Event::Done(document, language))),
        Ok(None) => {}
        Err(error) => drop(events.send(Event::Failed(error))),
    }
}

/// Real time: pieces from the playback position on, until stopped.
fn run_realtime(
    source: &Source,
    mut recognition: Recognition,
    start: f64,
    control: &Receiver<Control>,
    events: &Sender<Event>,
) {
    if let Err(error) = std::fs::create_dir_all(&source.work) {
        drop(events.send(Event::Failed(error.to_string())));
        return;
    }
    let mut position = start;
    let mut done = BTreeSet::new();
    loop {
        match control.recv_timeout(Duration::from_millis(200)) {
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Control::Position(seconds)) => position = seconds,
            Ok(Control::Cpu(cpu)) => recognition.gpu = !cpu,
            Err(RecvTimeoutError::Timeout) => {}
        }
        // Pieces are whole multiples of their length, from 0.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let first = (position.max(0.0) / PIECE_SECONDS) as u64;
        // Piece numbers stay far inside f64's exact range.
        #[allow(clippy::cast_precision_loss)]
        let next = (first..=first + PIECES_AHEAD).find(|index| {
            !done.contains(index) && (*index as f64) * PIECE_SECONDS < source.duration
        });
        let Some(index) = next else {
            continue;
        };
        // As above: a piece's number.
        #[allow(clippy::cast_precision_loss)]
        let begin = index as f64 * PIECE_SECONDS;
        let end = (begin + PIECE_SECONDS).min(source.duration);
        let wav = source.work.join("piece.wav");
        let result = extract(source, &wav, begin, end).and_then(|()| {
            recognize(
                &recognition,
                &wav,
                &source.work.join("piece"),
                control,
                &mut position,
                |_| {},
            )
        });
        match result {
            Ok(Some((mut document, _))) => {
                let offset = crate::subtitle_editor::milliseconds(begin);
                document.shift(offset);
                done.insert(index);
                drop(events.send(Event::Piece(index, document.cues)));
            }
            Ok(None) => return,
            Err(error) => {
                drop(events.send(Event::Failed(error)));
                return;
            }
        }
    }
}

impl App {
    fn settings_folder(&self) -> PathBuf {
        self.settings_path()
            .and_then(Path::parent)
            .map_or_else(|| self.state_dir().to_path_buf(), Path::to_path_buf)
    }

    /// The folder of whisper.cpp's models: `RVP_WHISPER_MODELS` (tests),
    /// the setting, or `whisper-models` beside the settings.
    #[must_use]
    pub fn model_folder(&self) -> PathBuf {
        std::env::var_os("RVP_WHISPER_MODELS")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| self.settings.speech.model_folder.clone())
            .unwrap_or_else(|| self.settings_folder().join("whisper-models"))
    }

    /// The folder of the user's engines.
    #[must_use]
    pub fn engine_folder(&self) -> PathBuf {
        self.settings_folder().join("whisper")
    }

    /// The program and model for `recognizer`, or why there are none.
    fn recognition(
        &self,
        recognizer: &Recognizer,
        threads: usize,
        background: bool,
    ) -> Result<Recognition, String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        let label = ENGINES
            .iter()
            .find(|(name, _)| *name == recognizer.engine)
            .map_or("whisper.cpp", |(_, label)| *label);
        let program = engine_program(
            &recognizer.engine,
            Some(&self.settings_folder()),
            exe_dir.as_deref(),
        )
        .ok_or_else(|| {
            format!(
                "{label} is not installed: put whisper.cpp's files ({ENGINES_URL}) into {}",
                self.engine_folder().join(&recognizer.engine).display()
            )
        })?;
        let model = model_file(&self.model_folder(), &recognizer.model);
        if !model.is_file() {
            return Err(format!(
                "The whisper model {} is not in {}: download it from {MODELS_URL}",
                model.file_name().unwrap_or_default().to_string_lossy(),
                self.model_folder().display()
            ));
        }
        Ok(Recognition {
            program,
            model,
            language: recognizer.language.clone(),
            gpu: matches!(recognizer.engine.as_str(), "vulkan" | "cuda") && !self.speech.cpu_only,
            threads,
            background,
        })
    }

    /// The playing file as a job's source.
    fn speech_source(&self, engine: &Engine, folder: &str) -> Option<Source> {
        let path = self.model.loaded_path()?.to_owned();
        // Just after a file loads mpv may not have told the duration yet.
        let duration = self
            .model
            .double(rvp_engine::PropertyId::Duration)
            .or_else(|| engine.get_double("duration"))
            .filter(|duration| *duration > 0.0)?;
        Some(Source {
            path,
            audio_track: engine.get_text("aid").and_then(|aid| aid.parse().ok()),
            duration,
            voice_filter: self.settings.speech.voice_filter,
            work: self.state_dir().join("speech").join(folder),
        })
    }

    /// Start (11371): makes the playing file's subtitles, or stops the job.
    pub(crate) fn speech_start(&mut self, engine: &Engine) -> UiRequest {
        if let Some(running) = self.speech.file.take() {
            drop(running.control.send(Control::Stop));
            return UiRequest::Osd("Subtitles from audio: stopped".to_owned());
        }
        let Some(source) = self.speech_source(engine, "file") else {
            return UiRequest::Osd("Open a video first".to_owned());
        };
        // The user waits for it: all cores but one, at normal priority.
        let threads = std::thread::available_parallelism()
            .map_or(4, usize::from)
            .saturating_sub(1)
            .clamp(1, 8);
        let recognition = match self.recognition(&self.settings.speech.file.clone(), threads, false)
        {
            Ok(recognition) => recognition,
            Err(why) => return UiRequest::Message(why),
        };
        let (control, control_end) = mpsc::channel();
        let (events_end, events) = mpsc::channel();
        let gpu = recognition.gpu;
        let video = source.path.clone();
        let title = engine
            .get_text("media-title")
            .filter(|title| engine.get_text("filename").as_ref() != Some(title));
        let spawned = std::thread::Builder::new()
            .name("rvp-speech".to_owned())
            .spawn(move || run_file(&source, &recognition, &control_end, &events_end));
        if let Err(error) = spawned {
            return UiRequest::Message(format!("cannot start the subtitles from audio: {error}"));
        }
        self.speech.file = Some(Running {
            control,
            events,
            video,
            title,
            gpu,
            progress: 0,
        });
        UiRequest::Osd("Subtitles from audio: extracting the audio".to_owned())
    }

    /// Real time on, for the playing file from the playback position.
    fn start_realtime(&mut self, engine: &Engine) -> Option<UiRequest> {
        let source = self.speech_source(engine, "realtime")?;
        // Half the cores: playback goes on beside it.
        let threads = std::thread::available_parallelism()
            .map_or(4, usize::from)
            .div_ceil(2)
            .clamp(1, 8);
        let recognition = match self.recognition(
            &self.settings.speech.realtime_recognizer.clone(),
            threads,
            true,
        ) {
            Ok(recognition) => recognition,
            Err(why) => return Some(UiRequest::Message(why)),
        };
        let start = self
            .model
            .double(rvp_engine::PropertyId::TimePos)
            .unwrap_or(0.0);
        let (control, control_end) = mpsc::channel();
        let (events_end, events) = mpsc::channel();
        let gpu = recognition.gpu;
        let video = source.path.clone();
        let spawned = std::thread::Builder::new()
            .name("rvp-speech-live".to_owned())
            .spawn(move || run_realtime(&source, recognition, start, &control_end, &events_end));
        if let Err(error) = spawned {
            return Some(UiRequest::Message(format!(
                "cannot start real-time subtitles: {error}"
            )));
        }
        self.speech.realtime = Some(Running {
            control,
            events,
            video,
            title: None,
            gpu,
            progress: 0,
        });
        self.speech.pieces = Pieces::default();
        None
    }

    /// Stops the jobs of the last file (another file starts, or the mode
    /// goes off).
    pub(crate) fn stop_speech(&mut self, file: bool, realtime: bool) {
        if file && let Some(running) = self.speech.file.take() {
            drop(running.control.send(Control::Stop));
        }
        if realtime && let Some(running) = self.speech.realtime.take() {
            drop(running.control.send(Control::Stop));
            self.speech.pieces = Pieces::default();
        }
    }

    /// A file started: the last file's jobs stop, and the automatic modes
    /// start theirs.
    pub(crate) fn speech_file_loaded(&mut self, engine: &Engine) -> Vec<UiRequest> {
        self.stop_speech(true, true);
        let subtitles = engine
            .tracks()
            .iter()
            .any(|track| track.kind == rvp_engine::TrackKind::Subtitle);
        let local = self
            .model
            .loaded_path()
            .is_some_and(|path| !path.contains("://"));
        let speech = &self.settings.speech;
        let file = match speech.auto.as_str() {
            "always" => local,
            "no-subtitles" => local && !subtitles,
            _ => false,
        };
        let realtime = match speech.realtime.as_str() {
            "always" => true,
            "no-subtitles" => !subtitles,
            "streams" => !local,
            _ => false,
        };
        let mut requests = Vec::new();
        if file {
            requests.push(self.speech_start(engine));
        }
        if realtime && !file {
            requests.extend(self.start_realtime(engine));
        }
        requests
    }

    /// Takes the jobs' events (each drain, and the window's tick once a
    /// second while mpv is quiet): progress, results, failures, and gives
    /// real time the position.
    pub fn speech_tick(&mut self, engine: &Engine, now: Instant) -> Vec<UiRequest> {
        let mut requests = Vec::new();
        if let Some(running) = self.speech.file.as_mut() {
            let mut finished = None;
            while let Ok(event) = running.events.try_recv() {
                match event {
                    Event::Progress(percent)
                        if percent >= running.progress.saturating_add(10) || percent == 100 =>
                    {
                        running.progress = percent;
                        requests.push(UiRequest::Osd(format!("Subtitles from audio: {percent} %")));
                    }
                    Event::Progress(_) | Event::Piece(..) => {}
                    Event::Done(document, language) => finished = Some(Ok((document, language))),
                    Event::Failed(error) => finished = Some(Err(error)),
                }
            }
            match finished {
                Some(Ok((document, language))) => {
                    let (video, title) = (running.video.clone(), running.title.clone());
                    self.speech.file = None;
                    requests.push(self.speech_done(
                        engine,
                        &video,
                        title.as_deref(),
                        &document,
                        &language,
                    ));
                }
                Some(Err(error)) => {
                    self.speech.file = None;
                    requests.push(UiRequest::Message(format!("Subtitles from audio: {error}")));
                }
                None => {}
            }
        }
        requests.extend(self.realtime_tick(engine, now));
        requests
    }

    fn realtime_tick(&mut self, engine: &Engine, now: Instant) -> Vec<UiRequest> {
        let mut requests = Vec::new();
        let Some(running) = self.speech.realtime.as_ref() else {
            return requests;
        };
        if self
            .speech
            .position_sent
            .is_none_or(|sent| now.duration_since(sent) >= POSITION_INTERVAL)
            && let Some(time) = self.model.double(rvp_engine::PropertyId::TimePos)
        {
            drop(running.control.send(Control::Position(time)));
            self.speech.position_sent = Some(now);
        }
        let mut arrived = false;
        let mut failed = None;
        while let Ok(event) = running.events.try_recv() {
            match event {
                Event::Piece(index, cues) => {
                    if self.speech.pieces.done.insert(index) {
                        self.speech.pieces.cues.extend(cues);
                        arrived = true;
                    }
                }
                Event::Failed(error) => failed = Some(error),
                Event::Progress(_) | Event::Done(..) => {}
            }
        }
        if let Some(error) = failed {
            self.speech.realtime = None;
            requests.push(UiRequest::Message(format!("Real-time subtitles: {error}")));
        }
        if arrived && let Err(error) = self.show_pieces(engine) {
            requests.push(UiRequest::Message(error));
        }
        requests
    }

    /// Real time's subtitle file: this process's own.
    pub(crate) fn realtime_subtitle_path(&self) -> PathBuf {
        self.state_dir()
            .join("subtitles")
            .join(crate::storage::process_file_name("realtime", "srt"))
    }

    /// Gives mpv real time's subtitles so far as one track.
    fn show_pieces(&mut self, engine: &Engine) -> Result<(), String> {
        let mut document = Document::new(Format::Srt);
        document.cues.clone_from(&self.speech.pieces.cues);
        document.sort();
        let path = self.realtime_subtitle_path();
        crate::storage::write_unflushed(&path, &document.to_bytes(Format::Srt))
            .map_err(|error| format!("Cannot write {}: {error}", path.display()))?;
        let old = self.speech.pieces.track;
        // The track keeps being shown when it was.
        let shown = old.is_some()
            && engine
                .get_text("sid")
                .and_then(|sid| sid.parse::<i64>().ok())
                == old;
        let flag = if shown || (old.is_none() && self.settings.speech.show) {
            "select"
        } else {
            "auto"
        };
        let text = path.to_string_lossy();
        engine
            .command("sub-add", &[&text, flag, "Real-time subtitles (whisper)"])
            .map_err(|error| format!("Cannot show real-time subtitles: {error}"))?;
        let added = crate::subtitle_editor::newest_external(engine);
        if let Some(old) = old.filter(|old| Some(*old) != added) {
            drop(engine.command("sub-remove", &[&old.to_string()]));
        }
        self.speech.pieces.track = added;
        Ok(())
    }

    /// The whole file's subtitles arrived: written next to the video (or
    /// in the state folder) and added.
    fn speech_done(
        &mut self,
        engine: &Engine,
        video: &str,
        title: Option<&str>,
        document: &Document,
        language: &str,
    ) -> UiRequest {
        if document.cues.is_empty() {
            return UiRequest::Osd("Subtitles from audio: no speech found".to_owned());
        }
        let stem = subtitle_stem(video, title);
        let name = format!("{stem}.{language}.srt");
        let beside = Path::new(video)
            .parent()
            .filter(|_| self.settings.speech.beside_video && !video.contains("://"));
        let bytes = document.to_bytes(Format::Srt);
        let mut written = None;
        if let Some(dir) = beside {
            let first = dir.join(&name);
            let target = if first.exists() {
                dir.join(format!("{stem}.{language}.whisper.srt"))
            } else {
                first
            };
            if crate::storage::write_unflushed(&target, &bytes).is_ok() {
                written = Some(target);
            }
        }
        let target = if let Some(target) = written {
            target
        } else {
            let target = self
                .state_dir()
                .join("subtitles")
                .join("generated")
                .join(&name);
            if let Err(error) = crate::storage::write_unflushed(&target, &bytes) {
                return UiRequest::Message(format!("Cannot save {}: {error}", target.display()));
            }
            target
        };
        let lines = document.cues.len();
        if self.model.loaded_path() == Some(video) {
            let flag = if self.settings.speech.show {
                "select"
            } else {
                "auto"
            };
            drop(engine.command(
                "sub-add",
                &[
                    &target.to_string_lossy(),
                    flag,
                    "Subtitles from audio (whisper)",
                    language,
                ],
            ));
            self.refresh_subtitle_files(engine);
        }
        UiRequest::Osd(format!(
            "Subtitles from audio: {lines} lines in {}",
            crate::view::file_name(&target.to_string_lossy())
        ))
    }

    /// The GPU budget's step: jobs from now on run on the CPU, or again
    /// as chosen.
    pub(crate) fn speech_cpu_only(&mut self, cpu: bool) {
        self.speech.cpu_only = cpu;
        if let Some(running) = self.speech.realtime.as_ref() {
            drop(running.control.send(Control::Cpu(cpu)));
        }
    }

    /// A job on the GPU runs now (the GPU budget's step would change it).
    #[must_use]
    pub(crate) fn speech_on_gpu(&self) -> bool {
        !self.speech.cpu_only
            && [&self.speech.file, &self.speech.realtime]
                .into_iter()
                .flatten()
                .any(|running| running.gpu)
    }

    /// The Create Subtitles from Audio menus' commands.
    pub(crate) fn speech_command(
        &mut self,
        engine: &Engine,
        command: SpeechCommand,
    ) -> Option<UiRequest> {
        let speech = &mut self.settings.speech;
        let on_off = |on: bool| if on { "on" } else { "off" };
        let text = match command {
            SpeechCommand::Start => return Some(self.speech_start(engine)),
            SpeechCommand::EngineFolder => {
                let folder = self.engine_folder();
                drop(std::fs::create_dir_all(&folder));
                return Some(UiRequest::OpenExternal(folder));
            }
            SpeechCommand::ModelFolder => {
                let folder = self.model_folder();
                drop(std::fs::create_dir_all(&folder));
                return Some(UiRequest::OpenExternal(folder));
            }
            SpeechCommand::VoiceFilter => {
                speech.voice_filter = !speech.voice_filter;
                format!(
                    "Voice filter for subtitles from audio: {}",
                    on_off(speech.voice_filter)
                )
            }
            SpeechCommand::BesideVideo => {
                speech.beside_video = !speech.beside_video;
                format!(
                    "Subtitles from audio next to the video: {}",
                    on_off(speech.beside_video)
                )
            }
            SpeechCommand::Auto(index) => {
                (*AUTO_MODES.get(usize::from(index))?).clone_into(&mut speech.auto);
                format!("Subtitles from audio when a file starts: {}", speech.auto)
            }
            SpeechCommand::Realtime(index) => {
                (*REALTIME_MODES.get(usize::from(index))?).clone_into(&mut speech.realtime);
                let mode = speech.realtime.clone();
                self.save_settings();
                return Some(self.realtime_changed(engine, &mode));
            }
        };
        self.save_settings();
        Some(UiRequest::Osd(text))
    }

    /// Real time's mode changed: on now for the playing file, or off.
    fn realtime_changed(&mut self, engine: &Engine, mode: &str) -> UiRequest {
        self.stop_speech(false, true);
        if mode == "off" || !self.model.media_loaded() {
            return UiRequest::Osd(format!("Real-time subtitles: {mode}"));
        }
        self.start_realtime(engine)
            .unwrap_or_else(|| UiRequest::Osd(format!("Real-time subtitles: {mode}")))
    }

    /// A choice of the run-time lists.
    pub(crate) fn speech_choice(&mut self, choice: SpeechChoice) -> Option<UiRequest> {
        let speech = &mut self.settings.speech;
        let (recognizer, what) = match choice {
            SpeechChoice::Engine(realtime, _)
            | SpeechChoice::Model(realtime, _)
            | SpeechChoice::Language(realtime, _) => {
                if realtime {
                    (&mut speech.realtime_recognizer, "Real-time subtitles")
                } else {
                    (&mut speech.file, "Subtitles from audio")
                }
            }
        };
        let text = match choice {
            SpeechChoice::Engine(_, index) => {
                let (name, label) = ENGINES.get(index)?;
                (*name).clone_into(&mut recognizer.engine);
                format!("{what}: {label}")
            }
            SpeechChoice::Model(_, index) => {
                (*MODELS.get(index)?).clone_into(&mut recognizer.model);
                format!("{what}: model {}", recognizer.model)
            }
            SpeechChoice::Language(_, index) => {
                let (code, name) = LANGUAGES.get(index)?;
                (*code).clone_into(&mut recognizer.language);
                format!("{what}: {name}")
            }
        };
        self.save_settings();
        Some(UiRequest::Osd(text))
    }

    /// The settings window's view: the folders, what is installed and the
    /// texts of the choices.
    #[must_use]
    pub(crate) fn speech_view(&self) -> serde_json::Value {
        let engines: serde_json::Map<String, serde_json::Value> = ENGINES
            .iter()
            .enumerate()
            .map(|(index, (name, label))| {
                let (_, installed) = self.speech_choice_state(SpeechChoice::Engine(false, index));
                let text = if installed {
                    (*label).to_owned()
                } else {
                    format!("{label} (not installed)")
                };
                ((*name).to_owned(), serde_json::Value::from(text))
            })
            .collect();
        let models: serde_json::Map<String, serde_json::Value> = MODELS
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let (_, present) = self.speech_choice_state(SpeechChoice::Model(false, index));
                let text = if present {
                    (*name).to_owned()
                } else {
                    format!("{name} (not downloaded)")
                };
                ((*name).to_owned(), serde_json::Value::from(text))
            })
            .collect();
        let languages: serde_json::Map<String, serde_json::Value> = LANGUAGES
            .iter()
            .map(|(code, name)| ((*code).to_owned(), serde_json::Value::from(*name)))
            .collect();
        serde_json::json!({
            "engines": engines,
            "models": models,
            "languages": languages,
            "model_folder": self.model_folder().display().to_string(),
            "engine_folder": self.engine_folder().display().to_string(),
            "models_url": MODELS_URL,
            "engines_url": ENGINES_URL,
            "running": self.speech.file.is_some(),
        })
    }

    /// The check marks of the menus' toggles and choices.
    #[must_use]
    pub(crate) fn speech_checked(&self, command: SpeechCommand) -> Option<bool> {
        let speech = &self.settings.speech;
        match command {
            SpeechCommand::Start => Some(self.speech.file.is_some()),
            SpeechCommand::VoiceFilter => Some(speech.voice_filter),
            SpeechCommand::BesideVideo => Some(speech.beside_video),
            SpeechCommand::Auto(index) => {
                Some(AUTO_MODES.get(usize::from(index)) == Some(&speech.auto.as_str()))
            }
            SpeechCommand::Realtime(index) => {
                Some(REALTIME_MODES.get(usize::from(index)) == Some(&speech.realtime.as_str()))
            }
            SpeechCommand::EngineFolder | SpeechCommand::ModelFolder => None,
        }
    }

    /// Whether a list's choice is the one set, and whether it is there
    /// (an engine installed, a model downloaded).
    #[must_use]
    pub fn speech_choice_state(&self, choice: SpeechChoice) -> (bool, bool) {
        let speech = &self.settings.speech;
        let recognizer = |realtime: bool| {
            if realtime {
                &speech.realtime_recognizer
            } else {
                &speech.file
            }
        };
        match choice {
            SpeechChoice::Engine(realtime, index) => {
                let name = ENGINES.get(index).map_or("", |(name, _)| *name);
                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(Path::to_path_buf));
                (
                    recognizer(realtime).engine == name,
                    engine_program(name, Some(&self.settings_folder()), exe_dir.as_deref())
                        .is_some(),
                )
            }
            SpeechChoice::Model(realtime, index) => {
                let name = MODELS.get(index).copied().unwrap_or_default();
                (
                    recognizer(realtime).model == name,
                    model_file(&self.model_folder(), name).is_file(),
                )
            }
            SpeechChoice::Language(realtime, index) => (
                LANGUAGES.get(index).map(|(code, _)| *code)
                    == Some(recognizer(realtime).language.as_str()),
                true,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engines_output_is_read() {
        assert_eq!(
            progress_of("whisper_print_progress_callback: progress =  35%"),
            Some(35)
        );
        assert_eq!(progress_of("whisper_init_from_file: loading"), None);
        assert_eq!(
            detected_language("whisper_full_with_state: auto-detected language: ru (p = 0.97)"),
            Some("ru".to_owned())
        );
    }

    #[test]
    fn subtitles_of_a_stream_get_a_name_every_file_system_takes() {
        let long = format!("a{}", "日".repeat(100));
        let cases: [(&str, Option<&str>, &str); 9] = [
            (
                "https://rr3---sn-4g5e6nsz.googlevideo.com/videoplayback?expire=1763700000\
                 &ei=Xy1a&ip=203.0.113.5&id=o-AB12&itag=22&source=youtube&mime=video%2Fmp4\
                 &sig=AJfQdSswRAIgXq|x:y*z",
                None,
                "videoplayback",
            ),
            (
                "https://media.example.com/talks/My%20Talk%3A%20Part%201.mp4?token=abc#t=30",
                None,
                "My Talk_ Part 1",
            ),
            (
                "https://cdn.example.com/live/master.m3u8?hdnts=exp=1763700000~acl=/*~hmac=beef",
                None,
                "master",
            ),
            // No path: the host and its credentials are no name.
            (
                "https://user:secret@cdn.example.com/?v=1",
                None,
                "subtitles",
            ),
            ("rtmp://user:secret@live.example.com", None, "subtitles"),
            // yt-dlp's entries carry the page's title.
            (
                "edl://%120%https://rr1---sn.googlevideo.com/videoplayback?expire=1&sig=x;",
                Some("  AC/DC: \"Back In Black\" <Live> | 1991?\n"),
                "AC_DC_ _Back In Black_ _Live_ _ 1991_",
            ),
            ("https://example.com/v.mp4", Some("Con"), "_Con"),
            (
                "https://example.com/v.mp4",
                Some("nul.and.void"),
                "_nul.and.void",
            ),
            (
                "https://example.com/v.mp4",
                Some("Console\u{7}"),
                "Console_",
            ),
        ];
        let dir = std::env::temp_dir().join(format!("rvp-speech-names-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("dir");
        for (video, title, wanted) in cases {
            let stem = subtitle_stem(video, title);
            assert_eq!(stem, wanted, "{video}");
            // The name a stream's subtitles are saved under.
            std::fs::write(dir.join(format!("{stem}.en.srt")), b"").expect("a file of that name");
        }
        let cut = subtitle_stem("https://example.com/v.mp4", Some(&long));
        assert!(
            cut.len() <= STREAM_NAME_BYTES && long.starts_with(&cut),
            "{cut}"
        );
        assert_eq!(cut.len(), 118);
        // A local file keeps its own name, for players to find them beside it.
        assert_eq!(
            subtitle_stem("/home/user/Videos/film.part1.mkv", Some("Title")),
            "film.part1"
        );
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn settings_are_checked() {
        let mut settings = SpeechSettings {
            auto: "sometimes".to_owned(),
            ..SpeechSettings::default()
        };
        "huge".clone_into(&mut settings.file.model);
        "klingon".clone_into(&mut settings.realtime_recognizer.language);
        let mut notes = Vec::new();
        settings.validate(&mut notes);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(settings.file.model, "base");
        assert_eq!(settings.realtime_recognizer.language, "auto");
        assert_eq!(settings.auto, "off");
    }

    #[test]
    fn choices_have_menu_ids() {
        for choice in [
            SpeechChoice::Engine(false, 2),
            SpeechChoice::Model(true, 12),
            SpeechChoice::Language(false, 100),
        ] {
            assert_eq!(SpeechChoice::parse(&choice.id()), Some(choice));
        }
        assert_eq!(SpeechChoice::parse("x1"), None);
    }

    #[test]
    fn the_lists_match_potplayers() {
        assert_eq!(LANGUAGES.len(), 101);
        assert_eq!(MODELS.len(), 13);
        let codes: BTreeSet<&str> = LANGUAGES.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes.len(), LANGUAGES.len());
    }
}
