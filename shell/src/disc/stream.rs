//! The segments' protocol (`rvpdisc://N`): libmpv's custom streams
//! (`mpv_stream_cb_add_ro`, `stream_cb.h`), declared here as the few C
//! items RVP needs, with the cancel callback libmpv2's own wrapper leaves
//! out. The open disc is process-wide: one window plays one disc.

// SAFETY: FFI points (AGENTS.md's unsafe rule): libmpv's custom stream
// callbacks. Each `unsafe` block below says why its pointer use holds.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Arc, Mutex, PoisonError};

use rvp_engine::Engine;

use super::{PROTOCOL, Segments};

/// libmpv's `MPV_ERROR_LOADING_FAILED` and `MPV_ERROR_UNSUPPORTED`.
const LOADING_FAILED: c_int = -13;
const UNSUPPORTED: i64 = -18;

/// `mpv_stream_cb_info`.
#[repr(C)]
struct StreamCbInfo {
    cookie: *mut c_void,
    read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, u64) -> i64>,
    seek_fn: Option<unsafe extern "C" fn(*mut c_void, i64) -> i64>,
    size_fn: Option<unsafe extern "C" fn(*mut c_void) -> i64>,
    close_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    cancel_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

type OpenFn = unsafe extern "C" fn(*mut c_void, *mut c_char, *mut StreamCbInfo) -> c_int;

unsafe extern "C" {
    fn mpv_stream_cb_add_ro(
        ctx: *mut c_void,
        protocol: *const c_char,
        user_data: *mut c_void,
        open_fn: Option<OpenFn>,
    ) -> c_int;
}

/// The disc the segments read.
static CURRENT: Mutex<Option<Arc<dyn Segments>>> = Mutex::new(None);

pub(super) fn set_current(shared: Option<Arc<dyn Segments>>) {
    *CURRENT.lock().unwrap_or_else(PoisonError::into_inner) = shared;
}

/// Forgets `shared` if it is still the current disc.
pub(super) fn clear_current(shared: &Arc<dyn Segments>) {
    let mut current = CURRENT.lock().unwrap_or_else(PoisonError::into_inner);
    if current
        .as_ref()
        .is_some_and(|c| std::ptr::addr_eq(Arc::as_ptr(c), Arc::as_ptr(shared)))
    {
        *current = None;
    }
}

/// Registers the protocol on a player core (once per core, before a
/// segment loads).
///
/// # Errors
/// Fails when libmpv refuses it.
pub(crate) fn register(engine: &Engine) -> Result<(), String> {
    let protocol = CString::new(PROTOCOL).map_err(|e| e.to_string())?;
    let ctx = engine.mpv().ctx.as_ptr().cast::<c_void>();
    // SAFETY: `ctx` is the engine's live mpv handle; libmpv copies the
    // protocol name; `open` is an `extern "C"` function with the type
    // libmpv calls; no user data is passed.
    let status =
        unsafe { mpv_stream_cb_add_ro(ctx, protocol.as_ptr(), std::ptr::null_mut(), Some(open)) };
    if status < 0 {
        return Err(format!("mpv refused the {PROTOCOL} protocol ({status})"));
    }
    Ok(())
}

/// One segment's cookie: the disc it reads, and which segment.
struct Segment {
    shared: Arc<dyn Segments>,
    number: u64,
}

/// `mpv_stream_cb_open_ro_fn`.
unsafe extern "C" fn open(
    _user_data: *mut c_void,
    uri: *mut c_char,
    info: *mut StreamCbInfo,
) -> c_int {
    let Some(shared) = CURRENT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    else {
        return LOADING_FAILED;
    };
    // SAFETY: libmpv passes the address as a NUL-terminated string that
    // lives through this call.
    let uri = unsafe { CStr::from_ptr(uri) };
    let Some(number) = uri.to_str().ok().and_then(|uri| {
        uri.strip_prefix(PROTOCOL)?
            .strip_prefix("://")?
            .parse()
            .ok()
    }) else {
        return LOADING_FAILED;
    };
    // mpv opens a file after `loadfile` returned: a segment the window
    // moved past by then opens, and ends at once.
    let seekable = shared.bind(number);
    let cookie = Box::into_raw(Box::new(Segment { shared, number })).cast::<c_void>();
    // SAFETY: libmpv passes a valid `info` it owns for this call; the
    // cookie is a leaked box that `close` takes back.
    unsafe {
        (*info).cookie = cookie;
        (*info).read_fn = Some(read);
        (*info).seek_fn = if seekable { Some(seek) } else { None };
        (*info).size_fn = if seekable { Some(size) } else { None };
        (*info).close_fn = Some(close);
        (*info).cancel_fn = Some(cancel);
    }
    0
}

/// The cookie as a segment.
///
/// # Safety
/// `cookie` must be one `open` made and `close` has not taken back.
unsafe fn segment<'a>(cookie: *mut c_void) -> &'a Segment {
    // SAFETY: the caller's contract; libmpv calls read, seek and size only
    // between open and close, from one thread at a time.
    unsafe { &*cookie.cast::<Segment>() }
}

unsafe extern "C" fn read(cookie: *mut c_void, buf: *mut c_char, nbytes: u64) -> i64 {
    let length = usize::try_from(nbytes).unwrap_or(usize::MAX);
    // SAFETY: libmpv passes a buffer of `nbytes` writable bytes and a
    // cookie from `open`.
    let (segment, out) = unsafe {
        (
            segment(cookie),
            std::slice::from_raw_parts_mut(buf.cast::<u8>(), length),
        )
    };
    i64::try_from(segment.shared.read(segment.number, out)).unwrap_or(-1)
}

unsafe extern "C" fn seek(cookie: *mut c_void, offset: i64) -> i64 {
    // SAFETY: a cookie from `open`.
    let segment = unsafe { segment(cookie) };
    u64::try_from(offset)
        .ok()
        .and_then(|offset| segment.shared.seek(segment.number, offset))
        .and_then(|done| i64::try_from(done).ok())
        .unwrap_or(UNSUPPORTED)
}

unsafe extern "C" fn size(cookie: *mut c_void) -> i64 {
    // SAFETY: a cookie from `open`.
    let segment = unsafe { segment(cookie) };
    segment
        .shared
        .size(segment.number)
        .and_then(|size| i64::try_from(size).ok())
        .unwrap_or(UNSUPPORTED)
}

unsafe extern "C" fn close(cookie: *mut c_void) {
    // SAFETY: the box `open` leaked; libmpv calls close once, last.
    drop(unsafe { Box::from_raw(cookie.cast::<Segment>()) });
}

/// A read blocks only on the disc itself: nothing to interrupt.
unsafe extern "C" fn cancel(_cookie: *mut c_void) {}
