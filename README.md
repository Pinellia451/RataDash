<div align="center">

# RataDash

**一个面向 Mihomo / Clash 控制接口的键盘优先终端控制面板。**

用一个轻量的 TUI 查看代理状态、切换节点、管理连接、浏览日志，并安全地控制已经运行的 Mihomo 核心。

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Ratatui](https://img.shields.io/badge/TUI-Ratatui-8A2BE2.svg)](https://ratatui.rs/)
[![Mihomo](https://img.shields.io/badge/core-Mihomo-1677FF.svg)](https://github.com/MetaCubeX/mihomo)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

</div>

---

## 📖 About

RataDash 只负责控制面板，不内置、下载或启动代理内核。它通过 Mihomo 的 REST API 和 WebSocket 接口连接到已有核心，适合在服务器、开发机或远程终端中快速完成日常代理管理。

它的交互重点是“面板焦点链”：左侧一级菜单始终可见，`Enter` 进入下一个面板，`←/→` 循环切换焦点。整个界面不依赖数字编号菜单。

## ✨ Features

- **总览**：上下行速率、内存、活动连接、累计流量和核心状态。
- **代理**：查看代理组和节点，切换节点，执行单节点或代理组测速。
- **连接**：浏览活动连接、筛选连接、查看详情并关闭单条或全部连接。
- **日志**：实时日志流、级别切换、暂停/继续和本地清屏。
- **设置**：切换 `rule → global → direct`，分别管理订阅 Provider 与规则 Provider。
- **焦点导航**：绿色边框表示当前面板，支持 `Enter` 和循环 `←/→`。
- **安全控制**：Secret 不落盘，支持只读模式，远程明文 HTTP 会给出警告。

## 🚀 Quick Start

### Prerequisites

- Rust 1.91.1 or newer
- 一个已经运行并开放 External Controller 的 Mihomo / Clash 核心

### One-command install

从 GitHub Releases 下载当前系统和 CPU 架构对应的最新版本：

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh | bash
```

默认安装到 `~/.local/bin/ratadash`。也可以指定安装目录：

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh \
  | bash -s -- --install-dir "$HOME/bin"
```

需要安装指定版本时，传入 Release Tag：

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh \
  | bash -s -- --version v0.3.0
```

安装脚本只下载 GitHub Release 中的预编译二进制，不会下载源码或安装 Rust。

### Build from source

```bash
# 在 RataDash 仓库目录执行
cargo build --release
```

构建产物位于 `target/release/ratadash`。

### Run

```bash
RATADASH_SECRET='your-secret' cargo run -- \
  --controller http://127.0.0.1:9090
```

生产环境或首次连接时，建议启用只读模式：

```bash
RATADASH_SECRET='your-secret' cargo run -- \
  --controller http://127.0.0.1:9090 \
  --read-only
```

也可以传入 `--secret`。程序不会保存控制地址或凭据；环境变量通常更适合避免 Secret 出现在 shell 历史中。

### Command-line options

```text
--controller <URL>          Mihomo External Controller 地址
--secret <SECRET>           控制端 Secret，也可使用 RATADASH_SECRET
--read-only                 阻止所有会改变核心状态的操作
--timeout <SECONDS>         REST 请求超时，默认 8 秒
--refresh-ms <MILLISECONDS> 周期刷新间隔，默认 5000 毫秒
```

## ⌨️ Keyboard

| Key | Action |
|---|---|
| `↑` / `↓`, `j` / `k` | 操作当前焦点面板中的列表 |
| `Enter` | 进入下一个面板；末级执行当前页面动作 |
| `←` / `→` | 循环切换焦点面板 |
| `t` / `T` | 节点测速 / 代理组测速 |
| `/` | 筛选连接或日志 |
| `d` / `D` | 关闭选中连接 / 关闭全部连接 |
| `Space` | 暂停或继续日志 |
| `l` / `c` | 切换日志级别 / 清空本地日志 |
| `m` | 切换运行模式 |
| `r` | 刷新当前页面 |
| `?` | 打开帮助 |
| `q` | 退出 |

## 🧭 Interface map

每个页面都有自己的焦点链：

```text
总览   菜单 → 指标 → 流量图 → 核心状态
代理   菜单 → 代理组 → 节点
连接   菜单 → 连接表 → 连接详情
日志   菜单 → 日志流 → 筛选 / 日志操作
设置   菜单 → 核心设置 → 订阅 Provider → 规则 Provider
```

订阅 Provider 来自 Mihomo 的 `/providers/proxies`，RataDash 会排除 `vehicleType=Compatible` 的内置分组，只展示外部 HTTP/File Provider。规则 Provider 来自 `/providers/rules`。

## 🧱 Technology stack

| Layer | Technology | Purpose |
|---|---|---|
| Language | Rust 2021 | 核心实现与类型安全 |
| TUI | [Ratatui](https://ratatui.rs/) | 面板布局、表格、图表和终端渲染 |
| Terminal input | [Crossterm](https://github.com/crossterm-rs/crossterm) | 键盘、鼠标、终端模式和事件流 |
| HTTP | [Reqwest](https://github.com/seanmonstar/reqwest) | Mihomo REST API 请求 |
| Async runtime | [Tokio](https://tokio.rs/) | 并发刷新、任务调度和 I/O |
| WebSocket | [Tokio Tungstenite](https://github.com/snapview/tokio-tungstenite) | traffic、memory、connections、logs 实时流 |
| Serialization | [Serde](https://serde.rs/) | Mihomo JSON 数据解析 |
| CLI | [Clap](https://docs.rs/clap/) | 命令行参数和环境变量 |

## 🔌 Mihomo integration

RataDash 通过 External Controller 访问 Mihomo，主要使用以下接口：

- `GET /version`
- `GET /configs`
- `GET /proxies`
- `GET /providers/proxies`
- `GET /providers/rules`
- `GET /connections`
- `GET /traffic`、`GET /memory`、`GET /logs` WebSocket 流
- `PUT /proxies/:name`、`PUT /providers/proxies/:name`、`PUT /providers/rules/:name`
- `PATCH /configs`
- `DELETE /connections/:id`、`DELETE /connections`

只读模式会在发送写请求前拦截节点切换、Provider 刷新、模式切换和连接关闭等操作。

## 📦 Prebuilt binary

预编译二进制只通过 GitHub Releases 发布，安装脚本会自动选择当前平台对应的附件：

| Platform | Release asset |
|---|---|
| Linux x86_64 | `ratadash-linux-x86_64` |
| macOS Apple Silicon | `ratadash-darwin-arm64` |
| macOS Intel | `ratadash-darwin-x86_64` |

其他平台建议从源码构建。`target/`、`.micromamba/` 和本地测试脚本不属于发布内容。

## 🤖 Automated releases

推送版本 Tag 后，GitHub Actions 会自动构建并发布三个版本：

```bash
git tag -a v0.3.0 -m "RataDash v0.3.0"
git push origin v0.3.0
```

Workflow 会生成 Linux x86_64（musl）、macOS arm64 和 macOS x86_64 二进制，并自动创建 GitHub Release 附件。

## 📚 References & notices

- [Mihomo](https://github.com/MetaCubeX/mihomo)：RataDash 当前控制的代理核心及 External Controller API 来源。
- [zashboard](https://github.com/Zephyruso/zashboard)：代理组、节点、连接、日志和设置等面板信息架构的参考项目。
- [CC-Switch CLI](https://github.com/SaladDay/cc-switch-cli)：README 信息组织与终端工具文档表达方式的参考项目。
- [Ratatui](https://github.com/ratatui/ratatui)：本项目使用的 TUI 框架。

RataDash 是独立项目，与 Mihomo、Clash、zashboard 或 CC-Switch CLI 没有隶属、赞助或官方合作关系。上述项目名称、商标和代码版权归各自权利人所有；本项目仅在 README 中进行技术引用和致谢。

本项目不包含 Mihomo 内核，也不负责下载、启动或分发代理节点订阅内容。使用者应遵守所在地区的法律法规，以及所连接服务和 Provider 的使用条款。

## 📄 License

RataDash 以 [MIT License](LICENSE) 发布。
