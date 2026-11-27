//! `--gallery`: every state of the four new cards on one page, for design
//! review and screenshots. Static data, no host.

use robroz_contract::ChoiceOption;
use robroz_ui::cards::interact::CardUi;
use robroz_ui::cards::routines::{RoutineRow, Routines};
use robroz_ui::cards::{
    ActionKind, ApprovalCard, ApprovalState, Card, ChoiceCard, ChoiceState, DraftCard,
    DraftChannel, DraftState,
};

use crate::source::DRAFT_BODY;

pub struct Sample {
    pub caption: &'static str,
    pub card: Card,
    pub ui: CardUi,
    /// Drawn as in a group chat (the v0.2 owner-only approval card).
    pub group: bool,
}

pub struct Gallery {
    pub choice: Vec<Sample>,
    pub approval: Vec<Sample>,
    pub draft: Vec<Sample>,
    pub routines: Routines,
    pub empty: Routines,
    /// Reference time for "today/tomorrow" labels and footnotes.
    pub now: i64,
}

fn opt(id: &str, label: &str, primary: bool) -> ChoiceOption {
    ChoiceOption {
        id: id.into(),
        label: label.into(),
        primary,
    }
}

fn install(state: ChoiceState) -> Card {
    Card::Choice(ChoiceCard {
        id: "c-install".into(),
        bot: "文档哥".into(),
        prompt: "安装说明先写哪一种？".into(),
        options: vec![
            opt("docker", "Docker 一键", true),
            opt("cargo", "cargo install", false),
            opt("both", "两种都写", false),
        ],
        multi: false,
        state,
    })
}

fn approval(state: ApprovalState, bot: &str) -> Card {
    Card::Approval(ApprovalCard {
        id: "a-logs".into(),
        bot: bot.into(),
        kind: ActionKind::Delete,
        summary: "清理 12 个旧日志文件".into(),
        path: Some("workspace/robroz/logs/2026-10/".into()),
        state,
        mcp_server: None,
        mcp_tool: None,
        reason: None,
        reason_code: None,
        owner_id: None,
        owner_name: None,
    })
}

fn draft(state: DraftState) -> Card {
    Card::Draft(DraftCard {
        id: "d-progress".into(),
        bot: "文档哥".into(),
        channel: DraftChannel::Email,
        to: "team@example.com".into(),
        subject: Some("robroz 第一期进度".into()),
        body: DRAFT_BODY.into(),
        state,
    })
}

fn sample(caption: &'static str, card: Card) -> Sample {
    Sample {
        caption,
        card,
        ui: CardUi::default(),
        group: false,
    }
}

/// A group approval card owned by `owner` (the viewer is [`FAKE_ME_ID`]).
fn group_approval(caption: &'static str, state: ApprovalState, owner: (&str, &str)) -> Sample {
    let Card::Approval(mut card) = approval(state, "Ada") else {
        unreachable!()
    };
    card.owner_id = Some(owner.0.into());
    card.owner_name = Some(owner.1.into());
    Sample {
        caption,
        card: Card::Approval(card),
        ui: CardUi::default(),
        group: true,
    }
}

impl Gallery {
    /// Narrow screens: flip the full/short bot name on every sample card
    /// with this id.
    pub fn toggle_name(&mut self, id: &str) {
        for s in self
            .choice
            .iter_mut()
            .chain(self.approval.iter_mut())
            .chain(self.draft.iter_mut())
        {
            if s.card.id() == id {
                s.ui.name_expanded = !s.ui.name_expanded;
            }
        }
    }

