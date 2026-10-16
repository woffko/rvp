//! The volume in PotPlayer's percent on a real player (headless libmpv, no
//! window, no sound): a volume slider pushed to its end reaches the highest
//! volume, with mpv's `volume-max` from RVP's settings, and says so on the
//! OSD. Three decimals of the exact maximum had made mpv refuse the
//! highest volume itself (the Ubuntu VM's self-test).

use std::time::{Duration, Instant};

use rvp_app::volume::{mpv_volume, percent};
use rvp_app::{App, Settings, UiRequest};
use rvp_engine::{Engine, PropertyId};

#[test]
fn the_volume_slider_reaches_the_highest_volume() {
    let dir = std::env::temp_dir().join(format!("rvp-volume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let settings = Settings {
        reopen_last_file: false,
        remember_positions: false,
        ipc_server: String::new(),
        ..Settings::default()
    };
    assert!((settings.volume_max - 200.0).abs() < f64::EPSILON);
    let mut app = App::new(settings, &dir).expect("app");
    let mut options = app.engine_options();
    options.extra.push(("ao".to_owned(), "null".to_owned()));
    options.extra.push(("vo".to_owned(), "null".to_owned()));
    let engine = Engine::new(&options).expect("libmpv starts");

    for (fraction, shown) in [(1.0, 200.0), (0.5, 100.0), (0.11, 22.0)] {
        let request = app.set_volume_fraction(&engine, fraction);
        assert_eq!(
            request,
            Some(UiRequest::Osd(format!("Volume: {shown:.0} %"))),
            "slider at {fraction}"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while (app.volume_percent() - shown).abs() > 0.01 {
            assert!(
                Instant::now() < deadline,
                "slider at {fraction}: mpv's volume {:?}, shown {}",
                app.model.double(PropertyId::Volume),
                app.volume_percent()
            );
            std::thread::sleep(Duration::from_millis(10));
            app.drain(&engine, Instant::now());
        }
        let volume = app.model.double(PropertyId::Volume).expect("a volume");
        assert!((volume - mpv_volume(shown)).abs() < 1e-3, "{volume}");
        assert!((percent(volume) - shown).abs() < 0.01);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
