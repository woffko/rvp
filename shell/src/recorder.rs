//! Audio recording (goal L, PotPlayer's Audio Recording): what RVP plays,
//! after its whole chain and its volume, captured from Windows' audio
//! engine for RVP's own process only (process loopback, Windows 10 build
//! 20348 or later) and written as a 32-bit float stereo WAV at 48 kHz.
//! Windows converts RVP's stream to that format. On Linux PulseAudio's
//! `parec` (which PipeWire's pulse server answers too) records the monitor
//! of RVP's own playback stream (`rvp_platform::pulse`), found by RVP's
//! process ID and found again when mpv opens a new stream; while RVP plays
//! nothing, nothing is written, as with Windows' loopback.
//!
//! A thread of its own captures; the window's thread only starts and stops
//! it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

/// The recording's format.
const RATE: u32 = 48_000;
const CHANNELS: u16 = 2;

/// A recording under way.
pub(crate) struct Recorder {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<u64, String>>>,
}

impl Recorder {
    /// Starts recording into `path` and returns once the capture runs.
    ///
    /// # Errors
    /// Fails when the file cannot be made or the system refuses the
    /// capture (and on systems RVP does not record on yet).
    pub(crate) fn start(path: PathBuf) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let (started, ready) = std::sync::mpsc::channel();
        let thread = {
            let (path, stop) = (path.clone(), Arc::clone(&stop));
            std::thread::Builder::new()
                .name("rvp-recorder".to_owned())
                .spawn(move || capture(&path, &stop, &started))
                .map_err(|error| format!("recording thread: {error}"))?
        };
        match ready.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self {
                path,
                stop,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                drop(thread.join());
                Err(error)
            }
            Err(_) => {
                stop.store(true, Ordering::Relaxed);
                Err("the capture did not start".to_owned())
            }
        }
    }

    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Stops and finishes the file; returns the seconds recorded.
    ///
    /// # Errors
    /// Fails when the capture or the file failed.
    pub(crate) fn stop(mut self) -> Result<f64, String> {
        self.stop.store(true, Ordering::Relaxed);
        let frames = self
            .thread
            .take()
            .ok_or_else(|| "no capture".to_owned())?
            .join()
            .map_err(|_| "the recording thread failed".to_owned())??;
        // Frame counts are far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        let seconds = frames as f64 / f64::from(RATE);
        Ok(seconds)
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// A WAV file written as it grows; the sizes go in at the end.
struct WavWriter {
    file: std::io::BufWriter<std::fs::File>,
    frames: u64,
}

impl WavWriter {
    fn create(path: &Path) -> std::io::Result<Self> {
        use std::io::Write as _;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
        let block = CHANNELS * 4;
        file.write_all(b"RIFF\0\0\0\0WAVEfmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&3u16.to_le_bytes())?;
        file.write_all(&CHANNELS.to_le_bytes())?;
        file.write_all(&RATE.to_le_bytes())?;
        file.write_all(&(RATE * u32::from(block)).to_le_bytes())?;
        file.write_all(&block.to_le_bytes())?;
        file.write_all(&32u16.to_le_bytes())?;
        file.write_all(b"data\0\0\0\0")?;
        Ok(Self { file, frames: 0 })
    }

    /// Frames as the little-endian bytes Windows lends.
    fn write_bytes(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write as _;
        self.file.write_all(bytes)?;
        self.frames += (bytes.len() / (usize::from(CHANNELS) * 4)) as u64;
        Ok(())
    }

    // Windows' loopback flags silent packets; Linux's stream has none.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn silence(&mut self, frames: u32) -> std::io::Result<()> {
        use std::io::Write as _;
        let zeros = vec![0u8; frames as usize * usize::from(CHANNELS) * 4];
        self.file.write_all(&zeros)?;
        self.frames += u64::from(frames);
        Ok(())
    }

    /// Writes the sizes (a RIFF file ends at 4 GiB).
    fn finish(mut self) -> std::io::Result<u64> {
        use std::io::{Seek as _, SeekFrom, Write as _};
        let data = u32::try_from(self.frames * u64::from(CHANNELS) * 4).unwrap_or(u32::MAX - 36);
        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&(data + 36).to_le_bytes())?;
        self.file.seek(SeekFrom::Start(40))?;
        self.file.write_all(&data.to_le_bytes())?;
        self.file.flush()?;
        Ok(self.frames)
    }
}

