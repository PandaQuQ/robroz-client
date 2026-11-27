#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! robroz desktop client: an IM for bros.
//!
//! Left: the conversation list (groups and bro DMs, newest first, unread
//! badges, 「＋ 新 bro」) and 例行任务. Right: the open chat's message stream
//! or the routines panel. At ≤480 wide the list and the chat each take the
//! whole window (tap in, 「‹ 聊天」 back).
//!
//! Events: the open chat streams live over SSE ([`source::stream`]); every
//! chat is also polled incrementally with `?since=` — every tick while the
//! stream is down, as a slow safety net while it is up. Unread counts come
//! from `GET /api/unread`; opening a chat marks it read. `--fake` /
//! `ROBROZ_FAKE=1` runs offline on a script. Card clicks go through
//! [`api::CardApi`]. `--gallery` shows every card state for design review.
//! Visuals come from the `robroz-ui` skin; the Settings page picks System,
//! Light or Dark (see [`prefs`]; `ROBROZ_THEME=light|dark` in fake runs).

mod api;
mod cli;
mod conn;
mod event_decode;
mod gallery;
mod host_state;
mod jump;
#[cfg(any(windows, test))]
mod logfile;
mod model;
mod prefs;
mod source;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{
    window, Alignment, Border, Element, Fill, Font, Length, Padding, Size, Subscription, Task,
};
use robroz_contract::{
    ApprovalDecision, Bro, ConfirmSubject, Event, McpServer, McpTestResult, McpTool, Me, Memory,
    Said, ScheduleEntry, Skill, Unread,
};
use robroz_contract::BroAvatar;
use robroz_ui::avatar;
use robroz_ui::cards::confirm::{self, ConfirmState, Fold};
use robroz_ui::cards::interact::{CardUi, Note};
use robroz_ui::cards::mcp::{self, Book as McpBook};
use robroz_ui::cards::memories::Book;

use robroz_ui::cards::model::{self as model_page, Current as ModelCurrent, Page as ModelPage};
use robroz_ui::cards::review_rules::{self, Page as ReviewRulesPage, Rule as ReviewRule};

use robroz_ui::cards::routines::{RoutineRow, Routines};
use robroz_ui::cards::search::{self as search_page, Current as SearchCurrent, Page as SearchPage};
use robroz_ui::cards::skills::Book as SkillBook;
use robroz_ui::cards::{ApprovalCard, ApprovalExtras};
use robroz_ui::cards::{ApprovalState, Card, ChoiceState, DraftState};
use robroz_ui::conversations::Conversations;
use robroz_ui::i18n::{self, Key as Text, Locale};
use robroz_ui::view::{self, color, Action, Caps};
use robroz_ui::{Mode, Skin};

use api::{ApiError, CardApi, FakeApi, HttpApi};
use conn::form::{self as conn_form, ConnForm};
use conn::Conn;
use gallery::Gallery;
use host_state::HostState;
use model::{attachment_label, change_time, DeliveryCard, Feed, Row, StatusCard};
use prefs::ThemePref;
use source::stream::{self, StreamItem};
use source::{
    approvals_chats, approvals_source, codes_chats, codes_source, fake_chats, fake_schedules,
    EventSource, FakeSource, GroupCard, GroupEvent, GroupTab, HttpSource, SourceError,
};

/// How often the conversation list refreshes unread counts.
const UNREAD_EVERY: Duration = Duration::from_secs(3);

pub fn main() -> iced::Result {
    cli::exit_if_requested();
    #[cfg(windows)]
    logfile::install();
    let size = env_size().unwrap_or((1040.0, 680.0));
    iced::application(App::boot, App::update, App::view)
        .title("robroz")
        .subscription(App::subscription)
        .theme(App::theme)
        .default_font(view::regular())
        .scale_factor(App::scale_factor)
        .window(window_settings(size))
        .run()
}

fn window_settings(size: (f32, f32)) -> window::Settings {
    window::Settings {
        size: Size::new(size.0, size.1),
        icon: window_icon(),
        // Matches StartupWMClass=robroz in client/packaging/robroz.desktop so
        // the Linux taskbar uses the app icon instead of the default.
        // `PlatformSpecific` only has `application_id` on Linux.
        #[cfg(target_os = "linux")]
        platform_specific: window::settings::PlatformSpecific {
            application_id: String::from("robroz"),
            ..window::settings::PlatformSpecific::default()
        },
        ..window::Settings::default()
    }
}

/// Edge length of the window icon passed to iced/winit.
///
/// The brand asset is 512×512, but a 512px `_NET_WM_ICON` is about 1 MiB.
/// xprop's default 500_000-byte cap then reads width and height without
/// enough pixels and prints an empty property; some window managers do the
/// same and show a generic icon. 128×128 is 64 KiB, fits the default X11
/// 256 KiB request, and is the size Linux WMs expect.
const WINDOW_ICON_SIZE: u32 = 128;

/// The brand icon, decoded from the embedded 512px PNG and scaled to
/// [`WINDOW_ICON_SIZE`]. `None` (default icon) if it cannot be decoded,
/// which must never stop the client from starting.
fn window_icon() -> Option<window::Icon> {
    let (rgba, width, height) = decode_window_icon()?;
    window::icon::from_rgba(rgba, width, height).ok()
}

/// RGBA bytes and size of the window icon.
fn decode_window_icon() -> Option<(Vec<u8>, u32, u32)> {
    static ICON_PNG: &[u8] = include_bytes!("../../assets/brand/icon-512.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(ICON_PNG))
        .read_info()
        .ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    if info.width == WINDOW_ICON_SIZE && info.height == WINDOW_ICON_SIZE {
        return Some((buf, info.width, info.height));
    }
    let rgba = downscale_rgba(
        &buf,
        info.width,
        info.height,
        WINDOW_ICON_SIZE,
        WINDOW_ICON_SIZE,
    )?;
    Some((rgba, WINDOW_ICON_SIZE, WINDOW_ICON_SIZE))
}

/// Integer-ratio box filter with premultiplied averaging, so transparent
/// edges stay clean. `src_w` and `src_h` must be exact multiples of `dst_w`
/// and `dst_h`.
fn downscale_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Option<Vec<u8>> {
    if src_w == 0
        || src_h == 0
        || dst_w == 0
        || dst_h == 0
        || src_w % dst_w != 0
        || src_h % dst_h != 0
        || src.len() != (src_w * src_h * 4) as usize
    {
        return None;
    }
    let nx = (src_w / dst_w) as usize;
    let ny = (src_h / dst_h) as usize;
    let src_w = src_w as usize;
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    for y in 0..dst_h as usize {
        for x in 0..dst_w as usize {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            for iy in 0..ny {
                for ix in 0..nx {
                    let i = ((y * ny + iy) * src_w + (x * nx + ix)) * 4;
                    let pa = src[i + 3] as u32;
                    r += src[i] as u32 * pa;
                    g += src[i + 1] as u32 * pa;
                    b += src[i + 2] as u32 * pa;
                    a += pa;
                }
            }
            let o = (y * dst_w as usize + x) * 4;
            if a != 0 {
                out[o] = (r / a) as u8;
                out[o + 1] = (g / a) as u8;
                out[o + 2] = (b / a) as u8;
                out[o + 3] = (a / (nx * ny) as u32) as u8;
            }
        }
    }
    Some(out)
}

fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

fn wants_fake() -> bool {
    has_flag("--fake") || std::env::var("ROBROZ_FAKE").is_ok_and(|v| v == "1")
}

/// Id of the open chat's scrollable feed (the reminder row jumps in it).
const FEED_ID: &str = "chat-feed";

/// How long a card's stroke stays accent after a jump: 300 ms. Fake runs
/// can hold it longer with `ROBROZ_FLASH_MS` for a screenshot.
fn flash_ms(fake: bool) -> u64 {
    fake.then(|| std::env::var("ROBROZ_FLASH_MS").ok())
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

/// `ROBROZ_WINDOW=WxH` (logical px), mainly for gallery screenshots.
fn env_size() -> Option<(f32, f32)> {
    let v = std::env::var("ROBROZ_WINDOW").ok()?;
    let (w, h) = v.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// This client's name: sent as `answeredBy` when it answers a card, and
/// shown as 「我」 when a card comes back with it. `ROBROZ_USER_NAME`, else
/// the computer's hostname, else `robroz`. Two devices must not share it, or
/// each shows the other's answers as 「我」.
fn user_name() -> &'static str {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| pick_name(std::env::var("ROBROZ_USER_NAME").ok(), hostname()))
}

/// First usable of `env` and `host`, cut to what the host stores
/// (one line, at most `ANSWERED_BY_MAX` characters); else `robroz`.
fn pick_name(env: Option<String>, host: Option<String>) -> String {
    [env, host]
        .into_iter()
        .flatten()
        .filter_map(|raw| {
            let line = raw.lines().map(str::trim).find(|l| !l.is_empty())?;
            Some(
                line.chars()
                    .take(robroz_contract::ANSWERED_BY_MAX)
                    .collect::<String>(),
            )
        })
        .find_map(|n| robroz_contract::clean_answered_by(Some(&n)).ok().flatten())
        .unwrap_or_else(|| "robroz".into())
}

/// The computer's hostname, if it can be read (`hostname` exists on
/// Linux, macOS and Windows).
fn hostname() -> Option<String> {
    let out = std::process::Command::new("hostname").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|h| !h.is_empty())
}

type Shared = Arc<Mutex<Box<dyn EventSource>>>;

#[derive(Debug, Clone)]
struct Polled {
    events: Vec<GroupEvent>,
    cards: Vec<GroupCard>,
    has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Group,
    Routines,
    Memories,
    Skills,
    Mcp,
    Search,

    Model,
    ReviewRules,
    /// 「引入一个 bro」: name, shape and colour, then 引入.
    NewBro,
    /// Your own bro's avatar (`…` in its DM).
    BroAvatar,
}

#[derive(Debug, Clone)]
enum Message {
    OpenRoutines,
    OpenMemories,
    OpenSkills,
    /// 联网搜索 page (sidebar, or 「去设置」 on a delivery).
    OpenSearch,
    SearchLoaded(Result<SearchCurrent, ApiError>),
    SearchSaved(Result<SearchCurrent, ApiError>),
    SearchTested(Result<(bool, Option<String>), ApiError>),

    OpenReviewRules,
    ReviewRulesLoaded(Result<Vec<ReviewRule>, ApiError>),
    ReviewRuleProposed(Result<Event, ApiError>),
    /// 模型 page (sidebar, or 「去设置」 on a delivery).
    OpenModel,
    ModelLoaded(Result<ModelCurrent, ApiError>),
    ModelSaved(Result<ModelCurrent, ApiError>),
    ModelTested(Result<(bool, Option<String>), ApiError>),

    MemoriesLoaded(Result<Vec<Memory>, ApiError>),
    SkillsLoaded(Result<Vec<Skill>, ApiError>),
    /// PATCH (edit) or POST (new) answered.
    MemorySaved {
        /// `None` for 记一条, the memory id for 改.
        id: Option<String>,
        result: Result<Memory, ApiError>,
    },
    MemoryForgot {
        id: String,
        result: Result<(), ApiError>,
    },
    /// 记下 / 不用 from the 记忆 page, answered on the entry's confirm card.
    MemoryDecided {
        id: String,
        keep: bool,
        result: Result<Option<(Event, Option<ApprovalExtras>)>, ApiError>,
    },
    /// PATCH (edit) or POST (new) answered on the 技能 page.
    SkillSaved {
        /// `None` for 加一项, the slug for 改.
        slug: Option<String>,
        result: Result<Skill, ApiError>,
    },
    SkillForgot {
        slug: String,
        result: Result<(), ApiError>,
    },
    /// 保存 / 不用 from the 技能 page, answered on the entry's confirm card.
    SkillDecided {
        slug: String,
        keep: bool,
        result: Result<Option<(Event, Option<ApprovalExtras>)>, ApiError>,
    },
    SkillToggled {
        slug: String,
        group: String,
        on: bool,
        result: Result<(), ApiError>,
    },
    GroupSkillsLoaded {
        group: String,
        result: Result<Vec<Skill>, ApiError>,
    },
    OpenMcp,
    McpLoaded(Result<Vec<McpServer>, ApiError>),
    McpSaved {
        result: Result<McpServer, ApiError>,
    },
    McpDeleted {
        id: String,
        result: Result<(), ApiError>,
    },
    McpToolsLoaded {
        id: String,
        result: Result<Vec<McpTool>, ApiError>,
    },
    McpTested {
        id: String,
        result: Result<McpTestResult, ApiError>,
    },
    McpEnabled {
        id: String,
        result: Result<McpServer, ApiError>,
    },
    McpGroupToggled {
        id: String,
        group: String,
        on: bool,
        result: Result<(), ApiError>,
    },
    GroupMcpLoaded {
        group: String,
        result: Result<Vec<McpServer>, ApiError>,
    },
    McpCommandApproved {
        id: String,
        result: Result<McpServer, ApiError>,
    },
    Tick,
    /// Ask the host for the groups again after the first listing failed.
    RetryLoad,
    Loaded(Result<Vec<GroupTab>, SourceError>),
    Polled(Result<Polled, SourceError>),
    TakeOver(String),
    /// Open or close the host `failureDetail` on a failed card.
    ToggleFailDetail(String),
    Ui(Action),
    Answered {
        card: String,
        sent: Vec<String>,
        result: Result<Option<Event>, ApiError>,
    },
    Acted {
        card: String,
        result: Result<(), ApiError>,
    },
    /// Approval answered (or refused) by the host.
    Decided {
        card: String,
        approve: bool,
        result: Result<Option<(Event, Option<ApprovalExtras>)>, ApiError>,
    },
    RoutinesLoaded(Result<Vec<ScheduleEntry>, ApiError>),
    Paused {
        id: String,
        wanted: bool,
        result: Result<ScheduleEntry, ApiError>,
    },
    ClearPauseNote(String),
    UnreadTick,
    UnreadLoaded(Result<Vec<Unread>, ApiError>),
    MarkedRead(String, Result<Unread, ApiError>),
    BrosLoaded(Result<Vec<Bro>, ApiError>),
    /// `GET /api/me`: who this client is (an older host has no such route).
    MeLoaded(Result<Me, ApiError>),
    /// The reminder row's jump: the group is open, scroll to the card.
    JumpTo(String),
    /// The card's 300 ms accent stroke is over.
    FlashOff(String),
    BroCreated(Result<Bro, ApiError>),
    /// robroz#194 avatar save. `prev` is the stored avatar before this
    /// change, restored when the save fails; `seq` drops stale replies.
    AvatarSaved {
        bro: String,
        prev: Option<BroAvatar>,
        seq: u64,
        result: Result<Bro, ApiError>,
    },
    GroupsReloaded(Result<Vec<GroupTab>, SourceError>),
    Live(stream::Live),
    /// Rename result for the bro behind DM `group`.
    Renamed {
        group: String,
        result: Result<Bro, ApiError>,
    },
    /// Composer result from `/say`: the stored message (+ task, if any).
    Sent {
        group: String,
        result: Result<Said, ApiError>,
    },
    /// Narrow screens: back from a chat to the list.
    Back,
    /// Appearance row on the Settings page: System / Light / Dark.
    SetTheme(ThemePref),
    /// The operating system's colour scheme, at start and when it changes.
    SystemTheme(iced::theme::Mode),
    /// Language row on the Settings (模型) page: English / 中文.
    SetLocale(Locale),
    /// Connection form (offline pane and Settings).
    Conn(conn_form::Event),
    ConnFinished(Result<Vec<GroupTab>, conn_form::FormError>),
    Resized(Size),
    TouchSeen,
    Shoot(Option<window::Id>),
    Shot(window::Screenshot),
}

/// The create-bro page's draft.
#[derive(Debug, Clone)]
struct NewBroDraft {
    name: String,
    avatar: BroAvatar,
    busy: bool,
    /// Last create error, shown under the preview.
    note: Option<String>,
}

impl Default for NewBroDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            avatar: avatar::default_for(""),
            busy: false,
            note: None,
        }
    }
}

/// Avatar of bro `id` from a bro list: stored, else the id default.
fn avatar_for(bros: &[Bro], id: &str) -> BroAvatar {
    avatar::resolve(bros.iter().find(|b| b.id == id).and_then(|b| b.avatar), id)
}

