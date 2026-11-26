# AGENTS.md

给在这个仓库里写代码、写文档的 agent 看的入口。这里和 `docs/` 是契约和精确参考；wiki 是给人看的说明，不是契约，两边冲突时以仓库为准。

## 仓库是什么

robroz 是单机私有化的多智能体协同，交互是 IM：人在「群」和「私聊」里给 bro（一个 agent，契约里叫 bot）派任务，bro 在 Docker 沙箱里干活，结果以卡片形式回到群里。全部用 Rust，没有 Go、Python 和网页客户端。本仓库只放客户端；宿主机、runner 和 `robroz-contract` 在 [`PandaQuQ/robroz`](https://github.com/PandaQuQ/robroz)，这里在 `Cargo.toml` 里按 git tag `contract-v*` 依赖它。

| 目录 | crate | 一句话 |
| --- | --- | --- |
| `client/` | `robroz-client` | iced 桌面客户端，Windows / Linux / macOS 同一套代码。 |
| `mobile/` | `robroz-mobile` | 手机客户端（Android、iOS），构建见 [`mobile/README.md`](mobile/README.md)。 |
| `crates/robroz-card` | `robroz-card` | 卡片的状态文案、色调和动作，客户端只照着画。 |
| `crates/robroz-tokens` | `robroz-tokens` | 设计 token，来源是 `tokens.json`，用 `cargo run -p robroz-tokens --bin gen` 生成代码。 |
| `crates/robroz-ui` | `robroz-ui` | 客户端共用的界面层：卡片视图模型、主题（Skin）。 |

## 硬规矩

违反任何一条的 PR 都不会合。

1. **契约只增不改，且先改宿主机仓库。** 契约在宿主机仓库；本仓库只升 `contract-v*` tag，不在这里复制或改契约类型。`robroz-contract` 里已有的字段名、类型、含义都不改；任务状态 `queued` / `running` / `blocked` / `done` 冻住，不加第五种。只能新增卡片类型、新增接口、给已有结构加**可选**字段（`#[serde(default, skip_serializing_if = "Option::is_none")]`，旧 JSON 不变）。确实要改已有字段，先停下来问 Panda。
2. **`botId` / `botName` 不改名。** 界面上叫 bro，代码、接口、数据库里照旧叫 bot。
3. **一个 PR 只做一件事。**
4. **提交身份固定：**
   ```sh
   git -c user.name=PandaQuQ -c user.email=105594350+PandaQuQ@users.noreply.github.com commit
   ```
5. **不 force-push。** 分支落后 `main` 时把 `main` merge 进来，不 rebase 已推送的提交。
6. **只有顶级GitHub哥合并**，并且要先同步 `main`、CI 全绿。agent 自己不合并 PR。`main` 有 ruleset `main-protection`：只能走 PR，三个检查（`client (ubuntu-latest)`、`cargo-deny`、`cla`）必须全绿，禁止 force-push 和删除分支，bypass 列表为空，管理员也绕不过。直接 `git push` 到 `main` 会被拒绝，推送前先跑 `git branch --show-current` 确认不在 `main`。
7. **不提交密钥。** 不提交 `.env`、`.pem`、`.dev.vars`，也不把任何密钥、令牌写进代码、测试、文档或提交信息。文档里一律用 `<你的密钥>`、`<你的令牌>` 这类占位符。
8. **密钥只有两个去处：** 宿主机的环境变量（或 `*_FILE` 指向的文件），或用户自己在设置接口里填。客户端只保存连接宿主机用的令牌（系统钥匙串，退回到 `0600` 的 `token` 文件）。不要发到群里，也不要写进仓库。
9. **净室：** 参考产品只看它们的 README、公开文档、LICENSE 和公开截图，不看、不抄、不翻译源码。参考清单和规则见宿主机仓库的 [`docs/reference-features.md`](https://github.com/PandaQuQ/robroz/blob/main/docs/reference-features.md)。
10. **提交信息和 PR 一律用英文，按 [Conventional Commits](https://www.conventionalcommits.org/) 写。** 提交信息、PR 标题和 PR 正文都用英文，格式是 `type(scope): summary`，比如 `feat(host): ...`、`fix(client): ...`、`docs: ...`。顶级GitHub哥用 squash merge 合并，PR 标题直接成为 `main` 上的提交信息，所以标题要写规范。
11. **README 中英两份同步。** `README.md`（英文，默认）和 `README.zh-CN.md`（简体中文）内容保持一致，改一份就在同一个 PR 里改另一份。客户端界面上的中文叫法（如「引入一个 bro」「聊天」）两份都保留原文，英文版在后面加英文注释。
12. **许可证是 `AGPL-3.0-only`，贡献者必须签 CLA。** 仓库按 [`LICENSE`](LICENSE)（AGPL-3.0 原文，不改字）开源，客户端只用 AGPL，不提供商业授权。每个 PR 的作者都要按 [`CLA.md`](CLA.md) 签署（PR 描述里加那行声明，或勾选模板里的 CLA 项），没签的不合。新 crate 的 `Cargo.toml` 写 `license.workspace = true`，继承根 `Cargo.toml` 里 `[workspace.package]` 的 `license = "AGPL-3.0-only"`。不要引入和 AGPL-3.0 不兼容的依赖（如 GPL-2.0-only、SSPL、CC-BY-NC、专有许可）。CI 的 `cargo-deny` 任务（`.github/workflows/ci.yml`，配置在根目录 `deny.toml`）会挡住：许可证不在 `[licenses] allow` 白名单里的依赖，以及来自未知 registry 或 git 源的 crate。加依赖前本地跑 `cargo deny check licenses bans sources`。不要为了过检查往白名单里加不兼容的许可证；确实需要新的兼容许可证，在 PR 里写明理由。`cargo deny check advisories`（安全公告）在同一任务里单独跑，`continue-on-error`，不挡合并。工作区自己的 crate 都是 `publish = false`，由 `[licenses.private] ignore = true` 跳过，新 crate 也要写 `publish = false`。

## 构建和测试

装工具、构建、跑客户端、跑测试的完整步骤见 [`docs/install.md`](docs/install.md)。提交前至少跑和 CI 相同的命令：

```sh
cargo build -p robroz-client
cargo test -p robroz-client -p robroz-card -p robroz-ui -p robroz-tokens
cargo deny check licenses bans sources   # 需要先装 cargo-deny
```

`against_host` 测试在进程内按 tag 构建宿主机，不需要 Docker。手动连宿主机验证时，宿主机用临时目录放 `ROBROZ_DB`、`ROBROZ_VOLUMES`、`ROBROZ_CONTROL`，客户端用临时的 `ROBROZ_CONFIG_DIR`，不要动自己的偏好文件。

## 配置

客户端的环境变量和偏好文件见 [`docs/configuration.md`](docs/configuration.md)；宿主机的变量见宿主机仓库的 [`docs/configuration.md`](https://github.com/PandaQuQ/robroz/blob/main/docs/configuration.md)。

## docs 索引

| 文件 | 内容 |
| --- | --- |
| [`docs/install.md`](docs/install.md) | 下载发布版、构建、运行客户端、连接宿主机（含远程）。 |
| [`docs/configuration.md`](docs/configuration.md) | 客户端的环境变量和保存的偏好（`lang`、`theme`、`host`、`token`）。 |
| [`docs/client.md`](docs/client.md) | 客户端界面：布局、各种卡怎么显示、处理人、记忆页、技能页、模型页（含「去设置」）、联网搜索页（含「去设置」）、MCP 页（含 stdio 命令批准）、例行任务。 |

`README.md` 和 `README.zh-CN.md` 只放简介、功能、快速开始、文档入口和参与开发的要点，不写接口细节。新功能的说明写进上面对应的 `docs/` 文件，宿主机侧的写进[宿主机仓库的 docs](https://github.com/PandaQuQ/robroz/tree/main/docs)；没有合适的文件就新建一个并加进这张表。
