//! The property model: the mpv properties RVP observes and their current
//! values, as the skin and the application read them.
//!
//! Every observed property has a [`PropertyId`]; its numeric value is the
//! `reply_userdata` of the observation, so property-change events map back
//! without string comparisons. A drain of the event queue reports which
//! properties changed as a [`Changes`] bit set.

/// mpv value type of an observed property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flag,
    Int,
    Double,
    Text,
}

macro_rules! properties {
    ($($id:ident = $name:literal, $kind:ident;)*) => {
        /// The observed properties. The discriminant is the observation's
        /// `reply_userdata` and the bit in [`Changes`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(u64)]
        pub enum PropertyId {
            $($id,)*
        }

        impl PropertyId {
            /// All observed properties, in declaration order.
            pub const ALL: &'static [PropertyId] = &[$(PropertyId::$id,)*];

            /// The mpv property name.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(PropertyId::$id => $name,)*
                }
            }

            /// The mpv value type it is observed as.
            #[must_use]
            pub const fn kind(self) -> Kind {
                match self {
                    $(PropertyId::$id => Kind::$kind,)*
                }
            }
        }
    };
}

properties! {
    Path = "path", Text;
    MediaTitle = "media-title", Text;
    PlaylistPos = "playlist-pos", Int;
    PlaylistCount = "playlist-count", Int;
    TimePos = "time-pos", Double;
    Duration = "duration", Double;
    Pause = "pause", Flag;
    IdleActive = "idle-active", Flag;
    EofReached = "eof-reached", Flag;
    Seeking = "seeking", Flag;
    Volume = "volume", Double;
    Mute = "mute", Flag;
    FileFormat = "file-format", Text;
    VideoFormat = "video-format", Text;
    VideoOutFormat = "video-params/pixelformat", Text;
    VideoWidth = "video-params/w", Int;
    VideoHeight = "video-params/h", Int;
    VideoBitrate = "video-bitrate", Double;
    ContainerFps = "container-fps", Double;
    EstimatedVfFps = "estimated-vf-fps", Double;
    AudioCodecName = "audio-codec-name", Text;
    AudioChannels = "audio-params/channel-count", Int;
    AudioSampleRate = "audio-params/samplerate", Int;
    AudioBitrate = "audio-bitrate", Double;
    VideoFilters = "vf", Text;
    CurrentAo = "current-ao", Text;
    HwdecCurrent = "hwdec-current", Text;
    SubVisibility = "sub-visibility", Flag;
    FrameDropCount = "frame-drop-count", Int;
    DecoderFrameDropCount = "decoder-frame-drop-count", Int;
    AudioFilters = "af", Text;
    VideoAspect = "video-params/aspect", Double;
}

impl PropertyId {
    /// The property for an observation's `reply_userdata`.
    #[must_use]
    pub fn from_userdata(value: u64) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|index| Self::ALL.get(index).copied())
    }

    const fn bit(self) -> u64 {
        1 << (self as u64)
    }
}

/// The set of properties that changed in one drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changes(u64);

impl Changes {
    pub fn insert(&mut self, id: PropertyId) {
        self.0 |= id.bit();
    }

    #[must_use]
    pub const fn contains(self, id: PropertyId) -> bool {
        self.0 & id.bit() != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True if any of `ids` changed.
    #[must_use]
    pub fn any(self, ids: &[PropertyId]) -> bool {
        ids.iter().any(|&id| self.contains(id))
    }
}

/// A property value as observed; `None` while mpv has no value for it.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Flag(bool),
    Int(i64),
    Double(f64),
    Text(String),
}

/// The current values of all observed properties.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerModel {
    values: Vec<Option<Value>>,
    /// The file between its `FileLoaded` and `EndFile` events.
    loaded_path: Option<String>,
}

