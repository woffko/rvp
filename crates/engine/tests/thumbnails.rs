//! The thumbnail core (M11) on the synthetic fixtures: picture sizes, the
//! time a picture takes, cancellation and superseded previews, the cache,
//! durations, and that rendering happens on the core's own thread, never
//! on the caller's (the GUI thread in RVP).
//!
//! `RVP_TESTDATA` points at `testdata/media` when the test runs elsewhere
//! (the Windows VM).

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rvp_engine::thumbnail::{FrameRequest, Group, Outcome, ThumbnailOptions, Thumbnailer};

/// 96x72, 10 fps H.264 with a keyframe every second; 60 s long.
const LONG: &str = "fixture-60s.mkv";
/// 160x90 at 24 fps: four coloured quadrants and a white mark at the top
/// left (`fixture-marker.png` is its first frame).
const MARKER: &str = "fixture-marker.mkv";
/// No such file.
const MISSING: &str = "none.mkv";

fn media(name: &str) -> String {
    let dir = std::env::var_os("RVP_TESTDATA").map_or_else(
        || PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/media")),
        PathBuf::from,
    );
    dir.join(name).to_string_lossy().into_owned()
}

/// A core whose `notify` sends on a channel.
fn start() -> (Thumbnailer, mpsc::Receiver<()>) {
    let (sender, receiver) = mpsc::channel();
    let notify = std::sync::Mutex::new(sender);
    let thumbnailer = Thumbnailer::start(ThumbnailOptions::default(), move || {
        if let Ok(sender) = notify.lock() {
            let _ = sender.send(());
        }
    })
    .expect("the thumbnail thread starts");
    (thumbnailer, receiver)
}

/// Waits for outcomes until `count` arrived or 20 s passed.
fn collect(thumbnailer: &Thumbnailer, wakeups: &mpsc::Receiver<()>, count: usize) -> Vec<Outcome> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut outcomes = Vec::new();
    while outcomes.len() < count {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "timed out: {outcomes:?}");
        let _ = wakeups.recv_timeout(left.min(Duration::from_millis(200)));
        outcomes.extend(thumbnailer.take());
    }
    outcomes
}

fn request(name: &str, time: f64, group: Group) -> FrameRequest {
    FrameRequest {
        path: media(name),
        time,
        max_width: 160,
        max_height: 160,
        fill: false,
        exact: false,
        group,
    }
}

#[test]
fn pictures_take_the_videos_shape_within_the_box() {
    let (thumbnailer, wakeups) = start();
    let ticket = thumbnailer.request(request(LONG, 10.0, Group::List));
    let mut boxed = request(LONG, 10.0, Group::List);
    boxed.max_width = 64;
    boxed.max_height = 64;
    boxed.fill = true;
    let filled = thumbnailer.request(boxed);
    let outcomes = collect(&thumbnailer, &wakeups, 2);
    for outcome in &outcomes {
        let Outcome::Picture {
            ticket: got,
            picture,
            ..
        } = outcome
        else {
            panic!("a picture: {outcome:?}");
        };
        let expected = if *got == ticket {
            // 96x72 (4:3) fitted into 160x160.
            (160, 120)
        } else {
            assert_eq!(*got, filled);
            (64, 64)
        };
        assert_eq!((picture.width, picture.height), expected);
        assert_eq!(
            picture.rgba.len(),
            (picture.width * picture.height * 4) as usize
        );
        assert!(
            picture
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255),
            "opaque"
        );
        // A keyframe at or before the time (one a second).
        assert!(
            picture.shown <= 10.05 && picture.shown > 8.9,
            "shown {}",
            picture.shown
        );
    }
}

#[test]
fn pictures_show_the_frame() {
    let (thumbnailer, wakeups) = start();
    let mut first = request(MARKER, 0.0, Group::Scenes);
    first.max_width = 160;
    first.max_height = 90;
    let _ = thumbnailer.request(first);
    let outcomes = collect(&thumbnailer, &wakeups, 1);
    let Outcome::Picture { picture, .. } = &outcomes[0] else {
        panic!("a picture: {outcomes:?}");
    };
    assert_eq!((picture.width, picture.height), (160, 90));
    let pixel = |x: usize, y: usize| {
        let at = (y * 160 + x) * 4;
        [picture.rgba[at], picture.rgba[at + 1], picture.rgba[at + 2]]
    };
    let near = |got: [u8; 3], want: [u8; 3]| {
        got.iter()
            .zip(want)
            .all(|(got, want)| got.abs_diff(want) < 40)
    };
    // The white mark at the top left, then the four quadrants.
    assert!(near(pixel(16, 10), [255, 255, 255]), "{:?}", pixel(16, 10));
    assert!(
        near(pixel(60, 30), [0xDC, 0x28, 0x28]),
        "{:?}",
        pixel(60, 30)
    );
    assert!(
        near(pixel(120, 20), [0x28, 0xC8, 0x3C]),
        "{:?}",
        pixel(120, 20)
    );
    assert!(
        near(pixel(40, 70), [0x28, 0x46, 0xDC]),
        "{:?}",
        pixel(40, 70)
    );
    assert!(
        near(pixel(120, 70), [0xE6, 0xD2, 0x28]),
        "{:?}",
        pixel(120, 70)
    );
}

