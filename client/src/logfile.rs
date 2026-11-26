//! Windows-only file log next to the client's config folder.
//!
//! The release Windows exe uses the GUI subsystem, so stderr is discarded.
//! On Windows this module creates `logs/robroz-client.log` under the same
//! directory as `lang` and `theme`, rotates it at about 1 MiB (keeping one
//! previous file), points stderr at it, and installs a panic hook that
//! writes there too. Other platforms are unchanged.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Rotate when the current log is at least this large.
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

const LOG_DIR: &str = "logs";
const LOG_FILE: &str = "robroz-client.log";
const LOG_PREV: &str = "robroz-client.log.1";

/// `logs/` under the config folder (`%APPDATA%\robroz` on Windows, or
/// `ROBROZ_CONFIG_DIR`).
pub fn log_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(LOG_DIR)
}

/// Full path of the current log file.
pub fn log_path(config_dir: &Path) -> PathBuf {
    log_dir(config_dir).join(LOG_FILE)
}

/// Rename a too-large current log to `robroz-client.log.1`, replacing any
/// previous file, then open the current path for append.
pub fn rotate_and_open(config_dir: &Path) -> io::Result<File> {
    rotate_and_open_with_limit(config_dir, MAX_LOG_BYTES)
}

fn rotate_and_open_with_limit(config_dir: &Path, max_bytes: u64) -> io::Result<File> {
    rotate_in(config_dir, max_bytes)?;
    let dir = log_dir(config_dir);
    std::fs::create_dir_all(&dir)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE))
}

/// If the current log is at least `max_bytes`, move it over the previous
/// file. Missing files are fine.
pub fn rotate_in(config_dir: &Path, max_bytes: u64) -> io::Result<()> {
    let dir = log_dir(config_dir);
    let current = dir.join(LOG_FILE);
    let prev = dir.join(LOG_PREV);
    let len = match std::fs::metadata(&current) {
        Ok(meta) => meta.len(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if len < max_bytes {
        return Ok(());
    }
    std::fs::create_dir_all(&dir)?;
    // Windows cannot rename over an existing file.
    let _ = std::fs::remove_file(&prev);
    std::fs::rename(&current, &prev)?;
    Ok(())
}

/// Redirect stderr into the log file, write a start line, and install a
/// panic hook that appends there. No-op when the config folder is unknown
/// or the file cannot be opened; the app still starts.
#[cfg(windows)]
pub fn install() {
    let Some(dir) = crate::prefs::config_dir() else {
        return;
    };
    let path = log_path(&dir);
    let mut file = match rotate_and_open(&dir) {
        Ok(file) => file,
        Err(_) => return,
    };
    let _ = writeln!(
        file,
        "{} {} started",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        crate::cli::version_string()
    );
    let _ = file.flush();
    if redirect_stderr(&file).is_err() {
        // Hook below still writes to the path directly.
    }
    // Keep the file handle alive so the redirected stderr stays valid.
    std::mem::forget(file);
    install_panic_hook(path);
}

#[cfg(windows)]
fn redirect_stderr(file: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    const STD_ERROR_HANDLE: u32 = 0xFFFFFFF4; // (u32)-12

    #[link(name = "kernel32")]
    extern "system" {
        fn SetStdHandle(std_handle: u32, handle: *mut core::ffi::c_void) -> i32;
    }

    let handle = file.as_raw_handle();
    let ok = unsafe { SetStdHandle(STD_ERROR_HANDLE, handle as *mut core::ffi::c_void) };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn install_panic_hook(path: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let line = format!("{stamp} panic in thread {name}: {info}\n");
        let wrote = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => file.write_all(line.as_bytes()).is_ok() && file.flush().is_ok(),
            Err(_) => false,
        };
        // If the file write failed, fall back to the default hook (stderr,
        // which may already be this same file after a successful redirect).
        if !wrote {
            eprint!("{line}");
            let _ = std::io::stderr().flush();
            previous(info);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "robroz-logfile-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rotates_when_at_least_max_bytes_and_keeps_one_previous() {
        let dir = temp_dir("rotate");
        let logs = log_dir(&dir);
        std::fs::create_dir_all(&logs).unwrap();
        let current = logs.join(LOG_FILE);
        let prev = logs.join(LOG_PREV);
        std::fs::write(&current, vec![b'a'; 64]).unwrap();
        std::fs::write(&prev, b"old-prev").unwrap();

        rotate_in(&dir, 64).unwrap();

        assert!(!current.exists(), "current log should have been renamed");
        assert_eq!(std::fs::read(&prev).unwrap(), vec![b'a'; 64]);

        let mut file = rotate_and_open_with_limit(&dir, 64).unwrap();
        file.write_all(b"fresh\n").unwrap();
        drop(file);
        assert_eq!(std::fs::read_to_string(&current).unwrap(), "fresh\n");
        assert_eq!(std::fs::read(&prev).unwrap(), vec![b'a'; 64]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn does_not_rotate_when_under_the_limit() {
        let dir = temp_dir("keep");
        let logs = log_dir(&dir);
        std::fs::create_dir_all(&logs).unwrap();
        let current = logs.join(LOG_FILE);
        let prev = logs.join(LOG_PREV);
        std::fs::write(&current, b"short").unwrap();

        rotate_in(&dir, 64).unwrap();

        assert_eq!(std::fs::read(&current).unwrap(), b"short");
        assert!(!prev.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotate_is_a_no_op_when_the_log_is_missing() {
        let dir = temp_dir("missing");
        rotate_in(&dir, 64).unwrap();
        assert!(!log_path(&dir).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_creates_logs_dir_and_file_next_to_config() {
        let dir = temp_dir("create");
        let mut file = rotate_and_open(&dir).unwrap();
        file.write_all(b"hello\n").unwrap();
        drop(file);
        assert_eq!(
            std::fs::read_to_string(dir.join("logs").join("robroz-client.log")).unwrap(),
            "hello\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
