<div align="center">

# RataDash

**A keyboard-first terminal dashboard for Mihomo-compatible proxy cores.**

Use a lightweight TUI to inspect proxy status, switch nodes, manage connections, browse logs, and safely control an already running Mihomo core.

[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Ratatui](https://img.shields.io/badge/TUI-Ratatui-8A2BE2.svg)](https://ratatui.rs/)
[![Mihomo](https://img.shields.io/badge/core-Mihomo-1677FF.svg)](https://github.com/MetaCubeX/mihomo)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[中文](README_CN.md)

</div>

---

## About

RataDash is a control panel only. It does not bundle, download, or start a proxy core. It connects to an existing Mihomo or Clash-compatible core through its REST API and WebSocket interfaces, making routine proxy management convenient on servers, development machines, and remote terminals.

Navigation is organized around a focus chain. The primary menu stays visible, `Enter` moves into the next panel, and `←` / `→` cycle through the panels on the current page. The interface does not depend on numbered menus.

## Features

- **Overview**: Upload and download rates, memory usage, active connections, cumulative traffic, and core status.
- **Proxies**: Browse proxy groups and nodes, switch nodes, and run node or group latency tests.
- **Connections**: Browse and filter active connections, inspect connection details, and close one or all connections.
- **Logs**: Stream logs in real time, change the log level, pause or resume the stream, and clear the local view.
- **Settings**: Switch between `rule`, `global`, and `direct` modes, and manage proxy and rule providers.
- **Focus navigation**: The green border marks the active panel; `Enter` and `←` / `→` control focus navigation.
- **Safer control**: Secrets are never persisted, read-only mode is available, and remote plain HTTP connections display a warning.

## Quick Start

### Prerequisites

- Rust 1.91.1 or newer
- A running Mihomo or Clash-compatible core with External Controller enabled

### One-command install

Download the latest prebuilt release for your operating system and CPU architecture:

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh | bash
```

The default installation path is `~/.local/bin/ratadash`. To choose another directory:

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh \
  | bash -s -- --install-dir "$HOME/bin"
```

To install a specific release tag:

```bash
curl -fsSL https://raw.githubusercontent.com/Pinellia451/RataDash/main/install.sh \
  | bash -s -- --version v0.3.0
```

The installer downloads only a prebuilt binary from a GitHub Release. It does not download source code or install Rust.

### Build from source

```bash
cargo build --release
```

The binary is written to `target/release/ratadash`.

### Run

```bash
RATADASH_SECRET='your-secret' cargo run -- \
  --controller http://127.0.0.1:9090
```

For production use or an initial connection, read-only mode is recommended:

```bash
RATADASH_SECRET='your-secret' cargo run -- \
  --controller http://127.0.0.1:9090 \
  --read-only
```

You can also pass the secret with `--secret`. If no secret is supplied and the process has an interactive terminal, RataDash prompts for it. The controller address and credentials are not saved; using `RATADASH_SECRET` also keeps the secret out of shell history.

### Command-line options

```text
--controller <URL>           Mihomo External Controller URL
--secret <SECRET>            Controller secret; RATADASH_SECRET is also supported
--read-only                  Block every operation that changes core state
--timeout <SECONDS>          REST request timeout; default: 8
--refresh-ms <MILLISECONDS>  Periodic refresh interval; default: 5000
```

## Keyboard shortcuts

| Key | Action |
|---|---|
| `↑` / `↓`, `j` / `k` | Move through the list in the focused panel |
| `Enter` | Enter the next panel; perform the page action in the last panel |
| `←` / `→` | Cycle through focus panels |
| `t` / `T` | Test the selected node / proxy group |
| `/` | Filter connections or logs |
| `d` / `D` | Close the selected connection / close all connections |
| `Space` | Pause or resume the log stream |
| `l` / `c` | Change log level / clear the local log view |
| `m` | Switch core mode |
| `r` | Refresh the current page |
| `?` | Open help |
| `q` | Quit |

## Interface map

Each page has its own focus chain:

```text
Overview    Menu → Metrics → Traffic charts → Core status
Proxies     Menu → Proxy groups → Nodes
Connections Menu → Connections table → Connection details
Logs        Menu → Log stream → Log controls
Settings    Menu → Core settings → Proxy providers → Rule providers
```

Proxy providers come from Mihomo's `/providers/proxies` endpoint. RataDash hides built-in groups with `vehicleType=Compatible` and shows external HTTP and file providers. Rule providers come from `/providers/rules`.

## Technology stack

| Layer | Technology | Purpose |
|---|---|---|
| Language | Rust 2021 | Core implementation and type safety |
| TUI | [Ratatui](https://ratatui.rs/) | Panel layout, tables, charts, and terminal rendering |
| Terminal input | [Crossterm](https://github.com/crossterm-rs/crossterm) | Keyboard, mouse, terminal modes, and event streams |
| HTTP | [Reqwest](https://github.com/seanmonstar/reqwest) | Mihomo REST API requests |
| Async runtime | [Tokio](https://tokio.rs/) | Concurrent refreshes, task scheduling, and I/O |
| WebSocket | [Tokio Tungstenite](https://github.com/snapview/tokio-tungstenite) | Real-time traffic, memory, connection, and log streams |
| Serialization | [Serde](https://serde.rs/) | Mihomo JSON parsing |
| CLI | [Clap](https://docs.rs/clap/) | Command-line arguments and environment variables |

## Mihomo integration

RataDash uses Mihomo's External Controller and currently accesses these endpoints:

- `GET /version`
- `GET /configs`
- `GET /proxies`
- `GET /providers/proxies`
- `GET /providers/rules`
- `GET /connections`
- WebSocket streams for `/traffic`, `/memory`, and `/logs`
- `PUT /proxies/:name`
- `PUT /providers/proxies/:name`
- `PUT /providers/rules/:name`
- `PATCH /configs`
- `DELETE /connections/:id`
- `DELETE /connections`

Read-only mode intercepts write requests before they are sent, including node switches, provider refreshes, mode changes, and connection closures.

## Prebuilt binaries

Prebuilt binaries are published through GitHub Releases. The installer selects the matching asset automatically:

| Platform | Release asset |
|---|---|
| Linux x86_64 | `ratadash-linux-x86_64` |
| macOS Apple Silicon | `ratadash-darwin-arm64` |
| macOS Intel | `ratadash-darwin-x86_64` |

For other platforms, build RataDash from source. `target/`, `.micromamba/`, and local test scripts are not part of the release contents.

## Automated releases

Pushing a version tag starts the GitHub Actions release workflow:

```bash
git tag -a v0.3.0 -m "RataDash v0.3.0"
git push origin v0.3.0
```

The workflow builds Linux x86_64 (musl), macOS arm64, and macOS x86_64 binaries, then attaches them to a GitHub Release.

## References and notices

- [Mihomo](https://github.com/MetaCubeX/mihomo): The proxy core controlled by RataDash and the source of its External Controller API.
- [zashboard](https://github.com/Zephyruso/zashboard): A reference for the information architecture of proxy groups, nodes, connections, logs, and settings.
- [CC-Switch CLI](https://github.com/SaladDay/cc-switch-cli): A reference for README organization and terminal tool documentation.
- [Ratatui](https://github.com/ratatui/ratatui): The TUI framework used by this project.

RataDash is an independent project and is not affiliated with, sponsored by, or officially connected to Mihomo, Clash, zashboard, or CC-Switch CLI. The names, trademarks, and copyrights of those projects belong to their respective owners. They are referenced here for technical context and attribution.

RataDash does not include the Mihomo core and does not download, start, or distribute proxy node subscriptions. Users are responsible for complying with applicable laws and with the terms of the services and providers they connect to.

## License

RataDash is released under the [MIT License](LICENSE).
