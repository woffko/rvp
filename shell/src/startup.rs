//! "Automatically run at system startup" (M14's General page): Windows'
//! Run key (`cpp/taskbar.cpp`), or an XDG autostart entry elsewhere.

/// Starts RVP with the user's session (`on`) or no longer; the error says
/// what could not be changed.
pub fn apply(on: bool) -> Result<(), String> {
    // An AppImage runs from a mount that changes: its own file starts it.
    let exe = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or("the program's path is unknown")?;
    if cfg!(windows) {
        let command = format!("\"{}\"", exe.display());
        return if crate::ffi::bridge::rvp_run_at_startup(on, &command) {
            Ok(())
        } else {
            Err("cannot change the Run key".to_owned())
        };
    }
    // A relative or empty XDG_CONFIG_HOME is to be ignored (the base
    // directory specification): `./autostart` was written (the Linux
    // audit's L9).
    let folder = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".config"))
        })
        .ok_or("the configuration folder is unknown")?
        .join("autostart");
    let entry = folder.join("rvp.desktop");
    if !on {
        return match std::fs::remove_file(&entry) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("cannot remove {}: {error}", entry.display()))
            }
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let text = format!(
        "[Desktop Entry]\nType=Application\nName=RVP\nExec={}\nIcon=rvp\nX-GNOME-Autostart-enabled=true\n",
        rvp_app::registration::desktop_exec(&exe.to_string_lossy())
    );
    std::fs::write(&entry, text)
        .map_err(|error| format!("cannot write {}: {error}", entry.display()))
}
