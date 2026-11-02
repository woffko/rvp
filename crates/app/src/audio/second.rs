//! The second audio device (M10, PotPlayer's "Audio Renderer (2nd
//! Device)"): libmpv has one audio output, so a second core without video
//! plays the same file to that device and follows the main core.
//!
//! * It plays files and discs only ([`plays`]): a stream may not open twice.
//! * It mirrors the main core's file, pause, speed, audio track, volume
//!   and mute. When its file has loaded and after the main core seeks
//!   (`PlaybackRestart`), it seeks to the main core's position.
//! * Its chain is the main core's (`App::apply_audio_chain`) without the
//!   normalizer's measurement: ffmpeg's log reaches the first core of a
//!   process only, where those lines would mix with the main core's. The
//!   normalizer's thread sends its gain to both cores
//!   (`normalizer::Control::follow`).
//! * A seek takes the second core a moment, while the main core plays on:
//!   it seeks that much further ([`Phase`]), learns how much from where
//!   each seek lands, and seeks once more when one lands off.
//! * The devices' clocks drift apart. Once a second the two cores'
//!   `audio-pts` (each device's own latency taken off) are compared; the
//!   smoothed offset is corrected with small speed changes, whose pitch
//!   mpv's tempo filter keeps, or with a seek beyond [`RESYNC`].
//!   `audio.second-delay` lines up what the devices add beyond what they
//!   report (a receiver's processing, for one).
//!
//! mpv's wakeup callback is not used for this core: `App::drain` and the
//! shell's tick drain its events, which never blocks. The once-a-second
//! position reads are its only waits for mpv.

use std::time::{Duration, Instant};

use rvp_engine::{
    AudioOutput, EndReason, Engine, EngineEvent, EngineOptions, PlayerModel, PropertyId, Seek,
    VideoOutput,
};

/// Offsets beyond this, in seconds, are corrected with a seek.
pub const RESYNC: f64 = 0.25;
/// How often the offset is measured.
const INTERVAL: Duration = Duration::from_secs(1);
/// How long after its seek the second core's position is measured.
const SETTLE: Duration = Duration::from_millis(300);
/// A seek that has not finished by then is asked for again.
const SEEK_TIMEOUT: Duration = Duration::from_secs(3);
/// A seek that lands further off, in seconds, is done once more.
const FINE: f64 = 0.015;
/// The seek lag assumed before one was measured, and the largest learnt.
const FIRST_LAG: f64 = 0.05;
const MAX_LAG: f64 = 0.5;
/// The speed correction per second of offset.
const GAIN: f64 = 0.5;
/// The largest speed correction (2 %).
const MAX_CORRECTION: f64 = 0.02;
/// Smaller offsets, in seconds, are left alone.
const DEADBAND: f64 = 0.002;
/// How much of a new measurement goes into the smoothed offset.
const SMOOTHING: f64 = 0.5;
/// How long a file that failed waits before it is tried again.
const RETRY: Duration = Duration::from_secs(5);

/// Whether the second core may open `path`: files and discs, not streams.
#[must_use]
pub fn plays(path: &str) -> bool {
    path.split_once("://").is_none_or(|(scheme, _)| {
        ["file", "dvd", "dvdnav", "bd", "bluray", "cdda"]
            .iter()
            .any(|known| scheme.eq_ignore_ascii_case(known))
    })
}

/// What the second core was last told, so that each value is sent once.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Mirrored {
    pause: Option<bool>,
    volume: Option<f64>,
    mute: Option<bool>,
    aid: Option<i64>,
    speed: Option<f64>,
}

/// Where the second core is in following the main core's position.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Its file is not open yet.
    Loading,
    /// It should seek to the main core's position.
    Resync,
    /// Its seek, asked for at `since`, has not finished; `playing` tells
    /// whether the main core played on meanwhile.
    Seeking { since: Instant, playing: bool },
    /// Its seek finished; where it landed is measured at `at`.
    Settling { at: Instant, playing: bool },
    /// It plays in step, kept there by its speed.
    Following,
}

