# The desktop client

How the `client/` crate (`robroz-client`) behaves. It is a native iced 0.14 app, with one codebase for Windows, Linux and macOS, and no web view. Build and run it with [install.md](install.md#run-the-client). Environment variables are in [configuration.md, Client](configuration.md#client-robroz-client). How events and the look-and-feel layer fit together is in [architecture.md](https://github.com/PandaQuQ/robroz/blob/main/docs/architecture.md#客户端).

The desktop client UI is English by default. Switch to 中文 with the Language / 语言 row on the Settings (Model) page, or on the Can't reach the host page if the host has never answered. `ROBROZ_LANG=en|zh` overrides the saved choice for one run and is not written back. Quoted UI text below is what the English UI shows; the Chinese is in parentheses where it helps.

Card colours, status lines and actions come from `robroz-card`; layout and sizes come from `robroz-ui`'s `Skin`. The client does not invent its own palette. UI copy below is taken from `crates/robroz-ui/src/i18n/en.rs`.

## Layout

- Left: the conversation list, titled Chats (「聊天」). On the same heading row, Add a bro (「引入一个 bro」) creates a bro and opens its DM. Below that are groups and DMs, newest activity first, with unread counts. An empty list shows No bros yet. Add your first one. (「还没有 bro，引入第一个」). That hint only appears once the host has listed the groups. Under the list are seven buttons, in order: Routines, Memories, Skills, Model, Web search, MCP, Allow rules. Right: the open conversation's message stream, or the matching page.
- When the window is narrower than 760px, the sidebar is hidden: the list and the conversation each take the whole window. Open a chat to see it; ‹ Chats (「‹ 聊天」) goes back. (The phone-sized rules for touch targets and shortened bro names still start at 480px.)
- The stream is chat bubbles, not full-width rows. On macOS the UI font is Hiragino Sans GB (`/System/Library/Fonts/Hiragino Sans GB.ttc`, which fontdb can index; PingFang SC is not used, because it lives in a hidden asset path). On Windows it is Microsoft YaHei UI. Linux keeps the generic sans-serif. This is so a Chinese string is not drawn glyph by glyph from different fonts. Your own messages sit on the right in an ink bubble (inverted on the dark theme). A bro's items sit on the left behind a round avatar with its initial, with the bro's name above the first item of a group. Bubbles are at most 68% of the stream width and 560px (80% in a window narrower than 760px), with 16px corners, the corner nearest the sender 4px, and 10/14 padding. Cards are a fixed 360px wide (the stream width if that is smaller), left-aligned with the bubbles. Items from the same sender less than 2 minutes after the previous one form a group: 4px apart inside a group, 16px between groups. The time of the group's last item shows below it in 11px light text. The host only writes `messages` from a person, so every message is yours; status, approval, choice and confirm cards belong to a bro. A delivery with `bot_name` is drawn as that bro's card and joins or starts that bro's group under the usual 2-minute rule. A delivery without `bot_name` is drawn alone: left-aligned with the bro cards (indented by the avatar width plus its gap), with no avatar and no name, and it starts a new group.
- The group list comes from `GET /api/groups`. If `ROBROZ_GROUPS` is set (comma-separated `id` or `id=name` items), only those groups are shown.
- Opening a conversation marks it read (`POST /api/groups/{id}/read`). Unread counts come from `GET /api/unread`.
- If the host has never answered, the sidebar stays empty and the main pane centres Can't reach the host (「连不上宿主」), with the host address on a lighter line below it and, 12px to the right, a short light-grey reason when there is one: Connection refused (「连接被拒绝」), Timed out (「超时」), or Host not found (「找不到主机」); otherwise no reason is shown, and the raw `io:` / `os error` text is never shown. Below that address/reason line, with a 24px gap, are the same Language / 语言 and Appearance / 「外观」 rows as the Model page, centred and at most 360px wide. Choosing a language or appearance switches immediately and is saved to `lang` or `theme` under the client config folder; `ROBROZ_LANG` still only overrides the current run. While the host has never answered, the footer is hidden, because the pane already shows why. The client retries on the poll interval. After the host has answered once, a later failure keeps what is already on screen and shows the error in the bottom bar.
- `--fake` / `ROBROZ_FAKE=1` plays a scripted run offline. `--gallery` lays out every card state for design review. Fake and gallery runs do not read or write the saved `lang` or `theme` files. In those runs, `ROBROZ_THEME=light|dark` sets the appearance; if it is unset, the run is Light.
- Every line from the composer goes to `POST /api/groups/{id}/say`. In a DM the whole message goes to that bro, or attaches as a follow-up if the bro already has an open task. In a group, every bro gets a turn; a leading `@name` or `＠name` gives that bro the first turn. The composer hint in a group is Message the group, or @name a bro.

## Language

The Language / 语言 row (`SettingsLanguage`; the same text in both UIs) sits on the Settings (Model) page, and also on the Can't reach the host page when the host has never answered. There is no separate general settings page and no footer switch. The options are English and 中文 (each name stays in its own language in both UIs). The choice is saved as one line (`en` or `zh`) in the `lang` file under the client config folder (`ROBROZ_CONFIG_DIR` if set, otherwise the usual per-OS folder; see [configuration.md](configuration.md)). `ROBROZ_LANG=en|zh` wins for the current run (`zh-CN`, `en_GB.UTF-8` and similar also work), is never written back, and unknown values are ignored. No file, or an unreadable one, means English.

Changing the language redraws on-screen cards and conversation-list previews immediately, including You (「我」) on cards. Status cards are drawn from the event each time; previews are cached and rebuilt from the stored events on the switch (`client/src/model.rs`, `Previews::redraw`).

## Appearance

Directly under the Language row, on the same two pages, is the Appearance / 「外观」 row (`SettingsAppearance`), with the same shape: the label on the left, and a segmented control on the right. The options are System / 「跟随系统」 (`SettingsThemeSystem`), Light / 「浅色」 (`ClientThemeLight`), and Dark / 「深色」 (`ClientThemeDark`). The default is System. Choosing System follows the operating system live (`iced::system::theme_changes()` in `client/src/main.rs`); Light and Dark pin the scheme, including the window frame. The choice is saved as one line (`system`, `light`, or `dark`) in a `theme` file next to `lang`. No file, or an unreadable one, means System. The footer no longer has a Dark control.

`ROBROZ_THEME=light|dark` is honoured only in fake and gallery runs, is never written back, and causes those runs to ignore the saved file. A fake or gallery run with the variable unset is Light, so screenshots do not follow the desktop. In a normal run the variable is ignored.

## Cards in the message stream

| Card | How it is shown |
| --- | --- |
| Text message | The body in your own bubble, on the right. |
| Image / file message | A labelled chip (Image / File) and the name on one line inside your bubble; there is no viewer. |
| Status card | One per task, updated in place by `taskId`. Bro name on the left, status line on the right. Rules below. |
| Delivery card | A summary plus a monospace path relative to the group folder, display only, not clickable. When `briefing` is true, a Briefing (「简报」) badge is added in the corner (`robroz_card::briefing_badge`). When `needsSetup` is `"search"` or `"model"`, a Set up (「去设置」) button is added on the right, and a grey line under the path says Skipped web search: not set up yet or Skipped: no model set up yet. When `failure` is set, the whole card becomes the grey Couldn't finish: {reason} line, with no path and no grey hint; if `needsSetup` is also set, Set up still appears on the right. A dead run's `failure` 「没写出结果」 is shown as Finished with no result, without the Couldn't finish: prefix (`robroz_card::failure_line`). |
| Choice card | Updated in place by `id`. Can be answered (`POST /api/choices/{id}/answer`) once. Who answered is described below. |
| Approval card | Approve and Deny, once. After an answer the footnote is two spans, with no "·" between them: Approved by You and the time when this client answered; Approved by {name} and the time when someone else did; Denied by You or Denied by {name} and the time for a denial. When `answeredBy` is `null`, the text is just Approved or Denied, plus the time. |
| Confirm card | See Confirm cards below. |

An event whose `type` the client does not recognise is skipped; other cards are unaffected.

### Status cards

A status card is a plain bro card (the bro bubble colour with a 1px border). The bro name is only in the group header, not inside the card. A small status dot sits 8px left of the status word: accent (`#c8572a` light, `#e0743f` dark) for a running card, green for done, amber for stuck, grey for queued, a dead run, waiting on approval and a failure. There is no filled bar. Wherever this section says "grey", "amber" or "green" for a state, that is the dot's colour. The status word (and "Step n of m", when shown) is left-aligned after the dot; a flexible gap pushes the Take over in DM button to the right. That button is outlined (transparent fill, 1px line border, ink text) with 6px vertical and 12px horizontal padding.

- `queued`: grey Queued (「排队」).
- `running` with `containerUp`: Running (「进行」). If `progress` is the Router step form (`{n}/{m} 步`, shown as Step n of m), it sits immediately to the left of the status word in a lighter colour, with a 12px gap and no separator; on a narrow window it is dropped rather than wrapping or shortening the status word. Any other `progress` is a light-grey line under the status row, shortened with "…" when it is too long. If there is no progress, that row is not reserved.
- `running` with `containerUp` false: grey Finished with no result, no action. This only appears in old data; the host now ends a dead run as `done` plus `failure` 「没写出结果」, and that card is also shown as grey Finished with no result, without Couldn't finish:.
- `blocked` with `waitingFor` `approval`: not treated as a takeover. Grey Needs your approval (「等你批准」), no button; answer the approval card beside it.
- `blocked` with `waitingFor` `takeover`, or with the field missing (an older host): amber Stuck (「卡住」) plus Take over in DM (「去私聊接管」). That button currently only logs; it is not wired up yet.
- `done`: Done (「完成」). With a `failure`, a grey Couldn't finish: {reason} (or Finished with no result when `failure` is 「没写出结果」), never the green Done, and no button.

Host-written `label`, `failure` and `progress` values on the wire are still Chinese. The client maps known strings into the English above.

### Conversation list previews

- When a live run has a progress line, the preview uses that line (mapped into the UI language), for example Ada: Thinking about a reply or Ada: Step 2 of 3, not Ada: Running. Long text is cut to the column width with "…".
- When `waitingFor` is `approval`, the preview is Ada: Needs your approval, and is not replaced by an approval card's summary that arrives at the same time.
- With no progress, or when the task is done, stuck, or failed, the preview uses the status line as usual.
- Switching the language redraws every preview immediately from the events the client already has.

### Who answered

When this client answers an approval, choice or confirm card, it sends `ROBROZ_USER_NAME` as `answeredBy` (the computer's hostname if that is unset, or `robroz` if the hostname cannot be read; first non-empty line only, at most 40 characters).

- If the returned `answeredBy` matches the name this client sent, the footnote uses You (「我」). If it is a different name, that name is shown. If it is `null`, no name is shown, only Approved (or the matching word) and the time.
- The time always comes from the host's `answeredAt`.
- Two devices must not share the same name, or each will show the other's answers as You.
- If someone else answered first (409), the card updates in place with that result.

## Confirm cards

- **Memory card:** a Memory tag in the top left, then the full text and a one-line scope (All bros or Only Ada), with Remember and Not now. After an answer the footnote is Remembered · You · HH:MM, or a grey Dismissed · … (the client joins those parts with "·"). Forgetting a pending memory on the Memories page updates the card to Dismissed · HH:MM, with no name.
- **Skill card:** a Skill tag in the top left, the title and `slug`, and a scope line under the instructions. Instructions show the first three lines by default; Show all opens them, Show less folds them back. The buttons are Save and Not now; after an answer the footnote is Saved · You · HH:MM or a grey Dismissed · …. After Save, the skill still has to be enabled in a group before it is added to the prompt.
- **Folding:** if one task has more than three pending confirm cards, from the fourth onwards they collapse to {N} more to review. Open that row to confirm them one by one.

## Memories page

Open it with the Memories button (next to Routines).

- Entries are grouped by scope: All bros first, then one group per bro (Only Ada). Inside a group, To review and Remembered N are listed separately; a group that only has kept items still shows Remembered N. Each item shows the text, a kind (Profile, Habit, Project) and the updated time. An empty page shows Nothing remembered yet.
- Pending items only have Remember and Not now, which call the confirm-card API with `confirmId`. The card in the chat updates with them.
- Kept items can be Edit (`PATCH`) or Forget (`DELETE`). Forget asks Forget this? Bros won't use it again. and then removes the item from the list.
- Add memory first asks for a scope. Until one is chosen, that row says Choose who can use this; after a choice it becomes the grey Only Bob can use this or All bros can use this. The row always takes space, so Save does not jump.
- Save is grey when the text is empty, unchanged, over 2000 characters, or no scope is chosen. After a success the updated time refreshes and Saved appears beside it. A failure shows Not saved: {reason} in place, and the draft is kept.

## Skills page

Open it with the Skills button. The layout matches the Memories page.

- Entries are grouped by scope. Inside a group, To review and Saved N are listed separately. Each item shows the title, `slug` and the first three lines of the instructions. An empty page shows No skills saved yet.
- Pending items have Save and Not now, which call the confirm-card API with `confirmId`.
- Saved items can be Edit (`PATCH`; `slug` cannot change) or Delete (`DELETE`). Delete asks Delete this? Bros won't use it again.
- A saved item has Use in these groups, with one switch per group: on is `PUT /api/groups/{id}/skills/{slug}`, off is `DELETE`. Opening the page first reads `GET /api/groups/{id}/skills`. Until that returns, the switch is grey and disabled, with Loading beside it; a failed read shows Couldn't load.
- Add skill needs a short English name (`slug`: lowercase letters, digits and hyphens), a title (at most 80 characters), instructions (at most 8,000 characters), and a scope. The scope hint, the greyed-out Save, Saved and Not saved: {reason} behave as on the Memories page.

## Model page

Open Model in the sidebar, or Set up on a delivery card with `needsSetup: "model"`. The API is `GET` / `PUT /api/settings/model` and `POST /api/settings/model/test` ([api.md](https://github.com/PandaQuQ/robroz/blob/main/docs/api.md#model-settings)). Code: state and copy in `crates/robroz-ui/src/cards/model.rs`, layout in `crates/robroz-ui/src/view.rs:model_panel`, requests in `client/src/api.rs`, message handling in `client/src/main.rs:on_model_action`. The Language / 语言 and Appearance / 「外观」 rows described above are on this page.

- **On open** the page `GET`s once and fills the form with the effective settings. When all three fields are empty it shows No model set up yet. A failed read shows the reason on the page (Can't reach the host when the host is down).
- **Provider:** a client-side shortcut; the host has no provider list. The options are, in order, Gemini (recommended), Groq, OpenRouter, Ollama, Custom. The empty-value placeholder is Choose a provider. Picking one fills the matching OpenAI-compatible URL (no trailing `/`); the model name and key still have to be entered. If the URL is edited so it no longer matches that shortcut, the provider becomes Custom. Picking Custom does not change a URL that is already filled in. When OpenRouter is selected, a grey line under the model name says Free model names end in :free.
- **URL / Model name / Key:** the labels are URL, Model name and Key (optional). Placeholders do not repeat the labels. The URL box starts empty; the model-name box uses that provider's example (Gemini `gemini-3.8-flash`, Groq `llama-3.3-70b-versatile`, OpenRouter `meta-llama/llama-3.3-70b-instruct:free`, Ollama `llama3.2`, Custom left empty). The key box also starts empty. Every provider, including Ollama, shows the key field.
- **Key** uses the shared `MaskedKey`. A stored key shows Set · last4, with only Clear beside it (the client joins Set and the last four characters with "·"). Clear does not send a request yet: the field changes to the grey Deleted on save plus Undo; Save is what sets `key` to `null` (that field then falls back to the environment). Save only submits the URL, model name and key changes in this form; it does not clear the other two fields.
- **Save:** both URL and model name are required, and the URL must start with `http://` or `https://`. The key is optional; the client no longer checks for a missing key locally. After a successful `PUT` the client automatically `POST`s `…/test` once. The result is Testing…, then ink Working with a green dot, or Saved, but can't connect: {reason}. When the cloud service is missing a key or the key is wrong, `/test` returns `error` 「密钥不对或没填」, and the page shows Saved, but can't connect: key is wrong or missing, not a raw 401. A failed save shows Not saved: {reason}.
- **From environment variables:** only shown when the host returns `source: "env"`. If the field is absent, nothing is labelled and nothing is guessed.
- **Set up:** shown only when a delivery card's `needsSetup` is `"model"`. The button appears whenever that field is present, even without a `failure`. Tapping it opens this page.

## Web search page

Open Web search in the sidebar, or Set up on a delivery card. The API is `GET` / `PUT /api/settings/search` and `POST /api/settings/search/test` ([api.md](https://github.com/PandaQuQ/robroz/blob/main/docs/api.md#search-settings)). Code: state and copy in `crates/robroz-ui/src/cards/search.rs`, layout in `crates/robroz-ui/src/view.rs:search_panel`, requests in `client/src/api.rs`, message handling in `client/src/main.rs:on_search_action`.

- **On open** the page `GET`s once and fills the form with the effective settings. When nothing is in effect (`provider` is null) and no service is selected, it shows Web search isn't set up yet. A failed read shows the reason on the page (Can't reach the host when the host is down).
- **From environment variables:** a tag next to the title, only when the optional `source` field in the response is `"env"`. `"settings"`, a missing value or an unknown value are not shown and not guessed. After a save the host returns `source: "settings"` and the tag disappears.
- **Provider:** Brave or SearXNG, two buttons, the selected one highlighted.
- **Brave key:** a masked field; the client never reads the raw key back.
  - When the host already has a key (`hasKey` is true), the input is hidden and the row shows Set · last4 (the `keyLast4` value, for example Set · abcd; or just Set when there is no last four). Only Clear sits beside it.
  - Clear first marks the key for deletion, shows the grey Deleted on save, and offers Undo. Save is what actually clears it; Undo returns to Set · …. The save sends `PUT {"provider":null,"url":null,"key":null}`, which clears the service, URL and key in the settings API, then falls back field by field to the environment. With no environment values the page returns to Web search isn't set up yet. Values that come from the environment cannot be cleared: after that save the page still shows the environment settings and From environment variables.
  - With no stored key the input is shown, with the placeholder API key.
- **SearXNG URL:** one input, placeholder SearXNG URL, e.g. http://127.0.0.1:8888. There is no Clear button under SearXNG.
- **Save:** grey when no service is selected or a request is in flight. Pressing Enter in an input also saves. The `PUT` only includes the fields that are needed: Brave with a new key sends `key`; Brave with a stored key sends only `{"provider":"brave"}`; SearXNG sends `provider` and `url` and leaves the key alone.
  - Local checks that fail are not sent: Brave with no stored key and nothing typed shows Enter a key; SearXNG with no URL shows Enter a URL.
  - A failed save shows Not saved: {reason} in place. The reason is the host's error text (see the 400 list in api.md), or Not saved: Can't reach the host when the host is down.
  - After a successful save the page **tests the connection once** (`POST …/test`). Next to Save it shows Testing…, then Working, or Saved, but can't connect: {reason}. A leading 「连不上：」 in the host's error is stripped so it is not doubled. There is no separate test button on the page.
- **Set up:** shown only when a delivery card's `needsSetup` is `"search"`, not by matching Web search isn't set up yet. The button appears whenever that field is present, even without a `failure`. When `web_search` is not configured the host returns that sentence to the model, the task usually still writes a result, and the delivery has no `failure`: the card still shows Set up, with the grey line Skipped web search: not set up yet under the path. With a `failure` the card is the grey Couldn't finish: {reason}, with no path and no grey hint, and the button stays on the right. Tapping Set up opens this page. You can also open Web search from the sidebar.

## MCP page

Open MCP in the sidebar. This page manages MCP servers on the host: add, edit, delete, test a connection, list tools, enable per group, and approve a stdio start command. The API is in [search-and-mcp.md](https://github.com/PandaQuQ/robroz/blob/main/docs/search-and-mcp.md#mcp) and [api.md](https://github.com/PandaQuQ/robroz/blob/main/docs/api.md). Code: state and copy in `crates/robroz-ui/src/cards/mcp.rs`, masked keys in `crates/robroz-ui/src/cards/secret.rs` (shared with the Web search page), layout in `crates/robroz-ui/src/view.rs:mcp_panel`, requests in `client/src/api.rs`, message handling in `client/src/main.rs:on_mcp_action`.

- **On open** the page reads `GET /api/mcp/servers`, then `GET /api/groups/{id}/mcp` for each conversation in the left list. A failed read shows the reason on the page (Can't reach the host when the host is down).
- **Heading:** MCP, with a count when there are servers; Add server on the right (hidden while the form is open).
- **Empty state:** with no servers and no open form, No MCP servers yet and Click “Add server” to connect tools your bros can use.
- **List:** newest `updatedAt` first. Each row is a bold name, a grey line under it, and an enable switch (`PATCH /api/mcp/servers/{id}` `{"enabled":…}`). The grey line is N tools (`toolCount`; 1 tool when the count is one), or Not counted yet when it has not been counted. Click the name to expand or collapse the row.
- **An unapproved stdio server** (`transport` is stdio and `commandApproved` is not true): a Command not approved tag appears next to the name (accent-coloured, `palette.accent`). While collapsed, the grey line is Approve the command to load its tools. The enable switch still works.
- **Once expanded,** in order: the tool list (or the command-approval panel, below), Use in these groups, Edit, Delete.
  - The tool list comes from `GET /api/mcp/servers/{id}/tools`, fetched the first time the row is expanded and cached after that. While loading it shows Loading, a failure is Can't connect: {reason}, and no tools is This server has no tools yet. Each item is the tool name, with Needs your approval beside it when `needsApproval` is true, and a grey description line when `description` is set.
  - Use in these groups: one switch per conversation in the left list. On is `PUT /api/groups/{id}/mcp/{serverId}`, off is `DELETE`. Until the real state is loaded the switch is grey and disabled, with Loading beside it; a failed read shows Couldn't load.
  - Delete first asks Delete this server? Groups won't use it any more. A second Delete sends `DELETE` (a 404 is treated as already deleted); Cancel folds the prompt back.
  - A failure under the row shows Not saved: {reason}.
- **Add server / Edit:** the form appears at the top of the list (add) or replaces that row in place (edit). Each input has a small label above it so the field stays identifiable once it has a value:
  - Name: required, at most 80 characters. The placeholder is the same as the label (Name).
  - stdio and HTTP choose the transport; stdio is the default.
  - stdio: Command (required) and Arguments (space-separated) (split on whitespace, so one argument cannot contain a space). The placeholders are the same as the labels.
  - HTTP: URL (must start with `http://` or `https://`). The placeholder is URL.
  - Add a secret adds a row: Secret name (no whitespace), Environment variables or Request headers (stdio defaults to Environment variables, HTTP to Request headers), Remove, and Secret. The placeholders are Secret name and Secret.
  - When a local check fails, Save is grey and one of these hints is shown: Enter a name, Name: max 80 characters, Enter a start command, Enter a URL, The URL must start with http:// or https://, Enter a secret name, Secret names can't contain spaces, Enter a value for the new secret. When the host rejects the save, Not saved: {reason} is shown in place.
  - Edit fills in the name, command, arguments and URL, and **does not fill in the raw secret**.
- **Secrets:** the input uses the same `MaskedKey` as the Web search and Model pages. A stored secret (`hasKey` is true) hides the input and shows Set · last4 (or just Set), with only Clear beside it.
  - Clear does not send a request immediately: the field changes to the grey Deleted on save plus Undo. Save is what sends this secret as `key: null`, and the host then deletes the whole entry (including the name). Undo returns to Set · ….
  - A stored secret that was not changed is omitted from the `PATCH`, and the host keeps the previous value.
  - Remove only takes the row out of the form. Secrets the `PATCH` does not mention are kept, so Remove plus Save **does not delete** a stored secret; use Clear and then Save.
- **After Save:** create uses `POST /api/mcp/servers`, edit uses `PATCH`. On success the row expands itself, **tests the connection once** (`POST …/test`) and reloads the tools. Under the name it shows Testing…, then Working · N tools (or just Working when there is no count), or Can't connect: {reason}. There is no separate test button on the page.
- **Command approval:** a stdio server's command and arguments must be approved before the host will start it ([search-and-mcp.md](https://github.com/PandaQuQ/robroz/blob/main/docs/search-and-mcp.md#mcp)). When `…/test` or `…/tools` returns 409 `command_not_approved`, the row expands and the tool list is replaced by the approval panel:
  - A Command label over the command, an Arguments label over each argument on its own line, all monospace, wrapping rather than truncated. The command and arguments come from the 409 body, and are what the host would start.
  - A grey line: This command runs on your computer.
  - Approve and run sends `POST /api/mcp/servers/{id}/approve-command`. On success the panel folds away and the page tests the connection and reloads the tools automatically. A failure under the row shows Not saved: {reason}.
  - Cancel only folds the panel, with no request; the row returns to Approve the command to load its tools. Expanding it again reloads the tools, and another 409 brings the panel back.
  - A new or edited stdio server is tested right after save, so the approval panel appears immediately.

## Allow rules page

Open Allow rules in the sidebar (under MCP). It lists allow rules that have already been confirmed. Adding or deleting a rule from this page returns 202 and an `approvalId`; the client then `GET /api/approvals/{id}` and shows the confirm card on this page (a new rule under the input, a deletion in place of that row). The owner approves or denies it here, with `POST /api/approvals/{id}/approve` / `deny`. `botId` is `SETTINGS_APPROVAL_BOT_ID` (`"settings"`), there is no `taskId`, the card is not sent to any chat, stream or unread count, and it works even with no bros. The client also filters these cards out of the conversation stream. After an approval the list is fetched again; a denial just dismisses the card. The API is in [api.md, Allow rules](https://github.com/PandaQuQ/robroz/blob/main/docs/api.md#allow-rules); the design is in [design/auto-review.md](https://github.com/PandaQuQ/robroz/blob/main/docs/design/auto-review.md). Code: state and copy in `crates/robroz-ui/src/cards/review_rules.rs`, layout in `crates/robroz-ui/src/view.rs:review_rules_panel`, requests in `client/src/api.rs`, message handling in `client/src/main.rs:on_review_action`.

- **Add a rule:** a one-line input at the top, placeholder One sentence, e.g. Allow merging PRs when CI is green. Submitting it returns 202 and an `approvalId`; the confirm card is shown under the input. If the text is too long, the page shows Max {0} characters. Remove {1} more.
- **One-line rule:** the rule text, a grey Added {date} line under it (from `createdAt`), and Delete on the right. There is no switch and no edit. After Delete the rule stays in the list until it is confirmed; the confirm card is shown in that row.
- **Confirm card** (on this settings page, reusing the confirm layout): no bold title. The body is the rule text, with a grey line under it: Confirm to add this allow rule, or Confirm to delete this rule. For an add the buttons are Add / Not now; for a delete they are a red Delete rule / Not now. If the card was already handled elsewhere, it is dismissed and a grey This card was already handled appears on the page, then the list is fetched again.
- **Expired card:** when `decision` is `denied` and `reason` equals the contract constant `APPROVAL_EXPIRED_REASON` (「审批已过期」), the card shows Expired, and the footnote Expired with no approval plus the time as two spans. The buttons are locked. It does not say Denied by You, and there is no "try again". In a chat an expired card still has a grey Expired dot in the top right, and keeps the action row.
- **Approval cards in a group:** (v0.1 client behaviour. A v0.2 host keeps the card in the group, adds an `approval_reminder` in the owner's DM, and stops sending 「已发到主人私聊」; this client skips the reminder and does not yet draw buttons on a pending group card, see [permissions.md](https://github.com/PandaQuQ/robroz/blob/main/docs/permissions.md#who-may-answer-and-where-the-card-goes-v02).) task cards and MCP cards are sent to the owner's DM with that bro, and the owner approves or denies them there. When a card is blocked, an extra grey line reads Blocked because: plus `reason`. In the group only the status card remains: `blocked` + `waitingFor: approval`, `progress` is Sent to owner's DM (`APPROVAL_SENT_TO_OWNER_DM_NOTICE`), with no Approve/Deny buttons. If the task was already running in that DM, the card stays there and this notice is not shown. A card a bro issues with `POST /api/groups/{id}/approvals` goes through auto-review first; an allowed card is not shown. If it is blocked or there are no rules, the card is sent to the owner's DM with that bro, and the owner approves or denies it there. The group the bro posted in also lists the same card (the same `id`); the client draws it with no buttons and Sent to owner's DM, and updates that copy when the owner answers in the DM.

## Routines

The Routines panel lists each group's routines (`GET /api/groups/{id}/schedules`) and can pause or resume them (`PATCH /api/schedules/{id}`). A paused routine is tagged Paused.

## Platforms and tests

- Running on Linux needs a graphical desktop, with `libxkbcommon-x11` (X11) or the Wayland libraries installed. The build itself does not need extra development packages.
- The client speaks HTTP or HTTPS (`ROBROZ_HOST`, or a saved `host` file next to `lang`). It sends `Authorization: Bearer <token>` on every request, including the live stream, when a token is set. `ROBROZ_HOST` and `ROBROZ_TOKEN` override any saved values for this run and are never written back. Otherwise the address comes from the `host` file, and the token comes from the OS keychain (Secret Service on Linux), falling back to a `token` file next to `lang` (mode `0600` on Unix). It does not read the host's `ROBROZ_TOKEN_FILE` variable. The token is never logged or shown. A 401 is shown as "The host rejected the token. Check ROBROZ_TOKEN." under the title "Token rejected" (「宿主拒绝了令牌，请检查 ROBROZ_TOKEN。」, title 「令牌被拒绝」). A host that cannot be reached is shown as "Can't reach host." (「连不上宿主。」), followed by the reason. The host itself does not do TLS; reach a remote host through an HTTPS reverse proxy or tunnel in front of it. The connection form reads [`load`](../client/src/conn/config.rs) and writes with [`save`](../client/src/conn/config.rs); after a successful save, build a new connection with `Conn::from_env`.
- `cargo test -p robroz-client` starts a real host router in-process and reads it back through `HttpSource`; Docker is not required.
