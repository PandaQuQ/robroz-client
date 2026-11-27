use std::collections::BTreeSet;

use super::*;

fn placeholders(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        let inner = &rest[start + 1..start + len];
        if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
            out.insert(inner.to_string());
        }
        rest = &rest[start + len + 1..];
    }
    out
}

#[test]
fn default_locale_is_english() {
    assert_eq!(Locale::default(), Locale::En);
    // A fresh test thread with no override sees the app default.
    std::thread::spawn(|| assert_eq!(locale(), Locale::En))
        .join()
        .unwrap();
    assert_eq!(super::super::cards::review_rules::ADD.as_str(), "Add");
}

#[test]
fn every_key_has_text_in_both_locales() {
    // `en::text` / `zh::text` are exhaustive matches, so a missing key does
    // not compile; this checks that no entry is left empty.
    assert!(!Key::ALL.is_empty());
    for key in Key::ALL {
        for l in Locale::ALL {
            assert!(!text_in(l, *key).is_empty(), "{key:?} is empty in {l}");
        }
    }
}

#[test]
fn both_locales_use_the_same_placeholders() {
    for key in Key::ALL {
        assert_eq!(
            placeholders(text_in(Locale::En, *key)),
            placeholders(text_in(Locale::Zh, *key)),
            "{key:?}"
        );
    }
}

#[test]
fn key_list_has_no_duplicates() {
    let set: std::collections::HashSet<_> = Key::ALL.iter().collect();
    assert_eq!(set.len(), Key::ALL.len());
}

#[test]
fn scoped_override_is_per_thread_and_restores() {
    assert_eq!(locale(), Locale::En);
    {
        let _zh = scoped(Locale::Zh);
        assert_eq!(locale(), Locale::Zh);
        assert_eq!(Key::ReviewRulesAdd.as_str(), "添加");
        std::thread::spawn(|| assert_eq!(locale(), Locale::En))
            .join()
            .unwrap();
    }
    assert_eq!(locale(), Locale::En);
}

#[test]
fn chinese_keeps_the_original_wording() {
    let _zh = scoped(Locale::Zh);
    assert_eq!(Key::ReviewRulesDrop.as_str(), "不用");
    assert_eq!(Key::ReviewRulesDeleteAction.as_str(), "删除规则");
    assert_eq!(
        Key::ReviewRulesTooLong.fill(&[&200, &5]),
        "最多 200 字，还要删掉 5 字"
    );
    assert_eq!(Key::StatusKindUnknown.as_str(), "操作");
}

#[test]
fn english_batch_one_copy() {
    let _en = scoped(Locale::En);
    assert_eq!(Key::ReviewRulesTitle.as_str(), "Allow rules");
    assert_eq!(Key::ReviewRulesDrop.as_str(), "Not now");
    assert_eq!(
        Key::ReviewRulesTooLong.fill(&[&200, &5]),
        "Max 200 characters. Remove 5 more."
    );
    assert_eq!(
        Key::ReviewRulesBlockedReasonPrefix.as_str(),
        "Blocked because: "
    );
    assert_eq!(Key::StatusKindUnknown.as_str(), "Action");
    assert_eq!(
        crate::cards::confirm::decide_failed("timeout"),
        "Failed: timeout"
    );
}

#[test]
fn locale_codes_parse() {
    assert_eq!(Locale::parse("en"), Some(Locale::En));
    assert_eq!(Locale::parse("en_GB.UTF-8"), Some(Locale::En));
    assert_eq!(Locale::parse("zh"), Some(Locale::Zh));
    assert_eq!(Locale::parse(" zh-CN\n"), Some(Locale::Zh));
    assert_eq!(Locale::parse("fr"), None);
    for l in Locale::ALL {
        assert_eq!(Locale::parse(l.code()), Some(l));
        assert_ne!(l.other(), l);
    }
}

#[test]
fn month_day_per_locale() {
    let _en = scoped(Locale::En);
    assert_eq!(month_day(11, 20), "Nov 20");
    assert_eq!(month_day(1, 3), "Jan 3");
    let _zh = scoped(Locale::Zh);
    assert_eq!(month_day(11, 20), "11月20日");
}