/// The core that plays to the second device.
pub struct Second {
    engine: Engine,
    model: PlayerModel,
    events: Vec<EngineEvent>,
    device: String,
    /// The main core's file it was told to play.
    path: Option<String>,
    phase: Phase,
    /// How long its seeks take, in seconds, as measured.
    lag: f64,
    /// Seeks done again since the main core's last seek.
    again: u8,
    /// The smoothed offset in seconds: positive when it plays ahead.
    offset: Option<f64>,
    /// The speed factor that corrects the drift.
    correction: f64,
    measured: Option<Instant>,
    mirrored: Mirrored,
    chain: Option<String>,
    /// A file that failed is tried again from then.
    retry_at: Option<Instant>,
}

impl Second {
    /// Starts a core for `device` (mpv's name of it); `extra` options come
    /// last (tests give `ao=null`).
    ///
    /// # Errors
    /// Fails when libmpv cannot start the core.
    pub fn start(device: &str, extra: &[(String, String)]) -> Result<Self, String> {
        let mut options = EngineOptions {
            video: VideoOutput::Null,
            audio: AudioOutput::Auto,
            hwdec: "no".to_owned(),
            ipc_server: None,
            extra: [
                ("audio-device", device),
                ("vid", "no"),
                ("sid", "no"),
                ("audio-display", "no"),
                ("pause", "yes"),
            ]
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
            optional: Vec::new(),
            builtin_scripts: false,
        };
        options.extra.extend_from_slice(extra);
        let engine = Engine::new(&options).map_err(|error| error.to_string())?;
        Ok(Self {
            engine,
            model: PlayerModel::new(),
            events: Vec::new(),
            device: device.to_owned(),
            path: None,
            phase: Phase::Loading,
            lag: FIRST_LAG,
            again: 0,
            offset: None,
            correction: 1.0,
            measured: None,
            mirrored: Mirrored::default(),
            chain: None,
            retry_at: None,
        })
    }

    /// The device it plays to.
    #[must_use]
    pub fn device(&self) -> &str {
        &self.device
    }

    /// Its core, for the normalizer's gain.
    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Plays its file now (its file is open and not paused).
    #[must_use]
    pub fn playing(&self) -> bool {
        self.phase != Phase::Loading && self.mirrored.pause == Some(false)
    }

    /// How long its seeks take, as measured (seconds).
    #[must_use]
    pub fn lag(&self) -> f64 {
        self.lag
    }

    /// The smoothed offset from where it should be, in seconds.
    #[must_use]
    pub fn offset(&self) -> Option<f64> {
        self.offset
    }

    /// The speed factor that corrects the drift now.
    #[must_use]
    pub fn correction(&self) -> f64 {
        self.correction
    }

    /// Sets its `af` value when it changed.
    pub fn set_chain(&mut self, chain: &str) {
        if self.chain.as_deref() == Some(chain) {
            return;
        }
        match self.engine.set_text("af", chain) {
            Ok(()) => self.chain = Some(chain.to_owned()),
            Err(error) => crate::log::report(
                crate::log::Level::Warning,
                &format!("the second device's audio filters: {error}"),
            ),
        }
    }

