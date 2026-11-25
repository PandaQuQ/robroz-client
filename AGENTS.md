# AGENTS.md

给在这个仓库里写代码、写文档的 agent 看的入口。这里和 `docs/` 是契约和精确参考；wiki 是给人看的说明，不是契约，两边冲突时以仓库为准。

## 仓库是什么

robroz 是单机私有化的多智能体协同，交互是 IM：人在「群」和「私聊」里给 bro（一个 agent，契约里叫 bot）派任务，bro 在 Docker 沙箱里干活，结果以卡片形式回到群里。全部用 Rust，没有 Go、Python 和网页客户端。

| 目录 | crate | 一句话 |
| --- | --- | --- |
| `host/` | `robroz-host` | 宿主机：HTTP 接口（axum）、SQLite、pump 线程（起停容器）、定时、调用模型、联网搜索、MCP 配置和工具调用。 |
| `runner/` | `robroz-runner` | 容器里的执行器：读 `/control/prompt.txt`，按规则处理 `robroz-action:` / `robroz-progress:` / `robroz-memory:` / `robroz-skill:` 行，写 `result.txt`。也定义宿主机和它共享的文件格式。 |
| `client/` | `robroz-client` | iced 桌面客户端，Windows / Linux / macOS 同一套代码。 |
| `crates/robroz-contract` | `robroz-contract` | 接口契约：请求体、返回体、事件（卡片）类型、任务状态。改这里要守下面的冻结规则。 |
| `crates/robroz-policy` | `robroz-policy` | 权限三级（放行 / 计数 / 接管）、出网计数、阻塞行判定、定时是否起容器。 |
| `crates/robroz-card` | `robroz-card` | 卡片的状态文案、色调和动作，客户端只照着画。 |
| `crates/robroz-tokens` | `robroz-tokens` | 设计 token，来源是 `tokens.json`，用 `cargo run -p robroz-tokens --bin gen` 生成代码。 |
| `crates/robroz-ui` | `robroz-ui` | 客户端共用的界面层：卡片视图模型、主题（Skin）。 |
| `crates/robroz-model` | `robroz-model` | 调 OpenAI 兼容的 `/chat/completions`，含工具调用。 |
| `crates/robroz-mcp` | `robroz-mcp` | 宿主机用的 MCP 客户端（stdio 和 Streamable HTTP），列出并调用工具。 |

## 硬规矩

违反任何一条的 PR 都不会合。

1. **契约只增不改。** `robroz-contract` 里已有的字段名、类型、含义都不改；任务状态 `queued` / `running` / `blocked` / `done` 冻住，不加第五种。只能新增卡片类型、新增接口、给已有结构加**可选**字段（`#[serde(default, skip_serializing_if = "Option::is_none")]`，旧 JSON 不变）。确实要改已有字段，先停下来问 Panda。
2. **`botId` / `botName` 不改名。** 界面上叫 bro，代码、接口、数据库里照旧叫 bot。
3. **一个 PR 只做一件事。**
4. **提交身份固定：**
   ```sh
   git -c user.name=PandaQuQ -c user.email=105594350+PandaQuQ@users.noreply.github.com commit
   ```
