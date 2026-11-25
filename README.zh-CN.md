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

> 本仓库是 robroz **客户端**：原生桌面应用（Windows、Linux、macOS）和手机应用（Android、iOS）。宿主机（服务端）、runner、接口契约和 Docker 镜像在 [PandaQuQ/robroz](https://github.com/PandaQuQ/robroz)。客户端要连一个在跑的宿主机。

自托管的多智能体工作台，用聊天来驱动。智能体（bro）在 Docker 沙箱里干活；审批和结果以卡片回到对话里。本仓库是原生客户端，用 Rust 写。

见 [CHANGELOG.md](CHANGELOG.md) 和 [docs/install.md](docs/install.md)。

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

下面是已接受的设计，或正在做，**还没有**合进 `main`。细节见 宿主机仓库的 [design/multi-agent.md](https://github.com/PandaQuQ/robroz/blob/main/docs/design/multi-agent.md)。

- 超时后取消步骤或停掉容器。

## 快速开始

需要 Rust stable 工具链（`rustup`、`cargo`）和一个在跑的宿主机。起宿主机按 [PandaQuQ/robroz](https://github.com/PandaQuQ/robroz#quick-start) 的快速开始做，默认监听 `127.0.0.1:8790`。不想自己构建客户端，可以装发布好的二进制，见 [docs/install.md](docs/install.md)。

```sh
cargo run -p robroz-client   # 默认连接 http://127.0.0.1:8790
cargo run -p robroz-client -- --fake   # 不连宿主机，只看界面
```

宿主机在别的机器上时，到设置的 Connection（「连接」）里填 Host 和 Token，或者只对这一次运行设 `ROBROZ_HOST` 和 `ROBROZ_TOKEN`。见 [docs/install.md](docs/install.md#run-the-client)。

在客户端 Chats（「聊天」）标题旁点 Add a bro（「引入一个 bro」），打开私聊发一条消息，这就是第一个任务。没设置模型时，交付是 No model set up yet（「还没设置模型」），卡片上有 Set up（「去设置」）。打开 Model（「模型」）或点这个链接，填地址和模型名后保存；密钥可选，由宿主机保存，客户端不存。

## 架构

桌面客户端（`robroz-client`，[iced](https://iced.rs/) 0.14，不套网页）通过 HTTP 和 SSE 连到宿主机（`robroz-host`）。请求、返回和卡片类型来自宿主机仓库的 `robroz-contract`，在 `Cargo.toml` 里按 git tag（`contract-v*`）固定。客户端的 `against_host` 测试按同一个 tag 构建宿主机，所以测客户端不需要 Docker。宿主机那边怎么配合见 [architecture.md](https://github.com/PandaQuQ/robroz/blob/main/docs/architecture.md)。

| 路径 | crate | 作用 |
| --- | --- | --- |
| `client/` | `robroz-client` | iced 桌面客户端 |
| `mobile/` | `robroz-mobile` | 手机客户端（Android 和 iOS），见 [mobile/README.md](mobile/README.md) |
| `crates/robroz-card` | `robroz-card` | 卡片的状态文案、色调和动作 |
| `crates/robroz-tokens` | `robroz-tokens` | 由 `tokens.json` 生成的设计 token |
| `crates/robroz-ui` | `robroz-ui` | 客户端共用的界面层和主题 |

「bro」是界面上的叫法，代码、接口和数据库里仍然叫 bot（`botId` / `botName`）。四个任务状态 `queued` / `running` / `blocked` / `done` 冻住。

## 文档

仓库里的文档是契约；[wiki](https://github.com/PandaQuQ/robroz/wiki) 和仓库不一致时，以仓库为准。给贡献者和编码 agent 的规则见 [AGENTS.md](AGENTS.md)。

| 文件 | 内容 |
| --- | --- |
| [CHANGELOG.md](CHANGELOG.md) | 发版说明 |
| [docs/install.md](docs/install.md) | 下载发布版、构建、运行客户端、连接宿主机 |
| [docs/configuration.md](docs/configuration.md) | 客户端的环境变量和保存的偏好 |
| [docs/client.md](docs/client.md) | 客户端界面行为 |
| [mobile/README.md](mobile/README.md) | 构建手机客户端 |

接口、卡片、权限、审批、记忆、技能、搜索、MCP 和宿主机配置，见[宿主机仓库的文档](https://github.com/PandaQuQ/robroz/tree/main/docs)。

## 状态

最新发布是预发布版，见 [Releases](https://github.com/PandaQuQ/robroz/releases)（v0.1.1 及以前都从宿主机仓库发布）。`main` 是 v0.2 的开发分支。

## 开发

提 pull request 前先跑和 CI 相同的命令：

```sh
cargo build -p robroz-client
cargo test -p robroz-client -p robroz-card -p robroz-ui -p robroz-tokens
```

CI 在 pull request 上只构建和测试 Linux 客户端；合进 `main` 后和每天定时会再跑 Windows、Linux、macOS。另有一个工作流构建 Android APK 和未签名的 iOS 应用。`cargo-deny` 任务检查新依赖的许可证和 AGPL 兼容、来源是已知 registry；安全公告只报告，不挡合并。装好 [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) 后可以在本地跑同样的检查：

```sh
cargo deny check licenses bans sources
cargo deny check advisories   # 只报告，不挡合并
```

## 参与贡献

- 一个 pull request 只做一件改动。
- 每个贡献者要签一次[贡献者许可协议](CLA.md)：勾选 pull request 模板里的 CLA 选项，或者把 `CLA.md` 里的那行声明加进 pull request 正文。没签的不合。
- 提交信息、pull request 标题和正文都用英文，按 [Conventional Commits](https://www.conventionalcommits.org/) 写：`feat(client): …`、`fix(mobile): …`、`docs: …`。pull request 用 squash 合并，标题就是提交信息。
- 契约改动先进宿主机仓库；本仓库再升到新的 `contract-v*` tag。
- `main` 受保护：只能通过 pull request 改，三个 CI 检查（`client (ubuntu-latest)`、`cargo-deny`、`cla`）全绿才能合。谁都绕不过，管理员也一样。
- 不 force-push。分支落后时把 `main` merge 进来。
- 不提交 `.env`、`.pem`、`.dev.vars`、密钥或令牌。
- `README.md` 和 `README.zh-CN.md` 保持同步，在同一个 pull request 里一起改。

完整规则见 [AGENTS.md](AGENTS.md)。

## 许可证

SPDX-License-Identifier: `AGPL-3.0-only`

客户端只按 [GNU Affero General Public License v3.0 only](LICENSE) 授权。修改后通过网络让别人使用，就要向他们提供修改后的源代码（AGPL 第 13 条）。客户端不提供商业授权；商业授权只覆盖 [PandaQuQ/robroz](https://github.com/PandaQuQ/robroz) 里的宿主机。

贡献按[贡献者许可协议](CLA.md)接受。
