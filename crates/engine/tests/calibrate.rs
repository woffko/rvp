//! The calibration core (`rvp_engine::calibrate`): it measures how fast a
//! generated picture goes through filters, and tells a filter that did
//! not run from one that raised the rate.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use rvp_engine::calibrate::{Probe, measure, source};

fn probe(vf: &str) -> Probe {
    Probe {
        width: 320,
        height: 180,
        rate: (24000, 1001),
        vf: vf.to_owned(),
        warm_up: Duration::from_millis(300),
        measure: Duration::from_secs(1),
        extra: Vec::new(),
    }
}

#[test]
fn the_picture_is_generated_at_the_size_and_rate() {
    assert_eq!(
        source(&probe("")),
        "av://lavfi:testsrc2=size=320x180:rate=24000/1001:duration=3600"
    );
}

#[test]
fn a_core_measures_the_filters_rate_and_speed() {
    let cancel = AtomicBool::new(false);
    // No filter: as fast as a measurement goes, at the picture's own rate.
    let plain = measure(&probe(""), &cancel, Duration::from_secs(20)).expect("plain");
    assert!(
        plain.speed > 0.8 * rvp_engine::calibrate::SPEED,
        "{plain:?}"
    );
    assert!((plain.filter_fps - 23.976).abs() < 1.0, "{plain:?}");
    // A filter that makes 60 frames a second out of 23.976.
    let raised = measure(
        &probe("lavfi=[fps=fps=60]"),
        &cancel,
        Duration::from_secs(20),
    )
    .expect("raised");
    assert!(raised.speed > 2.0, "{raised:?}");
    assert!((raised.filter_fps - 60.0).abs() < 2.0, "{raised:?}");
    // A slow filter: far below real time, and still at its rate.
    let slow = measure(
        &probe("lavfi=[fps=fps=60,scale=3840:2160,boxblur=20:5]"),
        &cancel,
        Duration::from_secs(30),
    )
    .expect("slow");
    assert!(slow.speed < 1.0, "{slow:?}");
    assert!((slow.filter_fps - 60.0).abs() < 2.0, "{slow:?}");
    // A filter that cannot be made: mpv plays on without it, at the
    // picture's rate, which is how a failed engine shows.
    let failed = measure(
        &probe("lavfi=[rvp_no_such_filter]"),
        &cancel,
        Duration::from_secs(20),
    );
    if let Ok(failed) = failed {
        assert!((failed.filter_fps - 23.976).abs() < 1.0, "{failed:?}");
    }
}

#[test]
fn a_cancelled_measurement_stops_at_once() {
    let cancel = AtomicBool::new(true);
    let started = std::time::Instant::now();
    assert!(measure(&probe(""), &cancel, Duration::from_secs(20)).is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
}
