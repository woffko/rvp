//! The Recycle Bin (M11's played files and the playlist's Delete files),
//! after the user confirmed in RVP's own dialog: Windows' shell file
//! operation with undo; on Linux the freedesktop.org trash
//! (`rvp_platform::trash`).

use std::path::PathBuf;

/// Sends files to the Recycle Bin.
///
/// # Errors
/// Fails when the shell refuses (an open file, a missing one), or the
/// trash cannot take a file.
pub(crate) fn send(files: &[PathBuf]) -> Result<(), String> {
    if files.is_empty() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        windows_send(files)
    }
    #[cfg(target_os = "linux")]
    {
        let trash = rvp_platform::trash::Trash::for_user()?;
        // The local time as the trash wants it: `yyyy-MM-ddTHH:mm:ss`.
        let clock = crate::ffi::bridge::rvp_local_time();
        let deleted = match clock.split_once(' ') {
            Some((day, time)) => format!("{day}T{}", time.replace('.', ":")),
            None => clock,
        };
        for file in files {
            trash.send(file, &deleted)?;
        }
        Ok(())
    }
}

#[cfg(windows)]
fn windows_send(files: &[PathBuf]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows::Win32::UI::Shell::{
        FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW,
        SHFileOperationW,
    };
    use windows::core::PCWSTR;

    // The paths, each ended by a NUL, the list by another. The shell
    // recycles full paths only (it deletes a relative one for good), and
    // `absolute` (GetFullPathNameW) also turns `/` into `\`.
    let mut list: Vec<u16> = Vec::new();
    for file in files {
        let full = std::path::absolute(file).map_err(|error| error.to_string())?;
        list.extend(full.as_os_str().encode_wide());
        list.push(0);
    }
    list.push(0);
    let flags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI;
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(list.as_ptr()),
        // The flags are a 16-bit field in the structure.
        #[allow(clippy::cast_possible_truncation)]
        fFlags: flags.0 as u16,
        ..Default::default()
    };
    // SAFETY: FFI point. `list` is the double-NUL-ended list the call
    // reads, alive for the call; the structure is ours and filled by
    // value; with FOF_NOERRORUI and FOF_SILENT no window opens. A non-zero
    // result or an abort is reported as an error.
    #[allow(unsafe_code)]
    let result = unsafe { SHFileOperationW(&raw mut operation) };
    if result != 0 {
        return Err(format!(
            "the Recycle Bin refused the files (error {result:#x})"
        ));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("the Recycle Bin left some files".to_owned());
    }
    Ok(())
}