struct App {
    source: Shared,
    api: Arc<dyn CardApi>,
    /// Which card buttons the backend can serve (drafts: fake only).
    caps: Caps,
    /// Live host connection for the SSE stream; `None` in fake mode.
    conn: Option<Conn>,
    convs: Conversations,
    /// The open chat's stream is connected (polling slows down).
    stream_up: bool,
    /// Narrow screens: a chat (or routines) is open over the list.
    narrow_open: bool,
    /// Known bros (DM owners, the rename box) and their avatars.
    bros: Vec<Bro>,
    /// The create-bro page.
    new_bro: NewBroDraft,
    /// The bro whose avatar page is open.
    avatar_bro: Option<String>,
    /// Last avatar save error, shown under the preview.
    avatar_note: Option<String>,
    /// Latest avatar save sent; older replies are ignored.
    avatar_seq: u64,
    /// The signed-in person (`GET /api/me`); decides who sees the approval
    /// buttons. `None` until the host answers, and on an older host.
    me: Option<Me>,
    /// Composer text per chat.
    drafts: HashMap<String, String>,
    sending: bool,
    /// Last composer problem, per chat.
    send_note: HashMap<String, String>,
    /// `--fake` / gallery: canned data, test-only env knobs allowed.
    fake: bool,
    /// Bro rename in progress: `(dm group, draft)`.
    rename: Option<(String, String)>,
    renaming: bool,
    /// Last rename error: `(dm group, note)`.
    rename_note: Option<(String, String)>,
    origin: String,
    interval: Duration,
    groups: Vec<GroupTab>,
    feeds: HashMap<String, Feed>,
    /// The event behind each chat's list preview (see `model::Previews`).
    previews: model::Previews,
    selected: Option<String>,
    page: Page,
    ui: HashMap<String, CardUi>,
    /// Tasks whose folded confirm cards were opened (「还有 N 条待确认」).
    unfolded: HashSet<String>,
    /// Failed cards whose host `failureDetail` is expanded.
    fail_open: HashSet<String>,
    routines: Routines,
    /// 记忆 page (robroz#62).
    memories: Book,
    /// 技能 page (robroz#71).
    skills: SkillBook,
    /// MCP page (robroz#81).
    mcp: McpBook,
    /// 联网搜索 page (robroz#76).
    search: SearchPage,

    /// 放行规则 page.
    review_rules: ReviewRulesPage,
    /// 模型 page (robroz#102).
    model_settings: ModelPage,
    /// Host and token form (offline pane and Settings).
    conn_form: ConnForm,

    /// The appearance chosen in Settings (or `ROBROZ_THEME` in fake runs).
    theme_pref: ThemePref,
    /// The operating system's scheme; used while `theme_pref` is System.
    system_mode: Mode,
    width: f32,
    loaded: bool,
    /// The latest attempt at the first group listing failed.
    load_failed: bool,
    in_flight: bool,
    has_more: bool,
    error: Option<String>,
    gallery: Option<Gallery>,
    scale: f32,
    /// Touch input seen (or `ROBROZ_TOUCH=1`): raises hit areas to the floor.
    touch: bool,
}

/// Run a blocking source call off the UI thread.
fn blocking<T: Send + 'static>(
    source: &Shared,
    f: impl FnOnce(&mut dyn EventSource) -> T + Send + 'static,
) -> impl std::future::Future<Output = T> {
    let source = source.clone();
    async move {
        tokio::task::spawn_blocking(move || {
            let mut guard = source.lock().expect("event source lock poisoned");
            f(guard.as_mut())
        })
        .await
        .expect("event source task panicked")
    }
}

/// Run a blocking API call off the UI thread.
fn call<T: Send + 'static>(
    api: &Arc<dyn CardApi>,
    f: impl FnOnce(&dyn CardApi) -> T + Send + 'static,
) -> impl std::future::Future<Output = T> {
    let api = api.clone();
    async move {
        tokio::task::spawn_blocking(move || f(api.as_ref()))
            .await
            .expect("api task panicked")
    }
}

