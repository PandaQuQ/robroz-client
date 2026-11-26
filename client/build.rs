//! Embeds the application icon into the Windows executable, and bakes in the
//! short git SHA used by `--version`.
//!
//! Icon embedding is a no-op for every other target, so Linux and macOS
//! builds are unaffected. The git SHA falls back to `unknown` when git is
//! missing or this is not a checkout (crate and tarball builds).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../assets/brand/icon.ico");
    emit_git_info();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/brand/icon.ico");
        res.set("ProductName", "robroz");
        res.set("FileDescription", "robroz");
        if let Err(e) = res.compile() {
            // Fail loudly: a silent fallback would ship an exe with the default icon.
            panic!("failed to embed the Windows icon: {e}");
        }
    }
}

fn emit_git_info() {
    println!("cargo:rerun-if-env-changed=ROBROZ_GIT_SHA");
    println!("cargo:rerun-if-env-changed=ROBROZ_GIT_DIRTY");
    let workspace = workspace_dir();
    if let Some(git_dir) = git_stdout(&workspace, &["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    let (sha, dirty) = git_info(&workspace);
    println!("cargo:rustc-env=ROBROZ_GIT_SHA={sha}");
    println!("cargo:rustc-env=ROBROZ_GIT_DIRTY={dirty}");
}

fn workspace_dir() -> PathBuf {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    manifest.parent().map(Path::to_path_buf).unwrap_or(manifest)
}

fn git_info(workspace: &Path) -> (String, &'static str) {
    if let Some(sha) = sanitize_sha(&std::env::var("ROBROZ_GIT_SHA").unwrap_or_default()) {
        let dirty = match std::env::var("ROBROZ_GIT_DIRTY") {
            Ok(v) if v == "1" || v.eq_ignore_ascii_case("true") || v == "dirty" => "1",
            _ => "0",
        };
        return (sha, dirty);
    }
    match git_stdout(workspace, &["rev-parse", "--short", "HEAD"]).and_then(|s| sanitize_sha(&s)) {
        Some(sha) => {
            let dirty = if git_is_dirty(workspace) { "1" } else { "0" };
            (sha, dirty)
        }
        None => ("unknown".into(), "0"),
    }
}

/// Keep only a single token of printable ASCII, so a bad override cannot
/// break `cargo:rustc-env`.
fn sanitize_sha(raw: &str) -> Option<String> {
    let sha = raw.trim().split_whitespace().next().unwrap_or("").trim();
    if sha.is_empty() {
        return None;
    }
    if !sha
        .bytes()
        .all(|b| b.is_ascii_graphic() && b != b'(' && b != b')')
    {
        return None;
    }
    Some(sha.to_string())
}

fn git_is_dirty(workspace: &Path) -> bool {
    let out = Command::new("git")
        .args(["status", "--porcelain", "-uno"])
        .current_dir(workspace)
        .stderr(Stdio::null())
        .output()
        .ok();
    match out {
        Some(o) if o.status.success() => !stdout_is_empty(&o.stdout),
        _ => false,
    }
}

fn git_stdout(workspace: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn stdout_is_empty(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| b.is_ascii_whitespace())
}
