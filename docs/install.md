# Install and first run

How to install the robroz client from a GitHub Release or build it from source, and connect it to a host. Starting the host, connecting a model and the first task over `curl` are in the host repository's [install.md](https://github.com/PandaQuQ/robroz/blob/main/docs/install.md). The client's environment variables are in [configuration.md](configuration.md).

The desktop client UI is English by default. Switch to 中文 under Settings, General, Language / 语言. Open Settings from your name at the bottom of the chat list. `ROBROZ_LANG=en|zh` overrides the saved choice for one run and is not written back. Quoted UI text below is what the English UI shows; the Chinese is in parentheses where it helps. See [configuration.md](configuration.md) for the saved `lang` file.

## Download a release

When a `v*` tag is published, download the archives and `SHA256SUMS.txt` from the GitHub Release (v0.1.1 and earlier: [PandaQuQ/robroz releases](https://github.com/PandaQuQ/robroz/releases); later client releases: [PandaQuQ/robroz-client releases](https://github.com/PandaQuQ/robroz-client/releases)) for that tag. The current release is v0.1.1, published on 2026-11-25 as a pre-release. The examples below use v0.1.1; replace it with the tag you download. Archives are named `<bin>-<tag>-<target>`.

| Platform | Archive | Contents |
| --- | --- | --- |
| Linux x86_64 | `robroz-client-v0.1.1-linux-x86_64.tar.gz` | `robroz-client`, `robroz.desktop`, `robroz.png` |
| Windows x86_64 | `robroz-client-v0.1.1-windows-x86_64.zip` | `robroz-client.exe` |
| macOS aarch64 | `robroz-client-v0.1.1-macos-aarch64.zip` | `robroz.app` |

The macOS client is Apple Silicon only; Intel Macs are not covered. Each archive also includes `LICENSE` and `README.md`. `robroz.app` includes `icon.icns`. After unpacking, the binary or `robroz.app` is inside a folder named after the archive. Builds are not code-signed. The Linux binaries are built on Ubuntu 22.04. The client needs glibc 2.35 or newer (Ubuntu 22.04+, Debian 12+). It does not run on RHEL 9 (glibc 2.34); build it from source there, see [Build](#build). Check your version with `ldd --version`.

Verify the files you download against `SHA256SUMS.txt` from the same release:

```sh
# Linux (only the files you downloaded need to be present)
sha256sum -c SHA256SUMS.txt --ignore-missing

# macOS: print the hash and compare it with the matching line in SHA256SUMS.txt
shasum -a 256 robroz-client-v0.1.1-macos-aarch64.zip
```

On Windows, compare `Get-FileHash -Algorithm SHA256 .\robroz-client-v0.1.1-windows-x86_64.zip` with the matching line in `SHA256SUMS.txt`.

The host binary, the runner image and the host's Docker image are released from the host repository; see its [install.md](https://github.com/PandaQuQ/robroz/blob/main/docs/install.md).

### Linux desktop entry

The Linux client runs straight from the unpacked folder. To get a launcher entry with an icon for your user, copy the binary, the desktop file and the icon into the standard per-user locations:

```sh
cd robroz-client-v0.1.1-linux-x86_64
install -Dm755 robroz-client ~/.local/bin/robroz-client
install -Dm644 robroz.desktop ~/.local/share/applications/robroz.desktop
install -Dm644 robroz.png ~/.local/share/icons/hicolor/512x512/apps/robroz.png
```

`robroz.desktop` runs `robroz-client` from your `PATH`, so `~/.local/bin` must be on it. If the entry does not appear, log out and back in, or run `update-desktop-database ~/.local/share/applications`.

### Unsigned builds

**macOS.** Gatekeeper warns the first time you open an unsigned `robroz.app`. To allow it: right-click the app, choose Open, and confirm; or go to System Settings > Privacy & Security and click Open Anyway. You can also clear the quarantine flag after unzipping:

```sh
xattr -dr com.apple.quarantine robroz-client-v0.1.1-macos-aarch64/robroz.app
```

**Windows.** SmartScreen may show "Windows protected your PC". Click More info, then Run anyway.

v0.1.0 also opens an extra command-line window next to the client; closing it quits the client. v0.1.1 and later do not.

## Prerequisites

| Need | Used for | Notes |
| --- | --- | --- |
| Rust stable toolchain (`rustup`, `cargo`) | Building the client | There is no `rust-toolchain` file. CI uses `dtolnay/rust-toolchain@stable`, so follow current stable. |
| A running `robroz-host` | Everything except `--fake` | See the host repository's [install.md](https://github.com/PandaQuQ/robroz/blob/main/docs/install.md). |

The client builds on Windows, Linux and macOS. CI builds it on Linux for pull requests, and also on Windows and macOS after a merge to `main` and on a nightly schedule. The mobile client has its own steps in [mobile/README.md](../mobile/README.md).

## Build

From the repository root:

```sh
cargo build -p robroz-client    # desktop client: target/debug/robroz-client
```

Add `--release` for a release build; the binary then lands in `target/release/`. The first build also fetches `robroz-contract` from the host repository at the git tag pinned in `Cargo.toml`.

## Run the client

The client is a native iced desktop app. A downloaded binary, `robroz.app`, or `cargo run -p robroz-client` all work the same way. It speaks HTTP or HTTPS (rustls) and connects to `http://127.0.0.1:8790` by default.

Fill in Host and Token on the offline pane or in Settings, under Connection (「连接」), then click Connect (「连接」). The address is saved in a `host` file next to `lang` in the client config folder. The token is stored in the OS keyring (macOS Keychain, Windows Credential Manager, Secret Service on Linux), falling back to a `token` file next to `lang` with mode `0600` on Unix. The client never logs or displays the token. A saved token is shown as `••••••••`.

`ROBROZ_HOST` and `ROBROZ_TOKEN` override saved values for this run and are never written back. A field set by the environment is read-only and shows Set by ROBROZ_HOST (「由 ROBROZ_HOST 设置」) or Set by ROBROZ_TOKEN (「由 ROBROZ_TOKEN 设置」). When both are set by the environment, Connect is disabled. The client reads `ROBROZ_TOKEN` only, not the host's `ROBROZ_TOKEN_FILE`. A non-local `http://` host shows Unencrypted. Use HTTPS for remote hosts. (「未加密，远程请用 HTTPS。」)

A 401 is titled Token rejected (「令牌被拒绝」). The reason is The host rejected the token. Check ROBROZ_TOKEN. (「宿主拒绝了令牌，请检查 ROBROZ_TOKEN。」). An unreachable host is titled Can't reach the host (「连不上宿主」); the reason line is Can't reach host. (「连不上宿主。」) followed by the detail. If the host has never answered, the form stays on that pane. After the host has answered once, a later failure keeps what is already on screen and shows the error in the bottom bar.

Linux / macOS:

```sh
cargo run -p robroz-client
# Host elsewhere, or a token is set
ROBROZ_HOST=http://<host-address>:8790 ROBROZ_TOKEN='<your-token>' cargo run -p robroz-client
```

Windows (PowerShell):

```powershell
$env:ROBROZ_HOST = "http://<host-address>:8790"
$env:ROBROZ_TOKEN = "<your-token>"
cargo run -p robroz-client
```

- The group list comes from `GET /api/groups`. If `ROBROZ_GROUPS` is set, the client shows only those groups and does not follow the host's list.
- To look at the UI without a host: `cargo run -p robroz-client -- --fake` (or `ROBROZ_FAKE=1`).

Linux needs a graphical session (X11 or Wayland). CI's `ubuntu-latest` can build the client without extra system packages.

In the client, click the + at the top of the chat list (Add a bro, 「引入一个 bro」), name the bro and press Add (「引入」), open the DM and send a message. That is a first task from the UI.

## Connecting to a remote host

The host does not terminate TLS. To reach it from another machine, put HTTPS in front of it and set a token; the host side (Cloudflare Tunnel, Caddy, Docker) is in the host repository's [install.md](https://github.com/PandaQuQ/robroz/blob/main/docs/install.md#connecting-to-a-remote-host). The client speaks HTTPS (rustls). Point it at the proxy address, not at the host's loopback port.

The simplest way is the connection form: enter the address and the token under Host and Token, on the offline pane or in Settings, and click Connect.

You can save the address and the token in the client. The address is written to a `host` file next to `lang` in the client config folder (`~/.config/robroz/host` on Linux, `~/Library/Application Support/robroz/host` on macOS, `%APPDATA%\robroz\host` on Windows). The token is stored in the OS keychain (`robroz` / `host-token`: macOS Keychain, Windows Credential Manager, or the Secret Service on Linux); if the keychain is unavailable or the write cannot be read back, it is written to a `token` file next to `lang`, mode `0600` on Unix. The client never logs or displays the token.

`ROBROZ_HOST` and `ROBROZ_TOKEN` override any saved values for this run and are never written back. The client does not read the host's `ROBROZ_TOKEN_FILE` variable.

```sh
ROBROZ_HOST=https://robroz.example.com ROBROZ_TOKEN='<your-token>' cargo run -p robroz-client
```

To seed the fallback file by hand (only needed if you are not saving from the client, and the keychain is not used):

```sh
mkdir -p "$HOME/.config/robroz"
(umask 077 && printf '%s\n' '<your-token>' > "$HOME/.config/robroz/token")
printf '%s\n' 'https://robroz.example.com' > "$HOME/.config/robroz/host"
cargo run -p robroz-client
```

If the host and the client share a machine, the same `$HOME/.config/robroz/token` file can be the host's `ROBROZ_TOKEN_FILE` and the client's fallback file. A missing or wrong token shows "The host rejected the token. Check ROBROZ_TOKEN." (「宿主拒绝了令牌，请检查 ROBROZ_TOKEN。」). A host that cannot be reached shows "Can't reach host." (「连不上宿主。」), followed by the reason.

## Tests

```sh
cargo test -p robroz-client -p robroz-card -p robroz-ui -p robroz-tokens
```

The client's `against_host` tests build the host in-process from the pinned tag and talk to it over HTTP, so they need no Docker and no running host.