mod host {
    use super::super::host::host_text_in;
    use super::super::Locale::{En, Zh};
    use robroz_contract::{
        TaskState, APPROVAL_EXPIRED_REASON, APPROVAL_SENT_TO_OWNER_DM_NOTICE, MODEL_TEST_BAD_KEY,
        SETTINGS_APPROVAL_BOT_NAME,
    };

    #[test]
    fn contract_constants_map_to_english() {
        assert_eq!(
            host_text_in(En, APPROVAL_SENT_TO_OWNER_DM_NOTICE),
            "Sent to owner's DM"
        );
        assert_eq!(
            host_text_in(En, APPROVAL_EXPIRED_REASON),
            "Approval expired"
        );
        assert_eq!(host_text_in(En, SETTINGS_APPROVAL_BOT_NAME), "Settings");
        assert_eq!(
            host_text_in(En, MODEL_TEST_BAD_KEY),
            "key is wrong or missing"
        );
        assert_eq!(
            host_text_in(En, robroz_card::NO_RESULT),
            "Finished with no result."
        );
    }

    #[test]
    fn robroz_card_copy_maps_to_english() {
        assert_eq!(
            host_text_in(En, robroz_card::status_line(TaskState::Queued, false)),
            "Queued"
        );
        assert_eq!(
            host_text_in(En, robroz_card::status_line(TaskState::Running, true)),
            "Running"
        );
        assert_eq!(
            host_text_in(En, robroz_card::status_line(TaskState::Done, false)),
            "Done"
        );
        assert_eq!(
            host_text_in(
                En,
                robroz_card::appearance(TaskState::Blocked, false)
                    .action
                    .unwrap()
            ),
            "Take over in DM"
        );
        assert_eq!(
            host_text_in(En, &robroz_card::failure_line("工具调用超过 8 轮")),
            "Couldn't finish: Stopped after 8 rounds of tool calls"
        );
        assert_eq!(
            host_text_in(En, &robroz_card::failure_line(robroz_card::NO_RESULT)),
            "Finished with no result."
        );
    }

    #[test]
    fn prefixes_map_and_keep_the_value() {
        assert_eq!(host_text_in(En, "在用 web_search"), "Using web_search");
        assert_eq!(
            host_text_in(En, "你拒绝了：发送周报"),
            "You denied: 发送周报"
        );
        assert_eq!(
            host_text_in(En, "要对外写入：git push"),
            "Writes to an outside service: git push"
        );
    }

    #[test]
    fn chinese_and_unknown_strings_pass_through() {
        assert_eq!(
            host_text_in(Zh, APPROVAL_SENT_TO_OWNER_DM_NOTICE),
            APPROVAL_SENT_TO_OWNER_DM_NOTICE
        );
        assert_eq!(host_text_in(Zh, "在用 web_search"), "在用 web_search");
        assert_eq!(host_text_in(En, "这条会改生产数据"), "这条会改生产数据");
        assert_eq!(host_text_in(En, "Ada"), "Ada");
        // A bare prefix with nothing after it is not rewritten.
        assert_eq!(host_text_in(En, "在用 "), "Using ");
    }
}

#[test]
fn placeholder_looking_user_text_is_not_interpreted() {
    assert_eq!(
        fill_template("{0}: {1}", &[&"value with {1}", &"tail"]),
        "value with {1}: tail"
    );
    assert_eq!(fill_template("literal {0", &[]), "literal {0");
}

#[test]
fn english_batch_two_copy() {
    let _en = scoped(Locale::En);
    assert_eq!(Key::CardsMe.as_str(), "You");
    assert_eq!(Key::InteractFailed.fill(&[&"x"]), "Failed: x");
    assert_eq!(Key::ModelPresetPlaceholder.as_str(), "Choose a provider");
    assert_eq!(crate::cards::mcp::tool_count_text(Some(1)), "1 tool");
    assert_eq!(crate::cards::mcp::tool_count_text(Some(3)), "3 tools");
    assert_eq!(Key::SettingsLanguage.as_str(), "Language / 语言");
    let _zh = scoped(Locale::Zh);
    assert_eq!(crate::cards::mcp::tool_count_text(Some(1)), "1 个工具");
    assert_eq!(crate::cards::mcp::tool_count_text(Some(3)), "3 个工具");
    assert_eq!(Key::SettingsLanguage.as_str(), "Language / 语言");
}

