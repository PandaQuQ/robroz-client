//! Early command-line flags that exit before the window opens.
//!
//! `--version` / `-V` print the crate version and the short git SHA baked in
//! at build time. `--help` / `-h` print a short usage list. Other flags
//! (`--fake`, `--gallery`) are handled later, as before.

/// Crate version plus the short git SHA from `build.rs`.
///
/// Dirty checkouts append ` dirty` inside the parentheses. Builds without
/// git (crates, tarballs) use `unknown`.
pub fn version_string() -> String {
    format_version(
        env!("CARGO_PKG_VERSION"),
        option_env!("ROBROZ_GIT_SHA").unwrap_or("unknown"),
        option_env!("ROBROZ_GIT_DIRTY") == Some("1"),
    )
}

/// `robroz-client <version> (<sha>[ dirty])`.
pub fn format_version(pkg: &str, sha: &str, dirty: bool) -> String {
    if dirty {
        format!("robroz-client {pkg} ({sha} dirty)")
    } else {
        format!("robroz-client {pkg} ({sha})")
    }
}

const HELP: &str = "\
Usage: robroz-client [OPTIONS]

Options:
  -V, --version    Print version and exit
  -h, --help       Print help
      --fake       Run offline on a canned script
      --gallery    Show every card state for design review
";

enum Early {
    Version,
    Help,
}

/// If `--version`/`-V` or `--help`/`-h` is present, print and exit 0
/// without opening a window. `--help` wins when both are given.
pub fn exit_if_requested() {
    match early_from_args(std::env::args()) {
        Some(Early::Help) => print_and_exit(HELP, 0),
        Some(Early::Version) => print_and_exit(&format!("{}\n", version_string()), 0),
        None => {}
    }
}

fn early_from_args<I, S>(args: I) -> Option<Early>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut version = false;
    let mut help = false;
    for arg in args.into_iter().skip(1) {
        match arg.as_ref() {
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            _ => {}
        }
    }
    if help {
        Some(Early::Help)
    } else if version {
        Some(Early::Version)
    } else {
        None
    }
}

fn print_and_exit(msg: &str, code: i32) -> ! {
    #[cfg(windows)]
    if write_console(msg) {
        std::process::exit(code);
    }
    print!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    std::process::exit(code);
}

/// Attach to the parent console (needed for a Windows GUI-subsystem
/// release exe) and write with `WriteFile`. Returns whether anything
/// was written.
#[cfg(windows)]
fn write_console(msg: &str) -> bool {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5; // (u32)-11
    const INVALID_HANDLE_VALUE: isize = -1;

    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(std_handle: u32) -> *mut core::ffi::c_void;
        fn WriteFile(
            handle: *mut core::ffi::c_void,
            buffer: *const u8,
            length: u32,
            written: *mut u32,
            overlapped: *mut core::ffi::c_void,
        ) -> i32;
    }

    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle as isize == INVALID_HANDLE_VALUE {
            return false;
        }
        let bytes = msg.as_bytes();
        let mut written = 0u32;
        let ok = WriteFile(
            handle,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            core::ptr::null_mut(),
        );
        ok != 0 && written > 0
    }
}

#[cfg(test)]
mod tests {
    use super::{early_from_args, format_version, version_string, Early};

    #[test]
    fn version_string_clean_and_dirty() {
        assert_eq!(
            format_version("0.1.0", "abc1234", false),
            "robroz-client 0.1.0 (abc1234)"
        );
        assert_eq!(
            format_version("0.1.0", "abc1234", true),
            "robroz-client 0.1.0 (abc1234 dirty)"
        );
        assert_eq!(
            format_version("0.1.0", "unknown", false),
            "robroz-client 0.1.0 (unknown)"
        );
    }

    #[test]
    fn baked_version_string_includes_crate_version() {
        let v = version_string();
        let prefix = concat!("robroz-client ", env!("CARGO_PKG_VERSION"), " (");
        assert!(v.starts_with(prefix), "{v}");
        assert!(v.ends_with(')'), "{v}");
        let inner = &v[prefix.len()..v.len() - 1];
        assert!(!inner.is_empty(), "{v}");
        assert!(
            inner == "unknown"
                || inner.ends_with(" dirty")
                || inner.bytes().all(|b| b.is_ascii_hexdigit()),
            "{v}"
        );
    }

    #[test]
    fn version_and_help_flags_are_recognized() {
        assert!(matches!(
            early_from_args(["robroz-client", "--version"]),
            Some(Early::Version)
        ));
        assert!(matches!(
            early_from_args(["robroz-client", "-V"]),
            Some(Early::Version)
        ));
        assert!(matches!(
            early_from_args(["robroz-client", "--help"]),
            Some(Early::Help)
        ));
        assert!(matches!(
            early_from_args(["robroz-client", "-h"]),
            Some(Early::Help)
        ));
        assert!(matches!(
            early_from_args(["robroz-client", "--version", "--help"]),
            Some(Early::Help)
        ));
        assert!(early_from_args(["robroz-client", "--fake"]).is_none());
        assert!(early_from_args(["robroz-client"]).is_none());
    }
}
