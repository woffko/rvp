//! The processors' load for the log's seconds of playback
//! (`rvp_app::log::Second`): between two readings, the share of all
//! processors' time that was busy and the share RVP's process took.
//! Windows gives the times through `GetSystemTimes` and
//! `GetProcessTimes`, Linux through `/proc` (`rvp_platform::load`).

/// The last reading: all processors' busy and whole time, and RVP's.
#[derive(Default)]
pub(crate) struct Load {
    last: Option<[u64; 3]>,
}

impl Load {
    /// The shares since the last call (0 to 1); `None` the first time and
    /// when the system gives no times.
    pub(crate) fn sample(&mut self) -> Option<(f64, f64)> {
        let now = times()?;
        let [busy, total, ours] = self.last.replace(now)?;
        let elapsed = now[1].checked_sub(total).filter(|elapsed| *elapsed > 0)?;
        // Processor times of a second or so: far inside f64's exact range.
        #[allow(clippy::cast_precision_loss)]
        let share =
            |now: u64, then: u64| (now.saturating_sub(then) as f64 / elapsed as f64).min(1.0);
        Some((share(now[0], busy), share(now[2], ours)))
    }
}

#[cfg(target_os = "linux")]
fn times() -> Option<[u64; 3]> {
    rvp_platform::load::times()
}

#[cfg(windows)]
fn times() -> Option<[u64; 3]> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes, GetSystemTimes};
    let value =
        |time: FILETIME| u64::from(time.dwHighDateTime) << 32 | u64::from(time.dwLowDateTime);
    let (mut idle, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: FFI point. GetSystemTimes writes three FILETIMEs through the
    // pointers, which point at the locals above and outlive the call; on
    // failure it returns an error and the reading ends.
    #[allow(unsafe_code)]
    unsafe {
        GetSystemTimes(
            Some(&raw mut idle),
            Some(&raw mut kernel),
            Some(&raw mut user),
        )
    }
    .ok()?;
    let mut ours = [FILETIME::default(); 4];
    let [created, exited, ours_kernel, ours_user] = &mut ours;
    // SAFETY: FFI point. GetCurrentProcess returns a pseudo handle, which
    // needs no closing; GetProcessTimes writes four FILETIMEs through
    // pointers to the array above, which outlives the call; on failure it
    // returns an error and the reading ends.
    #[allow(unsafe_code)]
    unsafe { GetProcessTimes(GetCurrentProcess(), created, exited, ours_kernel, ours_user) }
        .ok()?;
    // The kernel's time includes the idle time.
    let total = value(kernel) + value(user);
    Some([
        total.saturating_sub(value(idle)),
        total,
        value(ours[2]) + value(ours[3]),
    ])
}

#[cfg(not(any(windows, target_os = "linux")))]
fn times() -> Option<[u64; 3]> {
    None
}