#[cfg(target_os = "linux")]
fn capture(
    path: &Path,
    stop: &AtomicBool,
    started: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<u64, String> {
    use rvp_platform::pulse;
    use std::io::Read as _;
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Duration;

    let pid = std::process::id();
    // Before the start counts: the file, and a sound server that answers.
    let opened = WavWriter::create(path)
        .map_err(|error| format!("Audio recording: {error}"))
        .and_then(|wav| pulse::stream_of(pid).map(|_| wav));
    let mut wav = match opened {
        Ok(wav) => wav,
        Err(error) => {
            drop(std::fs::remove_file(path));
            drop(started.send(Err(error.clone())));
            return Err(error);
        }
    };
    drop(started.send(Ok(())));
    let frame = usize::from(pulse::CHANNELS) * 4;
    let mut pending: Vec<u8> = Vec::new();
    let mut failure = None;
    while !stop.load(Ordering::Relaxed) {
        let Some(index) = pulse::stream_of(pid).ok().flatten() else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        };
        let mut child = match pulse::record_stream(index) {
            Ok(child) => child,
            Err(error) => {
                failure = Some(error);
                break;
            }
        };
        let Some(mut output) = child.stdout.take() else {
            drop(child.kill());
            drop(child.wait());
            break;
        };
        // A parec that ended inside a frame left part of it: the next one's
        // bytes start a frame of their own (kept, they shifted every later
        // sample; the Linux audit's M15).
        pending.clear();
        let (sender, chunks) = mpsc::channel::<Vec<u8>>();
        let reader = std::thread::spawn(move || {
            let mut buffer = vec![0u8; 16 * 1024];
            while let Ok(read @ 1..) = output.read(&mut buffer) {
                if sender.send(buffer[..read].to_vec()).is_err() {
                    break;
                }
            }
        });
        // Until the stream ends (mpv opened another) or the stop.
        while !stop.load(Ordering::Relaxed) {
            match chunks.recv_timeout(Duration::from_millis(100)) {
                Ok(chunk) => {
                    pending.extend_from_slice(&chunk);
                    let whole = pending.len() / frame * frame;
                    if let Err(error) = wav.write_bytes(&pending[..whole]) {
                        failure = Some(format!("Audio recording: {error}"));
                        break;
                    }
                    pending.drain(..whole);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(child.kill());
        drop(child.wait());
        drop(reader.join());
        if failure.is_some() {
            break;
        }
    }
    let frames = wav
        .finish()
        .map_err(|error| format!("Audio recording: {error}"))?;
    failure.map_or(Ok(frames), Err)
}

#[cfg(windows)]
fn capture(
    path: &Path,
    stop: &AtomicBool,
    started: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<u64, String> {
    match windows_capture(path, stop, started) {
        Ok(frames) => Ok(frames),
        Err(error) => {
            // The window waits for the start; a later failure ends the file.
            drop(started.send(Err(error.clone())));
            Err(error)
        }
    }
}

// SAFETY: FFI point. `implement` generates the COM object's vtable and
// reference counting for the activation's completion handler; RVP's code
// in it only sends on a channel. Windows calls it once, on its own thread.
// The generated code's raw pointers and inlining are the macro's.
#[cfg(windows)]
#[allow(unsafe_code, clippy::ref_as_ptr, clippy::inline_always)]
mod loopback {
    use std::sync::mpsc::Sender;
    use windows::Win32::Media::Audio::{
        IActivateAudioInterfaceAsyncOperation, IActivateAudioInterfaceCompletionHandler,
        IActivateAudioInterfaceCompletionHandler_Impl,
    };

    /// The completion handler of the activation: it says when it is done.
    #[windows::core::implement(IActivateAudioInterfaceCompletionHandler)]
    pub(super) struct Done(pub(super) Sender<()>);

    impl IActivateAudioInterfaceCompletionHandler_Impl for Done_Impl {
        fn ActivateCompleted(
            &self,
            _operation: windows::core::Ref<IActivateAudioInterfaceAsyncOperation>,
        ) -> windows::core::Result<()> {
            // The capture thread may have given up waiting.
            let _ = self.0.send(());
            Ok(())
        }
    }
}

#[cfg(windows)]
fn failed(what: &str, error: &windows::core::Error) -> String {
    format!("Audio recording: {what}: {error}")
}

/// An audio client on RVP's own process's sound (process loopback).
#[cfg(windows)]
fn activate_loopback() -> Result<windows::Win32::Media::Audio::IAudioClient, String> {
    use windows::Win32::Media::Audio::{
        AUDIOCLIENT_ACTIVATION_PARAMS, AUDIOCLIENT_ACTIVATION_PARAMS_0,
        AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK, AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS,
        ActivateAudioInterfaceAsync, IActivateAudioInterfaceCompletionHandler, IAudioClient,
        PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE, VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
    };
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::{BLOB, CoTaskMemAlloc};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::System::Variant::VT_BLOB;
    use windows::core::Interface as _;

    // SAFETY: FFI point. The activation takes the parameters' blob and
    // frees it (`CoTaskMemFree`) when it is done: a blob on this stack
    // corrupted the heap (Windows 11 build 26340, the test VM). So the blob
    // comes from `CoTaskMemAlloc`, is written whole before the call and is
    // never touched or freed here after it; the PROPVARIANT that points to
    // it is never cleared. The out-parameters are locals.
    #[allow(unsafe_code)]
    unsafe {
        let parameters = AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                    TargetProcessId: GetCurrentProcessId(),
                    ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
                },
            },
        };
        let size = std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>();
        let blob = CoTaskMemAlloc(size).cast::<AUDIOCLIENT_ACTIVATION_PARAMS>();
        if blob.is_null() {
            return Err("Audio recording: out of memory".to_owned());
        }
        blob.write(parameters);
        let mut variant = PROPVARIANT::default();
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_BLOB;
        inner.Anonymous.blob = BLOB {
            cbSize: u32::try_from(size).unwrap_or_default(),
            pBlobData: blob.cast::<u8>(),
        };
        let (done, finished) = std::sync::mpsc::channel();
        let handler: IActivateAudioInterfaceCompletionHandler = loopback::Done(done).into();
        let operation = ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID,
            Some(&raw const variant),
            &handler,
        )
        .map_err(|e| failed("activation", &e))?;
        finished
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "Audio recording: the activation did not finish".to_owned())?;
        let mut status = windows::core::HRESULT(0);
        let mut interface = None;
        operation
            .GetActivateResult(&raw mut status, &raw mut interface)
            .map_err(|e| failed("activation", &e))?;
        status.ok().map_err(|e| failed("activation", &e))?;
        interface
            .ok_or_else(|| "Audio recording: no audio client".to_owned())?
            .cast()
            .map_err(|e| failed("audio client", &e))
    }
}

