<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/wordmark-dark.svg">
    <img src="assets/brand/wordmark-light.svg" alt="robroz" width="240">
  </picture>
</p>

<p align="center"><a href="README.md">English</a> | <b>简体中文</b></p>

<p align="center">
  <a href="https://github.com/PandaQuQ/robroz-client/actions/workflows/ci.yml"><img src="https://github.com/PandaQuQ/robroz-client/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
</p>

> **仓库已拆分。** 本仓库是 robroz 桌面和移动客户端。宿主机（服务端）、runner、共享契约和 Docker 镜像在 [PandaQuQ/robroz](https://github.com/PandaQuQ/robroz)。下面部分文档还同时描述两边，之后会按仓库重写。

自托管的多智能体工作台，用聊天来驱动。智能体（bro）在 Docker 沙箱里干活；审批和结果以卡片回到对话里。Rust 宿主机 + 原生桌面客户端。

v0.1.0 是单用户、自托管的。见 [CHANGELOG.md](CHANGELOG.md) 和 [docs/install.md](docs/install.md)。

## robroz 是什么

你在自己的机器上和一组 AI bro 群聊。用大白话派活，它们在沙盒里干，遇到有风险的操作会先问你。

适合想在自己控制的电脑上用一组 AI 智能体、并且风险步骤要自己点头的人。

<p>
  <img src="docs/assets/main-zh-light.png" alt="robroz 桌面客户端，浅色：群聊里的状态卡和交付卡" width="49%">
  <img src="docs/assets/main-zh-dark.png" alt="robroz 桌面客户端，深色：群聊里的状态卡和交付卡" width="49%">
</p>

<sub>演示数据</sub>

桌面客户端界面默认是英文。要换成中文，到设置（模型）页的「Language / 语言」那一行切换。`ROBROZ_LANG=en|zh` 只对这一次运行生效，不会写回已保存的选择。保存的 `lang` 文件见 [docs/configuration.md](docs/configuration.md)。

语言那一行下面是 Appearance / 「外观」：System / 「跟随系统」、Light / 「浅色」、Dark / 「深色」。默认是 System，会跟着操作系统实时切换。选择保存在 `lang` 旁边的 `theme` 文件里，一行，值为 `system`、`light` 或 `dark`。

## 现在能用的

- **聊天气泡。** 你自己的消息在右边；bro 的卡片在左边，按发送者分组。带 `bot_name` 的交付卡挂到对应的 bro 下面；没有 `bot_name` 的单独一条。
- **群轮次。** 在群里发一条消息，每个 bro 都可以回复，也可以选择不说话。消息开头用 `@name` 点名的那个 bro 先说。
- **bro 之间接话。** bro 在群里回了话，别的 bro 也能接着说，最多两轮就停。
- **追问。** 如果一个 bro 还在干活，你可以往这份活上补充，而不新开任务。在群里以 `@name` 开头；在私聊里直接发送即可。
- **大任务分步做。** 最多拆 3 步，卡片显示「n/m 步」。
- **私聊。** 可以和一个 bro 单独对话。
- **Docker 沙箱。** 每份工作在你机器上的一个容器里跑，容器没有网络。
- **审批和放行规则（Allow rules）。** 有风险的步骤会先问你。自动审核会按你的放行规则检查每张卡；没有规则放行时，卡会发到你的私聊。记忆和技能的变更也要先经你确认。
- **模型设置。** 可以接任意 OpenAI 兼容的模型。填了地址和模型名就算配好，密钥可选。可以在客户端填，也可以用环境变量。

还可以用联网搜索（Brave 或 SearXNG）、MCP 工具和例行任务。某一步跑得太久或卡住时，卡片上会出现一行提示；工作本身不会被取消。

## 还没做的

下面是已接受的设计，或正在做，**还没有**合进 `main`。细节见 [docs/design/multi-agent.md](docs/design/multi-agent.md)。

- 超时后取消步骤或停掉容器。

## 快速开始

需要 Rust stable 工具链（`rustup`、`cargo`）和 Docker。如果要安装已经发布的二进制，见 [docs/install.md](docs/install.md)。那里的逐步示例还用到 `curl` 和 `jq`。在仓库根目录：

```sh
cargo build -p robroz-host -p robroz-client
docker build -f runner/Dockerfile -t robroz-runner:local .
cargo run -p robroz-host     # 监听 127.0.0.1:8790，不用令牌
cargo run -p robroz-client   # 另开一个终端；默认连接 http://127.0.0.1:8790
```

宿主机默认写入 `./robroz.db`、`./data/groups` 和 `./data/control`。监听本机回环地址时可以不设令牌；监听非回环地址却没有 `ROBROZ_TOKEN` 或 `ROBROZ_TOKEN_FILE` 时，会拒绝启动。

在客户端 Chats（「聊天」）标题旁点 Add a bro（「引入一个 bro」），打开私聊发一条消息，这就是第一个任务。没设置模型时，交付是 No model set up yet（「还没设置模型」），卡片上有 Set up（「去设置」）。

### 配置模型

在客户端打开 Model（「模型」），或点交付卡上的 Set up（「去设置」）。填地址和模型名即可，密钥可选。保存时会试连。也可以用环境变量，并把密钥放在仓库外的文件里：

```sh
mkdir -p "$HOME/.config/robroz"
(umask 077 && printf '%s' '<你的密钥>' > "$HOME/.config/robroz/model-key")
ROBROZ_MODEL_BASE_URL=https://api.openai.com/v1 \
ROBROZ_MODEL=gpt-4o-mini \
ROBROZ_MODEL_KEY_FILE="$HOME/.config/robroz/model-key" \
cargo run -p robroz-host
```

`ROBROZ_MODEL=echo` 是测试开关（CI 会用），在没有配齐模型时把 prompt 原样写成结果。正式部署不要设。

其他常用的宿主机变量：`ROBROZ_STEP_TIMEOUT_SECS`（默认 `600`）和 `ROBROZ_STALL_SECS`（默认 `900`）控制卡片上多久出现「跑太久或卡住」的提示；`ROBROZ_APPROVAL_TTL_SECS`（默认 `86400`）是审批卡的有效期。完整列表见 [docs/configuration.md](docs/configuration.md)。从令牌到用 `curl` 走第一个任务，见 [docs/install.md](docs/install.md)。

## 架构

原生桌面客户端（`robroz-client`，[iced](https://iced.rs/) 0.14，Windows、Linux、macOS，不套网页）通过 HTTP 和 SSE 连到单机宿主机（`robroz-host`）。宿主机负责 SQLite、模型和工具调用、搜索、MCP、审批和容器泵。每个任务在一个没有网络的 Docker 容器（`robroz-runner`）里跑，容器看不到模型密钥。各部分怎么配合见 [docs/architecture.md](docs/architecture.md)。

| 路径 | crate | 作用 |
| --- | --- | --- |
| `host/` | `robroz-host` | HTTP 接口（axum）、SQLite、容器泵、定时、模型和工具调用、搜索、MCP、自动审核 |
| `runner/` | `robroz-runner` | 容器里的任务执行器 |
| `client/` | `robroz-client` | iced 桌面客户端 |
| `crates/robroz-contract` | `robroz-contract` | 接口契约：请求体和返回体、事件（卡片）类型、任务状态 |
| `crates/robroz-policy` | `robroz-policy` | 权限分级、出网计次、阻塞行 |
| `crates/robroz-card` | `robroz-card` | 卡片的状态文案、色调和动作 |
| `crates/robroz-tokens` | `robroz-tokens` | 由 `tokens.json` 生成的设计 token |
| `crates/robroz-ui` | `robroz-ui` | 客户端共用的界面层和主题 |
| `crates/robroz-model` | `robroz-model` | OpenAI 兼容 `/chat/completions` 客户端，含工具调用 |
| `crates/robroz-mcp` | `robroz-mcp` | MCP 客户端（stdio 和 Streamable HTTP） |

bro 是界面上对 bot 的叫法：代码、接口和数据库里照旧叫 bot（`botId` / `botName`）。四个任务状态 `queued` / `running` / `blocked` / `done` 已冻结。

## 文档

仓库里的文档才是契约。[wiki](https://github.com/PandaQuQ/robroz/wiki) 是非正式说明，两边冲突时以仓库为准。贡献者和编码 agent 的规则见 [AGENTS.md](AGENTS.md)。

| 文件 | 内容 |
| --- | --- |
| [CHANGELOG.md](CHANGELOG.md) | 版本说明 |
| [docs/install.md](docs/install.md) | 下载发行包、准备、构建、运行、接模型、第一个任务、测试 |
| [docs/docker-host.md](docs/docker-host.md) | 用 Docker 运行宿主机：compose、`docker.sock` 风险、同路径挂载、令牌 |
| [docs/configuration.md](docs/configuration.md) | 全部环境变量和默认值 |
| [docs/api.md](docs/api.md) | HTTP 接口和事件 |
| [docs/concepts.md](docs/concepts.md) | 术语、任务状态、卡片 |
| [docs/permissions.md](docs/permissions.md) | 三级权限、审批、接管、`failure` |
| [docs/design/auto-review.md](docs/design/auto-review.md) | 自动审核设计和放行规则 |
| [docs/memory-and-skills.md](docs/memory-and-skills.md) | 记忆、技能、确认卡 |
| [docs/search-and-mcp.md](docs/search-and-mcp.md) | 联网搜索和 MCP |
| [docs/client.md](docs/client.md) | 客户端界面行为 |
| [docs/architecture.md](docs/architecture.md) | 各部分分工、容器、外观层 |
| [docs/reference-features.md](docs/reference-features.md) | 参考产品和净室规则 |
| [docs/roadmap-phase2.md](docs/roadmap-phase2.md) | 后续路线图 |
| [docs/design/multi-agent.md](docs/design/multi-agent.md) | 群轮次、追问、Router 和 Watchdog 设计 |

`docs/` 下大多数文件目前仍以中文为主，会在后续 PR 里改成英文为主。引用的界面文案仍按客户端当前显示的语言写。

## 状态

v0.1.0 是计划中的第一个打标签版本。说明见 [CHANGELOG.md](CHANGELOG.md)。在标签发布之前，请把 `main` 当作开发分支，不要当作正式发版。

## 开发

开 pull request 前跑一遍和 CI 相同的命令：

```sh
cargo test -p robroz-mcp --features fixture
cargo test -p robroz-host -p robroz-contract -p robroz-policy -p robroz-card -p robroz-runner -p robroz-tokens
cargo build -p robroz-client
cargo test -p robroz-client -p robroz-ui -p robroz-tokens
scripts/e2e.sh   # 需要 Docker、curl 和 jq
```

CI 在 pull request 上只构建和测试 Linux 客户端；合进 `main` 后和每天定时会再跑 Windows、Linux、macOS。`cargo-deny` 会检查新依赖是否使用与 AGPL 兼容的许可证、是否来自已知的 registry；安全公告只报告，不挡合并。本地跑同样的检查（先安装 [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)）：

```sh
cargo deny check licenses bans sources
cargo deny check advisories   # 只报告，不挡合并
```

## 参与贡献

- 一个 pull request 只做一件事。
- 每位贡献者都要签一次[贡献者许可协议](CLA.md)：勾选 pull request 模板里的 CLA 框，或把 `CLA.md` 里那行声明加到正文里。没有签署的 pull request 不会合并。
- 提交信息、pull request 标题和正文都用英文，按 [Conventional Commits](https://www.conventionalcommits.org/) 写：`feat(host): …`、`fix(client): …`、`docs: …`。pull request 用 squash merge 合并，标题就是合并后的提交信息。
- 契约只增不改：可以新增卡片类型、接口和可选字段；已有字段、任务状态和 `botId` / `botName` 都冻住不改。
- `main` 受保护：只能通过 pull request 合入，五个 CI 检查（`host`、`cla`、`client (ubuntu-latest)`、`e2e`、`cargo-deny`）全部通过才能合并，管理员也不能绕过。
- 不 force-push。分支落后时把 `main` merge 进来。
- 不提交 `.env`、`.pem`、`.dev.vars`、密钥或令牌。
- `README.md` 和 `README.zh-CN.md` 保持一致，在同一个 pull request 里一起改。

完整规则见 [AGENTS.md](AGENTS.md)。

## 许可证

SPDX-License-Identifier: `AGPL-3.0-only`

robroz 采用 [GNU Affero 通用公共许可证 v3.0 only](LICENSE)。修改了 robroz 并通过网络给别人用的，必须向这些用户提供改过的源码（AGPL 第 13 条）。

闭源或专有用途可以另外取得商业授权，见 [COMMERCIAL.md](COMMERCIAL.md)。商业授权请[联系作者](https://github.com/PandaQuQ)。

贡献按[贡献者许可协议](CLA.md)接收。