#[test]
fn language_names_are_native_in_both_locales() {
    for l in [Locale::En, Locale::Zh] {
        let _s = scoped(l);
        assert_eq!(Locale::En.native_name(), "English");
        assert_eq!(Locale::Zh.native_name(), "中文");
        // Same bilingual label in both UIs, so the switch can be found
        // when the current language is unreadable.
        assert_eq!(Key::SettingsLanguage.as_str(), "Language / 语言");
    }
}

#[test]
fn appearance_row_copy_is_english_first_then_chinese() {
    let _en = scoped(Locale::En);
    assert_eq!(Key::SettingsAppearance.as_str(), "Appearance");
    assert_eq!(Key::SettingsThemeSystem.as_str(), "System");
    assert_eq!(Key::ClientThemeLight.as_str(), "Light");
    assert_eq!(Key::ClientThemeDark.as_str(), "Dark");
    let _zh = scoped(Locale::Zh);
    assert_eq!(Key::SettingsAppearance.as_str(), "外观");
    assert_eq!(Key::SettingsThemeSystem.as_str(), "跟随系统");
    assert_eq!(Key::ClientThemeLight.as_str(), "浅色");
    assert_eq!(Key::ClientThemeDark.as_str(), "深色");
}

#[test]
fn every_contract_host_string_maps_to_english() {
    use robroz_contract::host_text::{self as ht, HOST_UI_STRINGS};
    for &(name, text) in HOST_UI_STRINGS {
        // The stall suffix never maps on its own; its English exists only
        // as the whole `stall_hint` sentence (checked below).
        let whole;
        let text = if name == "WATCHDOG_STALL_SUFFIX" {
            whole = ht::stall_hint(5);
            whole.as_str()
        } else {
            text
        };
        let mapped = host_text_in(Locale::En, text);
        assert_ne!(mapped.as_ref(), text, "unmapped {name}: {text}");
        assert!(!mapped.contains('{'), "unfilled template {name}: {mapped}");
        assert!(
            !mapped
                .chars()
                .any(|c| ('\u{3400}'..='\u{9FFF}').contains(&c)),
            "Chinese left in {name}: {mapped}"
        );
        if let Some((n, m)) = ht::parse_router_step_progress(text) {
            // Display-only: host still sends 「n/m 步」, the card shows 「第 n/m 步」.
            assert_eq!(
                host_text_in(Locale::Zh, text),
                Key::HostRouterStep.fill_in(Locale::Zh, &[&n, &m])
            );
        } else {
            assert_eq!(host_text_in(Locale::Zh, text), text);
        }
    }
}

#[test]
fn contract_prefixes_suffixes_and_hint_functions_map() {
    use robroz_contract::host_text as ht;
    for &(name, text) in ht::HOST_UI_STRINGS {
        if name.ends_with("_PREFIX") && name != "WATCHDOG_STEP_TIMEOUT_PREFIX" {
            let source = format!("{text}unchanged-value");
            let mapped = host_text_in(Locale::En, &source);
            assert_ne!(mapped.as_ref(), source, "{name}");
            assert!(mapped.contains("unchanged-value"), "{name}: {mapped}");
        }
    }
    for n in [0, 1, 7, 180] {
        assert_eq!(
            host_text_in(Locale::En, &ht::step_timeout_hint(n)),
            format!("No reply from the model or tools for {n} min")
        );
        assert_eq!(
            host_text_in(Locale::En, &ht::stall_hint(n)),
            format!("No new progress for {n} min")
        );
    }
    assert_eq!(
        host_text_in(Locale::En, &ht::using_tool("web_search")),
        "Using web_search"
    );
    assert_eq!(
        host_text_in(
            Locale::En,
            &format!(
                "{}{}",
                ht::CANT_CONNECT_PREFIX,
                robroz_contract::MODEL_TEST_BAD_KEY
            )
        ),
        "Can't connect: key is wrong or missing"
    );
}