#[test]
fn rendering_runs_on_the_cores_thread_and_requests_return_at_once() {
    let (thumbnailer, wakeups) = start();
    let mut slowest = Duration::ZERO;
    for time in [5.0, 25.0, 45.0] {
        let asked = Instant::now();
        let _ = thumbnailer.request(request(LONG, time, Group::Scenes));
        slowest = slowest.max(asked.elapsed());
    }
    // Queueing never waits for mpv.
    assert!(
        slowest < Duration::from_millis(20),
        "request took {slowest:?}"
    );
    let outcomes = collect(&thumbnailer, &wakeups, 3);
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, Outcome::Picture { .. }))
    );
    let stats = thumbnailer.stats();
    assert_eq!(stats.rendered, 3);
    let thread = stats.render_thread.expect("rendered somewhere");
    assert_ne!(
        thread,
        std::thread::current().id(),
        "not the caller's thread"
    );
    let took = stats.last_render.expect("timed");
    println!("metric thumbnail_ms={}", took.as_millis());
    // A keyframe seek and a small picture of a short GOP.
    assert!(took < Duration::from_secs(2), "a picture took {took:?}");
}

#[test]
fn a_newer_preview_replaces_the_waiting_one() {
    let (thumbnailer, wakeups) = start();
    // One preview may be under way at once; the others wait in one slot,
    // which each newer request takes over.
    let tickets: Vec<_> = (0..6)
        .map(|n| thumbnailer.request(request(LONG, f64::from(n) * 9.0, Group::Preview)))
        .collect();
    let newest = *tickets.last().expect("six");
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut delivered = Vec::new();
    while !delivered.contains(&newest) {
        assert!(Instant::now() < deadline, "previews: {delivered:?}");
        let _ = wakeups.recv_timeout(Duration::from_millis(200));
        for outcome in thumbnailer.take() {
            match outcome {
                Outcome::Picture { ticket, .. } => delivered.push(ticket),
                other => panic!("a preview: {other:?}"),
            }
        }
    }
    assert!(delivered.len() < tickets.len(), "older ones were replaced");
    assert_eq!(thumbnailer.waiting(), 0);
    let stats = thumbnailer.stats();
    assert_eq!(
        stats.superseded,
        u64::try_from(tickets.len() - delivered.len()).expect("small")
    );
}