    pub fn new() -> Self {
        // A fixed local wall time keeps screenshots stable: today 01:00.
        let now = chrono::Local::now()
            .date_naive()
            .and_hms_opt(1, 0, 0)
            .and_then(|t| t.and_local_timezone(chrono::Local).single())
            .map_or(0, |t| t.timestamp());
        let at = |min: i64| now + 50 * 60 + min * 60;
        let iso = |t: i64| {
            chrono::DateTime::from_timestamp(t, 0)
                .unwrap_or_default()
                .to_rfc3339()
        };

        let me = (crate::api::FAKE_ME_ID, crate::api::FAKE_ME_NAME);
        let lin = ("u-lin", "Lin");
        let multi = |picks: Vec<&str>| {
            let card = Card::Choice(ChoiceCard {
                id: "c-pages".into(),
                bot: "前端".into(),
                prompt: "第一期客户端先做哪几页？".into(),
                options: vec![
                    opt("chat", "群聊", false),
                    opt("routines", "例行任务", false),
                    opt("settings", "设置", false),
                    opt("files", "文件", false),
                ],
                multi: true,
                state: ChoiceState::Open,
            });
            Sample {
                caption: "choice · open · multi",
                card,
                ui: CardUi {
                    picks: picks.into_iter().map(String::from).collect(),
                    ..CardUi::default()
                },
                group: false,
            }
        };
        let answered = sample(
            "choice · answered（同一张卡就地更新）",
            install(ChoiceState::Answered {
                selected: vec!["both".into()],
                at: Some(iso(at(2))),
                by: None,
            }),
        );

        let row = |id: &str, name: &str, hours: i64, paused: bool| RoutineRow {
            id: id.into(),
            name: name.into(),
            next_run_at: Some(now + hours * 3600),
            paused,
            group_id: "g".into(),
            assignee_bot_id: "bot".into(),
        };
        let mut routines = Routines::default();
        routines.load(vec![
            row("s1", "每日简报", 31, false),
            row("s2", "依赖安全检查", 80, false),
            row("s3", "周报草稿", 50, true),
            row("s4", "清理沙箱缓存", 22, false),
        ]);
        let mut empty = Routines::default();
        empty.load(Vec::new());

        Self {
            choice: vec![
                sample("choice · open · single", install(ChoiceState::Open)),
                answered,
                multi(vec!["chat", "routines"]),
                sample("choice · expired", install(ChoiceState::Expired)),
            ],
            approval: vec![
                sample(
                    "approval · pending",
                    approval(ApprovalState::Pending, "后端"),
                ),
                sample(
                    "approval · approved",
                    approval(
                        ApprovalState::Approved {
                            by: Some("我".into()),
                            at: at(2),
                        },
                        "后端",
                    ),
                ),
                sample(
                    "approval · denied",
                    approval(
                        ApprovalState::Denied {
                            by: None,
                            at: at(3),
                        },
                        "后端",
                    ),
                ),
                sample(
                    "approval · needs owner sign-in（群里只给这一个按钮）",
                    approval(ApprovalState::NeedsOwner, "顶级GitHub哥"),
                ),
                group_approval(
                    "approval · group · owner · waiting",
                    ApprovalState::Pending,
                    me,
                ),
                group_approval(
                    "approval · group · owner · approved",
                    ApprovalState::Approved {
                        by: None,
                        at: at(2),
                    },
                    me,
                ),
                group_approval(
                    "approval · group · not owner · waiting",
                    ApprovalState::Pending,
                    lin,
                ),
                group_approval(
                    "approval · group · not owner · approved",
                    ApprovalState::Approved {
                        by: None,
                        at: at(2),
                    },
                    lin,
                ),
                group_approval(
                    "approval · group · not owner · denied",
                    ApprovalState::Denied {
                        by: None,
                        at: at(3),
                    },
                    lin,
                ),
                group_approval(
                    "approval · group · expired",
                    ApprovalState::Expired { at: at(30) },
                    me,
                ),
            ],
            draft: vec![
                sample("draft · pending", draft(DraftState::Pending)),
                sample(
                    "draft · sent（预览收成一行）",
                    draft(DraftState::Sent {
                        by: "Panda".into(),
                        at: at(4),
                    }),
                ),
            ],
            routines,
            empty,
            now,
        }
    }
}