/// Captures from `client` into `writer` until `stop`; returns the frames.
#[cfg(windows)]
fn capture_loop(
    client: &windows::Win32::Media::Audio::IAudioClient,
    writer: &mut WavWriter,
    stop: &AtomicBool,
    started: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<(), String> {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::Media::Audio::{
        AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK,
        AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioCaptureClient, WAVEFORMATEX,
    };
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

    let block = CHANNELS * 4;
    let format = WAVEFORMATEX {
        wFormatTag: 3,
        nChannels: CHANNELS,
        nSamplesPerSec: RATE,
        nAvgBytesPerSec: RATE * u32::from(block),
        nBlockAlign: block,
        wBitsPerSample: 32,
        cbSize: 0,
    };
    let silent = u32::try_from(AUDCLNT_BUFFERFLAGS_SILENT.0).unwrap_or_default();
    // SAFETY: FFI point. The format and the out-parameters are locals. The
    // buffer `GetBuffer` lends holds `frames` frames of the float stereo
    // format asked for (8 bytes each) and is copied as bytes before
    // `ReleaseBuffer`. The event handle is closed once the capture stopped.
    #[allow(unsafe_code)]
    unsafe {
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK
                    | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                    | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                    | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                2_000_000,
                0,
                &raw const format,
                None,
            )
            .map_err(|e| failed("audio client", &e))?;
        let event = CreateEventW(None, false, false, None).map_err(|e| failed("event", &e))?;
        let result = (|| {
            client
                .SetEventHandle(event)
                .map_err(|e| failed("event", &e))?;
            let capture: IAudioCaptureClient =
                client.GetService().map_err(|e| failed("capture", &e))?;
            client.Start().map_err(|e| failed("start", &e))?;
            drop(started.send(Ok(())));
            while !stop.load(Ordering::Relaxed) {
                if WaitForSingleObject(event, 100) != WAIT_OBJECT_0 {
                    continue;
                }
                while capture
                    .GetNextPacketSize()
                    .map_err(|e| failed("capture", &e))?
                    > 0
                {
                    let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
                    capture
                        .GetBuffer(&raw mut data, &raw mut frames, &raw mut flags, None, None)
                        .map_err(|e| failed("capture", &e))?;
                    let written = if flags & silent != 0 || data.is_null() {
                        writer.silence(frames)
                    } else {
                        writer.write_bytes(std::slice::from_raw_parts(
                            data,
                            frames as usize * usize::from(block),
                        ))
                    };
                    capture
                        .ReleaseBuffer(frames)
                        .map_err(|e| failed("capture", &e))?;
                    written.map_err(|e| format!("Audio recording: {e}"))?;
                }
            }
            drop(client.Stop());
            Ok(())
        })();
        drop(CloseHandle(event));
        result
    }
}