#[test]
fn stall_sentences_map_whole_never_from_the_suffix() {
    use robroz_contract::host_text as ht;
    assert_eq!(
        host_text_in(Locale::En, &ht::stall_hint(12)),
        "No new progress for 12 min"
    );
    // The bare suffix and anything that isn't exactly `stall_hint(n)` pass
    // through unchanged instead of being stitched from fragments.
    for s in [
        ht::WATCHDOG_STALL_SUFFIX.to_string(),
        format!("abc{}", ht::WATCHDOG_STALL_SUFFIX),
        format!("07{}", ht::WATCHDOG_STALL_SUFFIX),
        format!("{}!", ht::stall_hint(3)),
    ] {
        assert_eq!(host_text_in(Locale::En, &s), s);
    }
    assert!(!Key::ALL
        .iter()
        .any(|k| text_in(Locale::En, *k).contains("without new progress")));
}

#[test]
fn every_english_entry_is_approved() {
    let en = include_str!("en.rs");
    assert!(!en.contains("[draft]"), "draft copy left in en.rs");
    let approved = en.matches("  [approved").count();
    // New strings waiting on the designer use this comment instead of a
    // false approved tag.
    let pending = en.matches("// Needs UI review.").count();
    assert_eq!(
        approved + pending,
        Key::ALL.len(),
        "every key carries an approved tag or is marked for UI review"
    );
}

#[test]
fn router_steps_and_tool_results_map_by_contract() {
    use robroz_contract::host_text as ht;
    assert_eq!(
        host_text_in(Locale::En, &ht::router_step_progress(2, 3)),
        "Step 2 of 3"
    );
    assert_eq!(
        host_text_in(Locale::Zh, &ht::router_step_progress(2, 3)),
        "第 2/3 步"
    );
    // Not a step line: left alone.
    assert_eq!(host_text_in(Locale::En, "2/3步"), "2/3步");
    assert_eq!(
        host_text_in(Locale::En, ht::TOOL_STILL_NEEDS_APPROVAL),
        "Call failed: this tool still needs approval"
    );
    assert_eq!(
        host_text_in(
            Locale::En,
            &format!("{}disk full", ht::TOOL_CALL_FAILED_PREFIX)
        ),
        "Call failed: disk full"
    );
}

#[test]
fn pr4_review_copy_is_in_place() {
    let zh = |k| text_in(Locale::Zh, k);
    let en = |k| text_in(Locale::En, k);
    assert_eq!(zh(Key::GroupMenuFind), "在聊天里找");
    assert_eq!(zh(Key::FindInChatTitle), "在聊天里找");
    assert_eq!(zh(Key::GroupMenuDelete), "删除群");
    assert_eq!(en(Key::GroupMenuDelete), "Delete group");
    assert_eq!(zh(Key::FindInChatEmpty), "输入关键词搜这个聊天");
    assert_eq!(en(Key::FindInChatEmpty), "Search this chat");
    assert_eq!(zh(Key::FindInChatNoResults), "没找到相关消息");
    assert_eq!(en(Key::FindInChatNoResults), "No matching messages");
    // The confirm button keeps the short verb.
    assert_eq!(zh(Key::GroupDeleteConfirm), "删除");
    assert_eq!(en(Key::GroupDeleteConfirm), "Delete");
    assert_eq!(zh(Key::GroupMenuRename), "改名");
    assert_eq!(en(Key::GroupMenuRename), "Rename");
    assert_eq!(zh(Key::ApprovalWaitingFor), "等这个 bro 的主人审批");
    assert_eq!(en(Key::ApprovalWaitingFor), "Waiting for this bro's owner");
    assert_eq!(zh(Key::ProcessWorkedUnder1), "用了不到 1 秒");
    assert_eq!(en(Key::ProcessWorkedUnder1), "Worked for under 1s");
    assert_eq!(zh(Key::ProcessWorkingUnder1), "处理中不到 1 秒");
    assert_eq!(en(Key::ProcessWorkingUnder1), "Working under 1s");
}