impl PlayerModel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: vec![None; PropertyId::ALL.len()],
            loaded_path: None,
        }
    }

    /// The file that is open, from its `FileLoaded` to its `EndFile` event
    /// (unlike the `path` property, it is never ahead of or behind the
    /// events).
    #[must_use]
    pub fn loaded_path(&self) -> Option<&str> {
        self.loaded_path.as_deref()
    }

    /// Replaces the loaded path, returning the previous one. The engine sets
    /// it from its events; previews and tests set it directly.
    pub fn set_loaded_path(&mut self, path: Option<String>) -> Option<String> {
        std::mem::replace(&mut self.loaded_path, path)
    }

    /// Stores a value; returns true if it differs from the previous one.
    pub fn set(&mut self, id: PropertyId, value: Option<Value>) -> bool {
        let slot = &mut self.values[id as usize];
        if *slot == value {
            false
        } else {
            *slot = value;
            true
        }
    }

    #[must_use]
    pub fn value(&self, id: PropertyId) -> Option<&Value> {
        self.values.get(id as usize).and_then(Option::as_ref)
    }

    #[must_use]
    pub fn flag(&self, id: PropertyId) -> Option<bool> {
        match self.value(id) {
            Some(Value::Flag(value)) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn int(&self, id: PropertyId) -> Option<i64> {
        match self.value(id) {
            Some(Value::Int(value)) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn double(&self, id: PropertyId) -> Option<f64> {
        match self.value(id) {
            Some(Value::Double(value)) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn text(&self, id: PropertyId) -> Option<&str> {
        match self.value(id) {
            Some(Value::Text(value)) => Some(value),
            _ => None,
        }
    }

    /// True while a file is loaded, not paused and not stopped at its end
    /// (with `keep-open`, mpv stays on the last frame without setting
    /// `pause`).
    #[must_use]
    pub fn playing(&self) -> bool {
        self.flag(PropertyId::IdleActive) == Some(false)
            && self.flag(PropertyId::Pause) == Some(false)
            && self.flag(PropertyId::EofReached) != Some(true)
    }

    /// True while a file is loaded (playing or paused).
    #[must_use]
    pub fn media_loaded(&self) -> bool {
        self.flag(PropertyId::IdleActive) == Some(false) && self.text(PropertyId::Path).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_userdata() {
        for (index, &id) in PropertyId::ALL.iter().enumerate() {
            assert_eq!(id as usize, index);
            assert_eq!(PropertyId::from_userdata(id as u64), Some(id));
        }
        assert_eq!(
            PropertyId::from_userdata(PropertyId::ALL.len() as u64),
            None
        );
        assert!(PropertyId::ALL.len() <= 64, "Changes is a 64-bit set");
    }

    #[test]
    fn changes_track_ids() {
        let mut changes = Changes::default();
        assert!(changes.is_empty());
        changes.insert(PropertyId::TimePos);
        assert!(changes.contains(PropertyId::TimePos));
        assert!(!changes.contains(PropertyId::Duration));
        assert!(changes.any(&[PropertyId::Duration, PropertyId::TimePos]));
    }

    #[test]
    fn set_reports_differences() {
        let mut model = PlayerModel::new();
        assert!(model.set(PropertyId::Pause, Some(Value::Flag(true))));
        assert!(!model.set(PropertyId::Pause, Some(Value::Flag(true))));
        assert_eq!(model.flag(PropertyId::Pause), Some(true));
        assert!(model.set(PropertyId::Pause, None));
        assert_eq!(model.flag(PropertyId::Pause), None);
    }

    #[test]
    fn playing_needs_a_loaded_unpaused_file() {
        let mut model = PlayerModel::new();
        assert!(!model.playing());
        model.set(PropertyId::IdleActive, Some(Value::Flag(false)));
        model.set(PropertyId::Pause, Some(Value::Flag(false)));
        model.set(PropertyId::Path, Some(Value::Text("a.mkv".into())));
        assert!(model.playing());
        assert!(model.media_loaded());
        model.set(PropertyId::EofReached, Some(Value::Flag(true)));
        assert!(!model.playing());
        model.set(PropertyId::EofReached, Some(Value::Flag(false)));
        model.set(PropertyId::Pause, Some(Value::Flag(true)));
        assert!(!model.playing());
        assert!(model.media_loaded());
    }
}