    /// Follows the main core: `model` is its state, `events` what its
    /// last drain brought, `delay` the second device's delay in seconds.
    pub fn follow(
        &mut self,
        main: &Engine,
        model: &PlayerModel,
        events: &[EngineEvent],
        delay: f64,
        now: Instant,
    ) {
        self.drain(now);
        let wanted = model.loaded_path().filter(|path| plays(path));
        if wanted != self.path.as_deref() {
            self.switch(wanted, model, now);
            return;
        }
        if self.phase == Phase::Loading {
            return;
        }
        if events
            .iter()
            .any(|event| matches!(event, EngineEvent::PlaybackRestart))
        {
            self.phase = Phase::Resync;
            self.again = 0;
        }
        let pause = model.flag(PropertyId::Pause).unwrap_or(true);
        self.mirror(model, pause);
        match self.phase {
            Phase::Loading => {}
            Phase::Seeking { since, .. } => {
                // A seek that never finished is asked for again.
                if now.duration_since(since) > SEEK_TIMEOUT {
                    self.phase = Phase::Resync;
                }
            }
            Phase::Resync => {
                if model.flag(PropertyId::Seeking) != Some(true) {
                    self.seek_to_main(main, model, delay, pause, now);
                }
            }
            Phase::Settling { at, playing } => {
                if now >= at {
                    self.settled(main, model, delay, playing && !pause, now);
                }
            }
            Phase::Following => {
                if !pause
                    && self
                        .measured
                        .is_none_or(|measured| now.duration_since(measured) >= INTERVAL)
                {
                    self.measure(main, model, delay, now);
                }
            }
        }
    }

    /// Handles its own events.
    fn drain(&mut self, now: Instant) {
        let mut events = std::mem::take(&mut self.events);
        self.engine.drain(&mut self.model, &mut events);
        for event in events.drain(..) {
            match event {
                EngineEvent::FileLoaded { .. } => {
                    self.phase = Phase::Resync;
                    self.again = 0;
                }
                EngineEvent::PlaybackRestart => {
                    if let Phase::Seeking { playing, .. } = self.phase {
                        self.phase = Phase::Settling {
                            at: now + SETTLE,
                            playing,
                        };
                    }
                }
                EngineEvent::EndFile { reason, .. } => {
                    self.phase = Phase::Loading;
                    if reason == EndReason::Error {
                        // Tried again later, as a new file.
                        self.path = None;
                        self.retry_at = Some(now + RETRY);
                    }
                }
                _ => {}
            }
        }
        self.events = events;
    }

    /// Opens the main core's new file, or stops.
    fn switch(&mut self, wanted: Option<&str>, model: &PlayerModel, now: Instant) {
        self.phase = Phase::Loading;
        self.offset = None;
        self.correction = 1.0;
        self.measured = None;
        match wanted {
            Some(_) if self.retry_at.is_some_and(|at| now < at) => {}
            Some(path) => {
                self.retry_at = None;
                // The track belongs to the file; the rest carries over.
                self.mirrored.aid = None;
                let pause = model.flag(PropertyId::Pause).unwrap_or(true);
                if self.mirrored.pause != Some(pause) && self.engine.set_pause(pause).is_ok() {
                    self.mirrored.pause = Some(pause);
                }
                match self.engine.load(path, false) {
                    Ok(()) => self.path = Some(path.to_owned()),
                    Err(error) => {
                        crate::log::report(
                            crate::log::Level::Warning,
                            &format!("the second device cannot play {path}: {error}"),
                        );
                        self.retry_at = Some(now + RETRY);
                    }
                }
            }
            None => {
                if self.path.take().is_some() {
                    drop(self.engine.stop());
                }
            }
        }
    }

    /// Sends the main core's pause, volume, mute, track and speed where
    /// they changed.
    fn mirror(&mut self, model: &PlayerModel, pause: bool) {
        if self.mirrored.pause != Some(pause) && self.engine.set_pause(pause).is_ok() {
            self.mirrored.pause = Some(pause);
        }
        if let Some(volume) = model.double(PropertyId::Volume)
            && self.mirrored.volume != Some(volume)
            && self.engine.set_volume(volume).is_ok()
        {
            self.mirrored.volume = Some(volume);
        }
        if let Some(mute) = model.flag(PropertyId::Mute)
            && self.mirrored.mute != Some(mute)
            && self.engine.set_mute(mute).is_ok()
        {
            self.mirrored.mute = Some(mute);
        }
        if let Some(aid) = model.int(PropertyId::Aid)
            && self.mirrored.aid != Some(aid)
            && self.engine.set_text("aid", &aid.to_string()).is_ok()
        {
            self.mirrored.aid = Some(aid);
        }
        let speed = model.double(PropertyId::Speed).unwrap_or(1.0) * self.correction;
        if self
            .mirrored
            .speed
            .is_none_or(|sent| (sent - speed).abs() > 1e-6)
            && self.engine.set_double("speed", speed).is_ok()
        {
            self.mirrored.speed = Some(speed);
        }
    }