/// Persist the form (unless an env var blocks the write) and list groups
/// with a fresh [`Conn::from_env`]. Runs off the UI thread.
fn connect_blocking(
    host: String,
    token: conn::TokenEdit,
) -> Result<Vec<GroupTab>, conn_form::FormError> {
    if let Err(err) = conn::save(&host, token) {
        match err {
            conn::SaveError::EnvOverride { .. } => {}
            conn::SaveError::Io(_) => {
                return Err(conn_form::FormError::Save(err.to_string()));
            }
        }
    }
    HttpSource::from_env()
        .groups()
        .map_err(|err| conn_form::FormError::Conn(err.as_conn_error()))
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let gallery = has_flag("--gallery").then(Gallery::new);
        let fake = wants_fake() || gallery.is_some();
        // Before anything builds text: fake and gallery runs ignore the
        // saved choice so their screenshots are reproducible.
        i18n::set_locale(prefs::initial_locale(!fake));
        type Parts = (Box<dyn EventSource>, Arc<dyn CardApi>, String, Option<Conn>);
        let (source, api, origin, conn): Parts = if fake {
            // `ROBROZ_FAKE_SCENE=approvals` or `status-codes`.
            let scene = std::env::var("ROBROZ_FAKE_SCENE").unwrap_or_default();
            let (unread, bros) = match scene.as_str() {
                "approvals" => approvals_chats(),
                "status-codes" => codes_chats(),
                _ => fake_chats(),
            };
            let source: Box<dyn EventSource> = match scene.as_str() {
                "approvals" => Box::new(approvals_source()),
                "status-codes" => Box::new(codes_source()),
                _ => Box::new(FakeSource::new()),
            };
            (
                source,
                Arc::new(FakeApi::new(fake_schedules()).with_chats(unread, bros)),
                Text::ClientFakeOrigin.into(),
                None,
            )
        } else {
            let http = HttpSource::from_env();
            let conn = http.conn().clone();
            let origin = http.base().to_string();
            let api = Arc::new(HttpApi::new(conn.clone()).answering_as(user_name()));
            (Box::new(http), api, origin, Some(conn))
        };
        let interval = source.interval();
        let theme_pref = prefs::initial_theme(fake);
        let scale = std::env::var("ROBROZ_SCALE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        let app = Self {
            source: Arc::new(Mutex::new(source)),
            api,
            caps: if fake { Caps::FAKE } else { Caps::HOST },
            conn,
            convs: Conversations::default(),
            stream_up: false,
            narrow_open: false,
            bros: Vec::new(),
            new_bro: NewBroDraft::default(),
            avatar_bro: None,
            avatar_note: None,
            avatar_seq: 0,
            me: None,
            drafts: HashMap::new(),
            sending: false,
            send_note: HashMap::new(),
            fake,
            rename: None,
            renaming: false,
            rename_note: None,
            origin,
            interval,
            groups: Vec::new(),
            feeds: HashMap::new(),
            previews: model::Previews::default(),
            selected: None,
            page: Page::Group,
            ui: HashMap::new(),
            unfolded: HashSet::new(),
            fail_open: HashSet::new(),
            routines: Routines::default(),
            memories: Book::default(),
            skills: SkillBook::default(),
            mcp: McpBook::default(),
            search: SearchPage::default(),

            review_rules: ReviewRulesPage::default(),
            model_settings: ModelPage::default(),
            conn_form: ConnForm::from_config(conn::load()),

            theme_pref,
            system_mode: Mode::Light,
            width: env_size().map_or(1040.0, |s| s.0),
            loaded: false,
            load_failed: false,
            in_flight: gallery.is_none(),
            has_more: gallery.is_none(),
            error: None,
            gallery,
            scale,
            touch: std::env::var("ROBROZ_TOUCH").is_ok_and(|v| v == "1"),
        };
        // `ROBROZ_SHOT=<file>` (gallery or fake): write the window's RGBA
        // after `ROBROZ_SHOT_DELAY` ms (default 2500), then exit. Used for
        // review screenshots.
        let shot = if std::env::var("ROBROZ_SHOT").is_ok() && fake {
            let ms = std::env::var("ROBROZ_SHOT_DELAY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2500);
            Task::perform(tokio_sleep(Duration::from_millis(ms)), |_| ())
                .then(|_| window::latest())
                .map(Message::Shoot)
        } else {
            Task::none()
        };
        if app.gallery.is_some() {
            return (app, shot);
        }
        let load = blocking(&app.source, |s| s.groups());
        (
            app,
            Task::batch([
                Task::perform(load, Message::Loaded),
                shot,
                iced::system::theme().map(Message::SystemTheme),
            ]),
        )
    }

    /// Whether the host has answered the first group listing.
    fn host_state(&self) -> HostState {
        HostState::of(self.loaded, self.load_failed)
    }

    /// The scheme to draw with: the chosen one, or the system's.
    fn mode(&self) -> Mode {
        self.theme_pref.resolve(self.system_mode)
    }

    fn skin(&self) -> Skin {
        Skin::for_mode(self.mode()).for_input(self.touch, self.width)
    }

    /// One pane at a time: the sidebar is hidden below
    /// [`robroz_ui::CHAT_COMPACT_WIDTH`] and the chat list and a chat take
    /// turns. (The skin's own `narrow` stays the phone-sized 480px rule.)
    fn narrow(&self) -> bool {
        self.width < robroz_ui::CHAT_COMPACT_WIDTH
    }

    fn load_unread(&self) -> Task<Message> {
        Task::perform(call(&self.api, |api| api.unread()), Message::UnreadLoaded)
    }

    fn load_me(&self) -> Task<Message> {
        Task::perform(call(&self.api, |api| api.me()), Message::MeLoaded)
    }

    fn load_bros(&self) -> Task<Message> {
        Task::perform(call(&self.api, |api| api.bros()), Message::BrosLoaded)
    }

    /// Open a chat: select it, drop its badge, tell the host it is read.
    fn open_chat(&mut self, group: String) -> Task<Message> {
        if self.selected.as_deref() != Some(group.as_str()) {
            // A new subscription starts for this chat; poll fast until it is up.
            self.stream_up = false;
        }
        self.selected = Some(group.clone());
        self.page = Page::Group;
        self.narrow_open = true;
        self.mark_read(group)
    }

    fn mark_read(&mut self, group: String) -> Task<Message> {
        self.convs.mark_read(&group);
        let id = group.clone();
        Task::perform(call(&self.api, move |api| api.mark_read(&id)), move |r| {
            Message::MarkedRead(group.clone(), r)
        })
    }

    /// Make a group known everywhere: list, conversation rows, source.
    fn learn_group(&mut self, tab: GroupTab, dm: bool) -> Task<Message> {
        self.convs.upsert(&tab.id, &tab.name, dm);
        match self.groups.iter_mut().find(|g| g.id == tab.id) {
            Some(g) => {
                g.name = tab.name;
                Task::none()
            }
            None => {
                self.groups.push(tab.clone());
                self.has_more = true;
                Task::perform(blocking(&self.source, move |s| s.add_group(tab)), |_| {
                    Message::Tick
                })
            }
        }
    }

    /// The bro whose DM is `group`.
    /// The avatar to draw for bro `id`: stored, else the id default.
    fn avatar_of(&self, id: &str) -> BroAvatar {
        avatar_for(&self.bros, id)
    }

    /// You own this bro (`ownerUserId` == `GET /api/me` id). Unknown
    /// either side means no: the edit entry stays hidden. An admin is not
    /// an exception (robroz `docs/api.md`, bro owners).
    fn owns(&self, bro: &Bro) -> bool {
        match (&self.me, &bro.owner_user_id) {
            (Some(me), Some(owner)) => &me.id == owner,
            _ => false,
        }
    }

    /// Give every known DM row its bro's avatar.
    fn sync_conv_avatars(&mut self) {
        let pairs: Vec<(String, BroAvatar)> = self
            .bros
            .iter()
            .filter_map(|b| {
                b.dm_group_id
                    .clone()
                    .map(|g| (g, avatar::resolve(b.avatar, &b.id)))
            })
            .collect();
        for (g, a) in pairs {
            self.convs.set_avatar(&g, a);
        }
    }

    /// Open the create-bro page with a fresh draft. The starting avatar
    /// is a default from a random seed (a fixed one in fake runs), so the
    /// preview shows exactly what 引入 will store.
    fn open_new_bro(&mut self) -> Task<Message> {
        let seed = if self.fake {
            "new-bro".to_string()
        } else {
            format!(
                "{:?}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
            )
        };
        self.new_bro = NewBroDraft {
            avatar: avatar::default_for(&seed),
            ..NewBroDraft::default()
        };
        self.page = Page::NewBro;
        self.narrow_open = true;
        iced::widget::operation::focus(robroz_ui::avatar_view::NEW_BRO_NAME)
    }

    /// Optimistic avatar save: the preview, feed and list change at once;
    /// a failed save puts the previous avatar back.
    fn save_avatar(&mut self, next: Option<BroAvatar>) -> Task<Message> {
        let Some(id) = self.avatar_bro.clone() else {
            return Task::none();
        };
        let Some(entry) = self.bros.iter_mut().find(|b| b.id == id) else {
            return Task::none();
        };
        let prev = entry.avatar;
        if prev == next {
            return Task::none();
        }
        entry.avatar = next;
        self.avatar_note = None;
        self.avatar_seq += 1;
        let seq = self.avatar_seq;
        self.sync_conv_avatars();
        let bro = id.clone();
        Task::perform(
            call(&self.api, move |api| api.set_avatar(&bro, next)),
            move |result| Message::AvatarSaved {
                bro: id.clone(),
                prev,
                seq,
                result,
            },
        )
    }

    /// `--fake` + `ROBROZ_PAGE=new-bro` (with `ROBROZ_NEW_BRO_NAME`) or
    /// `ROBROZ_PAGE=avatar` (the selected DM's bro; `ROBROZ_AVATAR_NOTE=
    /// owner|failed` shows that inline note): open the page for
    /// screenshots. Runs after bros and after `me` load; once.
    fn page_knob(&mut self) -> Task<Message> {
        if !self.fake || self.page != Page::Group {
            return Task::none();
        }
        match std::env::var("ROBROZ_PAGE").ok().as_deref() {
            Some("new-bro") => {
                let task = self.open_new_bro();
                self.new_bro.name = std::env::var("ROBROZ_NEW_BRO_NAME").unwrap_or_default();
                task
            }
            Some("avatar") => {
                let task = self.on_action(Action::AvatarPageOpen);
                if self.page == Page::BroAvatar {
                    if let Ok(note) = std::env::var("ROBROZ_AVATAR_NOTE") {
                        self.avatar_note = Some(match note.as_str() {
                            "owner" => Text::AvatarNotOwner.into(),
                            _ => Text::AvatarSaveFailed.into(),
                        });
                    }
                }
                task
            }
            _ => Task::none(),
        }
    }

    fn dm_bro(&self, group: &str) -> Option<&Bro> {
        self.bros
            .iter()
            .find(|b| b.dm_group_id.as_deref() == Some(group))
    }

    /// Click on a bro DM's name: edit it in place.
    fn start_rename(&mut self) -> Task<Message> {
        let Some(group) = self.selected.clone() else {
            return Task::none();
        };
        let Some(bro) = self.dm_bro(&group) else {
            return Task::none();
        };
        self.rename = Some((group, bro.name.clone()));
        self.rename_note = None;
        iced::widget::operation::focus(view::RENAME_INPUT)
    }

    fn save_rename(&mut self) -> Task<Message> {
        if self.renaming {
            return Task::none();
        }
        let Some((group, draft)) = self.rename.clone() else {
            return Task::none();
        };
        let Some(bro) = self.dm_bro(&group).cloned() else {
            self.rename = None;
            return Task::none();
        };
        if draft.trim() == bro.name {
            self.rename = None;
            return Task::none();
        }
        self.renaming = true;
        Task::perform(
            call(&self.api, move |api| api.rename_bro(&bro.id, &draft)),
            move |result| Message::Renamed {
                group: group.clone(),
                result,
            },
        )
    }

    /// Composer: one `/say`; the host stores it and picks the bro, if any.
    fn send(&mut self) -> Task<Message> {
        if self.sending {
            return Task::none();
        }
        let Some(group) = self.selected.clone() else {
            return Task::none();
        };
        let draft = self.drafts.get(&group).cloned().unwrap_or_default();
        let body = draft.trim().to_string();
        if body.is_empty() {
            return Task::none();
        }
        self.sending = true;
        let g = group.clone();
        let job = call(&self.api, move |api| api.say(&g, &body));
        Task::perform(job, move |result| Message::Sent {
            group: group.clone(),
            result,
        })
    }

    fn apply_event(&mut self, group: String, event: &Event) {
        self.apply_event_with(group, event, None);
    }

    fn apply_event_with(&mut self, group: String, event: &Event, extras: Option<&ApprovalExtras>) {
        // Settings-page rule cards never belong in a chat, stream or unread.
        if event_decode::is_settings_approval(event) {
            self.sync_review_pending(event, extras);
            return;
        }
        // A confirm card answered anywhere (chat, page, another device, or a
        // forgotten pending memory) moves its entry on the 记忆 page too.
        if let Event::Confirm {
            memory_id: Some(memory),
            decision: Some(d),
            ..
        } = event
        {
            self.memories
                .answered(memory, *d == ApprovalDecision::Approved);
        }
        if let Event::Confirm {
            subject: ConfirmSubject::Skill,
            slug: Some(slug),
            decision: Some(d),
            ..
        } = event
        {
            self.skills.answered(slug, *d == ApprovalDecision::Approved);
        }
        // Newest change time wins the list preview; a tie goes to the
        // outcome (see `model::preview_wins`), not to arrival order.
        let at = change_time(event);
        let shown = self.previews.offer(&group, event);
        self.convs.touch(&group, shown, Some(at));
        self.feeds
            .entry(group)
            .or_default()
            .apply_with(event, extras);
        // A 409 note ("refreshing") goes once the answered card arrives.
        let answered = match event {
            Event::Choice {
                id,
                answered_at: Some(_),
                ..
            }
            | Event::Approval {
                id,
                decision: Some(_),
                ..
            }
            | Event::Confirm {
                id,
                decision: Some(_),
                ..
            } => Some(id),
            _ => None,
        };
        if let Some(ui) = answered.and_then(|id| self.ui.get_mut(id)) {
            if matches!(
                ui.note,
                Some(
                    Note::AlreadyAnswered
                        | Note::AlreadyDecided
                        | Note::Settled
                        | Note::OnlyOwner(_)
                )
            ) {
                ui.note = None;
            }
        }
        self.sync_review_pending(event, extras);
    }

    /// Keep the settings-page confirm card in sync if this event is for it.
    fn sync_review_pending(&mut self, event: &Event, extras: Option<&ApprovalExtras>) {
        let Some(card) = ApprovalCard::from_event_with(event, extras) else {
            return;
        };
        let Some(pending) = self.review_rules.pending.as_ref() else {
            return;
        };
        if pending.id != card.id {
            return;
        }
        match card.state {
            ApprovalState::Expired { at } => self.review_rules.expire_pending(at),
            ApprovalState::Approved { .. } | ApprovalState::Denied { .. } => {
                self.review_rules.finish_pending();
            }
            ApprovalState::Pending | ApprovalState::NeedsOwner => {
                if let Some(pending) = self.review_rules.pending.as_mut() {
                    pending.state = card.state;
                }
            }
        }
    }

    fn refresh_review_rules(&self) -> Task<Message> {
        Task::perform(
            call(&self.api, |api| api.review_rules()),
            Message::ReviewRulesLoaded,
        )
    }

    fn poll(&mut self) -> Task<Message> {
        if self.in_flight || !self.loaded || !self.has_more {
            return Task::none();
        }
        self.in_flight = true;
        let poll = blocking(&self.source, |s| {
            s.poll().map(|events| Polled {
                events,
                cards: s.poll_cards(),
                has_more: s.has_more(),
            })
        });
        Task::perform(poll, Message::Polled)
    }

    /// The card with this id and the group it is in.
    fn find_card(&self, id: &str) -> Option<(String, Card)> {
        self.feeds
            .iter()
            .find_map(|(g, f)| f.card(id).map(|c| (g.clone(), c.clone())))
    }

    fn set_card(&mut self, group: &str, card: Card) {
        self.feeds
            .entry(group.to_string())
            .or_default()
            .apply_card(card);
    }

    fn load_routines(&self) -> Task<Message> {
        let groups: Vec<String> = self.groups.iter().map(|g| g.id.clone()).collect();
        let job = call(&self.api, move |api| {
            let mut all = Vec::new();
            for g in groups {
                all.extend(api.schedules(&g)?);
            }
            Ok(all)
        });
        Task::perform(job, Message::RoutinesLoaded)
    }

    fn load_memories(&mut self) -> Task<Message> {
        self.sync_memory_bros();
        Task::batch([
            Task::perform(
                call(&self.api, |api| api.memories()),
                Message::MemoriesLoaded,
            ),
            self.load_bros(),
        ])
    }

    /// Bro names for 「只给 …」 and the new-memory scope choices.
    fn sync_memory_bros(&mut self) {
        self.memories.bros = self
            .bros
            .iter()
            .map(|b| (b.id.clone(), b.name.clone()))
            .collect();
    }

    fn load_skills(&mut self) -> Task<Message> {
        self.sync_skill_context();
        let groups = self.skills.begin_enable_loads();
        let mut tasks = vec![
            Task::perform(call(&self.api, |api| api.skills()), Message::SkillsLoaded),
            self.load_bros(),
        ];
        tasks.extend(groups.into_iter().map(|g| self.fetch_group_skills(g)));
        Task::batch(tasks)
    }

    fn fetch_group_skills(&self, group: String) -> Task<Message> {
        let key = group.clone();
        let job = call(&self.api, move |api| api.group_skills(&key));
        Task::perform(job, move |result| Message::GroupSkillsLoaded {
            group,
            result,
        })
    }

    /// Fetch enable lists for chats that have not been asked yet.
    fn ensure_group_skill_fetches(&mut self) -> Task<Message> {
        self.sync_skill_context();
        let groups = self.skills.take_groups_to_load();
        Task::batch(groups.into_iter().map(|g| self.fetch_group_skills(g)))
    }

    /// Bro names and chats for the 技能 page (scope labels and toggles).
    fn sync_skill_context(&mut self) {
        self.skills.bros = self
            .bros
            .iter()
            .map(|b| (b.id.clone(), b.name.clone()))
            .collect();
        self.skills.chats = self
            .convs
            .rows()
            .iter()
            .map(|c| (c.group_id.clone(), c.name.clone()))
            .collect();
    }

    fn load_mcp(&mut self) -> Task<Message> {
        self.sync_mcp_context();
        let groups = self.mcp.begin_enable_loads();
        let mut tasks = vec![Task::perform(
            call(&self.api, |api| api.mcp_servers()),
            Message::McpLoaded,
        )];
        tasks.extend(groups.into_iter().map(|g| self.fetch_group_mcp(g)));
        Task::batch(tasks)
    }

    fn fetch_group_mcp(&self, group: String) -> Task<Message> {
        let key = group.clone();
        let job = call(&self.api, move |api| api.group_mcp(&key));
        Task::perform(job, move |result| Message::GroupMcpLoaded { group, result })
    }

    fn ensure_group_mcp_fetches(&mut self) -> Task<Message> {
        self.sync_mcp_context();
        let groups = self.mcp.take_groups_to_load();
        Task::batch(groups.into_iter().map(|g| self.fetch_group_mcp(g)))
    }

    fn sync_mcp_context(&mut self) {
        self.mcp.chats = self
            .convs
            .rows()
            .iter()
            .map(|c| (c.group_id.clone(), c.name.clone()))
            .collect();
    }

    fn fetch_mcp_tools(&self, id: String) -> Task<Message> {
        let key = id.clone();
        let job = call(&self.api, move |api| api.mcp_tools(&key));
        Task::perform(job, move |result| Message::McpToolsLoaded { id, result })
    }

    fn test_mcp(&self, id: String) -> Task<Message> {
        let key = id.clone();
        let job = call(&self.api, move |api| api.test_mcp_server(&key));
        Task::perform(job, move |result| Message::McpTested { id, result })
    }

    fn on_mcp_action(&mut self, action: Action) -> Task<Message> {
        let book = &mut self.mcp;
        match action {
            Action::McpAddOpen => book.open_add(),
            Action::McpFormClose => book.close_form(),
            Action::McpFormName { text } => {
                if let Some(f) = book.form_mut() {
                    f.name = text;
                }
            }
            Action::McpFormTransport { http } => {
                if let Some(f) = book.form_mut() {
                    f.transport = if http {
                        mcp::Transport::Http
                    } else {
                        mcp::Transport::Stdio
                    };
                }
            }
            Action::McpFormCommand { text } => {
                if let Some(f) = book.form_mut() {
                    f.command = text;
                }
            }
            Action::McpFormArgs { text } => {
                if let Some(f) = book.form_mut() {
                    f.args = text;
                }
            }
            Action::McpFormUrl { text } => {
                if let Some(f) = book.form_mut() {
                    f.url = text;
                }
            }
            Action::McpSecretName { index, text } => {
                if let Some(f) = book.form_mut() {
                    if let Some(s) = f.secrets.get_mut(index) {
                        s.name = text;
                    }
                }
            }
            Action::McpSecretPlacement { index, header } => {
                if let Some(f) = book.form_mut() {
                    if let Some(s) = f.secrets.get_mut(index) {
                        s.placement = if header {
                            robroz_contract::McpSecretIn::Header
                        } else {
                            robroz_contract::McpSecretIn::Env
                        };
                    }
                }
            }
            Action::McpSecretKey { index, text } => {
                if let Some(f) = book.form_mut() {
                    if let Some(s) = f.secrets.get_mut(index) {
                        s.key.draft = text;
                    }
                }
            }
            Action::McpSecretClear { index } => {
                if let Some(f) = book.form_mut() {
                    if let Some(s) = f.secrets.get_mut(index) {
                        s.key.clear();
                    }
                }
            }
            Action::McpSecretCancel { index } => {
                if let Some(f) = book.form_mut() {
                    if let Some(s) = f.secrets.get_mut(index) {
                        s.key.cancel();
                    }
                }
            }
            Action::McpSecretAdd => book.add_secret(),
            Action::McpSecretRemove { index } => {
                if let Some(f) = book.form_mut() {
                    if index < f.secrets.len() {
                        f.secrets.remove(index);
                    }
                }
            }
            Action::McpSave => {
                let Some(save) = book.save_to_send() else {
                    return Task::none();
                };
                let job = call(&self.api, move |api| {
                    if save.id.is_some() {
                        api.update_mcp_server(&save)
                    } else {
                        api.create_mcp_server(&save)
                    }
                });
                return Task::perform(job, move |result| Message::McpSaved { result });
            }
            Action::McpEdit { id } => book.start_edit(&id),
            Action::McpOpen { id } => {
                if book.open_server(&id) {
                    return self.fetch_mcp_tools(id);
                }
            }
            Action::McpDeleteAsk { id } => book.ask_delete(&id),
            Action::McpDeleteCancel => book.cancel_delete(),
            Action::McpDeleteConfirm => {
                let Some(id) = book.delete_to_send() else {
                    return Task::none();
                };
                let key = id.clone();
                let job = call(&self.api, move |api| api.delete_mcp_server(&key));
                return Task::perform(job, move |result| Message::McpDeleted {
                    id: id.clone(),
                    result,
                });
            }
            Action::McpToggleEnabled { id } => {
                let Some(on) = book.toggle_enabled_to_send(&id) else {
                    return Task::none();
                };
                let key = id.clone();
                let job = call(&self.api, move |api| api.set_mcp_enabled(&key, on));
                return Task::perform(job, move |result| Message::McpEnabled {
                    id: id.clone(),
                    result,
                });
            }
            Action::McpToggleGroup { id, group } => {
                let Some(on) = book.toggle_group_to_send(&id, &group) else {
                    return Task::none();
                };
                let (sid, gid) = (id.clone(), group.clone());
                let job = call(&self.api, move |api| api.set_group_mcp(&gid, &sid, on));
                return Task::perform(job, move |result| Message::McpGroupToggled {
                    id: id.clone(),
                    group: group.clone(),
                    on,
                    result,
                });
            }
            Action::McpApproveCommand { id } => {
                let Some(key) = book.approve_command_to_send(&id) else {
                    return Task::none();
                };
                let job = call(&self.api, move |api| api.approve_mcp_command(&key));
                return Task::perform(job, move |result| Message::McpCommandApproved {
                    id: id.clone(),
                    result,
                });
            }
            Action::McpCancelCommand { id } => book.cancel_command_prompt(&id),
            _ => {}
        }
        Task::none()
    }

    fn on_memory_action(&mut self, action: Action) -> Task<Message> {
        let book = &mut self.memories;
        match action {
            Action::MemoryAddOpen => book.open_add(),
            Action::MemoryAddClose => book.close_add(),
            Action::MemoryAddText { text } => {
                if let Some(new) = &mut book.adding {
                    new.text = text;
                }
            }
            Action::MemoryAddScope { scope } => {
                if let Some(new) = &mut book.adding {
                    new.scope = Some(scope);
                }
            }
            Action::MemoryAddKind { kind } => {
                if let Some(new) = &mut book.adding {
                    new.kind = kind;
                }
            }
            Action::MemoryAddSave => {
                if book.busy {
                    return Task::none();
                }
                let Some(new) = book.add_to_send() else {
                    return Task::none();
                };
                book.busy = true;
                let scope = new.scope.clone().unwrap_or_default();
                let job = call(&self.api, move |api| {
                    api.create_memory(&scope, new.kind, &new.text)
                });
                return Task::perform(job, |result| Message::MemorySaved { id: None, result });
            }
            Action::MemoryEdit { id } => book.start_edit(&id),
            Action::MemoryEditText { text } => book.edit_text(text),
            Action::MemoryEditCancel => book.cancel_edit(),
            Action::MemoryEditSave => {
                if book.busy {
                    return Task::none();
                }
                let Some((id, text)) = book.edit_to_save() else {
                    return Task::none();
                };
                book.busy = true;
                let mid = id.clone();
                let job = call(&self.api, move |api| api.update_memory(&mid, &text));
                return Task::perform(job, move |result| Message::MemorySaved {
                    id: Some(id.clone()),
                    result,
                });
            }
            Action::MemoryDecide { id, keep } => {
                let Some(card) = book.decide_to_send(&id) else {
                    return Task::none();
                };
                let job = call(&self.api, move |api| api.decide(&card, keep));
                return Task::perform(job, move |result| Message::MemoryDecided {
                    id: id.clone(),
                    keep,
                    result,
                });
            }
            Action::MemoryForgetAsk { id } => book.ask_forget(&id),
            Action::MemoryForgetCancel => book.cancel_forget(),
            Action::MemoryForgetConfirm => {
                if book.busy {
                    return Task::none();
                }
                let Some(id) = book.forget_to_send() else {
                    return Task::none();
                };
                book.busy = true;
                let rid = id.clone();
                let job = call(&self.api, move |api| api.forget_memory(&rid));
                return Task::perform(job, move |result| Message::MemoryForgot {
                    id: id.clone(),
                    result,
                });
            }
            _ => {}
        }
        Task::none()
    }

    fn on_skill_action(&mut self, action: Action) -> Task<Message> {
        let book = &mut self.skills;
        match action {
            Action::SkillAddOpen => book.open_add(),
            Action::SkillAddClose => book.close_add(),
            Action::SkillAddSlug { text } => {
                if let Some(new) = &mut book.adding {
                    new.slug = text;
                }
            }
            Action::SkillAddTitle { text } => {
                if let Some(new) = &mut book.adding {
                    new.title = text;
                }
            }
            Action::SkillAddInstruction { text } => {
                if let Some(new) = &mut book.adding {
                    new.instruction = text;
                }
            }
            Action::SkillAddScope { scope } => {
                if let Some(new) = &mut book.adding {
                    new.scope = Some(scope);
                }
            }
            Action::SkillAddSave => {
                if book.busy {
                    return Task::none();
                }
                let Some(new) = book.add_to_send() else {
                    return Task::none();
                };
                book.busy = true;
                let scope = new.scope.clone().unwrap_or_default();
                let job = call(&self.api, move |api| {
                    api.create_skill(&scope, &new.slug, &new.title, &new.instruction)
                });
                return Task::perform(job, |result| Message::SkillSaved { slug: None, result });
            }
            Action::SkillEdit { slug } => book.start_edit(&slug),
            Action::SkillEditTitle { text } => book.edit_title(text),
            Action::SkillEditInstruction { text } => book.edit_instruction(text),
            Action::SkillEditCancel => book.cancel_edit(),
            Action::SkillEditSave => {
                if book.busy {
                    return Task::none();
                }
                let Some((slug, title, instruction)) = book.edit_to_save() else {
                    return Task::none();
                };
                book.busy = true;
                let key = slug.clone();
                let job = call(&self.api, move |api| {
                    api.update_skill(&key, &title, &instruction)
                });
                return Task::perform(job, move |result| Message::SkillSaved {
                    slug: Some(slug.clone()),
                    result,
                });
            }
            Action::SkillDecide { slug, keep } => {
                let Some(card) = book.decide_to_send(&slug) else {
                    return Task::none();
                };
                let job = call(&self.api, move |api| api.decide(&card, keep));
                return Task::perform(job, move |result| Message::SkillDecided {
                    slug: slug.clone(),
                    keep,
                    result,
                });
            }
            Action::SkillDeleteAsk { slug } => book.ask_delete(&slug),
            Action::SkillDeleteCancel => book.cancel_delete(),
            Action::SkillDeleteConfirm => {
                if book.busy {
                    return Task::none();
                }
                let Some(slug) = book.delete_to_send() else {
                    return Task::none();
                };
                book.busy = true;
                let key = slug.clone();
                let job = call(&self.api, move |api| api.forget_skill(&key));
                return Task::perform(job, move |result| Message::SkillForgot {
                    slug: slug.clone(),
                    result,
                });
            }
            Action::SkillToggleInstruction { slug } => book.toggle_instruction(&slug),
            Action::SkillToggleGroup { slug, group } => {
                let Some(on) = book.toggle_to_send(&slug, &group) else {
                    return Task::none();
                };
                let (s, g) = (slug.clone(), group.clone());
                let job = call(&self.api, move |api| api.set_skill_enabled(&g, &s, on));
                return Task::perform(job, move |result| Message::SkillToggled {
                    slug: slug.clone(),
                    group: group.clone(),
                    on,
                    result,
                });
            }
            _ => {}
        }
        Task::none()
    }

    fn on_search_action(&mut self, action: Action) -> Task<Message> {
        let page = &mut self.search;
        match action {
            Action::SearchPick { provider } => page.pick(provider),
            Action::SearchUrl { text } => page.url = text,
            Action::SearchKey { text } => page.key.draft = text,
            Action::SearchClearKey => page.clear_key(),
            Action::SearchCancelKey => page.cancel_key(),
            Action::SearchSave => {
                if page.busy {
                    return Task::none();
                }
                let Some(put) = page.to_send() else {
                    return Task::none();
                };
                page.busy = true;
                page.test = None;
                return Task::perform(
                    call(&self.api, move |api| api.put_search(&put)),
                    Message::SearchSaved,
                );
            }
            _ => {}
        }
        Task::none()
    }

    fn on_review_action(&mut self, action: Action) -> Task<Message> {
        let page = &mut self.review_rules;
        match action {
            Action::ReviewDraft { text } => page.set_draft(text),
            Action::ReviewAdd => {
                if page.locked() {
                    return Task::none();
                }
                let Some(text) = page.to_add() else {
                    return Task::none();
                };
                page.begin_add(text.clone());
                return Task::perform(
                    call(&self.api, move |api| {
                        let id = api.propose_review_rule(&text)?;
                        api.approval(&id)
                    }),
                    Message::ReviewRuleProposed,
                );
            }
            Action::ReviewDelete { id } => {
                if page.locked() {
                    return Task::none();
                }
                let Some(text) = page
                    .rules
                    .as_ref()
                    .and_then(|rules| rules.iter().find(|r| r.id == id))
                    .map(|r| r.text.clone())
                else {
                    return Task::none();
                };
                page.begin_delete(id.clone(), text);
                return Task::perform(
                    call(&self.api, move |api| {
                        let id = api.propose_delete_review_rule(&id)?;
                        api.approval(&id)
                    }),
                    Message::ReviewRuleProposed,
                );
            }
            _ => {}
        }
        Task::none()
    }

    fn on_model_action(&mut self, action: Action) -> Task<Message> {
        let page = &mut self.model_settings;
        match action {
            Action::ModelPick { preset } => {
                if page.busy {
                    return Task::none();
                }
                page.pick(preset);
            }
            Action::ModelUrl { text } => page.set_url(text),
            Action::ModelName { text } => page.set_model(text),
            Action::ModelKey { text } => page.key.draft = text,
            Action::ModelClearKey => {
                if !page.busy {
                    page.clear_key();
                }
            }
            Action::ModelCancelKey => page.cancel_key(),
            Action::ModelSave => {
                if page.busy {
                    return Task::none();
                }
                let Some(put) = page.to_send() else {
                    return Task::none();
                };
                page.busy = true;
                page.test = None;
                return Task::perform(
                    call(&self.api, move |api| api.put_model(&put)),
                    Message::ModelSaved,
                );
            }
            _ => {}
        }
        Task::none()
    }

    fn on_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::MemoryAddOpen
            | Action::MemoryAddClose
            | Action::MemoryAddText { .. }
            | Action::MemoryAddScope { .. }
            | Action::MemoryAddKind { .. }
            | Action::MemoryAddSave
            | Action::MemoryEdit { .. }
            | Action::MemoryEditText { .. }
            | Action::MemoryEditSave
            | Action::MemoryEditCancel
            | Action::MemoryForgetAsk { .. }
            | Action::MemoryForgetConfirm
            | Action::MemoryForgetCancel
            | Action::MemoryDecide { .. } => self.on_memory_action(action),
            Action::SkillAddOpen
            | Action::SkillAddClose
            | Action::SkillAddSlug { .. }
            | Action::SkillAddTitle { .. }
            | Action::SkillAddInstruction { .. }
            | Action::SkillAddScope { .. }
            | Action::SkillAddSave
            | Action::SkillEdit { .. }
            | Action::SkillEditTitle { .. }
            | Action::SkillEditInstruction { .. }
            | Action::SkillEditSave
            | Action::SkillEditCancel
            | Action::SkillDeleteAsk { .. }
            | Action::SkillDeleteConfirm
            | Action::SkillDeleteCancel
            | Action::SkillDecide { .. }
            | Action::SkillToggleInstruction { .. }
            | Action::SkillToggleGroup { .. } => self.on_skill_action(action),
            Action::McpAddOpen
            | Action::McpFormClose
            | Action::McpFormName { .. }
            | Action::McpFormTransport { .. }
            | Action::McpFormCommand { .. }
            | Action::McpFormArgs { .. }
            | Action::McpFormUrl { .. }
            | Action::McpSecretName { .. }
            | Action::McpSecretPlacement { .. }
            | Action::McpSecretKey { .. }
            | Action::McpSecretClear { .. }
            | Action::McpSecretCancel { .. }
            | Action::McpSecretAdd
            | Action::McpSecretRemove { .. }
            | Action::McpSave
            | Action::McpEdit { .. }
            | Action::McpOpen { .. }
            | Action::McpDeleteAsk { .. }
            | Action::McpDeleteConfirm
            | Action::McpDeleteCancel
            | Action::McpToggleEnabled { .. }
            | Action::McpToggleGroup { .. }
            | Action::McpApproveCommand { .. }
            | Action::McpCancelCommand { .. } => self.on_mcp_action(action),
            Action::SearchPick { .. }
            | Action::SearchUrl { .. }
            | Action::SearchKey { .. }
            | Action::SearchClearKey
            | Action::SearchCancelKey
            | Action::SearchSave => self.on_search_action(action),

            Action::ReviewDraft { .. } | Action::ReviewAdd | Action::ReviewDelete { .. } => {
                self.on_review_action(action)
            }
            Action::ModelPick { .. }
            | Action::ModelUrl { .. }
            | Action::ModelName { .. }
            | Action::ModelKey { .. }
            | Action::ModelClearKey
            | Action::ModelCancelKey
            | Action::ModelSave => self.on_model_action(action),

            Action::ToggleInstruction { card } => {
                let ui = self.ui.entry(card).or_default();
                ui.more_open = !ui.more_open;
                Task::none()
            }
            Action::UnfoldConfirms { task } => {
                self.unfolded.insert(task);
                Task::none()
            }
            Action::Choose { card, option } => {
                let Some((_, Card::Choice(c))) = self.find_card(&card) else {
                    return Task::none();
                };
                let ui = self.ui.entry(card.clone()).or_default();
                let Some(ids) = ui.begin_choice(&c, Some(&option)) else {
                    return Task::none();
                };
                self.send_choice(card, ids)
            }
            Action::TogglePick { card, option } => {
                if let Some((_, Card::Choice(c))) = self.find_card(&card) {
                    self.ui.entry(card).or_default().toggle_pick(&c, &option);
                }
                Task::none()
            }
            Action::ConfirmChoice { card } => {
                let Some((_, Card::Choice(c))) = self.find_card(&card) else {
                    return Task::none();
                };
                let ui = self.ui.entry(card.clone()).or_default();
                let Some(ids) = ui.begin_choice(&c, None) else {
                    return Task::none();
                };
                self.send_choice(card, ids)
            }
            Action::OpenApproval { group, card } => {
                // Open the group, then (once it is on screen) scroll to the
                // card and flash its stroke.
                let open = self.open_chat(group);
                let jump = Task::perform(tokio_sleep(Duration::from_millis(80)), move |_| {
                    Message::JumpTo(card.clone())
                });
                Task::batch([open, jump])
            }
            Action::Decide { card, approve } => {
                if !self.ui.entry(card.clone()).or_default().begin_action() {
                    return Task::none();
                }
                if self
                    .review_rules
                    .pending
                    .as_ref()
                    .is_some_and(|p| p.id == card)
                {
                    self.review_rules.busy = true;
                }
                let id = card.clone();
                let job = call(&self.api, move |api| api.decide(&id, approve));
                Task::perform(job, move |result| Message::Decided {
                    card: card.clone(),
                    approve,
                    result,
                })
            }
            // Drafts are phase 2: the button is not drawn against a real host,
            // and a stray action is dropped rather than sent to a 404.
            Action::SendDraft { .. } | Action::EditDraftInDm { .. } if !self.caps.draft_actions => {
                Task::none()
            }
            Action::SendDraft { card } => {
                if !self.ui.entry(card.clone()).or_default().begin_action() {
                    return Task::none();
                }
                let id = card.clone();
                let job = call(&self.api, move |api| api.send_draft(&id));
                Task::perform(job, move |result| Message::Acted {
                    card: format!(">{card}"),
                    result,
                })
            }
            // Opening a bot's DM is not wired yet; this only logs.
            Action::EditDraftInDm { card } => {
                eprintln!("修改: open DM with draft {card} (not wired yet)");
                Task::none()
            }
            Action::TakeOver { card } => {
                eprintln!("去私聊接管: card {card} (not wired yet)");
                Task::none()
            }
            Action::ToggleBotName { card } => {
                if let Some(g) = &mut self.gallery {
                    g.toggle_name(&card);
                }
                let ui = self.ui.entry(card).or_default();
                ui.name_expanded = !ui.name_expanded;
                Task::none()
            }
            Action::OpenChat { group } => self.open_chat(group),
            Action::BackToList => {
                self.narrow_open = false;
                Task::none()
            }
            Action::ComposerEdit { text } => {
                if let Some(g) = self.selected.clone() {
                    self.send_note.remove(&g);
                    self.drafts.insert(g, text);
                }
                Task::none()
            }
            Action::ComposerSend => self.send(),
            Action::RenameStart => self.start_rename(),
            Action::RenameEdit { text } => {
                if let Some((_, draft)) = &mut self.rename {
                    *draft = text;
                }
                self.rename_note = None;
                Task::none()
            }
            Action::RenameCancel => {
                if !self.renaming {
                    self.rename = None;
                    self.rename_note = None;
                }
                Task::none()
            }
            Action::RenameSave => self.save_rename(),
            Action::NewBro => self.open_new_bro(),
            Action::NewBroName { text } => {
                if !self.new_bro.busy {
                    self.new_bro.name = text;
                }
                Task::none()
            }
            Action::NewBroCreate => {
                let name = self.new_bro.name.trim().to_string();
                if self.new_bro.busy || name.is_empty() {
                    return Task::none();
                }
                self.new_bro.busy = true;
                self.new_bro.note = None;
                let a = self.new_bro.avatar;
                Task::perform(
                    call(&self.api, move |api| api.create_bro(Some(&name), Some(a))),
                    Message::BroCreated,
                )
            }
            Action::AvatarPageClose => {
                self.page = Page::Group;
                self.avatar_bro = None;
                self.avatar_note = None;
                if self.selected.is_none() {
                    self.narrow_open = false;
                }
                Task::none()
            }
            Action::AvatarPageOpen => {
                let Some(group) = self.selected.clone() else {
                    return Task::none();
                };
                let Some(bro) = self.dm_bro(&group).cloned() else {
                    return Task::none();
                };
                if !self.owns(&bro) {
                    return Task::none();
                }
                self.avatar_bro = Some(bro.id.clone());
                self.avatar_note = None;
                self.page = Page::BroAvatar;
                self.narrow_open = true;
                Task::none()
            }
            Action::AvatarShape(shape) => {
                if self.page == Page::NewBro {
                    if !self.new_bro.busy {
                        self.new_bro.avatar.shape = shape;
                    }
                    return Task::none();
                }
                let Some(id) = self.avatar_bro.clone() else {
                    return Task::none();
                };
                let cur = self.avatar_of(&id);
                self.save_avatar(Some(BroAvatar::new(shape, cur.color)))
            }
            Action::AvatarColor(c) => {
                if self.page == Page::NewBro {
                    if !self.new_bro.busy {
                        self.new_bro.avatar.color = c;
                    }
                    return Task::none();
                }
                let Some(id) = self.avatar_bro.clone() else {
                    return Task::none();
                };
                let cur = self.avatar_of(&id);
                self.save_avatar(Some(BroAvatar::new(cur.shape, c)))
            }
            Action::AvatarReset => self.save_avatar(None),
            Action::TogglePause { id } => {
                let Some(wanted) = self.routines.toggle(&id) else {
                    return Task::none();
                };
                let rid = id.clone();
                let job = call(&self.api, move |api| api.set_paused(&rid, wanted));
                Task::perform(job, move |result| Message::Paused {
                    id: id.clone(),
                    wanted,
                    result,
                })
            }
        }
    }

    fn send_choice(&self, card: String, ids: Vec<String>) -> Task<Message> {
        let (id, body) = (card.clone(), ids.clone());
        let job = call(&self.api, move |api| api.answer_choice(&id, &body));
        Task::perform(job, move |result| Message::Answered {
            card: card.clone(),
            sent: ids.clone(),
            result,
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenRoutines => {
                self.page = Page::Routines;
                self.narrow_open = true;
                return self.load_routines();
            }
            Message::OpenMemories => {
                self.page = Page::Memories;
                self.narrow_open = true;
                return self.load_memories();
            }
            Message::OpenSearch => {
                self.page = Page::Search;
                self.narrow_open = true;
                self.search.test = None;
                self.search.note = None;
                return Task::perform(
                    call(&self.api, |api| api.search_settings()),
                    Message::SearchLoaded,
                );
            }
            Message::SearchLoaded(result) => match result {
                Ok(cur) => self.search.load(cur),
                Err(err) => self.search.note = Some(err.reason()),
            },
            Message::SearchSaved(result) => match result {
                Ok(cur) => {
                    self.search.saved(cur);
                    return Task::perform(
                        call(&self.api, |api| api.test_search()),
                        Message::SearchTested,
                    );
                }
                Err(err) => self.search.save_failed(&err.reason()),
            },
            Message::SearchTested(result) => match result {
                Ok((ok, error)) => self.search.tested(ok, error),
                Err(err) => self.search.tested(false, Some(err.reason())),
            },
            Message::OpenModel => {
                self.page = Page::Model;
                self.narrow_open = true;
                self.model_settings.test = None;
                self.model_settings.note = None;
                return Task::perform(
                    call(&self.api, |api| api.model_settings()),
                    Message::ModelLoaded,
                );
            }
            Message::ModelLoaded(result) => match result {
                Ok(cur) => self.model_settings.load(cur),
                Err(err) => self.model_settings.note = Some(err.reason()),
            },
            Message::ModelSaved(result) => match result {
                Ok(cur) => {
                    self.model_settings.saved(cur);
                    return Task::perform(
                        call(&self.api, |api| api.test_model()),
                        Message::ModelTested,
                    );
                }
                Err(err) => self.model_settings.save_failed(&err.reason()),
            },
            Message::ModelTested(result) => match result {
                Ok((ok, error)) => self.model_settings.tested(ok, error),
                Err(err) => self.model_settings.tested(false, Some(err.reason())),
            },
            Message::OpenSkills => {
                self.page = Page::Skills;
                self.narrow_open = true;
                return self.load_skills();
            }
            Message::OpenMcp => {
                self.page = Page::Mcp;
                self.narrow_open = true;
                return self.load_mcp();
            }
            Message::OpenReviewRules => {
                self.page = Page::ReviewRules;
                self.narrow_open = true;
                self.review_rules.reset_for_open();
                return self.refresh_review_rules();
            }
            Message::ReviewRulesLoaded(result) => match result {
                Ok(rules) => self.review_rules.load(rules),
                Err(err) => self.review_rules.load_failed(&err.reason()),
            },
            Message::ReviewRuleProposed(result) => match result {
                Ok(event) => self.review_rules.proposed_event(&event),
                Err(err) => self.review_rules.propose_failed(&err.reason()),
            },
            Message::MemoriesLoaded(result) => match result {
                Ok(list) => {
                    self.memories.load(list);
                    self.memories.note = None;
                }
                Err(err) => self.memories.note = Some(err.reason()),
            },
            Message::MemorySaved { id, result } => match (id, result) {
                (None, Ok(m)) => self.memories.added(m),
                (Some(_), Ok(m)) => self.memories.saved(m),
                (None, Err(err)) => self.memories.add_failed(&err.reason()),
                (Some(id), Err(err)) => self.memories.entry_failed(&id, &err.reason()),
            },
            Message::MemoryDecided { id, keep, result } => match result {
                Ok(reply) => {
                    self.memories.decided(&id, keep);
                    // The chat card shows the answer at once; the stream
                    // brings the same snapshot again later.
                    if let Some((event @ Event::Confirm { .. }, extras)) = reply {
                        let card = robroz_ui::cards::confirm::ConfirmCard::from_event(&event);
                        let group = card.and_then(|c| self.find_card(&c.id)).map(|(g, _)| g);
                        if let Some(group) = group {
                            self.apply_event_with(group, &event, extras.as_ref());
                        }
                    }
                }
                Err(ApiError::Conflict(_)) => {
                    // Answered elsewhere: say so here and fetch the real state.
                    self.memories
                        .decide_failed(&id, confirm::ALREADY_ANSWERED.as_str());
                    return Task::perform(
                        call(&self.api, |api| api.memories()),
                        Message::MemoriesLoaded,
                    );
                }
                Err(err) => self
                    .memories
                    .decide_failed(&id, &confirm::decide_failed(&err.reason())),
            },
            Message::MemoryForgot { id, result } => match result {
                Ok(()) | Err(ApiError::NotFound(_)) => self.memories.forgotten(&id),
                Err(err) => self.memories.entry_failed(&id, &err.reason()),
            },
            Message::SkillsLoaded(result) => {
                self.sync_skill_context();
                match result {
                    Ok(list) => {
                        self.skills.load(list);
                        self.skills.note = None;
                    }
                    Err(err) => self.skills.note = Some(err.reason()),
                }
                return self.ensure_group_skill_fetches();
            }
            Message::SkillSaved { slug, result } => match (slug, result) {
                (None, Ok(s)) => self.skills.added(s),
                (Some(_), Ok(s)) => self.skills.saved(s),
                (None, Err(err)) => self.skills.add_failed(&err.reason()),
                (Some(slug), Err(err)) => self.skills.entry_failed(&slug, &err.reason()),
            },
            Message::SkillDecided { slug, keep, result } => match result {
                Ok(reply) => {
                    self.skills.decided(&slug, keep);
                    if let Some((event @ Event::Confirm { .. }, extras)) = reply {
                        let card = robroz_ui::cards::confirm::ConfirmCard::from_event(&event);
                        let group = card.and_then(|c| self.find_card(&c.id)).map(|(g, _)| g);
                        if let Some(group) = group {
                            self.apply_event_with(group, &event, extras.as_ref());
                        }
                    }
                }
                Err(ApiError::Conflict(_)) => {
                    self.skills
                        .decide_failed(&slug, confirm::ALREADY_ANSWERED.as_str());
                    return Task::perform(
                        call(&self.api, |api| api.skills()),
                        Message::SkillsLoaded,
                    );
                }
                Err(err) => self
                    .skills
                    .decide_failed(&slug, &confirm::decide_failed(&err.reason())),
            },
            Message::SkillForgot { slug, result } => match result {
                Ok(()) | Err(ApiError::NotFound(_)) => self.skills.deleted(&slug),
                Err(err) => self.skills.entry_failed(&slug, &err.reason()),
            },
            Message::SkillToggled {
                slug,
                group,
                on,
                result,
            } => match result {
                Ok(()) => self.skills.toggled(&slug, &group, on),
                Err(err) => self.skills.entry_failed(&slug, &err.reason()),
            },
            Message::GroupSkillsLoaded { group, result } => match result {
                Ok(list) => self.skills.apply_group_skills(&group, &list),
                Err(_) => self.skills.group_skills_failed(&group),
            },
            Message::McpLoaded(result) => {
                self.sync_mcp_context();
                match result {
                    Ok(list) => {
                        self.mcp.load(list);
                        self.mcp.note = None;
                    }
                    Err(err) => self.mcp.note = Some(err.reason()),
                }
                return self.ensure_group_mcp_fetches();
            }
            Message::McpSaved { result } => match result {
                Ok(server) => {
                    let id = server.id.clone();
                    self.mcp.saved(server);
                    return Task::batch([self.test_mcp(id.clone()), self.fetch_mcp_tools(id)]);
                }
                Err(err) => self.mcp.save_failed(&err.reason()),
            },
            Message::McpDeleted { id, result } => match result {
                Ok(()) | Err(ApiError::NotFound(_)) => self.mcp.deleted(&id),
                Err(err) => self.mcp.delete_failed(&id, &err.reason()),
            },
            Message::McpToolsLoaded { id, result } => match result {
                Ok(tools) => self.mcp.tools_loaded(&id, tools),
                Err(err) => {
                    if let Some(denied) = err.command_not_approved() {
                        self.mcp.show_command_prompt(
                            &id,
                            denied.command.clone(),
                            denied.args.clone(),
                        );
                    } else {
                        self.mcp.tools_failed(&id, &err.reason());
                    }
                }
            },
            Message::McpTested { id, result } => match result {
                Ok(test) => self.mcp.tested(&id, &test),
                Err(err) => {
                    if let Some(denied) = err.command_not_approved() {
                        self.mcp.show_command_prompt(
                            &id,
                            denied.command.clone(),
                            denied.args.clone(),
                        );
                    } else {
                        self.mcp.test_failed(&id, &err.reason());
                    }
                }
            },
            Message::McpCommandApproved { id, result } => match result {
                Ok(server) => {
                    let sid = server.id.clone();
                    self.mcp.command_approved(server);
                    return Task::batch([self.test_mcp(sid.clone()), self.fetch_mcp_tools(sid)]);
                }
                Err(err) => self.mcp.approve_command_failed(&id, &err.reason()),
            },
            Message::McpEnabled { id, result } => match result {
                Ok(server) => self.mcp.set_enabled(&server.id, server.enabled),
                Err(err) => self.mcp.entry_failed(&id, &err.reason()),
            },
            Message::McpGroupToggled {
                id,
                group,
                on,
                result,
            } => match result {
                Ok(()) => self.mcp.group_toggled(&id, &group, on),
                Err(err) => self.mcp.entry_failed(&id, &err.reason()),
            },
            Message::GroupMcpLoaded { group, result } => match result {
                Ok(list) => self.mcp.apply_group_mcp(&group, &list),
                Err(_) => self.mcp.group_mcp_failed(&group),
            },
            Message::Back => self.narrow_open = false,
            Message::UnreadTick => return self.load_unread(),
            Message::UnreadLoaded(result) => match result {
                Ok(list) => {
                    let mut unknown = false;
                    let mut read_open = None;
                    for u in &list {
                        unknown |= !self.groups.iter().any(|g| g.id == u.group_id);
                        self.convs.apply_unread(
                            &u.group_id,
                            u.unread,
                            u.last_activity_at.as_deref(),
                        );
                        // New things in the chat you are looking at are read.
                        let looking = self.page == Page::Group
                            && self.selected.as_deref() == Some(u.group_id.as_str())
                            && (!self.narrow() || self.narrow_open);
                        if looking && u.unread > 0 {
                            read_open = Some(u.group_id.clone());
                        }
                    }
                    let mut tasks = Vec::new();
                    if let Some(g) = read_open {
                        tasks.push(self.mark_read(g));
                    }
                    if unknown {
                        // A group or DM made elsewhere: fetch names, start polling it.
                        tasks.push(Task::perform(
                            blocking(&self.source, |s| s.groups()),
                            Message::GroupsReloaded,
                        ));
                        tasks.push(self.load_bros());
                    }
                    if self.page == Page::Skills {
                        tasks.push(self.ensure_group_skill_fetches());
                    }
                    if self.page == Page::Mcp {
                        tasks.push(self.ensure_group_mcp_fetches());
                    }
                    return Task::batch(tasks);
                }
                // The list keeps its last state; the footer shows why.
                Err(err) => self.error = Some(err.note().text()),
            },
            Message::MarkedRead(group, result) => {
                if let Ok(u) = result {
                    self.convs
                        .apply_unread(&group, u.unread, u.last_activity_at.as_deref());
                }
            }
            Message::MeLoaded(result) => {
                // An error leaves it unknown: the group card then shows no
                // buttons (a click would only be refused).
                self.me = result.ok();
                return self.page_knob();
            }
            Message::JumpTo(card) => {
                self.ui.entry(card.clone()).or_default().flash = true;
                let off = card.clone();
                return Task::batch([
                    jump::to_card(iced::widget::Id::new(FEED_ID), view::card_anchor(&card)),
                    Task::perform(
                        tokio_sleep(Duration::from_millis(flash_ms(self.fake))),
                        move |_| Message::FlashOff(off.clone()),
                    ),
                ]);
            }
            Message::FlashOff(card) => {
                if let Some(ui) = self.ui.get_mut(&card) {
                    ui.flash = false;
                }
            }
            Message::BrosLoaded(result) => {
                if let Ok(bros) = result {
                    self.bros = bros.clone();
                    self.sync_memory_bros();
                    self.sync_conv_avatars();
                    // `--fake` + `ROBROZ_RENAME=<draft>`: open the rename box
                    // on the selected DM (screenshots).
                    let mut tasks: Vec<_> = bros
                        .into_iter()
                        .filter_map(|b| {
                            b.dm_group_id.map(|id| GroupTab { id, name: b.name })
                        })
                        .map(|tab| self.learn_group(tab, true))
                        .collect();
                    if let Some(draft) = std::env::var("ROBROZ_RENAME").ok().filter(|_| self.fake) {
                        if let Some(g) = self.selected.clone().filter(|g| self.dm_bro(g).is_some())
                        {
                            self.rename = Some((g, draft));
                            // `ROBROZ_RENAME_SAVE=1`: also press 保存.
                            if std::env::var("ROBROZ_RENAME_SAVE").is_ok_and(|v| v == "1") {
                                tasks.push(self.save_rename());
                            }
                        }
                    }
                    tasks.push(self.page_knob());
                    return Task::batch(tasks);
                }
            }
            Message::BroCreated(result) => {
                self.new_bro.busy = false;
                match result {
                    Ok(bro) => {
                        if !self.bros.iter().any(|b| b.id == bro.id) {
                            self.bros.push(bro.clone());
                        }
                        self.new_bro = NewBroDraft::default();
                        self.page = Page::Group;
                        let Some(dm) = bro.dm_group_id.clone() else {
                            return Task::none();
                        };
                        let learn = self.learn_group(
                            GroupTab {
                                id: dm.clone(),
                                name: bro.name.clone(),
                            },
                            true,
                        );
                        let now = chrono::Utc::now().to_rfc3339();
                        self.convs.touch(&dm, None, Some(&now));
                        self.sync_conv_avatars();
                        let open = self.open_chat(dm);
                        return Task::batch([learn, open]);
                    }
                    Err(err) => {
                        self.new_bro.note = Some(Text::ClientNewBroFailed.fill(&[&err.reason()]))
                    }
                }
            }
            Message::AvatarSaved {
                bro,
                prev,
                seq,
                result,
            } => {
                if seq != self.avatar_seq {
                    return Task::none();
                }
                match result {
                    Ok(entry) => {
                        if let Some(b) = self.bros.iter_mut().find(|b| b.id == entry.id) {
                            *b = entry;
                        }
                        self.avatar_note = None;
                    }
                    Err(err) => {
                        if let Some(b) = self.bros.iter_mut().find(|b| b.id == bro) {
                            b.avatar = prev;
                        }
                        self.avatar_note = Some(match err {
                            ApiError::NotBroOwner { .. } => Text::AvatarNotOwner.into(),
                            _ => Text::AvatarSaveFailed.into(),
                        });
                    }
                }
                self.sync_conv_avatars();
            }
            Message::GroupsReloaded(result) => {
                if let Ok(groups) = result {
                    let tasks: Vec<_> = groups
                        .into_iter()
                        .map(|g| self.learn_group(g, false))
                        .collect();
                    return Task::batch(tasks);
                }
            }
            Message::Renamed { group, result } => {
                self.renaming = false;
                match result {
                    Ok(bro) => {
                        self.rename = None;
                        self.rename_note = None;
                        if let Some(b) = self.bros.iter_mut().find(|b| b.id == bro.id) {
                            b.name = bro.name.clone();
                            b.avatar = bro.avatar;
                        }
                        return self.learn_group(
                            GroupTab {
                                id: bro.dm_group_id.unwrap_or(group),
                                name: bro.name,
                            },
                            true,
                        );
                    }
                    Err(err) => {
                        self.rename_note =
                            Some((group, Text::ClientRenameFailed.fill(&[&err.reason()])));
                    }
                }
            }
            Message::Sent { group, result } => {
                self.sending = false;
                match result {
                    Ok(Said { message: m, .. }) => {
                        self.drafts.remove(&group);
                        self.send_note.remove(&group);
                        let event = Event::Message {
                            id: m.id,
                            kind: m.kind,
                            body: m.body,
                            created_at: m.created_at,
                            author_name: None,
                            author_user_id: None,
                        };
                        self.apply_event(group.clone(), &event);
                        // Pick up the new status card without waiting a tick.
                        return self.poll();
                    }
                    Err(err) => {
                        self.send_note
                            .insert(group, Text::ClientSendFailed.fill(&[&err.reason()]));
                    }
                }
            }
            Message::Live((group, item)) => match item {
                StreamItem::Connected => {
                    if self.selected.as_deref() == Some(group.as_str()) {
                        self.stream_up = true;
                    }
                }
                StreamItem::Event { event, extras } => {
                    self.apply_event_with(group, &event, extras.as_ref())
                }
                StreamItem::Ping => {}
                // Polling takes over at full speed until it reconnects.
                StreamItem::Down(_) => self.stream_up = false,
            },
            Message::Tick => return self.poll(),
            Message::Loaded(result) => {
                self.in_flight = false;
                // `--fake` + `ROBROZ_PAGE=nohost`: treat the demo load as failed and
                // show the "can't reach the host" page (platform
                // screenshots, see `.github/workflows/shots.yml`).
                let result =
                    if self.fake && std::env::var("ROBROZ_PAGE").is_ok_and(|v| v == "nohost") {
                        Err(SourceError(Text::ClientConnectFailed.fill(&[
                            &"http://127.0.0.1:8790",
                            &"io: Connection refused (os error 111)",
                        ])))
                    } else {
                        result
                    };
                match result {
                    Ok(groups) => {
                        let load = self.apply_loaded_groups(groups);
                        // `--fake` + `ROBROZ_PAGE=model`: then switch to the
                        // Model settings page (platform screenshots, see
                        // `.github/workflows/shots.yml`).
                        if self.fake && std::env::var("ROBROZ_PAGE").is_ok_and(|v| v == "model") {
                            return Task::batch([load, Task::done(Message::OpenModel)]);
                        }
                        return load;
                    }
                    Err(err) => {
                        self.load_failed = !self.loaded;
                        self.error = Some(err.0.clone());
                        self.conn_form.error =
                            Some(conn_form::FormError::Conn(err.as_conn_error()));
                    }
                }
            }
            Message::RetryLoad => {
                if !self.in_flight && self.host_state().retries() {
                    self.in_flight = true;
                    let load = blocking(&self.source, |s| s.groups());
                    return Task::perform(load, Message::Loaded);
                }
            }
            Message::Polled(result) => {
                self.in_flight = false;
                match result {
                    Ok(polled) => {
                        for ge in polled.events {
                            self.apply_event_with(
                                ge.group_id,
                                &ge.event,
                                ge.approval_extras.as_ref(),
                            );
                        }
                        for gc in polled.cards {
                            self.set_card(&gc.group_id, gc.card);
                        }
                        self.has_more = polled.has_more;
                        self.error = None;
                    }
                    // Keep what is on screen and try again next tick.
                    Err(err) => self.error = Some(err.0),
                }
            }
            // Opening the private chat is not wired yet; this only logs.
            Message::TakeOver(task_id) => eprintln!("去私聊接管: task {task_id} (not wired yet)"),
            Message::ToggleFailDetail(id) => {
                if !self.fail_open.insert(id.clone()) {
                    self.fail_open.remove(&id);
                }
            }
            Message::Ui(action) => return self.on_action(action),
            Message::Answered { card, sent, result } => {
                let found = self.find_card(&card);
                let ui = self.ui.entry(card.clone()).or_default();
                match result {
                    Ok(reply) => {
                        ui.choice_sent();
                        match (reply, found) {
                            (Some(event), Some((group, _))) => {
                                self.feeds.entry(group).or_default().apply(&event);
                            }
                            // No echo (fake API): show it answered now; the
                            // next event for this id replaces it anyway.
                            (None, Some((group, Card::Choice(mut c)))) => {
                                c.state = ChoiceState::Answered {
                                    selected: sent,
                                    at: Some(chrono::Utc::now().to_rfc3339()),
                                    by: Some(robroz_ui::cards::ME_MARK.into()),
                                };
                                self.set_card(&group, Card::Choice(c));
                            }
                            _ => {}
                        }
                    }
                    Err(err) => {
                        let conflict = matches!(err, ApiError::Conflict(_));
                        ui.choice_failed(err.note());
                        if conflict {
                            // Someone answered first: fetch the real state.
                            self.has_more = true;
                            return self.poll();
                        }
                    }
                }
            }
            Message::Decided {
                card,
                approve,
                result,
            } => {
                let found = self.find_card(&card);
                let ui = self.ui.entry(card.clone()).or_default();
                match result {
                    Ok(reply) => {
                        ui.action_done(None);
                        let in_page = self
                            .review_rules
                            .pending
                            .as_ref()
                            .is_some_and(|p| p.id == card);
                        if in_page {
                            match &reply {
                                Some((event, extras)) => {
                                    self.sync_review_pending(event, extras.as_ref());
                                }
                                None => self.review_rules.finish_pending(),
                            }
                            if self
                                .review_rules
                                .pending
                                .as_ref()
                                .is_some_and(|p| matches!(p.state, ApprovalState::Expired { .. }))
                            {
                                return Task::none();
                            }
                            return self.refresh_review_rules();
                        }
                        match (reply, found) {
                            // The host's answered card replaces the pending one.
                            (Some((event, extras)), Some((group, _))) => {
                                self.apply_event_with(group, &event, extras.as_ref());
                            }
                            // No echo (fake API): show the answer locally.
                            (None, Some((group, Card::Approval(mut c)))) => {
                                let at = now_unix();
                                c.state = if approve {
                                    ApprovalState::Approved {
                                        by: Some(robroz_ui::cards::ME_MARK.into()),
                                        at,
                                    }
                                } else {
                                    ApprovalState::Denied {
                                        by: Some(robroz_ui::cards::ME_MARK.into()),
                                        at,
                                    }
                                };
                                self.set_card(&group, Card::Approval(c));
                            }
                            (None, Some((group, Card::Confirm(mut c)))) => {
                                let (by, at) = (Some(robroz_ui::cards::ME_MARK.into()), now_unix());
                                c.state = if approve {
                                    ConfirmState::Kept { by, at }
                                } else {
                                    ConfirmState::Dropped { by, at }
                                };
                                if let Some(m) = &c.memory_id {
                                    self.memories.answered(m, approve);
                                }
                                if let Some(slug) = &c.slug {
                                    self.skills.answered(slug, approve);
                                }
                                self.set_card(&group, Card::Confirm(c));
                            }
                            _ => {}
                        }
                    }
                    Err(err @ (ApiError::Conflict(_) | ApiError::ApprovalSettled)) => {
                        if self
                            .review_rules
                            .pending
                            .as_ref()
                            .is_some_and(|p| p.id == card)
                        {
                            self.review_rules.answered_elsewhere();
                            return self.refresh_review_rules();
                        }
                        // Answered already (elsewhere), or expired: fetch the
                        // real state; the card that lands clears this note.
                        // A v0.2 403 `decided` / `expired` is no error to
                        // show, so its note has no text.
                        ui.action_done(Some(match err {
                            ApiError::ApprovalSettled => Note::Settled,
                            _ => Note::AlreadyDecided,
                        }));
                        self.has_more = true;
                        return self.poll();
                    }
                    Err(err @ ApiError::NotApprovalOwner { .. }) => {
                        // Not the owner: no buttons, "Only {owner} can
                        // approve this."; fetch the card again as well.
                        let owner = found.as_ref().and_then(|(_, c)| match c {
                            Card::Approval(a) => a.owner_name.clone(),
                            _ => None,
                        });
                        let note = match err.note() {
                            Note::OnlyOwner(None) => Note::OnlyOwner(owner),
                            other => other,
                        };
                        ui.action_done(Some(note));
                        self.has_more = true;
                        return self.poll();
                    }
                    Err(err) => {
                        if self
                            .review_rules
                            .pending
                            .as_ref()
                            .is_some_and(|p| p.id == card)
                        {
                            self.review_rules.busy = false;
                            self.review_rules.note = Some(err.reason());
                        }
                        ui.action_done(Some(err.note()));
                    }
                }
            }
            Message::Acted { card, result } => {
                let (tag, id) = card.split_at(1);
                let id = id.to_string();
                let ui = self.ui.entry(id.clone()).or_default();
                match result {
                    Ok(()) => {
                        ui.action_done(None);
                        let (by, at) = (user_name().to_string(), now_unix());
                        match self.find_card(&id) {
                            Some((group, Card::Draft(mut c))) if tag == ">" => {
                                c.state = DraftState::Sent { by, at };
                                self.set_card(&group, Card::Draft(c));
                            }
                            _ => {}
                        }
                    }
                    Err(err) => ui.action_done(Some(err.note())),
                }
            }
            Message::RoutinesLoaded(result) => match result {
                Ok(entries) => {
                    self.routines
                        .load(entries.iter().map(RoutineRow::from_entry).collect());
                    self.error = None;
                }
                Err(err) => self.error = Some(err.note().text()),
            },
            Message::Paused { id, wanted, result } => match result {
                Ok(entry) => self.routines.confirm(RoutineRow::from_entry(&entry)),
                Err(_) => {
                    self.routines.revert(&id, wanted);
                    let ms = self.skin().motion.note_visible_ms;
                    return Task::perform(
                        tokio_sleep(Duration::from_millis(ms.into())),
                        move |_| Message::ClearPauseNote(id.clone()),
                    );
                }
            },
            Message::ClearPauseNote(id) => self.routines.clear_failed(&id),
            Message::SetTheme(pref) => {
                self.theme_pref = pref;
                if !self.fake {
                    prefs::save_theme(pref);
                }
            }
            Message::SystemTheme(mode) => {
                // `None` means the system gave no preference: light.
                self.system_mode = match mode {
                    iced::theme::Mode::Dark => Mode::Dark,
                    _ => Mode::Light,
                };
            }
            Message::SetLocale(locale) => {
                if locale != i18n::locale() {
                    i18n::set_locale(locale);
                    // Stored text was drawn in the old language. Cards and
                    // statuses render on the fly; previews are cached.
                    for (group, line) in self.previews.redraw() {
                        self.convs.set_preview(group, line);
                    }
                    if !self.fake {
                        prefs::save(locale);
                    }
                }
            }
            Message::Conn(event) => match event {
                conn_form::Event::Host(host) => self.conn_form.set_host(host),
                conn_form::Event::Token(token) => self.conn_form.set_token(token),
                conn_form::Event::ToggleShow => self.conn_form.toggle_show(),
                conn_form::Event::Clear => self.conn_form.request_clear(),
                conn_form::Event::Connect => {
                    if self.conn_form.connecting {
                        return Task::none();
                    }
                    return self.start_connect();
                }
            },
            Message::ConnFinished(result) => return self.finish_connect(result),
            Message::Resized(size) => self.width = size.width,
            Message::TouchSeen => self.touch = true,
            Message::Shoot(Some(id)) => return window::screenshot(id).map(Message::Shot),
            Message::Shoot(None) => return iced::exit(),
            Message::Shot(shot) => {
                if let Ok(path) = std::env::var("ROBROZ_SHOT") {
                    // Raw RGBA; a script turns it into PNG.
                    let _ = std::fs::write(&path, &shot.rgba);
                    println!("shot {}x{} -> {path}", shot.size.width, shot.size.height);
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let resize = window::resize_events().map(|(_, size)| Message::Resized(size));
        let touch = iced::event::listen_with(|event, _, _| match event {
            iced::Event::Touch(_) => Some(Message::TouchSeen),
            _ => None,
        });
        let system = iced::system::theme_changes().map(Message::SystemTheme);
        let mut subs = vec![resize, system];
        if !self.touch {
            subs.push(touch);
        }
        if self.loaded && self.has_more {
            let every = stream::poll_interval(self.stream_up, self.interval);
            subs.push(iced::time::every(every).map(|_| Message::Tick));
        }
        if self.host_state().retries() && self.gallery.is_none() {
            subs.push(iced::time::every(self.interval).map(|_| Message::RetryLoad));
        }
        if self.loaded && self.gallery.is_none() {
            subs.push(iced::time::every(UNREAD_EVERY).map(|_| Message::UnreadTick));
        }
        if self.rename.is_some() {
            // The rename box captures Esc, so listen to captured keys too.
            subs.push(iced::event::listen_with(|event, _, _| match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) => Some(Message::Ui(Action::RenameCancel)),
                _ => None,
            }));
        }
        if let (Some(conn), Some(group), Page::Group) = (&self.conn, &self.selected, self.page) {
            let key = stream::Key {
                conn: conn.clone(),
                group_id: group.clone(),
            };
            subs.push(Subscription::run_with(key, stream::subscribe).map(Message::Live));
        }
        Subscription::batch(subs)
    }

    /// `None` while following the system, so the window frame follows the
    /// OS as well; an explicit choice pins both.
    fn theme(&self) -> Option<iced::Theme> {
        match self.theme_pref {
            ThemePref::System => None,
            ThemePref::Light => Some(iced::Theme::Light),
            ThemePref::Dark => Some(iced::Theme::Dark),
        }
    }

    fn scale_factor(&self) -> f32 {
        self.scale
    }

    fn view(&self) -> Element<'_, Message> {
        let t = self.skin();
        if let Some(g) = &self.gallery {
            return self.gallery_view(&t, g);
        }
        let main: Element<'_, Message> = if self.narrow() {
            // Phone: one screen at a time.
            if !self.host_state().shows_sidebar() || (self.narrow_open && self.page == Page::Group)
            {
                self.page_view(&t)
            } else if self.narrow_open {
                let back = button(text(Text::ViewBackToChats).size(t.type_.row_name))
                    .height(Length::Fixed(
                        t.controls.touch_target.max(t.controls.button_h),
                    ))
                    .style(view::nav_button_style(t, false))
                    .on_press(Message::Back);
                column![
                    container(back).padding(Padding::from([4.0, 8.0])),
                    self.page_view(&t)
                ]
                .height(Fill)
                .into()
            } else {
                self.group_list(&t, Length::Fill)
            }
        } else {
            row![
                self.group_list(&t, Length::Fixed(t.controls.sidebar_w)),
                self.page_view(&t)
            ]
            .height(Fill)
            .into()
        };
        let live = match (&self.conn, self.stream_up) {
            (None, _) => "",
            (Some(_), true) => Text::ClientLive.as_str(),
            (Some(_), false) => Text::ClientPolling.as_str(),
        };
        // While the host has never answered, the main pane already shows
        // why, so the footer drops the raw "Can't reach the host … io: …
        // (os error …)" line and disappears.
        if self.host_state() == HostState::Unreachable {
            return container(main).style(view::page_style(t)).into();
        }
        let status = match &self.error {
            Some(err) => err.clone(),
            None => format!("{}{live}", self.origin),
        };
        let footer = row![text(status)
            .size(t.type_.foot)
            .color(color(t.palette.text3))]
        .align_y(Alignment::Center);
        container(column![
            main,
            container(footer).padding(Padding::from([4.0, 10.0]))
        ])
        .style(view::page_style(t))
        .into()
    }

    /// The Language row and, directly under it, the Appearance row.
    fn language_row(&self, t: &Skin) -> Element<'_, Message> {
        column![self.language_card(t), self.appearance_card(t)]
            .spacing(t.metrics.row_pad_h)
            .into()
    }

    /// The Appearance row: label on the left, System / Light / Dark on the
    /// right, the active one highlighted, same shape as the Language row.
    fn appearance_card(&self, t: &Skin) -> Element<'_, Message> {
        let mut choices = row![]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center);
        for pref in ThemePref::ALL {
            let label = match pref {
                ThemePref::System => Text::SettingsThemeSystem,
                ThemePref::Light => Text::ClientThemeLight,
                ThemePref::Dark => Text::ClientThemeDark,
            };
            choices = choices.push(
                button(text(label).size(t.type_.body))
                    .height(Length::Fixed(t.controls.button_h))
                    .style(view::segment_button_style(*t, pref == self.theme_pref))
                    .on_press(Message::SetTheme(pref)),
            );
        }
        let r = row![
            text(Text::SettingsAppearance)
                .size(t.type_.body)
                .color(color(t.palette.text)),
            space::horizontal(),
            choices,
        ]
        .align_y(Alignment::Center);
        container(container(r).padding(Padding::from([
            t.metrics.row_pad_h * 0.75,
            t.metrics.row_pad_h,
        ])))
        .width(Fill)
        .style(view::card_style(*t))
        .into()
    }

    /// The one Settings row for the UI language: label on the left,
    /// English / 中文 (each in its own language) on the right, the active
    /// one highlighted. Sizes follow the text, so longer copy never clips.
    fn language_card(&self, t: &Skin) -> Element<'_, Message> {
        let active = i18n::locale();
        let mut choices = row![]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center);
        for locale in Locale::ALL {
            choices = choices.push(
                button(text(locale.native_name()).size(t.type_.body))
                    .height(Length::Fixed(t.controls.button_h))
                    .style(view::segment_button_style(*t, locale == active))
                    .on_press(Message::SetLocale(locale)),
            );
        }
        let r = row![
            text(Text::SettingsLanguage)
                .size(t.type_.body)
                .color(color(t.palette.text)),
            space::horizontal(),
            choices,
        ]
        .align_y(Alignment::Center);
        container(container(r).padding(Padding::from([
            t.metrics.row_pad_h * 0.75,
            t.metrics.row_pad_h,
        ])))
        .width(Fill)
        .style(view::card_style(*t))
        .into()
    }

    /// Same connection form as the offline pane, as wide as the Language row.
    /// `titled` is Settings only: a heading matching the Model card.
    fn connection_form(&self, t: &Skin, titled: bool) -> Element<'_, Message> {
        conn_form::view(t, &self.conn_form, titled).map(Message::Conn)
    }

    /// Save (when the fields are not env-locked) and probe the host off the
    /// UI thread, then rebuild [`Conn`] with [`Conn::from_env`].
    fn start_connect(&mut self) -> Task<Message> {
        if !self.conn_form.connect_enabled() {
            return Task::none();
        }
        self.conn_form.connecting = true;
        self.conn_form.error = None;
        // Pause the unreachable-pane auto-retry while this attempt is in flight.
        self.in_flight = true;
        let host = self.conn_form.host_for_save();
        let token = self.conn_form.token_edit();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || connect_blocking(host, token))
                    .await
                    .expect("connect task panicked")
            },
            Message::ConnFinished,
        )
    }

    fn finish_connect(
        &mut self,
        result: Result<Vec<GroupTab>, conn_form::FormError>,
    ) -> Task<Message> {
        self.conn_form.connecting = false;
        self.in_flight = false;
        match result {
            Ok(groups) => {
                self.conn_form.error = None;
                self.conn_form.reload();
                if self.fake {
                    return Task::none();
                }
                self.adopt_conn_from_env();
                // Reuse the first-listing success path, then stay on Settings
                // if that's where Connect was pressed.
                let stay = self.page;
                let task = self.apply_loaded_groups(groups);
                if stay != Page::Group {
                    self.page = stay;
                }
                task
            }
            Err(err) => {
                if !self.fake {
                    self.adopt_conn_from_env();
                    self.loaded = false;
                    self.load_failed = true;
                    self.stream_up = false;
                }
                self.conn_form.reload();
                self.conn_form.error = Some(err);
                Task::none()
            }
        }
    }

    /// The host listed groups: record them and start the usual follow-up
    /// loads. `--fake` + `ROBROZ_OPEN` opens that chat; otherwise the
    /// desktop client opens the first one.
    fn apply_loaded_groups(&mut self, groups: Vec<GroupTab>) -> Task<Message> {
        for g in &groups {
            self.convs.upsert(&g.id, &g.name, false);
        }
        self.groups = groups;
        self.loaded = true;
        self.load_failed = false;
        self.error = None;
        let mut tasks = vec![
            self.poll(),
            self.load_unread(),
            self.load_bros(),
            self.load_me(),
        ];
        let wanted = std::env::var("ROBROZ_OPEN")
            .ok()
            .filter(|g| self.fake && self.groups.iter().any(|x| &x.id == g));
        if self.fake {
            if let Ok(raw) = std::env::var("ROBROZ_EXPAND_DETAIL") {
                for id in raw.split(',') {
                    let id = id.trim();
                    if !id.is_empty() {
                        self.fail_open.insert(id.to_string());
                    }
                }
            }
        }
        // `--fake` + `ROBROZ_JUMP=<group>:<approval id>`: press the DM
        // reminder row's jump after a moment (screenshots).
        if let Some((group, card)) = std::env::var("ROBROZ_JUMP")
            .ok()
            .filter(|_| self.fake)
            .and_then(|v| {
                v.split_once(':')
                    .map(|(g, c)| (g.to_string(), c.to_string()))
            })
        {
            tasks.push(Task::perform(
                tokio_sleep(Duration::from_millis(600)),
                move |_| {
                    Message::Ui(Action::OpenApproval {
                        group: group.clone(),
                        card: card.clone(),
                    })
                },
            ));
        }
        if let Some(g) = wanted {
            tasks.push(self.open_chat(g));
        } else if let Some(first) = self.groups.first().map(|g| g.id.clone()) {
            // Desktop opens the first chat; narrow starts on the list.
            if !self.narrow() {
                tasks.push(self.open_chat(first));
            }
        }
        Task::batch(tasks)
    }

    /// Point the live source, API and SSE connection at [`Conn::from_env`].
    fn adopt_conn_from_env(&mut self) {
        let http = HttpSource::from_env();
        self.origin = http.base().to_string();
        self.conn = Some(http.conn().clone());
        self.api = Arc::new(HttpApi::new(http.conn().clone()).answering_as(user_name()));
        *self.source.lock().expect("event source lock poisoned") = Box::new(http);
        self.stream_up = false;
    }

    fn group_list(&self, t: &Skin, width: Length) -> Element<'_, Message> {
        if !self.host_state().shows_sidebar() {
            // Nothing to list until the host answers: no empty-account hint.
            return container(space::horizontal())
                .width(width)
                .height(Fill)
                .padding(8)
                .style(view::sidebar_style(*t))
                .into();
        }
        let selected = (self.page == Page::Group)
            .then_some(self.selected.as_deref())
            .flatten();
        let list_w = match width {
            Length::Fixed(w) => w,
            _ => self.width,
        } - 16.0;
        let list =
            view::conversation_list(t, &self.convs, selected, now_unix(), list_w).map(Message::Ui);
        let routines = button(text(robroz_ui::cards::routines::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Routines))
            .on_press(Message::OpenRoutines);
        let memories = button(text(robroz_ui::cards::memories::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Memories))
            .on_press(Message::OpenMemories);
        let skills = button(text(robroz_ui::cards::skills::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Skills))
            .on_press(Message::OpenSkills);
        let model_nav = button(text(model_page::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Model))
            .on_press(Message::OpenModel);
        let search = button(text(search_page::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Search))
            .on_press(Message::OpenSearch);
        let mcp = button(text(robroz_ui::cards::mcp::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::Mcp))
            .on_press(Message::OpenMcp);
        let review_rules = button(text(review_rules::TITLE).size(t.type_.row_name))
            .width(Fill)
            .height(Length::Fixed(
                t.controls.touch_target.max(t.controls.button_h),
            ))
            .padding(Padding::from([0.0, 12.0]))
            .style(view::nav_button_style(*t, self.page == Page::ReviewRules))
            .on_press(Message::OpenReviewRules);
        container(
            column![
                scrollable(list).height(Fill),
                routines,
                memories,
                skills,
                model_nav,
                search,
                mcp,
                review_rules
            ]
            .spacing(4),
        )
        .width(width)
        .height(Fill)
        .padding(8)
        .style(view::sidebar_style(*t))
        .into()
    }

    fn content_width(&self, t: &Skin) -> f32 {
        let chrome = if self.gallery.is_some() || self.narrow() {
            0.0
        } else {
            t.controls.sidebar_w
        };
        (self.width - chrome - 2.0 * t.metrics.stream_padding).max(200.0)
    }

    fn page_view(&self, t: &Skin) -> Element<'_, Message> {
        let state = self.host_state();
        if !state.shows_sidebar() {
            // Before the host answers: blank while waiting, or a headline
            // that follows the error (「令牌被拒绝」 / 「连不上宿主」) with
            // only the address on a lighter line below. The reason is
            // shown once, under the connection form. The Language row is
            // the same control as the Model page.
            let notice = state.notice(self.error.as_deref()).map(|key| {
                let detail = text(self.origin.as_str())
                    .size(t.type_.foot)
                    .color(color(t.palette.text3));
                let head = column![
                    text(key).size(t.type_.body).color(color(t.palette.text)),
                    detail,
                ]
                .spacing(4)
                .align_x(Alignment::Center);
                if state.shows_language_row() {
                    // Offline pane only: 360px centred Language row and the
                    // same-width connection form, 24px below the address
                    // line. The Model page still uses the full-width row.
                    Element::from(
                        column![
                            head,
                            container(self.language_row(t)).width(Length::Fixed(360.0)),
                            container(self.connection_form(t, false)).width(Length::Fixed(360.0)),
                        ]
                        .spacing(24)
                        .align_x(Alignment::Center),
                    )
                } else {
                    Element::from(head)
                }
            });
            return container(
                notice.map_or_else(|| Element::from(space::horizontal()), Element::from),
            )
            .padding(t.metrics.stream_padding)
            .center(Fill)
            .into();
        }
        if self.page == Page::NewBro {
            let d = &self.new_bro;
            return robroz_ui::avatar_view::create_bro_page(
                t,
                &d.name,
                d.avatar,
                d.note.as_deref(),
                d.busy,
            )
            .map(Message::Ui);
        }
        if self.page == Page::BroAvatar {
            if let Some(bro) = self
                .avatar_bro
                .as_deref()
                .and_then(|id| self.bros.iter().find(|b| b.id == id))
            {
                return robroz_ui::avatar_view::avatar_page(
                    t,
                    &bro.name,
                    avatar::resolve(bro.avatar, &bro.id),
                    bro.avatar.is_some(),
                    self.avatar_note.as_deref(),
                )
                .map(Message::Ui);
            }
        }
        if self.page == Page::Memories {
            return scrollable(
                container(view::memories_panel(t, &self.memories, now_unix()).map(Message::Ui))
                    .padding(t.metrics.stream_padding)
                    .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::Skills {
            return scrollable(
                container(
                    view::skills_panel(t, &self.skills, now_unix(), self.content_width(t))
                        .map(Message::Ui),
                )
                .padding(t.metrics.stream_padding)
                .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::Mcp {
            return scrollable(
                container(
                    view::mcp_panel(t, &self.mcp, now_unix(), self.content_width(t))
                        .map(Message::Ui),
                )
                .padding(t.metrics.stream_padding)
                .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::Model {
            return scrollable(
                container(
                    column![
                        view::model_panel(t, &self.model_settings).map(Message::Ui),
                        self.language_row(t),
                        self.connection_form(t, true),
                    ]
                    .spacing(t.metrics.row_pad_h),
                )
                .padding(t.metrics.stream_padding)
                .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::Search {
            return scrollable(
                container(view::search_panel(t, &self.search).map(Message::Ui))
                    .padding(t.metrics.stream_padding)
                    .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::ReviewRules {
            return scrollable(
                container(view::review_rules_panel(t, &self.review_rules).map(Message::Ui))
                    .padding(t.metrics.stream_padding)
                    .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        if self.page == Page::Routines {
            return scrollable(
                container(view::routines_panel(t, &self.routines, now_unix()).map(Message::Ui))
                    .padding(t.metrics.stream_padding)
                    .width(Fill),
            )
            .height(Fill)
            .width(Fill)
            .into();
        }
        let Some(id) = &self.selected else {
            return container(text(Text::ClientPickChat).color(color(t.palette.text2)))
                .center(Fill)
                .into();
        };
        let in_group = self.convs.get(id).is_some_and(|c| !c.dm);
        let me = self.me.as_ref().map(|m| m.id.as_str());
        let empty = CardUi::default();
        let rows = self
            .feeds
            .get(id)
            .into_iter()
            .flat_map(Feed::items)
            .collect::<Vec<_>>();
        // More than three pending confirm cards from one task fold away
        // behind 「还有 N 条待确认」.
        let plan = confirm::fold_plan(
            rows.iter().map(|(r, _)| match r {
                Row::Card(Card::Confirm(c)) => c.task_id.as_deref().map(|t| (t, c.is_pending())),
                _ => None,
            }),
            &self.unfolded,
        );
        let items = rows
            .into_iter()
            .zip(plan)
            .filter_map(|((r, meta), fold)| match fold {
                Fold::Show => Some((FeedItem::Row(r), meta.clone())),
                Fold::Hide => None,
                Fold::More { task, n } => Some((FeedItem::More { task, n }, meta.clone())),
            });
        let blocks = model::chat_blocks(items, t.chat.group_secs, &chrono::Local);
        let chrome = if self.gallery.is_some() || self.narrow() {
            0.0
        } else {
            t.controls.sidebar_w
        };
        let area = (self.width - chrome).max(200.0);
        let column_w = t.chat_column_width(area);
        let (bubble_max, card_w) = t.chat_widths(self.width, column_w);
        let now = now_unix();
        let mut feed_col = column![].spacing(0).width(Fill);
        let mut prev_date = false;
        for (i, block) in blocks.into_iter().enumerate() {
            match block {
                robroz_ui::runs::Block::Date(at) => {
                    feed_col = feed_col.push(view::date_separator(
                        t,
                        robroz_ui::runs::date_label(at, now, &chrono::Local),
                    ));
                    prev_date = true;
                }
                robroz_ui::runs::Block::Run(g) => {
                    if i == 0 || !prev_date {
                        feed_col =
                            feed_col.push(iced::widget::Space::new().height(t.chat.gap_between));
                    }
                    feed_col = feed_col.push(group_view(
                        t,
                        g,
                        &self.bros,
                        &self.ui,
                        &empty,
                        self.caps,
                        (bubble_max, card_w),
                        in_group,
                        me,
                        &self.fail_open,
                    ));
                    prev_date = false;
                }
            }
        }
        // Bottom padding keeps the last item clear of the in-flow composer
        // (composer height is already reserved below the scrollable) plus
        // a 16px gap. When the composer overlays the feed, this would be
        // composer_height + 16 instead; here the composer sits under the
        // feed, so the extra 16px is the visible gap.
        feed_col = feed_col.push(iced::widget::Space::new().height(16.0));
        let feed_body = container(container(feed_col).width(Length::Fixed(column_w)))
            .width(Fill)
            .center_x(Fill);
        let feed = scrollable(feed_body)
            .id(FEED_ID)
            .height(Fill)
            .width(Fill)
            .anchor_bottom();
        let (name, dm) = self
            .convs
            .get(id)
            .map_or((id.as_str(), false), |c| (c.name.as_str(), c.dm));
        let hint = robroz_ui::conversations::composer_hint(name, dm);
        let rename = self.dm_bro(id).map(|_| view::HeaderRename {
            draft: self
                .rename
                .as_ref()
                .filter(|(g, _)| g == id)
                .map(|(_, d)| d.as_str()),
            saving: self.renaming,
            note: self
                .rename_note
                .as_ref()
                .filter(|(g, _)| g == id)
                .map(|(_, n)| n.as_str()),
        });
        // Title capsule: a DM shows its bro; a group shows the bros seen in
        // its feed, newest first (the contract has no member list yet).
        let header_ids: Vec<String> = match self.dm_bro(id) {
            Some(b) => vec![b.id.clone()],
            None => {
                let mut ids: Vec<String> = Vec::new();
                for (_, meta) in self.feeds.get(id).into_iter().flat_map(Feed::items).rev() {
                    if let model::Sender::Bro { id: bro, .. } = &meta.sender {
                        if !ids.contains(bro) {
                            ids.push(bro.clone());
                        }
                    }
                }
                ids
            }
        };
        let (shown, more) = avatar::stack(&header_ids);
        let header_avatars: Vec<BroAvatar> = shown.iter().map(|b| self.avatar_of(b)).collect();
        let can_edit = self.dm_bro(id).is_some_and(|b| self.owns(b));
        let draft = self.drafts.get(id).map_or("", String::as_str);
        let note = self.send_note.get(id).map(String::as_str);
        let side = ((area - column_w) / 2.0).max(t.chat.column_margin);
        // The scrollable is the full main area so the scrollbar sits against
        // the window edge, not on the 720px column (where it clipped cards
        // and the own-bubble timestamp). The column width is unchanged.
        column![
            view::chat_header(
                t,
                name,
                dm,
                self.narrow(),
                rename,
                view::HeaderBros {
                    avatars: &header_avatars,
                    more,
                    can_edit,
                },
            )
            .map(Message::Ui),
            feed,
            container(view::composer(t, draft, &hint, self.sending, note).map(Message::Ui))
                .width(Fill)
                .center_x(Fill)
                .padding(Padding {
                    top: 0.0,
                    right: side,
                    bottom: 0.0,
                    left: side,
                }),
        ]
        .height(Fill)
        .width(Fill)
        .into()
    }

    fn gallery_view<'a>(&'a self, t: &Skin, g: &'a Gallery) -> Element<'a, Message> {
        let width = self.content_width(t);
        let two_columns = width >= 900.0;
        let col_width = if two_columns {
            (width - t.metrics.stream_padding) / 2.0
        } else {
            width
        };
        let caption = |s: &'static str| -> Element<'a, Message> {
            text(s)
                .font(Font::MONOSPACE)
                .size(t.type_.tag)
                .color(color(t.palette.text3))
                .into()
        };
        let title = |s: &'static str| -> Element<'a, Message> {
            text(s)
                .size(t.type_.list_title)
                .font(view::bold())
                .color(color(t.palette.text))
                .into()
        };
        let section =
            |name: &'static str, samples: &'a [gallery::Sample]| -> Element<'a, Message> {
                let mut c = column![title(name)].spacing(6);
                for s in samples {
                    c = c.push(caption(s.caption)).push(
                        view::card(
                            t,
                            &s.card,
                            &s.ui,
                            Caps::FAKE,
                            col_width,
                            s.group,
                            false,
                            Some(api::FAKE_ME_ID),
                        )
                        .map(Message::Ui),
                    );
                }
                c.into()
            };
        // The frozen status card, to review its dark-mode outline.
        let running = robroz_card::palette(robroz_card::Tone::Ink);
        let status_sample: Element<'a, Message> = container(
            row![text("文档哥"), space::horizontal(), text("进行中")].align_y(Alignment::Center),
        )
        .padding(view::status_card_padding(t))
        .width(Fill)
        .style(view::status_card_style(*t, running.fill, running.text))
        .into();
        let left = column![
            section("选项卡 choice", &g.choice),
            caption("status · running（冻结的状态卡，深色加 1px 描边）"),
            status_sample,
        ]
        .spacing(6)
        .width(Fill);
        let right = column![
            section("审批卡 approval", &g.approval),
            section("草稿卡 draft", &g.draft),
            title("例行任务列表 routines"),
            caption("routines · list"),
            view::routines_panel(t, &g.routines, g.now).map(Message::Ui),
            caption("routines · empty"),
            view::routines_panel(t, &g.empty, g.now).map(Message::Ui),
        ]
        .spacing(6)
        .width(Fill);
        let body: Element<'a, Message> = if two_columns {
            row![left, right].spacing(t.metrics.stream_padding).into()
        } else {
            column![left, right].spacing(18).into()
        };
        container(scrollable(
            container(body)
                .padding(t.metrics.stream_padding)
                .width(Fill),
        ))
        .width(Fill)
        .height(Fill)
        .style(view::page_style(*t))
        .into()
    }
}

async fn tokio_sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

/// One item of the chat feed: a row, or the fold line for hidden confirm cards.
enum FeedItem<'a> {
    Row(&'a Row),
    More { task: String, n: usize },
}

/// A run of items from one sender. Own items go right in ink bubbles, with
/// no avatar. A bro's go left: the name once above the first item in a
/// group chat, and a small avatar only at the bottom-left of the last
/// item. Cards fill the text column beside the avatar gutter. The time
/// closes the run.
/// Avatar for a run's sender (only bros draw one).
fn sender_avatar(bros: &[Bro], sender: &model::Sender) -> BroAvatar {
    match sender {
        model::Sender::Bro { id, .. } => avatar_for(bros, id),
        _ => avatar::default_for(sender.name()),
    }
}

fn group_view<'a>(
    t: &Skin,
    group: robroz_ui::runs::Run<FeedItem<'a>, model::Sender>,
    bros: &[Bro],
    ui: &HashMap<String, CardUi>,
    empty: &CardUi,
    caps: Caps,
    (bubble_max, card_w): (f32, f32),
    in_group: bool,
    me: Option<&str>,
    fail_open: &HashSet<String>,
) -> Element<'a, Message> {
    // A reminder is one full-width row: no avatar, no name, no clock.
    if group.sender.is_reminder() {
        return column(group.items.into_iter().map(|item| match item {
            FeedItem::Row(r) => item_view(
                t, r, ui, empty, caps, bubble_max, card_w, true, in_group, me, fail_open,
            ),
            FeedItem::More { .. } => space::horizontal().into(),
        }))
        .into();
    }
    let own = group.sender.is_own();
    let delivery = group.sender.is_delivery();
    let gutter = t.chat_gutter();
    let n = group.items.len();
    let mut col = column![].spacing(t.chat.gap_in);
    if in_group {
        if let model::Sender::Bro { id, name } = &group.sender {
            // The bro's colour; every palette entry is ≥4.5:1 on this theme.
            let tint = avatar::color(avatar_for(bros, id).color, t.mode == Mode::Dark);
            let label = text(name.clone()).size(t.type_.foot).color(color(tint));
            col = col.push(row![iced::widget::Space::new().width(gutter), label]);
        }
    }
    for (i, item) in group.items.into_iter().enumerate() {
        let first = i == 0;
        let last = i + 1 == n;
        let body = match item {
            FeedItem::Row(r) => item_view(
                t, r, ui, empty, caps, bubble_max, card_w, first, in_group, me, fail_open,
            ),
            FeedItem::More { task, n } => {
                container(view::confirm_more(t, &task, n).map(Message::Ui))
                    .width(Length::Fixed(card_w))
                    .into()
            }
        };
        if own {
            col = col.push(body);
        } else if delivery || !last {
            col = col.push(row![iced::widget::Space::new().width(gutter), body]);
        } else {
            col = col.push(
                row![
                    container(view::chat_avatar(t, sender_avatar(bros, &group.sender)))
                        .width(Length::Fixed(t.chat.avatar))
                        .align_y(Alignment::End),
                    body
                ]
                .spacing(t.chat.avatar_gap)
                .align_y(Alignment::End),
            );
        }
    }
    if let Some(at) = group.last_at {
        let clock = robroz_ui::cards::status::clock_in(at, &chrono::Local);
        if !clock.is_empty() {
            // Only under the last item of the run, 12px text3. Own times
            // are right-aligned to the bubble; bro times start at the
            // left edge of the last bubble or card, after the avatar gutter.
            let clock = text(clock)
                .size(t.chat.time_size)
                .color(color(t.palette.text3));
            if own {
                let clock = container(clock)
                    .width(Length::Fill)
                    .align_x(Alignment::End)
                    .padding(Padding {
                        right: 2.0,
                        ..Padding::ZERO
                    });
                col = col.push(
                    container(clock)
                        .max_width(bubble_max)
                        .width(Length::Shrink)
                        .align_x(Alignment::End),
                );
            } else {
                col = col.push(row![iced::widget::Space::new().width(gutter), clock,]);
            }
        }
    }
    if own {
        row![space::horizontal(), col.align_x(Alignment::End)].into()
    } else {
        col.into()
    }
}

#[allow(clippy::too_many_arguments)]
fn item_view<'a>(
    t: &Skin,
    r: &'a Row,
    ui: &HashMap<String, CardUi>,
    empty: &CardUi,
    caps: Caps,
    bubble_max: f32,
    card_w: f32,
    first: bool,
    in_group: bool,
    me: Option<&str>,
    fail_open: &HashSet<String>,
) -> Element<'a, Message> {
    let own_text = color(t.chat.own_text);
    let bubble = |content: Element<'a, Message>| -> Element<'a, Message> {
        container(content)
            .padding(view::bubble_padding(t))
            .max_width(bubble_max)
            .style(view::bubble_style(*t, true, first))
            .into()
    };
    let card = |e: Element<'a, Message>| -> Element<'a, Message> {
        container(e).width(Length::Fixed(card_w)).into()
    };
    match r {
        Row::Text { body } => bubble(text(body).size(t.chat.mine_size).color(own_text).into()),
        Row::Attachment { kind, name } => bubble(
            row![
                container(
                    text(attachment_label(*kind))
                        .size(t.type_.tag)
                        .color(own_text)
                )
                .padding(Padding::from([2.0, t.metrics.tag_pad_h + 2.0]))
                .style(view::chip_style(*t)),
                text(name).size(t.chat.mine_size).color(own_text),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        ),
        Row::Status(c) => card(status_view(t, c, card_w, fail_open.contains(&c.task_id))),
        Row::Delivery(c) => card(delivery_view(t, c, card_w, fail_open.contains(&c.id))),
        Row::Reminder(r) => view::reminder_row(t, r).map(Message::Ui),
        Row::Card(c) => {
            let state = ui.get(c.id()).unwrap_or(empty);
            card(view::card(t, c, state, caps, card_w, in_group, true, me).map(Message::Ui))
        }
    }
}

fn fail_detail_row<'a>(t: &Skin, id: &str, detail: &str, open: bool) -> Element<'a, Message> {
    let label = if open {
        Text::FailureDetailHide.as_str()
    } else {
        Text::FailureDetailShow.as_str()
    };
    let toggle = button(text(label).size(13.0).color(color(t.chat.accent)))
        .padding(0)
        .style(fail_detail_style(*t))
        .on_press(Message::ToggleFailDetail(id.to_string()));
    if open {
        column![
            toggle,
            text(detail.to_string())
                .size(13.0)
                .color(color(t.palette.text2)),
        ]
        .spacing(t.metrics.dot_gap)
        .into()
    } else {
        toggle.into()
    }
}

fn fail_detail_style(t: Skin) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: hovered.then_some(color(t.palette.control_hover).into()),
            text_color: color(t.chat.accent),
            border: Border::default().rounded(t.radii.button),
            ..button::Style::default()
        }
    }
}

/// The status dot: accent for a running card, green done, amber stuck,
/// grey otherwise (queued, a dead run, waiting on approval).
fn status_dot_kind(card: &StatusCard) -> Option<robroz_ui::cards::status::Dot> {
    use robroz_ui::cards::status::Dot;
    match card.tone {
        robroz_card::Tone::Ink => None,
        robroz_card::Tone::Green => Some(Dot::Done),
        robroz_card::Tone::Amber => Some(Dot::Stuck),
        robroz_card::Tone::Gray => Some(Dot::Void),
    }
}

fn status_dot_view<'a>(t: &Skin, card: &StatusCard) -> Element<'a, Message> {
    match status_dot_kind(card) {
        None => view::rgb_dot(t, t.chat.accent),
        Some(d) => view::status_dot(t, d).map(Message::Ui),
    }
}

/// Status card: a plain bro card with a status dot and the status word.
/// The bro name lives in the group header, not inside the card. No fill.
/// Layout: [dot 8px status (and "Step n of m" if shown)] [flex] [Take over].
fn status_view<'a>(
    t: &Skin,
    card: &'a StatusCard,
    width: f32,
    detail_open: bool,
) -> Element<'a, Message> {
    use iced::widget::text::Wrapping;
    let status_word = text(card.shown_line()).wrapping(Wrapping::None);
    let texts: Element<'a, Message> = if let Some(step) = visible_step_progress(t, card, width) {
        let rgb = robroz_ui::cards::status::progress_rgb(t.chat.bro_bg, t.palette.text);
        row![
            text(model::display(step).into_owned())
                .color(color(rgb))
                .wrapping(Wrapping::None),
            status_word,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    } else {
        status_word.into()
    };
    let mut line = row![
        row![status_dot_view(t, card), texts]
            .spacing(view::STATUS_DOT_TEXT_GAP)
            .align_y(Alignment::Center),
        space::horizontal(),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if let Some(action) = card.action {
        let [pad_v, pad_h] = view::TAKE_OVER_PAD;
        line = line.push(
            button(text(model::display(action).into_owned()).size(t.type_.button))
                .padding(Padding::from([pad_v, pad_h]))
                .style(view::takeover_button_style(*t))
                .on_press(Message::TakeOver(card.task_id.clone())),
        );
    }
    let mut body = column![line];
    if card.progress.is_some() {
        body = body.push(progress_cell(t, card, width));
    }
    if let Some(detail) = card.collapsed_detail() {
        body = body.push(fail_detail_row(t, &card.task_id, &detail, detail_open));
    }
    container(body.spacing(t.metrics.dot_gap))
        .padding(view::status_card_padding(t))
        .width(Fill)
        .style(view::card_style(*t))
        .into()
}

