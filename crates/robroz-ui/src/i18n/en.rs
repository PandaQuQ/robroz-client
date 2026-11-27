//! English copy: one entry per [`Key`], the comment above each is the
//! Chinese source (`zh.rs`).
//!
//! Copy owner: the designer. Every entry is approved and marked with the
//! batch it came in (`[approved, batch 1]`, `[approved, batch 2]`,
//! `[approved, batch 3]` for the host strings from
//! `robroz_contract::host_text`, `[approved, settings row]` or
//! `[approved, disconnected state]`).
//! Edit the strings here only; keep each `{0}`, `{1}` placeholder that the
//! Chinese has. The Settings language row (`SettingsLanguage`, `LocaleName*`)
//! keeps the option names native in both files.

use super::Key;

pub(super) fn text(key: Key) -> &'static str {
    match key {
        // ---- Host strings mapped by host_text (contract constants and host-crate text) ----
        // 已发到主人私聊  [approved, batch 1]
        Key::HostSentToOwnerDm => "Sent to owner's DM",
        // 审批已过期  [approved, batch 1]
        Key::HostApprovalExpired => "Approval expired",
        // 设置  [approved, batch 2]
        Key::HostSettingsBotName => "Settings",
        // 密钥不对或没填  [approved, batch 1]
        Key::HostModelBadKey => "key is wrong or missing",
        // 没写出结果。  [approved, batch 1]
        Key::HostNoResult => "Finished with no result.",
        // 开始干活  [approved, batch 2]
        Key::HostProgressStarted => "Getting started",
        // 在想怎么回  [approved, batch 2]
        Key::HostProgressThinking => "Thinking about a reply",
        // 在沙箱里跑  [approved, batch 2]
        Key::HostProgressInSandbox => "Running in the sandbox",
        // 自动审核没有成功，需要你批准  [approved, batch 2]
        Key::HostReviewFailed => "Auto-review failed. Needs your approval.",
        // 没有放行规则允许这个操作  [approved, batch 2]
        Key::HostReviewNoMatch => "No allow rule covers this action",
        // 还没设置模型，没法自动审核  [approved, batch 2]
        Key::HostReviewNoModel => "No model set up, so auto-review can't run",
        // 工具调用超过 8 轮  [approved, batch 2]
        Key::HostToolRoundLimit => "Stopped after 8 rounds of tool calls",
        // 调用失败：这个工具还需要批准  [approved, batch 2]
        Key::HostToolNeedsApproval => "Call failed: this tool still needs approval",
        // 还没设置模型  [approved, batch 2]
        Key::HostModelNotConfigured => "No model set up yet",
        // 还没设置联网搜索  [approved, batch 2]
        Key::HostSearchNotConfigured => "Web search isn't set up yet",
        // 不支持这个动作  [approved, batch 2]
        Key::HostUnsupportedAction => "This action isn't supported",
        // 请先回答审批卡  [approved, batch 2]
        Key::HostResumeNeedsApproval => "Answer the approval card first",
        // 恢复失败，找不到卡住记录  [approved, batch 2]
        Key::HostResumeNoIntent => "Couldn't resume: no record of where it got stuck",
        // 恢复失败，无法写入接管标记  [approved, batch 2]
        Key::HostResumeMarkerFailed => "Couldn't resume: couldn't save the takeover marker",
        // 这个名字已经有了  [approved, batch 2]
        Key::HostNameTaken => "That name is already taken",
        // MCP 服务器不存在  [approved, batch 2]
        Key::HostMcpNotFound => "MCP server not found",
        // 群不存在  [approved, batch 2]
        Key::HostGroupNotFound => "Group not found",
        // 头像的形状或颜色超出范围  [approved, 最优雅UI姐 avatar errors]
        Key::HostInvalidAvatar => "Couldn't save the avatar. Try again",
        // 只有建群的人或管理员能改这个群  [approved, contract-v0.3.0 codes rev 2]
        Key::HostNotGroupCreator => "Only the creator or an admin can change this group",
        // 你不在这个群里  [approved, contract-v0.3.0 codes rev 2]
        Key::HostNotGroupMember => "You're not in this group",
        // 私聊不能改名或删除  [approved, contract-v0.3.0 codes rev 2]
        Key::HostGroupIsDm => "DMs can't be renamed or deleted",
        // 只有 stdio 服务器需要批准启动命令  [approved, batch 2]
        Key::HostStdioOnlyApprove => "Only stdio servers need their start command approved",
        // 这个 bro 不存在  [approved, batch 2]
        Key::HostUnknownBro => "That bro doesn't exist",
        // 记忆不存在  [approved, batch 2]
        Key::HostMemoryNotFound => "Memory not found",
        // 技能不存在  [approved, batch 2]
        Key::HostSkillNotFound => "Skill not found",
        // 这个 slug 已经有了  [approved, batch 2]
        Key::HostSlugTaken => "That slug is already taken",
        // slug 只能是小写字母、数字和连字符  [approved, batch 2]
        Key::HostBadSlug => "A slug can only use lowercase letters, digits and hyphens",
        // 在用 {0}  [approved, batch 2]
        Key::HostProgressUsing => "Using {0}",
        // 你拒绝了：{0}  [approved, batch 2]
        Key::HostDeniedFailure => "You denied: {0}",
        // 要对外写入：{0}  [approved, batch 2]
        Key::HostExternalWrite => "Writes to an outside service: {0}",
        // 调用失败：{0}  [approved, batch 2]
        Key::HostCallFailed => "Call failed: {0}",
        // 没做成：{0}  [approved, batch 2]
        Key::HostFailedBecause => "Couldn't finish: {0}",
        // 排队  [approved, batch 2]
        Key::CardQueued => "Queued",
        // 进行中  [approved, batch 2]
        Key::CardRunning => "Running",
        // 完成  [approved, batch 2]
        Key::CardDone => "Done",
        // 卡住  [approved, batch 2]
        Key::CardBlocked => "Stuck",
        // 简报  [approved, batch 2]
        Key::CardBriefing => "Briefing",
        // 去私聊接管  [approved, batch 2]
        Key::CardTakeOver => "Take over in DM",
        // 这个 Bot 不在群里  [approved, batch 3]
        Key::HostBotNotInGroup => "That bro isn't in this group",
        // 请填写内容  [approved, batch 3]
        Key::HostNeedText => "Enter some text",
        // 请填写名称  [approved, batch 3]
        Key::HostNeedName => "Enter a name",
        // 请填写标题  [approved, batch 3]
        Key::HostNeedTitle => "Enter a title",
        // 请填写说明  [approved, batch 3]
        Key::HostNeedInstruction => "Enter the instructions",
        // 请填写地址  [approved, batch 3]
        Key::HostNeedUrl => "Enter a URL",
        // 请填写模型名  [approved, batch 3]
        Key::HostNeedModelName => "Enter a model name",
        // 请填写启动命令  [approved, batch 3]
        Key::HostNeedCommand => "Enter a start command",
        // 请填写密钥名称  [approved, batch 3]
        Key::HostNeedSecretName => "Enter a secret name",
        // 新增密钥时请填写值  [approved, batch 3]
        Key::HostNeedSecretValue => "Enter a value for the new secret",
        // 请填写规则  [approved, batch 3]
        Key::HostNeedRule => "Enter a rule",
        // 请填写要做什么  [approved, batch 3]
        Key::HostNeedAction => "Describe what to do",
        // 请填写 SearXNG 地址  [approved, batch 3]
        Key::HostNeedSearxngUrl => "Enter the SearXNG URL",
        // 请求格式不正确  [approved, batch 3]
        Key::HostBadRequestFormat => "The request isn't in the right format",
        // 请求体不是 JSON  [approved, batch 3]
        Key::HostBodyNotJson => "The request body isn't JSON",
        // 请求体无效  [approved, batch 3]
        Key::HostBodyInvalid => "The request body is invalid",
        // 没有要改的内容  [approved, batch 3]
        Key::HostNothingToChange => "Nothing to change",
        // 数据库错误  [approved, batch 3]
        Key::HostDatabaseError => "Database error",
        // 已经答过了  [approved, batch 3]
        Key::HostAlreadyAnswered => "Already answered",
        // since 不是合法时间  [approved, batch 3]
        Key::HostSinceInvalid => "since isn't a valid time",
        // readAt 不是合法时间  [approved, batch 3]
        Key::HostReadAtInvalid => "readAt isn't a valid time",
        // 地址要以 http:// 或 https:// 开头  [approved, batch 3]
        Key::HostUrlNeedsHttp => "The URL must start with http:// or https://",
        // 密钥不能为空  [approved, batch 3]
        Key::HostKeyEmpty => "The key can't be empty",
        // 密钥里不能有空白  [approved, batch 3]
        Key::HostKeyHasWhitespace => "The key can't contain spaces",
        // 参数无效  [approved, batch 3]
        Key::HostArgsInvalid => "Invalid arguments",
        // scope 只能是 user 或 bot:<botId>  [approved, batch 3]
        Key::HostBadScope => "scope must be user or bot:<botId>",
        // status 只能是 pending、kept 或 rejected  [approved, batch 3]
        Key::HostBadStatus => "status must be pending, kept or rejected",
        // kind 只能是 profile、habit 或 project  [approved, batch 3]
        Key::HostBadMemoryKind => "kind must be profile, habit or project",
        // kind 只能是 text、image 或 file  [approved, batch 3]
        Key::HostBadMessageKind => "kind must be text, image or file",
        // cron 格式不对  [approved, batch 3]
        Key::HostBadCron => "Invalid cron format",
        // transport 只能是 stdio 或 http  [approved, batch 3]
        Key::HostBadTransport => "transport must be stdio or http",
        // provider 只能是 brave 或 searxng  [approved, batch 3]
        Key::HostBadSearchProvider => "provider must be brave or searxng",
        // 缺少或错误的令牌  [approved, batch 3]
        Key::HostTokenRejected => "Missing or wrong token",
        // 名字太长  [approved, batch 3]
        Key::HostNameTooLong => "Name too long",
        // 名称最多 80 个字  [approved, batch 3]
        Key::HostNameMax80 => "Name: max 80 characters",
        // 名称只能一行  [approved, batch 3]
        Key::HostNameOneLine => "Name: one line only",
        // 标题太长，最多 80 个字  [approved, batch 3]
        Key::HostTitleTooLong => "Title: max 80 characters",
        // 说明太长，最多 8000 个字  [approved, batch 3]
        Key::HostInstructionTooLong => "Instructions: max 8000 characters",
        // 内容太长，最多 2000 个字  [approved, batch 3]
        Key::HostMemoryTooLong => "Max 2000 characters",
        // 说明只能一行，最多 200 字  [approved, batch 3]
        Key::HostSummaryOneLine => "Description: one line, max 200 characters",
        // 规则只能一行，最多 200 字  [approved, batch 3]
        Key::HostRuleOneLine => "Rule: one line, max 200 characters",
        // 启动命令无效  [approved, batch 3]
        Key::HostCommandInvalid => "Invalid start command",
        // 密钥名称最多 64 个字  [approved, batch 3]
        Key::HostSecretNameMax => "Secret name: max 64 characters",
        // 密钥名称里不能有空白  [approved, batch 3]
        Key::HostSecretNameWhitespace => "Secret names can't contain spaces",
        // bro 不存在  [approved, batch 3]
        Key::HostBroNotFound => "Bro not found",
        // 任务不存在  [approved, batch 3]
        Key::HostTaskNotFound => "Task not found",
        // 审批卡不存在  [approved, batch 3]
        Key::HostApprovalNotFound => "Approval card not found",
        // 审批卡不见了  [approved, batch 3]
        Key::HostApprovalVanished => "The approval card is gone",
        // 确认卡不存在  [approved, batch 3]
        Key::HostConfirmCardNotFound => "Confirm card not found",
        // 选项卡不存在  [approved, batch 3]
        Key::HostChoiceCardNotFound => "Choice card not found",
        // 规则不存在  [approved, batch 3]
        Key::HostRuleNotFound => "Rule not found",
        // 例行任务不存在  [approved, batch 3]
        Key::HostRoutineNotFound => "Routine not found",
        // 这个群还没启用这台服务器  [approved, batch 3]
        Key::HostMcpNotEnabledInGroup => "This server isn't enabled in this group yet",
        // 私聊里只能有一个 bro  [approved, batch 3]
        Key::HostDmOneBro => "A DM can only have one bro",
        // 这个动作只能由宿主机发出  [approved, batch 3]
        Key::HostActionHostOnly => "Only the host can send this action",
        // 批准失败，无法写入继续标记  [approved, batch 3]
        Key::HostApproveContinueMarkerFailed => {
            "Couldn't approve: couldn't save the continue marker"
        }
        // 批准失败，无法写入接管标记  [approved, batch 3]
        Key::HostApproveTakeoverMarkerFailed => {
            "Couldn't approve: couldn't save the takeover marker"
        }
        // 这次不能恢复  [approved, batch 3]
        Key::HostCantResume => "This task can't be resumed",
        // 选项需要 2 到 4 个  [approved, batch 3]
        Key::HostChoiceOptionCount => "Needs 2 to 4 options",
        // 推荐项最多一个  [approved, batch 3]
        Key::HostChoiceOneRecommended => "At most one option can be recommended",
        // 选项不能为空  [approved, batch 3]
        Key::HostChoiceOptionEmpty => "Options can't be empty",
        // 选项重复  [approved, batch 3]
        Key::HostChoiceDuplicate => "Duplicate option",
        // 至少选一个  [approved, batch 3]
        Key::HostChoicePickAtLeastOne => "Pick at least one",
        // 只能选一个  [approved, batch 3]
        Key::HostChoicePickOnlyOne => "Pick only one",
        // 没有这个选项  [approved, batch 3]
        Key::HostChoiceNoSuchOption => "No such option",
        // 已经选过了  [approved, batch 3]
        Key::HostChoiceAlreadyPicked => "Already picked",
        // 模型密钥里不能有空白  [approved, batch 3]
        Key::HostModelKeyHasWhitespace => "The model key can't contain spaces",
        // ROBROZ_MODEL_KEY 和 ROBROZ_MODEL_KEY_FILE 只能设一个  [approved, batch 3]
        Key::HostModelEnvKeyConflict => {
            "Set only one of ROBROZ_MODEL_KEY and ROBROZ_MODEL_KEY_FILE"
        }
        // ROBROZ_MODEL_KEY_FILE 是空的  [approved, batch 3]
        Key::HostModelEnvKeyFileEmpty => "ROBROZ_MODEL_KEY_FILE is empty",
        // ROBROZ_MODEL_KEY_FILE 读不出来：{0}  [approved, batch 3]
        Key::HostModelEnvKeyFileUnreadable => "Can't read ROBROZ_MODEL_KEY_FILE: {0}",
        // 连不上：{0}  [approved, batch 3]
        Key::HostCantConnect => "Can't connect: {0}",
        // 连接中断：{0}  [approved, batch 3]
        Key::HostConnectionLost => "Connection lost: {0}",
        // 地址无效：{0}  [approved, batch 3]
        Key::HostUrlInvalid => "Invalid URL: {0}",
        // 搜索服务返回 {0}  [approved, batch 3]
        Key::HostSearchServiceStatus => "Search service returned {0}",
        // 搜索返回的不是 JSON  [approved, batch 3]
        Key::HostSearchNotJson => "The search reply isn't JSON",
        // 搜索返回里没有 web.results  [approved, batch 3]
        Key::HostSearchNoWebResults => "The search reply has no web.results",
        // 搜索返回里没有 results  [approved, batch 3]
        Key::HostSearchNoResults => "The search reply has no results",
        // 不支持的搜索服务：{0}  [approved, batch 3]
        Key::HostSearchProviderUnsupported => "Unsupported search service: {0}",
        // 模型和工具还没回，已等 {0} 分钟  [approved, batch 3]
        Key::HostStepTimeoutHint => "No reply from the model or tools for {0} min",
        // {0} 分钟没有新进度  [approved, batch 3]
        Key::HostStallHint => "No new progress for {0} min",
        // 第 {0}/{1} 步  [approved, batch 3]
        Key::HostRouterStep => "Step {0} of {1}",
        // 已写入结果  [approved, batch 3]
        Key::HostDeliveryWritten => "Result written",
        // 未执行  [approved, batch 3]
        Key::HostDeliveryNotRun => "Didn't run",
        // 这个任务  [approved, batch 3]
        Key::HostGateFallbackLine => "this task",
        // MCP 工具  [approved, batch 3]
        Key::HostDeniedMcpFallback => "an MCP tool",
        // 模型没回：{0}  [approved, batch 3]
        Key::HostModelNoReply => "No reply from the model: {0}",
        // 等待批准：{0}  [approved, batch 3]
        Key::HostWaitingApproval => "Waiting for approval: {0}",
        // 出网超额  [approved, batch 3]
        Key::HostOutboundOverQuota => "Outbound request limit reached",
        // 无法保存审批状态：{0}  [approved, batch 3]
        Key::HostSaveApprovalStateFailed => "Couldn't save the approval state: {0}",
        // 无法继续已批准的调用：{0}  [approved, batch 3]
        Key::HostResumeApprovedCallFailed => "Couldn't continue the approved call: {0}",
        // 无法写入审批状态：{0}  [approved, batch 3]
        Key::HostPendingWriteFailed => "Couldn't write the approval state: {0}",
        // 找不到待批准的调用：{0}  [approved, batch 3]
        Key::HostPendingNotFound => "Couldn't find the call waiting for approval: {0}",
        // 待批准状态损坏：{0}  [approved, batch 3]
        Key::HostPendingCorrupt => "The pending approval state is corrupted: {0}",
        // 调用还没有被批准  [approved, batch 3]
        Key::HostPendingNotApproved => "The call hasn't been approved yet",
        // 待批准状态缺少 messages  [approved, batch 3]
        Key::HostPendingMissingMessages => "The pending approval state is missing messages",
        // 待批准状态缺少 remaining  [approved, batch 3]
        Key::HostPendingMissingRemaining => "The pending approval state is missing remaining",
        // tool call 缺少 id  [approved, batch 3]
        Key::HostPendingCallMissingId => "tool call is missing id",
        // tool call 缺少 name  [approved, batch 3]
        Key::HostPendingCallMissingName => "tool call is missing name",
        // message 缺少 role  [approved, batch 3]
        Key::HostPendingMessageMissingRole => "message is missing role",
        // 未知的 message role：{0}  [approved, batch 3]
        Key::HostPendingUnknownRole => "Unknown message role: {0}",
        // 符合放行规则  [approved, batch 3]
        Key::HostReviewAllowFallback => "Matches an allow rule",
        // 模型和工具还没回，已等   [approved, batch 3]
        Key::HostStepTimeoutPrefix => "No reply from the model or tools for ",
        //  分钟  [approved, batch 3]
        Key::HostMinutesSuffix => " min",

        // ---- Card status, footnotes and approval buttons (cards/status.rs) ----
        // 去私聊接管  [approved, batch 1]
        Key::StatusTakeOver => "Take over in DM",
        // 这一步需要你本人登录，群里不处理  [approved, batch 1]
        Key::StatusNeedsOwnerLine => "This step needs your own sign-in. Not handled in the group.",
        // 可多选  [approved, batch 1]
        Key::StatusMultiHint => "Pick any",
        // 没发出去，再点一次  [approved, batch 1]
        Key::StatusRetryNote => "Didn't send. Tap again.",
        // 没改成功  [approved, batch 1]
        Key::StatusPauseFailedNote => "Didn't change",
        // 等你选  [approved, batch 1]
        Key::StatusChoiceOpen => "Waiting for you",
        // 已选  [approved, batch 1]
        Key::StatusChoiceAnswered => "Chosen",
        // 已失效  [approved, batch 1]
        Key::StatusChoiceExpired => "Expired",
        // 等你批准  [approved, batch 1]
        Key::StatusApprovalPending => "Needs your approval",
        // 已批准  [approved, batch 1]
        Key::StatusApprovalApproved => "Approved",
        // 已拒绝  [approved, batch 1]
        Key::StatusApprovalDenied => "Denied",
        // 卡住  [approved, batch 1]
        Key::StatusApprovalStuck => "Stuck",
        // 草稿待确认  [approved, batch 1]
        Key::StatusDraftPending => "Draft to review",
        // 已发送  [approved, batch 1]
        Key::StatusDraftSent => "Sent",
        // 已放弃  [approved, batch 1]
        Key::StatusDraftDiscarded => "Discarded",
        // 删除  [approved, batch 1]
        Key::StatusKindDelete => "Delete",
        // 发送  [approved, batch 1]
        Key::StatusKindSend => "Send",
        // 付款  [approved, batch 1]
        Key::StatusKindSpend => "Pay",
        // 调用  [approved, batch 1]
        Key::StatusKindCall => "Call",
        // 操作  [approved, batch 1]
        Key::StatusKindUnknown => "Action",
        // 邮件  [approved, batch 1]
        Key::StatusChannelEmail => "Email",
        // 消息  [approved, batch 1]
        Key::StatusChannelMessage => "Message",
        // 确认（{0}）  [approved, batch 1]
        Key::StatusConfirmPicked => "Confirm ({0})",
        // 已过期，没有人作答  [approved, batch 1]
        Key::StatusChoiceExpiredFoot => "Expired, no one answered",
        // 、  [approved, batch 2]
        Key::StatusListSeparator => ", ",
        // {0} 选了「{1}」  [approved, batch 1]
        Key::StatusChoiceByFoot => "{0} chose \"{1}\"",
        // 已选「{0}」  [approved, batch 1]
        Key::StatusChoiceFoot => "Chose \"{0}\"",
        // 已批准  [approved, batch 1]
        Key::StatusApprovedFoot => "Approved",
        // {0}已批准  [approved, batch 1]
        Key::StatusApprovedByFoot => "Approved by {0}",
        // 已拒绝  [approved, batch 1]
        Key::StatusDeniedFoot => "Denied",
        // {0} 拒绝  [approved, batch 1]
        Key::StatusDeniedByFoot => "Denied by {0}",
        // {0} 确认发送  [approved, batch 1]
        Key::StatusDraftSentFoot => "Sent by {0}",
        // {0} 放弃  [approved, batch 1]
        Key::StatusDraftDiscardedFoot => "Discarded by {0}",
        // 今天 {0}  [approved, batch 1]
        Key::StatusToday => "Today {0}",
        // 明天 {0}  [approved, batch 1]
        Key::StatusTomorrow => "Tomorrow {0}",
        // 周一  [approved, batch 1]
        Key::StatusMon => "Mon",
        // 周二  [approved, batch 1]
        Key::StatusTue => "Tue",
        // 周三  [approved, batch 1]
        Key::StatusWed => "Wed",
        // 周四  [approved, batch 1]
        Key::StatusThu => "Thu",
        // 周五  [approved, batch 1]
        Key::StatusFri => "Fri",
        // 周六  [approved, batch 1]
        Key::StatusSat => "Sat",
        // 周日  [approved, batch 1]
        Key::StatusSun => "Sun",
        // {0}月{1}日  [approved, batch 1]
        Key::DateMonthDay => "{0} {1}",
        // 1  [approved, batch 1]
        Key::DateMonth1 => "Jan",
        // 2  [approved, batch 1]
        Key::DateMonth2 => "Feb",
        // 3  [approved, batch 1]
        Key::DateMonth3 => "Mar",
        // 4  [approved, batch 1]
        Key::DateMonth4 => "Apr",
        // 5  [approved, batch 1]
        Key::DateMonth5 => "May",
        // 6  [approved, batch 1]
        Key::DateMonth6 => "Jun",
        // 7  [approved, batch 1]
        Key::DateMonth7 => "Jul",
        // 8  [approved, batch 1]
        Key::DateMonth8 => "Aug",
        // 9  [approved, batch 1]
        Key::DateMonth9 => "Sep",
        // 10  [approved, batch 1]
        Key::DateMonth10 => "Oct",
        // 11  [approved, batch 1]
        Key::DateMonth11 => "Nov",
        // 12  [approved, batch 1]
        Key::DateMonth12 => "Dec",

        // ---- Cards (cards/mod.rs) ----
        // 我  [approved, batch 2]
        Key::CardsMe => "You",

        // ---- Allow-rules settings page and rule confirm cards (cards/review_rules.rs) ----
        // 放行规则  [approved, batch 1]
        Key::ReviewRulesTitle => "Allow rules",
        // 添加  [approved, batch 1]
        Key::ReviewRulesAdd => "Add",
        // 删除  [approved, batch 1]
        Key::ReviewRulesDelete => "Delete",
        // 还没有放行规则。有风险的操作都会先问你。  [approved, batch 1]
        Key::ReviewRulesEmpty => "No rules yet. Anything risky asks you first.",
        // 这张卡已经处理过了  [approved, batch 1]
        Key::ReviewRulesAlreadyAnswered => "This card was already handled",
        // 用一句话写，比如：允许合并 CI 全绿的 PR  [approved, batch 1]
        Key::ReviewRulesPlaceholder => "One sentence, e.g. Allow merging PRs when CI is green",
        // 请填写规则  [approved, batch 1]
        Key::ReviewRulesNeedText => "Write a rule first",
        // 读取中  [approved, batch 1]
        Key::ReviewRulesLoading => "Loading…",
        // 读不到  [approved, batch 1]
        Key::ReviewRulesLoadFailed => "Couldn't load",
        // 确认后加入放行规则  [approved, batch 1]
        Key::ReviewRulesAddHint => "Confirm to add this allow rule",
        // 确认后删除这条规则  [approved, batch 1]
        Key::ReviewRulesDeleteHint => "Confirm to delete this rule",
        // 添加  [approved, batch 1]
        Key::ReviewRulesAddAction => "Add",
        // 删除规则  [approved, batch 1]
        Key::ReviewRulesDeleteAction => "Delete rule",
        // 不用  [approved, batch 1]
        Key::ReviewRulesDrop => "Not now",
        // 被拦原因：  [approved, batch 1]
        Key::ReviewRulesBlockedReasonPrefix => "Blocked because: ",
        // 已发到主人私聊  [approved, batch 1]
        Key::ReviewRulesSentToOwner => "Sent to owner's DM",
        // 已过期  [approved, batch 1]
        Key::ReviewRulesExpired => "Expired",
        // 没人批准，已过期  [approved, batch 1]
        Key::ReviewRulesExpiredFoot => "Expired with no approval",
        // 最多 {0} 字，还要删掉 {1} 字  [approved, batch 1]
        Key::ReviewRulesTooLong => "Max {0} characters. Remove {1} more.",
        // 添加于 {0}  [approved, batch 1]
        Key::ReviewRulesAddedOn => "Added {0}",
        // 读不到：{0}  [approved, batch 2]
        Key::ReviewRulesLoadFailedBecause => "Couldn't load: {0}",
        // 审批卡不存在  [approved, batch 1]
        Key::ReviewRulesCardMissing => "Approval card not found",

        // ---- Card footnotes after a click (cards/interact.rs) ----
        // 已经有人选过了，正在刷新  [approved, batch 2]
        Key::InteractAlreadyPicked => "Someone already picked. Refreshing…",
        // 这张卡已经处理过了，正在刷新  [approved, batch 2]
        Key::InteractAlreadyDecided => "This card was already handled. Refreshing…",
        // 等待 {owner} 审批  [approved, group approval spec]
        Key::ApprovalWaitingFor => "Waiting for {0}",
        // {0} 于 {1} 批准  [approved, group approval spec]
        Key::ApprovalApprovedBy => "Approved by {0} at {1}",
        // {0} 于 {1} 拒绝  [approved, group approval spec]
        Key::ApprovalDeniedBy => "Denied by {0} at {1}",
        // 你于 {0} 批准  [approved, group approval spec]
        Key::ApprovalApprovedByYou => "Approved by you at {0}",
        // 你于 {0} 拒绝  [approved, group approval spec]
        Key::ApprovalDeniedByYou => "Denied by you at {0}",
        // 你已批准  [approved, status-code i18n; needs UI review]
        Key::ApprovalDmApprovedByYou => "Approved by you",
        // 你已拒绝  [approved, status-code i18n; needs UI review]
        Key::ApprovalDmDeniedByYou => "Denied by you",
        // {0} 在 {1} 等你审批  [approved, group approval spec]
        Key::ApprovalDmReminder => "{0} is waiting for your approval in {1}",
        // 只有 {0} 能审批。  [approved, group approval spec]
        Key::ApprovalOnlyOwner => "Only {0} can approve this.",
        // 它的主人  [approved, group approval spec]
        Key::OwnerFallback => "its owner",
        // 只有 {0} 能修改这个 bro。  [approved, status-code i18n; needs UI review]
        Key::BroOnlyOwner => "Only {0} can change this bro.",
        // 没写出结果  [approved, contract-v0.3.0 codes rev 2]
        Key::FailNoResult => "Finished with no result",
        // 你拒绝了 {0}  [approved, contract-v0.3.0 codes rev 2]
        Key::FailDenied => "you declined {0}",
        // 你拒绝了这一步  [approved, contract-v0.3.0 codes rev 2]
        Key::FailDeniedStep => "you declined this step",
        // 审批过期了  [approved, contract-v0.3.0 codes rev 2]
        Key::FailApprovalExpired => "the approval expired",
        // 模型没回  [approved, contract-v0.3.0 codes rev 2]
        Key::FailModelNoReply => "the model didn't reply",
        // 模型没回（HTTP {0}）  [approved, contract-v0.3.0 codes rev 2]
        Key::FailModelNoReplyHttp => "the model didn't reply (HTTP {0})",
        // 工具调用轮数到上限了  [approved, contract-v0.3.0 codes rev 2]
        Key::FailToolRoundLimit => "it hit the tool call limit",
        // 出网请求超额  [approved, contract-v0.3.0 codes rev 2]
        Key::FailOutboundOverQuota => "too many outbound requests",
        // 出网请求超额（{0}）  [approved, contract-v0.3.0 codes rev 2]
        Key::FailOutboundOverQuotaCount => "too many outbound requests ({0})",
        // 只允许 GET  [approved, contract-v0.3.0 codes rev 2]
        Key::FailOutboundNotGet => "only GET is allowed",
        // 只允许 GET，这次用了 {0}  [approved, contract-v0.3.0 codes rev 2]
        Key::FailOutboundNotGetUsed => "only GET is allowed, this used {0}",
        // 审批状态没存上  [approved, contract-v0.3.0 codes rev 2]
        Key::FailSaveApprovalState => "couldn't save the approval state",
        // 批准后没能接着调用  [approved, contract-v0.3.0 codes rev 2]
        Key::FailResumeApprovedCall => "couldn't continue the approved call",
        // 还在等批准  [approved, contract-v0.3.0 codes rev 2]
        Key::FailWaitingApproval => "still waiting for approval",
        // 还在等批准：{0}  [approved, contract-v0.3.0 codes rev 2]
        Key::FailWaitingApprovalTool => "still waiting for approval: {0}",
        // 工具调用失败  [approved, contract-v0.3.0 codes rev 2]
        Key::FailToolCall => "a tool call failed",
        // 批准过的调用又要审批  [approved, contract-v0.3.0 codes rev 2]
        Key::FailToolNeedsApproval => "the approved call needed approval again",
        // 出错了  [approved, contract-v0.3.0 codes rev 2]
        Key::CodeUnknown => "Something went wrong",
        // 开始干活  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressStarted => "Starting",
        // 在想怎么回  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressThinking => "Thinking",
        // 在沙箱里跑  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressInSandbox => "Running in the sandbox",
        // 在用 {0}  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressUsingTool => "Using {0}",
        // 第 {0}/{1} 步  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressRouterStep => "Step {0} of {1}",
        // 模型和工具还没回，已等 {0} 分钟  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressStepTimeout => "No reply from the model or tools for {0} min",
        // {0} 分钟没有新进度  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressStalled => "No progress for {0} min",
        // 进行中  [approved, contract-v0.3.0 codes rev 2]
        Key::ProgressUnknown => "In progress",
        // 符合放行规则  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonAllowed => "Allowed by your rules",
        // 没有放行规则允许这个操作  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonNoMatch => "No rule allows this",
        // 还没设置模型，没法自动审核  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonNoModel => "No model set, so auto-review can't run",
        // 自动审核没成功，需要你批准  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonFailed => "Auto-review didn't work. It needs your approval",
        // 审核放行了，但任务没能接着跑，你来定  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonAllowFailed => {
            "Auto-review allowed it, but the task couldn't continue. Your call"
        }
        // 审批已过期  [approved, contract-v0.3.0 codes rev 2]
        Key::ReasonExpired => "Approval expired",
        // 请求没成功。  [approved, status-code i18n; needs UI review]
        Key::ClientRequestFailed => "The request failed.",
        // 没选上：{0}  [approved, batch 2]
        Key::InteractRejected => "Choice not accepted: {0}",
        // 没成功：{0}  [approved, batch 2]
        Key::InteractFailed => "Failed: {0}",

        // ---- Memory and skill confirm cards (cards/confirm.rs) ----
        // 记忆  [approved, batch 1]
        Key::ConfirmMemoryTag => "Memory",
        // 技能  [approved, batch 1]
        Key::ConfirmSkillTag => "Skill",
        // 待确认  [approved, batch 1]
        Key::ConfirmPending => "To review",
        // 记下  [approved, batch 1]
        Key::ConfirmKeep => "Remember",
        // 保存  [approved, batch 1]
        Key::ConfirmSave => "Save",
        // 不用  [approved, batch 1]
        Key::ConfirmDrop => "Not now",
        // 记下了  [approved, batch 1]
        Key::ConfirmKept => "Remembered",
        // 保存了  [approved, batch 1]
        Key::ConfirmSaved => "Saved",
        // 不用了  [approved, batch 1]
        Key::ConfirmDropped => "Dismissed",
        // 已经有人处理过了，正在刷新  [approved, batch 1]
        Key::ConfirmAlreadyAnswered => "Already handled. Refreshing…",
        // 展开  [approved, batch 1]
        Key::ConfirmShowAll => "Show all",
        // 收起  [approved, batch 1]
        Key::ConfirmShowLess => "Show less",
        // 没成功：{0}  [approved, batch 1]
        Key::ConfirmDecideFailed => "Failed: {0}",
        // 还有 {0} 条待确认  [approved, batch 1]
        Key::ConfirmMoreLabel => "{0} more to review",

        // ---- Client feed (client/src/model.rs) ----
        // {0}：{1}  [approved, batch 2]
        Key::ClientPreviewLine => "{0}: {1}",
        // 图片  [approved, batch 2]
        Key::ClientAttachImage => "Image",
        // 文件  [approved, batch 2]
        Key::ClientAttachFile => "File",
        // 文本  [approved, batch 2]
        Key::ClientAttachText => "Text",
        // 等你批准  [approved, batch 1]
        Key::ClientAwaitingApproval => "Needs your approval",

        // ---- Shared widgets (view.rs) ----
        // 批准  [approved, batch 2]
        Key::ViewApprove => "Approve",
        // 拒绝  [approved, batch 2]
        Key::ViewDeny => "Deny",
        // 发给  [approved, batch 2]
        Key::ViewDraftTo => "To",
        // 发送  [approved, batch 2]
        Key::ViewDraftSend => "Send",
        // 修改  [approved, batch 2]
        Key::ViewDraftEdit => "Edit",
        // 这台服务器还没有工具  [approved, batch 2]
        Key::ViewMcpNoTools => "This server has no tools yet",
        // ‹ 聊天  [approved, batch 2]
        Key::ViewBackToChats => "‹ Chats",
        // 保存  [approved, batch 2]
        Key::ViewRenameSave => "Save",
        // 取消  [approved, batch 2]
        Key::ViewRenameCancel => "Cancel",
        // 私聊  [approved, batch 2]
        Key::ViewDmTag => "DM",
        // 调用   [approved, batch 2]
        Key::ViewCallPrefix => "Call ",
        // ：  [approved, batch 2]
        Key::ViewCallArgsSep => ": ",

        // ---- Chat list and composer (conversations.rs) ----
        // 还没有 bro，引入第一个  [approved, batch 2]
        Key::ConversationsEmptyHint => "No bros yet. Add your first one.",
        // 引入一个 bro  [approved, batch 2]
        Key::ConversationsNewBro => "Add a bro",
        // 改名  [approved, batch 2]
        Key::ConversationsRenameHint => "Rename",
        // bro 的新名字，回车保存，Esc 取消  [approved, batch 2]
        Key::ConversationsRenamePlaceholder => "New name. Enter to save, Esc to cancel",
        // 发消息，@名字 派活  [approved, batch 2]
        Key::ConversationsGroupHint => "Message the group, or @name a bro",
        // 发送  [approved, batch 2]
        Key::ConversationsSend => "Send",
        // 聊天  [approved, batch 2]
        Key::ConversationsTitle => "Chats",
        // 发给 {0}…  [approved, batch 2]
        Key::ConversationsDmHint => "Message {0}…",
        // 昨天  [approved, batch 2]
        Key::ConversationsYesterday => "Yesterday",

        // ---- Home list, group menu and self-row (PR4) ----
        // Needs UI review.
        Key::HomeSearchPlaceholder => "Search",
        // Needs UI review.
        Key::GroupMenuProfile => "View profile",
        // Needs UI review.
        Key::GroupMenuFind => "Find in chat",
        // Needs UI review.
        Key::GroupMenuDelete => "Delete",
        // Needs UI review.
        Key::GroupProfileEdit => "Edit",
        // 3 个开着 / "3 on"  [UI姐 PR4 spec]
        // Needs UI review.
        Key::GroupProfileRoutinesOn => "{0} on",
        // Needs UI review.
        Key::GroupProfileMembers => "Members",
        // Needs UI review.
        Key::GroupProfileCopyId => "Copy ID",
        // "Copied"  [UI姐 PR4 spec]
        // Needs UI review.
        Key::GroupProfileCopied => "Copied",
        // "Delete {group}?"  [UI姐 PR4 spec]
        // Needs UI review.
        Key::GroupDeleteTitle => "Delete {0}?",
        // Needs UI review.
        Key::GroupDeleteBody => "Messages, tasks and routines are deleted, and running tasks are stopped. Bros are kept.",
        // Needs UI review.
        Key::FindInChatTitle => "Find in chat",
        // Needs UI review.
        Key::FindInChatPlaceholder => "Search messages",
        // Needs UI review.
        Key::FindInChatEmpty => "Type to search this chat.",
        // Needs UI review.
        Key::FindInChatNoResults => "No messages matched.",
        // Needs UI review.
        Key::MembersTitle => "Members",
        // Needs UI review.
        Key::SettingsTitle => "Settings",
        // Needs UI review.
        Key::SignOutOnThisDevice => "Sign out on this device",

        // 今天  [approved, chat column]
        Key::ChatDateToday => "Today",
        // 昨天  [approved, chat column]
        Key::ChatDateYesterday => "Yesterday",
        // {1}年{0}  [approved, chat column]
        Key::ChatDateYear => "{0}, {1}",

        // ---- App shell (client/src/main.rs) ----
        // 假事件  [approved, batch 2]
        Key::ClientFakeOrigin => "Demo events",
        // 没能引入新 bro：{0}  [approved, batch 2]
        Key::ClientNewBroFailed => "Couldn't add a bro: {0}",
        // 没改成：{0}  [approved, batch 2]
        Key::ClientRenameFailed => "Couldn't rename: {0}",
        // 只有这个 bro 的主人能改头像  [approved, PR2 avatar]
        Key::AvatarNotOwner => "Only this bro's owner can change its avatar",
        // 头像没存上，再试一次  [approved, PR2 avatar]
        Key::AvatarSaveFailed => "Couldn't save the avatar. Try again",
        // 头像  [approved, PR2 avatar]
        Key::AvatarTitle => "Avatar",
        // 形状  [approved, PR2 avatar]
        Key::AvatarShape => "Shape",
        // 颜色  [approved, PR2 avatar]
        Key::AvatarColor => "Color",
        // 用默认头像  [approved, PR2 avatar]
        Key::AvatarUseDefault => "Use default",
        // 给 bro 起个名字  [approved, PR2 avatar]
        Key::CreateBroNamePlaceholder => "Name your bro",
        // 引入  [approved, PR2 avatar]
        Key::CreateBroCreate => "Add",
        // 没发出去：{0}  [approved, batch 2]
        Key::ClientSendFailed => "Didn't send: {0}",
        //  · 实时  [approved, batch 2]
        Key::ClientLive => " · live",
        //  · 轮询  [approved, batch 2]
        Key::ClientPolling => " · polling",
        // 深色  [approved, batch 2]
        Key::ClientThemeDark => "Dark",
        // 浅色  [approved, batch 2]
        Key::ClientThemeLight => "Light",
        // 选一个聊天  [approved, batch 2]
        Key::ClientPickChat => "Pick a chat",
        // 连不上宿主  [approved, disconnected state]
        Key::ClientHostUnreachable => "Can't reach the host",
        // 连接被拒绝  [approved, disconnected state]
        Key::ClientConnectionRefused => "Connection refused",
        // 超时  [approved, disconnected state]
        Key::ClientTimedOut => "Timed out",
        // 找不到主机  [approved, disconnected state]
        Key::ClientHostNotFound => "Host not found",
        // 宿主拒绝了令牌，请检查 ROBROZ_TOKEN。  [approved, connection form]
        Key::ClientTokenRejected => "The host rejected the token. Check ROBROZ_TOKEN.",
        // 令牌被拒绝  [approved, connection form]
        Key::ClientTokenRejectedTitle => "Token rejected",
        // 连不上宿主。  [approved, disconnected state]
        Key::ClientCantReachHost => "Can't reach host.",
        // 地址  [approved, connection form]
        Key::ConnHostLabel => "Host",
        // 令牌  [approved, connection form]
        Key::ConnTokenLabel => "Token",
        // 连接  [approved, connection form]
        Key::ConnConnect => "Connect",
        // 连接  [approved, connection form]
        Key::ConnSectionTitle => "Connection",
        // 连接中…  [approved, connection form]
        Key::ConnConnecting => "Connecting…",
        // 显示  [approved, connection form]
        Key::ConnShow => "Show",
        // 隐藏  [approved, connection form]
        Key::ConnHide => "Hide",
        // 清除  [approved, connection form]
        Key::ConnClear => "Clear",
        // 由 ROBROZ_HOST 设置  [approved, connection form]
        Key::ConnSetByHost => "Set by ROBROZ_HOST",
        // 由 ROBROZ_TOKEN 设置  [approved, connection form]
        Key::ConnSetByToken => "Set by ROBROZ_TOKEN",
        // 未加密，远程请用 HTTPS。  [approved, connection form]
        Key::ConnUnencryptedHint => "Unencrypted. Use HTTPS for remote hosts.",
        // Language / 语言  [approved, settings row]
        Key::SettingsLanguage => "Language / 语言",
        // English  [approved, settings row; native name in both files]
        Key::LocaleNameEn => "English",
        // 中文  [approved, settings row; native name in both files]
        Key::LocaleNameZh => "中文",
        // 外观  [approved, appearance row]
        Key::SettingsAppearance => "Appearance",
        // 跟随系统  [approved, appearance row]
        Key::SettingsThemeSystem => "System",

        // ---- Client-side errors (client/src/api.rs) ----
        // 连不上主机  [approved, batch 2]
        Key::ClientNoHost => "Can't reach the host",
        // 与契约不符: 需要 approvalId  [approved, batch 2]
        Key::ClientDecodeNeedsApprovalId => "Unexpected reply from the host: approvalId missing",
        // 与契约不符: {0}  [approved, batch 2]
        Key::ClientDecodeFailed => "Unexpected reply from the host: {0}",
        // 连不上宿主机 {0}: {1}  [approved, batch 2]
        Key::ClientConnectFailed => "Can't reach the host {0}: {1}",
        // 读 {0} 失败: {1}  [approved, batch 2]
        Key::ClientReadFailed => "Couldn't read {0}: {1}",
        // {0} {1} 返回 {2}  [approved, batch 2]
        Key::ClientHttpStatus => "{0} {1} returned {2}",
        // 没有这台服务器  [approved, batch 2]
        Key::ClientNoSuchServer => "No such server",

        // ---- Connection errors (client/src/source) ----
        // 实时推送断开: {0}  [approved, batch 2]
        Key::ClientStreamDroppedBecause => "Live updates dropped: {0}",
        // 实时推送断开  [approved, batch 2]
        Key::ClientStreamDropped => "Live updates dropped",

        // ---- Routines page (cards/routines.rs) ----
        // 已暂停  [approved, batch 2]
        Key::RoutinesPausedTag => "Paused",
        // 例行任务  [approved, batch 2]
        Key::RoutinesTitle => "Routines",
        // 还没有例行任务  [approved, batch 2]
        Key::RoutinesEmptyTitle => "No routines yet",
        // 在群里对 bro 说「每天 8 点给我发简报」就会出现在这里  [approved, batch 2]
        Key::RoutinesEmptyHint => {
            "Tell a bro in a group “send me a briefing every day at 8” and it shows up here"
        }

        // ---- Memories page (cards/memories.rs) ----
        // 记忆  [approved, batch 2]
        Key::MemoriesTitle => "Memories",
        // 还没有记下任何东西  [approved, batch 2]
        Key::MemoriesEmptyTitle => "Nothing remembered yet",
        // 点「记一条」写下想让 bro 记住的事  [approved, batch 2]
        Key::MemoriesEmptyHint => {
            "Click “Add memory” to write down something for your bros to remember"
        }
        // 所有 bro  [approved, batch 2]
        Key::MemoriesAllBros => "All bros",
        // 待确认  [approved, batch 2]
        Key::MemoriesPendingTag => "To review",
        // 待确认  [approved, batch 2]
        Key::MemoriesPendingHead => "To review",
        // 已记下  [approved, batch 2]
        Key::MemoriesKeptHead => "Remembered",
        // 记一条  [approved, batch 2]
        Key::MemoriesAdd => "Add memory",
        // 改  [approved, batch 2]
        Key::MemoriesEdit => "Edit",
        // 保存  [approved, batch 2]
        Key::MemoriesSave => "Save",
        // 取消  [approved, batch 2]
        Key::MemoriesCancel => "Cancel",
        // 忘掉  [approved, batch 2]
        Key::MemoriesForget => "Forget",
        // 忘掉这条？bro 以后不会再用到它。  [approved, batch 2]
        Key::MemoriesForgetAsk => "Forget this? Bros won't use it again.",
        // 想让 bro 记住什么  [approved, batch 2]
        Key::MemoriesNewPlaceholder => "What should your bros remember?",
        // 最多 2000 字  [approved, batch 2]
        Key::MemoriesTooLong => "Max 2000 characters",
        // 写点什么再保存  [approved, batch 2]
        Key::MemoriesEmptyText => "Write something before saving",
        // 已保存  [approved, batch 2]
        Key::MemoriesSaved => "Saved",
        // 选一下谁能用这条  [approved, batch 2]
        Key::MemoriesPickScope => "Choose who can use this",
        // 资料  [approved, batch 2]
        Key::MemoriesKindProfile => "Profile",
        // 习惯  [approved, batch 2]
        Key::MemoriesKindHabit => "Habit",
        // 项目  [approved, batch 2]
        Key::MemoriesKindProject => "Project",
        // 只给 {0}  [approved, batch 2]
        Key::MemoriesOnlyFor => "Only {0}",
        // 只给 {0} 用  [approved, batch 2]
        Key::MemoriesChosenBot => "Only {0} can use this",
        // 所有 bro 都能用  [approved, batch 2]
        Key::MemoriesChosenAll => "All bros can use this",

        // ---- Skills page (cards/skills.rs) ----
        // 技能  [approved, batch 2]
        Key::SkillsTitle => "Skills",
        // 还没有保存任何技能  [approved, batch 2]
        Key::SkillsEmptyTitle => "No skills saved yet",
        // 点「加一项」写下想让 bro 会的做法  [approved, batch 2]
        Key::SkillsEmptyHint => {
            "Click “Add skill” to write down how you want your bros to do something"
        }
        // 待确认  [approved, batch 2]
        Key::SkillsPendingTag => "To review",
        // 待确认  [approved, batch 2]
        Key::SkillsPendingHead => "To review",
        // 已保存  [approved, batch 2]
        Key::SkillsKeptHead => "Saved",
        // 加一项  [approved, batch 2]
        Key::SkillsAdd => "Add skill",
        // 改  [approved, batch 2]
        Key::SkillsEdit => "Edit",
        // 保存  [approved, batch 2]
        Key::SkillsSave => "Save",
        // 取消  [approved, batch 2]
        Key::SkillsCancel => "Cancel",
        // 删除  [approved, batch 2]
        Key::SkillsDelete => "Delete",
        // 删除这项？bro 以后不会再用到它。  [approved, batch 2]
        Key::SkillsDeleteAsk => "Delete this? Bros won't use it again.",
        // 英文短名，比如 weekly-report  [approved, batch 2]
        Key::SkillsSlugPlaceholder => "Short English name, e.g. weekly-report",
        // 标题  [approved, batch 2]
        Key::SkillsTitlePlaceholder => "Title",
        // 想让 bro 怎么做  [approved, batch 2]
        Key::SkillsInstructionPlaceholder => "How should your bros do it?",
        // 先填一个英文短名  [approved, batch 2]
        Key::SkillsEmptySlug => "Enter a short English name first",
        // 短名只能是小写字母、数字和连字符  [approved, batch 2]
        Key::SkillsBadSlug => "Short names can only use lowercase letters, digits and hyphens",
        // 写个标题再保存  [approved, batch 2]
        Key::SkillsEmptyTitleText => "Add a title before saving",
        // 写点说明再保存  [approved, batch 2]
        Key::SkillsEmptyInstruction => "Add instructions before saving",
        // 标题最多 80 个字  [approved, batch 2]
        Key::SkillsTitleTooLong => "Title: max 80 characters",
        // 说明最多 8000 个字  [approved, batch 2]
        Key::SkillsInstructionTooLong => "Instructions: max 8000 characters",
        // 已保存  [approved, batch 2]
        Key::SkillsSaved => "Saved",
        // 在这些群里用  [approved, batch 2]
        Key::SkillsGroupEnable => "Use in these groups",
        // 读取中  [approved, batch 2]
        Key::SkillsLoading => "Loading",
        // 读不到  [approved, batch 2]
        Key::SkillsLoadFailed => "Couldn't load",

        // ---- Model settings page (cards/model.rs) ----
        // 模型  [approved, batch 2]
        Key::ModelTitle => "Model",
        // 保存  [approved, batch 2]
        Key::ModelSave => "Save",
        // 来自环境变量  [approved, batch 2]
        Key::ModelFromEnv => "From environment variables",
        // 还没设置模型  [approved, batch 2]
        Key::ModelNotSet => "No model set up yet",
        // 试连中…  [approved, batch 2]
        Key::ModelTesting => "Testing…",
        // 能用了  [approved, batch 2]
        Key::ModelWorks => "Working",
        // 服务商  [approved, batch 2]
        Key::ModelLabelPreset => "Provider",
        // 地址  [approved, batch 2]
        Key::ModelLabelUrl => "URL",
        // 模型名  [approved, batch 2]
        Key::ModelLabelModel => "Model name",
        // 密钥（选填）  [approved, batch 2]
        Key::ModelLabelKey => "Key (optional)",
        // 请填写地址  [approved, batch 2]
        Key::ModelNeedUrl => "Enter a URL",
        // 地址要以 http:// 或 https:// 开头  [approved, batch 2]
        Key::ModelBadUrl => "The URL must start with http:// or https://",
        // 请填写模型名  [approved, batch 2]
        Key::ModelNeedModel => "Enter a model name",
        // 去设置  [approved, batch 2]
        Key::ModelGoSetup => "Set up",
        // 这次还没设置模型  [approved, batch 2]
        Key::ModelNeedsSetupHint => "Skipped: no model set up yet",
        // 免费模型的名字以 :free 结尾  [approved, batch 2]
        Key::ModelOpenrouterHint => "Free model names end in :free",
        // 已保存，但连不上：{0}  [approved, batch 1]
        Key::SavedButUnreachable => "Saved, but can't connect: {0}",
        // Gemini（推荐）  [approved, batch 2]
        Key::ModelPresetGemini => "Gemini (recommended)",
        // 自定义  [approved, batch 2]
        Key::ModelPresetCustom => "Custom",
        // 选一个服务商  [approved, batch 2]
        Key::ModelPresetPlaceholder => "Choose a provider",

        // ---- Web search settings page (cards/search.rs) ----
        // 联网搜索  [approved, batch 2]
        Key::SearchTitle => "Web search",
        // API 密钥  [approved, batch 2]
        Key::SearchKeyPlaceholder => "API key",
        // SearXNG 地址，如 http://127.0.0.1:8888  [approved, batch 2]
        Key::SearchUrlPlaceholder => "SearXNG URL, e.g. http://127.0.0.1:8888",
        // 保存  [approved, batch 2]
        Key::SearchSave => "Save",
        // 取消  [approved, batch 2]
        Key::SearchCancel => "Cancel",
        // 来自环境变量  [approved, batch 2]
        Key::SearchFromEnv => "From environment variables",
        // 还没设置联网搜索  [approved, batch 2]
        Key::SearchNotSet => "Web search isn't set up yet",
        // 试连中…  [approved, batch 2]
        Key::SearchTesting => "Testing…",
        // 能用了  [approved, batch 2]
        Key::SearchWorks => "Working",
        // 请填写 key  [approved, batch 2]
        Key::SearchNeedKey => "Enter a key",
        // 请填写地址  [approved, batch 2]
        Key::SearchNeedUrl => "Enter a URL",
        // 去设置  [approved, batch 2]
        Key::SearchGoSetup => "Set up",
        // 这次没联网搜索：还没设置  [approved, batch 2]
        Key::SearchNeedsSetupHint => "Skipped web search: not set up yet",

        // ---- MCP page (cards/mcp.rs) ----
        // 加一台  [approved, batch 2]
        Key::McpAdd => "Add server",
        // 还没有 MCP 服务器  [approved, batch 2]
        Key::McpEmptyTitle => "No MCP servers yet",
        // 点「加一台」接上 bro 可以用的工具服务  [approved, batch 2]
        Key::McpEmptyHint => "Click “Add server” to connect tools your bros can use",
        // 保存  [approved, batch 2]
        Key::McpSave => "Save",
        // 取消  [approved, batch 2]
        Key::McpCancel => "Cancel",
        // 删除  [approved, batch 2]
        Key::McpDelete => "Delete",
        // 删除这台服务器？群里也不会再用到它。  [approved, batch 2]
        Key::McpDeleteAsk => "Delete this server? Groups won't use it any more.",
        // 改  [approved, batch 2]
        Key::McpEdit => "Edit",
        // 要你批准  [approved, batch 1]
        Key::McpNeedsApproval => "Needs your approval",
        // 在这些群里用  [approved, batch 2]
        Key::McpGroupEnable => "Use in these groups",
        // 读取中  [approved, batch 2]
        Key::McpLoading => "Loading",
        // 读不到  [approved, batch 2]
        Key::McpLoadFailed => "Couldn't load",
        // 试连中…  [approved, batch 2]
        Key::McpTesting => "Testing…",
        // 能用了  [approved, batch 2]
        Key::McpWorks => "Working",
        // 加一项密钥  [approved, batch 2]
        Key::McpAddSecret => "Add a secret",
        // 去掉  [approved, batch 2]
        Key::McpRemoveSecret => "Remove",
        // 名称  [approved, batch 2]
        Key::McpLabelName => "Name",
        // 命令  [approved, batch 2]
        Key::McpLabelCommand => "Command",
        // 参数（用空格分开）  [approved, batch 2]
        Key::McpLabelArgs => "Arguments (space-separated)",
        // 地址  [approved, batch 2]
        Key::McpLabelUrl => "URL",
        // 密钥名称  [approved, batch 2]
        Key::McpLabelSecretName => "Secret name",
        // 密钥  [approved, batch 2]
        Key::McpLabelSecretKey => "Secret",
        // 命令  [approved, batch 2]
        Key::McpLabelPromptCommand => "Command",
        // 参数  [approved, batch 2]
        Key::McpLabelPromptArgs => "Arguments",
        // 环境变量  [approved, batch 2]
        Key::McpEnv => "Environment variables",
        // 请求头  [approved, batch 2]
        Key::McpHeader => "Request headers",
        // 请填写名称  [approved, batch 2]
        Key::McpEmptyName => "Enter a name",
        // 名称最多 80 个字  [approved, batch 2]
        Key::McpNameTooLong => "Name: max 80 characters",
        // 请填写启动命令  [approved, batch 2]
        Key::McpNeedCommand => "Enter a start command",
        // 请填写地址  [approved, batch 2]
        Key::McpNeedUrl => "Enter a URL",
        // 地址要以 http:// 或 https:// 开头  [approved, batch 2]
        Key::McpBadUrl => "The URL must start with http:// or https://",
        // 请填写密钥名称  [approved, batch 2]
        Key::McpEmptySecretName => "Enter a secret name",
        // 密钥名称里不能有空白  [approved, batch 2]
        Key::McpBadSecretName => "Secret names can't contain spaces",
        // 新增密钥时请填写值  [approved, batch 2]
        Key::McpNeedSecretKey => "Enter a value for the new secret",
        // 命令未批准  [approved, batch 2]
        Key::McpCommandUnapproved => "Command not approved",
        // 批准命令后才能读取工具  [approved, batch 1]
        Key::McpToolsNeedApproval => "Approve the command to load its tools",
        // 这条命令会在你的电脑上运行  [approved, batch 2]
        Key::McpCommandRunsLocal => "This command runs on your computer",
        // 批准运行  [approved, batch 2]
        Key::McpApproveRun => "Approve and run",
        // 批准失败：{0}  [approved, batch 2]
        Key::McpApproveFailed => "Approval failed: {0}",
        // 连不上：{0}  [approved, batch 2]
        Key::CannotConnect => "Can't connect: {0}",
        // 关于  [approved]
        Key::MobileAbout => "About",
        // 版本 {0}  [approved]
        Key::MobileVersion => "Version {0}",
        // 没保存：{0}  [approved, batch 2]
        Key::NotSaved => "Not saved: {0}",
        // {0} 个工具  [approved, batch 2]
        Key::McpToolCount => "{0} tools",
        // 还没数过  [approved, batch 2]
        Key::McpToolsNotCounted => "Not counted yet",
        // 原因不明  [approved, batch 2]
        Key::McpUnknownReason => "Unknown reason",
        // 1 个工具  [approved, batch 2]
        Key::McpToolCountOne => "1 tool",

        // ---- Secret fields (cards/secret.rs) ----
        // 清除  [approved, batch 2]
        Key::SecretClear => "Clear",
        // 保存后删除  [approved, batch 2]
        Key::SecretPendingClear => "Deleted on save",
        // 撤销  [approved, batch 2]
        Key::SecretUndo => "Undo",
        // 已填  [approved, batch 2]
        Key::SecretFilled => "Set",
        // 处理中 {0}  [approved, PR3 process]
        Key::ProcessWorking => "Working {0}",
        // 处理中约 {0}  [approved, PR3 process]
        Key::ProcessWorkingAbout => "Working ~{0}",
        // 用了 {0}  [approved, PR3 process]
        Key::ProcessWorked => "Worked for {0}",
        // 用了约 {0}  [approved, PR3 process]
        Key::ProcessWorkedAbout => "Worked for ~{0}",
        // {0} 秒  [approved, PR3 process]
        Key::ProcessSecs => "{0}s",
        // {0} 分 {1} 秒  [approved, PR3 process]
        Key::ProcessMins => "{0}m {1}s",
        // {0} 在处理…  [approved, PR3 process]
        Key::WorkingRowOne => "{0} is working…",
        // {0} 和 {1} 在处理…  [approved, PR3 process]
        Key::WorkingRowTwo => "{0} and {1} are working…",
        // {0} 和另外 {1} 个  [approved, PR3 process]
        Key::WorkingRowMany => "{0} and {1} others",
        // 正在重新连接  [approved, PR3 process]
        Key::Reconnecting => "Reconnecting…",
    }
}