    /// Seeks to where the main core plays, less the delay; while the main
    /// core plays, as far again as the seek will take.
    fn seek_to_main(
        &mut self,
        main: &Engine,
        model: &PlayerModel,
        delay: f64,
        pause: bool,
        now: Instant,
    ) {
        let Some(position) = main
            .get_double("audio-pts")
            .or_else(|| model.double(PropertyId::TimePos))
        else {
            return;
        };
        self.offset = None;
        self.correction = 1.0;
        self.mirror(model, pause);
        let lead = if pause { 0.0 } else { self.lag };
        if self
            .engine
            .seek(Seek::Absolute((position - delay + lead).max(0.0)))
            .is_ok()
        {
            self.phase = Phase::Seeking {
                since: now,
                playing: !pause,
            };
        }
    }

    /// Where a seek landed: what the next seek leads by, and a second seek
    /// when it landed off.
    fn settled(
        &mut self,
        main: &Engine,
        model: &PlayerModel,
        delay: f64,
        playing: bool,
        now: Instant,
    ) {
        self.measured = Some(now);
        self.phase = Phase::Following;
        let Some(error) = self.error(main, delay) else {
            return;
        };
        if playing {
            // Behind by `error`: the seek took that much longer.
            self.lag = (self.lag - error).clamp(0.0, MAX_LAG);
        }
        if error.abs() > FINE && self.again == 0 {
            self.again = 1;
            self.phase = Phase::Resync;
            return;
        }
        self.offset = Some(error);
        self.correct(model);
    }

    /// Its offset from where it should be now, in seconds.
    fn error(&self, main: &Engine, delay: f64) -> Option<f64> {
        let main_position = main.get_double("audio-pts")?;
        let position = self.engine.get_double("audio-pts")?;
        Some(position - (main_position - delay))
    }

    /// Measures the offset and corrects the speed, or asks for a seek.
    fn measure(&mut self, main: &Engine, model: &PlayerModel, delay: f64, now: Instant) {
        self.measured = Some(now);
        let Some(error) = self.error(main, delay) else {
            return;
        };
        let offset = self
            .offset
            .map_or(error, |offset| offset + SMOOTHING * (error - offset));
        self.offset = Some(offset);
        if offset.abs() > RESYNC {
            self.phase = Phase::Resync;
            self.again = 0;
            return;
        }
        self.correct(model);
    }

    /// Sets the speed that takes the offset away.
    fn correct(&mut self, model: &PlayerModel) {
        let offset = self.offset.unwrap_or(0.0);
        self.correction = if offset.abs() < DEADBAND {
            1.0
        } else {
            1.0 - (offset * GAIN).clamp(-MAX_CORRECTION, MAX_CORRECTION)
        };
        self.mirror(model, false);
    }
}

impl Drop for Second {
    fn drop(&mut self) {
        // The core shuts down when its handle goes; stopping first lets
        // its output close cleanly (goal D rule 6).
        drop(self.engine.stop());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_and_discs_play_twice_streams_do_not() {
        assert!(plays("C:\\Music\\a.flac"));
        assert!(plays("/home/u/a.mkv"));
        assert!(plays("file:///home/u/a.mkv"));
        assert!(plays("dvd://1"));
        assert!(plays("BD://"));
        assert!(!plays("https://example.org/a.mp3"));
        assert!(!plays("rtsp://camera/stream"));
    }
}