#[test]
fn a_seeks_target_comes_first_and_is_not_cached() {
    let (thumbnailer, wakeups) = start();
    // Pictures queued before the seek wait behind its target (at most the
    // one under way comes first); only the newest seek target counts.
    let list: Vec<_> = (0..8)
        .map(|n| thumbnailer.request(request(LONG, f64::from(n) * 6.0, Group::List)))
        .collect();
    let preview = thumbnailer.request(request(LONG, 50.0, Group::Preview));
    let replaced = thumbnailer.request(exact(request(LONG, 31.0, Group::Seek)));
    let seek = thumbnailer.request(exact(request(LONG, 32.3, Group::Seek)));
    let outcomes = collect(&thumbnailer, &wakeups, list.len() + 2);
    let order: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Outcome::Picture { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect();
    let at = |ticket| order.iter().position(|got| *got == ticket);
    assert_eq!(at(replaced), None, "replaced: {order:?}");
    assert!(at(seek).expect("the target") <= 1, "{order:?}");
    assert!(at(seek) < at(preview), "{order:?}");
    let shown = outcomes.iter().find_map(|outcome| match outcome {
        Outcome::Picture {
            ticket, picture, ..
        } if *ticket == seek => Some(picture.shown),
        _ => None,
    });
    // The exact frame: 10 fps, so within a frame of the time.
    let shown = shown.expect("the target's picture");
    assert!((shown - 32.3).abs() < 0.11, "{shown}");
    assert!(
        thumbnailer
            .cached(&exact(request(LONG, 32.3, Group::Seek)))
            .is_none()
    );
    assert_eq!(thumbnailer.stats().superseded, 1);
    // A stream of seek targets (a drag) takes turns with the preview.
    let first = thumbnailer.request(exact(request(LONG, 12.0, Group::Seek)));
    let preview = thumbnailer.request(request(LONG, 40.0, Group::Preview));
    let second = thumbnailer.request(exact(request(LONG, 14.0, Group::Seek)));
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut turns = Vec::new();
    while !turns.contains(&second) {
        assert!(Instant::now() < deadline, "turns: {turns:?}");
        let _ = wakeups.recv_timeout(Duration::from_millis(200));
        for outcome in thumbnailer.take() {
            if let Outcome::Picture { ticket, .. } = outcome {
                turns.push(ticket);
            }
        }
    }
    // The preview may still come after the second target.
    std::thread::sleep(Duration::from_millis(300));
    for outcome in thumbnailer.take() {
        if let Outcome::Picture { ticket, .. } = outcome {
            turns.push(ticket);
        }
    }
    assert!(
        turns == [first, preview, second] || turns == [second, preview],
        "{turns:?} for {first}, {preview}, {second}"
    );
}

fn exact(request: FrameRequest) -> FrameRequest {
    FrameRequest {
        exact: true,
        ..request
    }
}

#[test]
fn cancelling_drops_a_groups_requests() {
    let (thumbnailer, wakeups) = start();
    let list: Vec<_> = (0..20)
        .map(|n| thumbnailer.request(request(LONG, f64::from(n) * 2.5, Group::List)))
        .collect();
    let scene = thumbnailer.request(request(LONG, 33.0, Group::Scenes));
    thumbnailer.cancel(Group::List);
    let outcomes = collect(&thumbnailer, &wakeups, 1);
    std::thread::sleep(Duration::from_millis(500));
    let mut outcomes = outcomes;
    outcomes.extend(thumbnailer.take());
    let delivered: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Outcome::Picture { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect();
    assert!(delivered.contains(&scene), "the scene picture stays");
    let list_delivered = delivered.iter().filter(|t| list.contains(t)).count();
    // At most the one under way when the group was cancelled.
    assert!(list_delivered <= 1, "list pictures: {delivered:?}");
    assert_eq!(thumbnailer.waiting(), 0);
    assert!(thumbnailer.stats().cancelled >= 19);
}

#[test]
fn cached_pictures_come_without_the_core() {
    let (thumbnailer, wakeups) = start();
    let _ = thumbnailer.request(request(LONG, 20.0, Group::List));
    collect(&thumbnailer, &wakeups, 1);
    let asked = Instant::now();
    let _ = thumbnailer.request(request(LONG, 20.0, Group::List));
    let outcomes = thumbnailer.take();
    assert!(asked.elapsed() < Duration::from_millis(20));
    assert!(matches!(outcomes.as_slice(), [Outcome::Picture { .. }]));
    let stats = thumbnailer.stats();
    assert_eq!((stats.rendered, stats.cache_hits), (1, 1));
    assert!(
        thumbnailer
            .cached(&request(LONG, 20.0, Group::List))
            .is_some()
    );
}

#[test]
fn durations_come_in_the_background() {
    let (thumbnailer, wakeups) = start();
    let _ = thumbnailer.duration(&media(LONG));
    let _ = thumbnailer.duration(&media(MARKER));
    let _ = thumbnailer.duration(&media(MISSING));
    let outcomes = collect(&thumbnailer, &wakeups, 3);
    let duration = |name: &str| {
        let path = media(name);
        outcomes.iter().find_map(|outcome| match outcome {
            Outcome::Duration { path: got, seconds } if *got == path => Some(*seconds),
            _ => None,
        })
    };
    let long = duration(LONG).flatten().expect("the 60 s fixture");
    assert!((long - 60.0).abs() < 0.2, "{long}");
    let marker = duration(MARKER).flatten().expect("the marker");
    assert!((marker - 10.0).abs() < 0.2, "{marker}");
    assert_eq!(duration(MISSING), Some(None));
}

#[test]
fn a_file_without_video_fails_and_the_core_goes_on() {
    let (thumbnailer, wakeups) = start();
    let _ = thumbnailer.request(request(MISSING, 1.0, Group::List));
    let _ = thumbnailer.request(request(LONG, 1.0, Group::List));
    let outcomes = collect(&thumbnailer, &wakeups, 2);
    assert!(
        matches!(outcomes[0], Outcome::Failed { .. }),
        "{outcomes:?}"
    );
    assert!(
        matches!(outcomes[1], Outcome::Picture { .. }),
        "{outcomes:?}"
    );
}
