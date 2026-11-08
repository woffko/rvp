//! Writes frames of a file, put through mpv's video filters, to PNG files
//! (M18: what the smoothing engines make of the same frames), with no
//! window or sound: mpv's `vo=image`, untimed and without frame drops, so
//! every frame the filters make is kept.
//!
//! `cargo run --release -p rvp-engine --example render_vf -- FILE START
//! FRAMES OUT [VF]` writes FRAMES frames from START seconds into OUT
//! (mpv's names, `00000001.png`...). A VapourSynth filter needs
//! `VSSCRIPT_PATH` (RVP's runtime).

use rvp_engine::libmpv2::Mpv;
use rvp_engine::libmpv2::events::Event;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(file), Some(start), Some(frames), Some(out)) =
        (args.first(), args.get(1), args.get(2), args.get(3))
    else {
        eprintln!("usage: render_vf FILE START FRAMES OUT [VF]");
        std::process::exit(2);
    };
    std::fs::create_dir_all(out).expect("output folder");
    let mpv = Mpv::with_initializer(|init| {
        init.set_option("vo", "image")?;
        init.set_option("vo-image-format", "png")?;
        init.set_option("vo-image-outdir", out.as_str())?;
        init.set_option("untimed", "yes")?;
        init.set_option("framedrop", "no")?;
        init.set_option("ao", "null")?;
        init.set_option("aid", "no")?;
        init.set_option("sid", "no")?;
        init.set_option("hwdec", "no")?;
        init.set_option("start", start.as_str())?;
        init.set_option("frames", frames.as_str())?;
        init.set_option("osd-level", "0")?;
        init.set_option("input-default-bindings", "no")?;
        init.set_option("terminal", "no")?;
        for name in ["osc", "ytdl", "load-scripts"] {
            drop(init.set_option(name, "no"));
        }
        if let Some(vf) = args.get(4) {
            init.set_option("vf", vf.as_str())?;
        }
        Ok(())
    })
    .expect("core");
    drop(mpv.request_log_messages("warn"));
    mpv.command("loadfile", &[file.as_str()]).expect("loadfile");
    loop {
        match mpv.wait_event(1.0) {
            Some(Ok(Event::LogMessage { prefix, text, .. })) => {
                eprint!("mpv [{prefix}] {text}");
            }
            Some(Ok(Event::EndFile(_) | Event::Shutdown)) => break,
            _ => {}
        }
    }
    drop(mpv.command("quit", &[]));
}