/// Router step text left of the status word. Dropped when the card is too
/// narrow so the status word is never wrapped or truncated. The bro name
/// and status word are drawn at iced's default size (16), so the check
/// uses the same size.
fn visible_step_progress<'a>(t: &Skin, card: &'a StatusCard, width: f32) -> Option<&'a str> {
    let raw = card.step_progress.as_deref()?;
    let text_w = |s: &str, size: f32| {
        s.chars()
            .map(|c| if c.is_ascii() { 1.0 } else { 2.0 })
            .sum::<f32>()
            * size
            * t.type_.unit_width
    };
    const WORD: f32 = 16.0;
    let inner = width - 2.0 * (t.metrics.status_pad_h + t.stroke.card);
    let mut needed = t.metrics.dot
        + view::STATUS_DOT_TEXT_GAP
        + text_w(&model::display(raw), WORD)
        + 12.0
        + text_w(&card.shown_line(), WORD);
    if let Some(action) = card.action {
        needed +=
            text_w(&model::display(action), t.type_.button) + 2.0 * view::TAKE_OVER_PAD[1] + 12.0;
    }
    (needed <= inner).then_some(raw)
}

/// Optional progress line under the status row. One line, cut with "…",
/// at a fixed line height. Not reserved when there is no progress. The
/// bro name is not drawn here.
fn progress_cell<'a>(t: &Skin, card: &'a StatusCard, width: f32) -> Element<'a, Message> {
    use iced::widget::text::{LineHeight, Wrapping};
    let Some(progress) = &card.progress else {
        return space::horizontal().into();
    };
    let size = t.type_.status;
    let room = width - 2.0 * (t.metrics.status_pad_h + t.stroke.card);
    let cols = (room / (size * t.type_.unit_width)).max(8.0) as usize;
    let fitted = robroz_ui::cards::status::clamp_lines(&model::display(progress), cols, 1);
    let rgb = robroz_ui::cards::status::progress_rgb(t.chat.bro_bg, t.palette.text);
    let line = text(fitted)
        .size(size)
        .line_height(LineHeight::Absolute(progress_line_height(size).into()))
        .wrapping(Wrapping::None)
        .color(color(rgb));
    container(container(line).clip(true).width(Fill))
        .width(Fill)
        .into()
}

