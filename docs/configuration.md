# Configuration

The client is configured with environment variables and a few preference files it saves itself. The host's variables (listen address, token, storage, model, search) are in the host repository's [configuration.md](https://github.com/PandaQuQ/robroz/blob/main/docs/configuration.md).

“Read at” in the tables is `file:function`, as on the `main` branch. An empty or whitespace-only value is treated as unset for most variables; the tables call out the exceptions.

Placeholders such as `<your-token>` and `<host-address>` in the examples are not real secrets. Do not put real tokens in the repository, in a group chat, or in a ticket.

## Client (robroz-client)

| Variable | Read at | Default | Meaning | Example |
| --- | --- | --- | --- | --- |
| `ROBROZ_HOST` | `client/src/conn/config.rs:load` / `Conn::from_env` | `http://127.0.0.1:8790` | Host address for this run; a trailing `/` is stripped. HTTP or HTTPS. Overrides a saved `host` file and is never written back. The host itself does not do TLS; put a remote host behind HTTPS, see [install.md, Connecting to a remote host](install.md#connecting-to-a-remote-host). | `https://robroz.example.com` |
| `ROBROZ_TOKEN` | `client/src/conn/config.rs:load` / `Conn::from_env` | unset | When set, every request, including the live SSE stream, sends `Authorization: Bearer <token>`, with leading and trailing whitespace stripped. Overrides a saved token (OS keychain, or a `token` file next to `lang`, mode `0600` on Unix) and is never written back. The client does not read the host's `ROBROZ_TOKEN_FILE` variable. A missing or wrong token is shown as "The host rejected the token. Check ROBROZ_TOKEN." under the title "Token rejected". A host that cannot be reached is shown as "Can't reach host." plus the reason. The value is never logged or displayed. | `<your-token>` |
| `ROBROZ_GROUPS` | `client/src/source/http.rs:HttpSource::from_env` | unset; the group list comes from `GET /api/groups` | Show only these groups; the list is no longer synced from the host. Comma-separated items, each `id` or `id=name`. | `<groupId>=website,<groupId2>` |
| `ROBROZ_USER_NAME` | `client/src/main.rs:user_name` | the computer's hostname, or `robroz` if that cannot be read | This client's name. Sent as `answeredBy` when it answers an approval or choice card; when a card comes back with the same name, the client shows You (「我」). First non-empty line only, at most 40 characters. Two devices must not share a name. | `study-computer` |
| `ROBROZ_LANG` | `client/src/prefs.rs:initial_locale` | English, or the language last picked with the language setting on the Settings page | UI language for this run: `en` or `zh` (`zh-CN`, `en_GB.UTF-8` and similar also work). Wins over the saved choice and is never written back. Unknown values are ignored. | `zh` |
| `ROBROZ_CONFIG_DIR` | `client/src/prefs.rs:config_dir` | `$XDG_CONFIG_HOME/robroz` or `~/.config/robroz` (Linux), `~/Library/Application Support/robroz` (macOS), `%APPDATA%\robroz` (Windows) | Folder for client preferences. The language setting on the Settings page saves `en` or `zh` in `lang`. The Appearance setting saves `system`, `light` or `dark` in `theme` next to `lang`; no file means System, which follows the OS live. If `ROBROZ_TOKEN` is unset, the client also reads a `token` file in this folder (mode `0600` on Unix). Fake and gallery runs neither read nor write these files. | `/tmp/robroz-client` |
| `ROBROZ_SCALE` | `client/src/main.rs:App::boot` | `1.0` | UI scale factor, a decimal. An unparsable value becomes `1.0`. | `1.25` |
| `ROBROZ_TOUCH` | `client/src/main.rs:App::boot` | off | Set to `1` to start with a larger touch target. If unset, the client still switches automatically after it detects touch input. | `1` |

## Development and tests only

These variables are for screenshots and UI review. You do not need them for everyday use.

| Variable | Read at | Default | Meaning | Example |
| --- | --- | --- | --- | --- |
| `ROBROZ_FAKE` | `client/src/main.rs:wants_fake` | off | Set to `1` to run the client offline on built-in fake data, the same as `--fake`. The `--gallery` flag also turns on fake mode. | `1` |
| `ROBROZ_THEME` | `client/src/prefs.rs:initial_theme` | Light in fake and gallery runs; ignored otherwise | Fake and gallery only (`--fake`, `ROBROZ_FAKE=1`, or `--gallery`). `light` or `dark` (case-insensitive) sets the appearance for that run and is never written back. The saved `theme` file is ignored in these runs. A fake or gallery run with the variable unset is Light, so screenshots do not follow the desktop. In a normal run the Settings Appearance row is used instead (`system`, `light`, or `dark` in `theme` next to `lang`; no file means System). | `dark` |
| `ROBROZ_WINDOW` | `client/src/main.rs:env_size` | `1040x680` | Initial window size, in logical pixels, `widthxheight`. A bad format keeps the default. | `390x844` |
| `ROBROZ_SHOT` | `client/src/main.rs:App::boot`, `App::update` | unset | Fake mode only: after `ROBROZ_SHOT_DELAY` milliseconds, write the window as raw RGBA to this path and exit. This is not a PNG; convert it separately. | `/tmp/shot.rgba` |
| `ROBROZ_SHOT_DELAY` | `client/src/main.rs:App::boot` | `2500` | Milliseconds to wait before the screenshot. | `4000` |
| `ROBROZ_OPEN` | `client/src/main.rs:App::update` | unset | Fake mode only: open this group on startup (the value is a group id), for a chat-page screenshot. | `<groupId>` |
| `ROBROZ_RENAME` | `client/src/main.rs:App::update` | unset | Fake mode only: when the open chat is a DM, open the rename box and fill in this value. | `New name` |
| `ROBROZ_RENAME_SAVE` | `client/src/main.rs:App::update` | off | Used with `ROBROZ_RENAME`; set to `1` to also press Save (「保存」). | `1` |