#[cfg(windows)]
fn windows_capture(
    path: &Path,
    stop: &AtomicBool,
    started: &std::sync::mpsc::Sender<Result<(), String>>,
) -> Result<u64, String> {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

    let mut writer = WavWriter::create(path).map_err(|e| format!("Audio recording: {e}"))?;
    // SAFETY: FFI point. The capture thread joins COM's multithreaded
    // apartment for the capture and leaves it after the COM objects, which
    // the closure owns, were dropped.
    #[allow(unsafe_code)]
    let joined = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
    let result =
        activate_loopback().and_then(|client| capture_loop(&client, &mut writer, stop, started));
    if joined {
        // SAFETY: FFI point: balances the join above on the same thread.
        #[allow(unsafe_code)]
        unsafe {
            CoUninitialize();
        }
    }
    result?;
    writer.finish().map_err(|e| format!("Audio recording: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_gets_its_sizes_at_the_end() {
        let dir = std::env::temp_dir().join(format!("rvp-recorder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("take.wav");
        let mut writer = WavWriter::create(&path).expect("create");
        let samples: Vec<u8> = [0.5f32, -0.5, 0.25, -0.25]
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        writer.write_bytes(&samples).expect("write");
        writer.silence(3).expect("silence");
        assert_eq!(writer.finish().expect("finish"), 5);
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(bytes.len(), 44 + 5 * 8);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().expect("4")), 40);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().expect("4")), 76);
        assert_eq!(bytes[44..48], 0.5f32.to_le_bytes());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The capture on the system it runs on (Windows; run by hand with
    /// `--ignored`): it starts and stops, or says why not.
    #[test]
    #[ignore = "uses the system's audio engine"]
    fn records_its_own_sound() {
        let dir = std::env::temp_dir().join(format!("rvp-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("take.wav");
        eprintln!("starting");
        match Recorder::start(path.clone()) {
            Ok(recorder) => {
                eprintln!("started");
                std::thread::sleep(std::time::Duration::from_secs(1));
                eprintln!("stopped: {:?}", recorder.stop());
                let bytes = std::fs::read(&path).expect("the file");
                assert!(bytes.len() >= 44, "{} bytes", bytes.len());
            }
            Err(error) => eprintln!("refused: {error}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