/// Fixed height of the progress line: the card never jumps as the text changes.
fn progress_line_height(size: f32) -> f32 {
    (size * 1.5).round()
}

fn delivery_setup(card: &DeliveryCard) -> Vec<(Message, Text, Text)> {
    let mut out = Vec::new();
    if card.needs_model_setup {
        out.push((
            Message::OpenModel,
            model_page::GO_SETUP,
            model_page::NEEDS_SETUP_HINT,
        ));
    }
    if card.needs_search_setup {
        out.push((
            Message::OpenSearch,
            search_page::GO_SETUP,
            search_page::NEEDS_SETUP_HINT,
        ));
    }
    out
}

fn delivery_view<'a>(
    t: &Skin,
    card: &'a DeliveryCard,
    width: f32,
    detail_open: bool,
) -> Element<'a, Message> {
    // Did not get done: the same grey as 「没写出结果」, no path, no buttons.
    if let Some(failure) = card.shown_failure() {
        // Not done: a plain bro card with a grey dot, no grey fill.
        let line = row![
            view::status_dot(t, robroz_ui::cards::status::Dot::Void).map(Message::Ui),
            text(failure),
        ]
        .spacing(view::STATUS_DOT_TEXT_GAP)
        .align_y(Alignment::Center);
        let setup = delivery_setup(card);
        let mut body: Element<'a, Message> = if setup.is_empty() {
            line.into()
        } else {
            let mut r = row![line, space::horizontal()];
            for (msg, label, _) in setup {
                r = r.push(
                    button(text(label).size(t.type_.button))
                        .style(view::nav_button_style(*t, false))
                        .on_press(msg),
                );
            }
            r.align_y(Alignment::Center).into()
        };
        if let Some(detail) = card.collapsed_detail() {
            body = column![body, fail_detail_row(t, &card.id, &detail, detail_open)]
                .spacing(t.metrics.dot_gap)
                .into();
        }
        return container(body)
            .padding(view::status_card_padding(t))
            .width(Fill)
            .style(view::card_style(*t))
            .into();
    }
    let path = view::display_path(
        t,
        &card.path,
        width - 2.0 * (t.controls.card_pad_h + t.stroke.card),
    );
    let mut head = row![
        text(model::display(&card.summary).into_owned()).color(color(t.palette.text)),
        space::horizontal()
    ]
    .align_y(Alignment::Center);
    if let Some(badge) = card.shown_badge() {
        head = head.push(
            container(text(badge).size(t.type_.tag).color(color(t.palette.text2)))
                .padding(Padding::from([2.0, t.metrics.tag_pad_h]))
                .style(view::card_style(*t)),
        );
    }
    for (msg, label, _) in delivery_setup(card) {
        head = head.push(
            button(text(label).size(t.type_.button))
                .style(view::nav_button_style(*t, false))
                .on_press(msg),
        );
    }
    // Display only: plain text, not a link or button.
    let path_line = text(path)
        .wrapping(iced::widget::text::Wrapping::None)
        .font(Font::MONOSPACE)
        .size(t.type_.mono)
        .color(color(t.palette.text2));
    let mut body = column![head, path_line].spacing(6);
    for (_, _, hint) in delivery_setup(card) {
        body = body.push(text(hint).size(t.type_.foot).color(color(t.palette.text2)));
    }
    container(body)
        .padding(view::card_padding(t))
        .width(Fill)
        .style(view::card_style(*t))
        .into()
}