5. **不 force-push。** 分支落后 `main` 时把 `main` merge 进来，不 rebase 已推送的提交。
6. **只有顶级GitHub哥合并**，并且要先同步 `main`、CI 全绿。agent 自己不合并 PR。`main` 有 ruleset `main-protection`：只能走 PR，五个检查（`host`、`cla`、`client (ubuntu-latest)`、`e2e`、`cargo-deny`）必须全绿，禁止 force-push 和删除分支，bypass 列表为空，管理员也绕不过。直接 `git push` 到 `main` 会被拒绝，推送前先跑 `git branch --show-current` 确认不在 `main`。
7. **不提交密钥。** 不提交 `.env`、`.pem`、`.dev.vars`，也不把任何密钥、令牌写进代码、测试、文档或提交信息。文档里一律用 `<你的密钥>`、`<你的令牌>` 这类占位符。
8. **密钥只有两个去处：** 宿主机的环境变量（或 `*_FILE` 指向的文件），或用户自己在设置接口里填。不要发到群里，也不要写进仓库。
9. **净室：** 参考产品只看它们的 README、公开文档、LICENSE 和公开截图，不看、不抄、不翻译源码。参考清单和规则见 [`docs/reference-features.md`](docs/reference-features.md)。
10. **提交信息和 PR 一律用英文，按 [Conventional Commits](https://www.conventionalcommits.org/) 写。** 提交信息、PR 标题和 PR 正文都用英文，格式是 `type(scope): summary`，比如 `feat(host): ...`、`fix(client): ...`、`docs: ...`。顶级GitHub哥用 squash merge 合并，PR 标题直接成为 `main` 上的提交信息，所以标题要写规范。
11. **README 中英两份同步。** `README.md`（英文，默认）和 `README.zh-CN.md`（简体中文）内容保持一致，改一份就在同一个 PR 里改另一份。客户端界面上的中文叫法（如「引入一个 bro」「聊天」）两份都保留原文，英文版在后面加英文注释。
12. **许可证是 `AGPL-3.0-only`，贡献者必须签 CLA。** 仓库按 [`LICENSE`](LICENSE)（AGPL-3.0 原文，不改字）开源，客户端只用 AGPL，不提供商业授权。每个 PR 的作者都要按 [`CLA.md`](CLA.md) 签署（PR 描述里加那行声明，或勾选模板里的 CLA 项），没签的不合。新 crate 的 `Cargo.toml` 写 `license.workspace = true`，继承根 `Cargo.toml` 里 `[workspace.package]` 的 `license = "AGPL-3.0-only"`。不要引入和 AGPL-3.0 不兼容的依赖（如 GPL-2.0-only、SSPL、CC-BY-NC、专有许可）。CI 的 `cargo-deny` 任务（`.github/workflows/ci.yml`，配置在根目录 `deny.toml`）会挡住：许可证不在 `[licenses] allow` 白名单里的依赖，以及来自未知 registry 或 git 源的 crate。加依赖前本地跑 `cargo deny check licenses bans sources`。不要为了过检查往白名单里加不兼容的许可证；确实需要新的兼容许可证，在 PR 里写明理由。`cargo deny check advisories`（安全公告）在同一任务里单独跑，`continue-on-error`，不挡合并。工作区自己的 crate 都是 `publish = false`，由 `[licenses.private] ignore = true` 跳过，新 crate 也要写 `publish = false`。

## 构建和测试

装工具、构建、跑宿主机和客户端、跑测试的完整步骤见 [`docs/install.md`](docs/install.md)。提交前至少跑和 CI 相同的命令：

```sh
cargo test -p robroz-mcp --features fixture
cargo test -p robroz-host -p robroz-contract -p robroz-policy -p robroz-card -p robroz-runner -p robroz-tokens
cargo build -p robroz-client
cargo test -p robroz-client -p robroz-ui -p robroz-tokens
scripts/e2e.sh   # 需要 docker、curl、jq
cargo deny check licenses bans sources   # 需要先装 cargo-deny
```

测试不要用默认数据库路径 `./robroz.db`，也不要写仓库里的 `./data`。手动起宿主机验证时先用 `ss -ltn` 找一个没被占用的端口，并用临时目录放 `ROBROZ_DB`、`ROBROZ_VOLUMES`、`ROBROZ_CONTROL`。

## 配置

全部环境变量（宿主机、容器、客户端、开发测试专用），每个的读取位置、默认值和示例见 [`docs/configuration.md`](docs/configuration.md)。

## docs 索引

| 文件 | 内容 |
| --- | --- |
| [`docs/install.md`](docs/install.md) | 环境要求、构建、跑宿主机和客户端、接模型、第一个任务、测试和 e2e。 |
| [`docs/configuration.md`](docs/configuration.md) | 所有环境变量，搜索设置和环境变量的优先级。 |
| [`docs/api.md`](docs/api.md) | 所有 HTTP 接口：请求、返回、状态码、原文报错、代码位置；事件（卡片）类型和字段。 |
| [`docs/reference-features.md`](docs/reference-features.md) | 参考产品功能清单和净室规则。 |
| [`docs/roadmap-phase2.md`](docs/roadmap-phase2.md) | 第二期路线图。 |
| [`docs/design/multi-agent.md`](docs/design/multi-agent.md) | Router、Thread 订阅、异步工具与 Watchdog 的已接受设计（非契约）。 |
| [`docs/concepts.md`](docs/concepts.md) | 术语表、bro 的名字、任务状态、状态卡字段、卡片类型、未读。 |
| [`docs/permissions.md`](docs/permissions.md) | 三级权限、阻塞行和接管标记、resume、审批卡、`waitingFor`、`failure`、模型工具调用的限制。 |
| [`docs/memory-and-skills.md`](docs/memory-and-skills.md) | 记忆和技能的规则、拼进提示（4000 / 8000 字符）、bro 怎么提议、确认卡。 |
| [`docs/search-and-mcp.md`](docs/search-and-mcp.md) | 联网搜索（怎么填、优先级、`source`、`needsSetup`）和 MCP（配置、模型调用、审批卡）。 |
| [`docs/client.md`](docs/client.md) | 客户端界面：布局、各种卡怎么显示、处理人、记忆页、技能页、模型页（含「去设置」）、联网搜索页（含「去设置」）、MCP 页（含 stdio 命令批准）、例行任务。 |
| [`docs/architecture.md`](docs/architecture.md) | 宿主机、pump、容器、runner、模型、例行任务、实时推送、卡片和外观层（Skin）、客户端事件来源。 |

`README.md` 和 `README.zh-CN.md` 只放简介、功能、快速开始、文档入口和参与开发的要点，不写接口细节。新功能的说明写进上面对应的 `docs/` 文件；没有合适的文件就新建一个并加进这张表。
