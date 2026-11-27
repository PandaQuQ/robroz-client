# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Home list, floating chat top bar, group menu and group profile: the home list has a `+` on top, 64px rows (group avatars stacked like the title capsule, no empty line under a name without preview) with an 8px unread dot, a floating search capsule and a self-row (Settings, and Sign out on this device, which deletes the token stored on this device only). The chat top bar is floating, with a back button that carries the unread count and a ••• menu that opens 4px under the button. View profile, Find in chat, Rename, Members and Delete group talk to the contract-v0.3.0 group APIs from robroz#195. Rename, Members and Delete group are shown together, and only when the chat is not a DM and the user is an admin or created the group; otherwise the entries are omitted. Delete confirms with: "Messages, tasks and routines are deleted, and running tasks are stopped. Bros are kept." / 「聊天记录、任务和例行任务会删掉，正在跑的任务会停掉。bro 不会删。」 `HostNotGroupCreator` and `HostGroupIsDm` use the already-approved copy. iced has no backdrop-blur, so floating buttons use a translucent surface fill.

- Host Docker image and compose example ([#170](https://github.com/PandaQuQ/robroz/pull/170)): `host/Dockerfile` (`debian:bookworm-slim`, Docker CLI, no Node), `deploy/compose.host.yml`, and [docs/docker-host.md](https://github.com/PandaQuQ/robroz/blob/main/docs/docker-host.md). The host binary release is unchanged.
- Release builds push `ghcr.io/pandaquq/robroz-host:<tag>` next to the runner image ([#171](https://github.com/PandaQuQ/robroz/pull/171)): linux/amd64 only, version tags only, no `latest` tag.
- Bro owners and owner-only approvals (v0.2 accounts, first part): a `users` table with a bootstrap admin (the env token and no-token loopback act as it), `GET /api/me`, `Bro.ownerUserId`, one DM per (user, bro), `ownerUserId` / `ownerName` on approval cards, and `authorUserId` / `authorName` on `message` events. Approval cards stay in the group they were raised in; the owner's DM gets a new `approval_reminder` event. See [docs/permissions.md](https://github.com/PandaQuQ/robroz/blob/main/docs/permissions.md).

- Stable machine codes for what the host used to send only as Chinese text, so the host no longer depends on `robroz-card` and that crate can move to the client repo. Additive: no field is removed or changed. `status` events get `statusCode` (`queued` / `running` / `blocked` / `done` / `no_result`), `failureCode` and `failureDetail` (the raw reason without a prefix, so the client adds exactly one 「没做成：」); `delivery` events get `badge` (`briefing`), `failureCode` and `failureDetail`. Every 4xx/5xx JSON error body has a `code` (`robroz_contract::error_codes`); `error` and `message` are for logs only. The legacy Chinese card copy moved to `robroz_contract::legacy` (deprecated). See [docs/api.md](https://github.com/PandaQuQ/robroz/blob/main/docs/api.md#error-codes-v02).

### Changed

- Settings page: grouped cards (radius 16, surface fill, 24px between groups, 12px text3 group titles 8px above the card), 44px rows with 1px border dividers and 16px side padding, the current value in text2 followed by `›`. Groups: General (Language, Appearance), Bros (Model, Search, MCP, Skills, Memories, Routines, Review rules) and Connection (host address). Language and Appearance open a list with a tick on the current choice; Connection opens the existing host form. Sign out on this device sits in its own red card (44px, text centred), with the version in 12px text3 centred under it and 24px (plus a safe-area inset on a phone) below. Mobile gets the same General group, sign-out card and version line. Every settings sub-page, including on a wide desktop, uses the round back button and a 20px title, at most 720px wide and centred. The Allow rules page has no outer card; its input matches the Connection fields. The Model page now shows only the model; Language, Appearance and Connection moved to Settings.

- MCP page: Connect turns a server on and tests it (a stdio server whose command is not yet approved opens the approval panel); Disconnect is `PATCH` `{"enabled":false}` in the expanded row, with Edit and an outline Delete in `danger` text. Connected is muted text with a 6px `status.done` green dot, not a greyed-out button. Groups that use the server are listed one per row with the switch on the right. Transport capsules (All / stdio / HTTP) show when both transports are present; the icon is the name's first letter, because `contract-v0.3.0` has no category or icon field (real categories wait for v0.3). The "{0} connected" count is 13px muted text, not a capsule. The page uses the same 720px centred chrome, round back button and 20px title as other Settings sub-pages. On a phone there is no Add, Edit or Disconnect; the page opens from a Settings row when MCP is available.

- Chat process line: one collapsed line per round with a wall-clock timer. A local timer starts when this session sees a transition into a live run and is exact. Otherwise the fallback is always marked approximate: a live card uses `now − createdAt`, a finished card uses `updatedAt − createdAt`. After a follow-up re-queue, `createdAt` is not reset, so from the second round on the fallback is cumulative. Blocked and queued cards show no timer; a finished card shows the total. A "working" row lists the live bros above the composer. The SSE stream reconnects after 45s with no bytes; the reconnecting banner appears only after more than 30s of continuous failure. A host max-age close reconnects immediately and does not count as a failure. `Last-Event-ID` is sent back verbatim. The contract crate stays on `contract-v0.3.0`. Host `startedAt` is not read in this change; that moves to the small PR after `contract-v0.3.1`.

- Client on `contract-v0.3.0`: failure, progress and approval-reason lines are drawn from `failureCode` + `failureDetail`, `progressCode` + `progressParams` and `reasonCode` with the client's own zh/en copy; host text is never shown. Raw details such as `response_not_json` sit on a monospace detail line under the failure (the old "Show details" toggle is gone). Unknown codes show 「出错了」/ "Something went wrong" (progress: 「进行中」/ "In progress") and are logged.
- `POST /api/approvals/{id}/approve` and `/deny` return 403 `{"code":"approval_forbidden","reason":"not_owner"|"decided"|"expired"}` instead of 409 for an answered or expired card, and for a caller who is not the bro's owner. Renaming a bro, opening its DM or adding it to a group by someone else is 403 `not_bro_owner`. `GET /api/bros` lists the caller's bros. The host no longer writes 「已发到主人私聊」.

### Fixed

- Approve / Reject follow who may answer on the host (`may_answer`): rule and settings cards on the review-rules page show the buttons only to an admin from `/api/me`, and anyone else sees "Waiting for an admin" / 「等管理员确认」 and the rule text, with no "Confirm to add this allow rule" / 「确认后加入放行规则」 line and no buttons; a bro's approval card in a DM shows the buttons only to the bro's owner. If `/api/me` has not loaded, no buttons are shown. No contract change.
- Approval, process and find-in-chat follow-ups from the PR4 review: a non-owner sees "Waiting for this bro's owner" / 「等这个 bro 的主人审批」 (not the owner's account name); a duration under 1s reads "Worked for under 1s" / 「用了不到 1 秒」 (live: "Working under 1s" / 「处理中不到 1 秒」); a task blocked on an approval keeps the `status.blocked` yellow dot; find-in-chat uses the same search capsule as the home list. A truncated sidebar name shows the full name on hover. The sidebar stays 220px.

## [0.1.1] - 2026-11-25

Published as a pre-release.

### Changed

- Release notes are written in English and Chinese, from `.github/release-notes/<tag>.md` or a template, and `v0.*` releases are marked as pre-releases ([#166](https://github.com/PandaQuQ/robroz/pull/166)).

### Fixed

- The Linux client's X11 window icon (`_NET_WM_ICON`) is now 128×128 instead of 512×512, so window managers and taskbars that use it show the robroz icon instead of a generic one. The 512×512 property was about 1 MiB, larger than many X11 clients read ([#167](https://github.com/PandaQuQ/robroz/pull/167)).
- The Windows client no longer opens an extra command-line window on start ([#165](https://github.com/PandaQuQ/robroz/pull/165)). Release builds use the Windows GUI subsystem. Debug builds keep the console. When the client is started from Explorer, stderr output such as a failed preference save or a panic message is not shown; start it from a shell to see it.

## [0.1.0] - 2026-11-25

Published as a pre-release. First release of robroz: a single-user, self-hosted multi-agent workspace with a Rust host and a native desktop client.

### Added

- Window icon for the desktop client, and a Windows exe icon ([#150](https://github.com/PandaQuQ/robroz/pull/150)).
- App icons in release packages ([#161](https://github.com/PandaQuQ/robroz/pull/161)): `robroz.app` ships `icon.icns` (`CFBundleIconFile`); the Linux client tarball ships `robroz.desktop` and `robroz.png`. The same PR adds a `workflow_dispatch` dry run: it builds and packages the binaries, and builds the runner image without pushing it or logging in to GHCR. It does not create a release.
- Release packaging when a `v*` tag is pushed ([#149](https://github.com/PandaQuQ/robroz/pull/149)): draft GitHub Release assets, unsigned, with `SHA256SUMS.txt`. Client archives are Linux x86_64 (`.tar.gz`), Windows x86_64 (`.zip` with the `.exe`), and macOS aarch64 only (`.zip` containing `robroz.app`). The host ships for Linux x86_64 only. Intel Macs are not covered.
- Runner image `ghcr.io/pandaquq/robroz-runner:<tag>` on `v*` tags ([#151](https://github.com/PandaQuQ/robroz/pull/151)): `linux/amd64` only, tagged with the git tag, no `latest` tag. Example: `ROBROZ_IMAGE=ghcr.io/pandaquq/robroz-runner:v0.1.0`.
- Manual platform screenshot workflow for Windows and macOS ([#153](https://github.com/PandaQuQ/robroz/pull/153), [#154](https://github.com/PandaQuQ/robroz/pull/154)).
- Client HTTPS via rustls, and persisted connection settings ([#152](https://github.com/PandaQuQ/robroz/pull/152)). The host address is stored in a `host` file next to `lang`. The token is stored in the OS keyring (macOS Keychain, Windows Credential Manager, Secret Service on Linux), falling back to a `token` file next to `lang` with mode `0600` on Unix. `ROBROZ_HOST` and `ROBROZ_TOKEN` override saved values for the current run and are never written back.
- In-app connection form with Host, Token, and Connect, on the offline pane and in Settings ([#159](https://github.com/PandaQuQ/robroz/pull/159)). A field set by the environment is read-only (`Set by ROBROZ_HOST` / `Set by ROBROZ_TOKEN`). When both are set by the environment, Connect is disabled. A non-local `http://` host shows `Unencrypted. Use HTTPS for remote hosts.` A 401 is titled `Token rejected` (「令牌被拒绝」), with the reason `The host rejected the token. Check ROBROZ_TOKEN.` An unreachable host is titled `Can't reach the host` (「连不上宿主」); the reason line is `Can't reach host.` (「连不上宿主。」) plus the detail.
- Chat feed rendered as bubbles ([#155](https://github.com/PandaQuQ/robroz/pull/155)). Your messages sit on the right; bro status, approval, choice, and confirm cards sit on the left. A delivery with `bot_name` is attached to that bro and follows the usual 2-minute grouping; a delivery without `bot_name` stands alone, with no avatar or name. On macOS the UI font is Hiragino Sans GB; on Windows it is Microsoft YaHei UI.
- Optional `taskId`, `botId`, and `botName` on `Event::Delivery` ([#158](https://github.com/PandaQuQ/robroz/pull/158)). The fields are omitted when absent, so older JSON serializes unchanged.
- Appearance setting ([#160](https://github.com/PandaQuQ/robroz/pull/160)): a Settings row Appearance / 「外观」 under Language, also on the Can't reach the host page, with System / 「跟随系统」, Light / 「浅色」, and Dark / 「深色」. The default is System, which follows the OS live (`iced::system::theme_changes()`). The choice is saved as one line (`system`, `light`, or `dark`) in a `theme` file next to `lang`; no file means System. The footer Dark toggle is removed. `ROBROZ_THEME=light|dark` is honoured only in fake and gallery runs; a fake run with no variable defaults to Light.

### Fixed

- Dark-mode segmented controls show the selected option more clearly ([#162](https://github.com/PandaQuQ/robroz/pull/162)).
- Linux release builds run on older distributions ([#164](https://github.com/PandaQuQ/robroz/pull/164)). They are built on Ubuntu 22.04, and the release job fails if a binary needs glibc newer than 2.35. The client needs glibc 2.35 or newer; the host needs 2.34 or newer. The Linux icon is renamed to `robroz.png` to match `Icon=robroz` in `robroz.desktop`.

### Known issues

- Windows: an extra command-line window opens when the client starts. Leave it open; closing it also quits the client. Fixed in the next release.

[Unreleased]: https://github.com/PandaQuQ/robroz/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/PandaQuQ/robroz/releases/tag/v0.1.1
[0.1.0]: https://github.com/PandaQuQ/robroz/releases/tag/v0.1.0