#[cfg(test)]
mod tests {
    use super::{decode_window_icon, pick_name, window_icon, WINDOW_ICON_SIZE};

    #[test]
    fn embedded_window_icon_decodes() {
        let (rgba, width, height) = decode_window_icon().expect("embedded icon should decode");
        assert_eq!((width, height), (WINDOW_ICON_SIZE, WINDOW_ICON_SIZE));
        assert_eq!(
            rgba.len(),
            (WINDOW_ICON_SIZE * WINDOW_ICON_SIZE * 4) as usize
        );
        assert!(
            rgba.iter().any(|&b| b != 0),
            "window icon RGBA should not be all zeros"
        );
        assert!(window_icon().is_some());
    }

    #[test]
    fn user_name_env_then_hostname_then_robroz() {
        let s = |v: &str| Some(v.to_string());
        assert_eq!(pick_name(s(" Panda 的 Mac "), s("mbp")), "Panda 的 Mac");
        assert_eq!(pick_name(s("  "), s("mbp.local\n")), "mbp.local");
        assert_eq!(pick_name(None, s("box")), "box");
        assert_eq!(pick_name(None, None), "robroz");
        assert_eq!(pick_name(s("  "), s("")), "robroz");
        let long = "字".repeat(60);
        assert_eq!(pick_name(Some(long), None).chars().count(), 40);
    }

    fn live_step_card() -> crate::model::StatusCard {
        use robroz_contract::TaskState;
        let look = robroz_card::appearance(TaskState::Running, true);
        crate::model::StatusCard {
            task_id: "t".into(),
            bot_name: "Ada".into(),
            status_line: robroz_card::status_line(TaskState::Running, true).into(),
            tone: look.tone,
            palette: robroz_card::palette(look.tone),
            action: look.action,
            progress: None,
            step_progress: Some(robroz_contract::host_text::router_step_progress(2, 3)),
            status_code: None,
            failure_code: None,
            failure_detail: None,
            failure: None,
        }
    }

    #[test]
    fn every_status_card_is_a_plain_card_with_a_dot() {
        use robroz_card::Tone;
        use robroz_ui::cards::status::Dot;
        use robroz_ui::{Mode, Skin};
        for mode in [Mode::Light, Mode::Dark] {
            let t = Skin::for_mode(mode);
            let mut card = live_step_card();
            assert_eq!(
                super::status_dot_kind(&card),
                None,
                "running uses the accent"
            );
            assert_eq!(
                t.chat.accent,
                if mode == Mode::Light {
                    [0xc8, 0x57, 0x2a]
                } else {
                    [0xe0, 0x74, 0x3f]
                }
            );
            for (tone, dot) in [
                (Tone::Green, Dot::Done),
                (Tone::Amber, Dot::Stuck),
                (Tone::Gray, Dot::Void),
            ] {
                card.tone = tone;
                assert_eq!(super::status_dot_kind(&card), Some(dot));
            }
        }
    }

    #[test]
    fn take_over_button_padding_is_6_by_12() {
        assert_eq!(robroz_ui::view::TAKE_OVER_PAD, [6.0, 12.0]);
    }

    #[test]
    fn status_dot_sits_8px_before_the_status_word() {
        assert_eq!(robroz_ui::view::STATUS_DOT_TEXT_GAP, 8.0);
    }

    #[test]
    fn router_step_fits_on_a_desktop_card_and_is_dropped_when_narrow() {
        use robroz_ui::i18n::{scoped, Locale};
        use robroz_ui::{Mode, Skin};
        let t = Skin::for_mode(Mode::Light);
        let card = live_step_card();
        let mut no_step = live_step_card();
        no_step.step_progress = None;
        assert_eq!(super::visible_step_progress(&t, &no_step, 800.0), None);

        let _en = scoped(Locale::En);
        assert_eq!(
            super::visible_step_progress(&t, &card, 800.0)
                .map(crate::model::display)
                .as_deref(),
            Some("Step 2 of 3")
        );
        assert_eq!(
            super::visible_step_progress(&t, &card, 200.0),
            None,
            "drop the step rather than wrapping the status word"
        );

        let _zh = scoped(Locale::Zh);
        assert_eq!(
            super::visible_step_progress(&t, &card, 800.0)
                .map(crate::model::display)
                .as_deref(),
            Some("第 2/3 步")
        );
        assert_eq!(super::visible_step_progress(&t, &card, 120.0), None);
    }
}
