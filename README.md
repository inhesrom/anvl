# Anvl

A most distinguished terminal-based multi-workspace manager, wrought in the noble language of Rust, for the discerning practitioner of the computational arts.

## Features

- **Multi-workspace management** with Git integration — branch tracking, status monitoring, and inline diffs, affording the operator a commanding view of all concurrent endeavours
- **Embedded terminal sessions** (agent & shell tabs) via PTY, with full input passthrough, as though one were seated before the telegraph itself
- **Attention system** that detects prompts, errors, and activity in terminal output, serving as a most vigilant watchman over one's processes
- **Session persistence** with a daemon/attach model for long-running workspaces, ensuring no labour is lost to the vicissitudes of disconnection
- **Web UI** with real-time WebSocket updates, served from an embedded HTTP server, a modern marvel of instantaneous correspondence
- **Mouse support** and terminal scrollback via mouse wheel, for those who prefer the pointing instrument
- **Vim-style navigation** throughout the interface, honouring the venerable traditions of text manipulation

## Architecture

Anvl is organised as a Cargo workspace comprising three crates, each attending to its particular office with the utmost diligence:

| Crate | Description |
|---|---|
| `protocol` | Serializable types for IPC — workspace routing, attention levels, terminal kinds, and command/event enums |
| `core` | Application state management — workspaces, Git, terminal PTY spawning, attention detection, SSH, and the async event loop |
| `tui` | Terminal UI built with Ratatui — renders home/workspace screens, handles input, and manages sessions |

Consult [docs/repo-diagram.md](/Users/ianhersom/repo/anvl/docs/repo-diagram.md) for a rendered diagram of the repository and an overview of its workings at runtime.

## Getting Started

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/inhesrom/anvl/master/install.sh | bash
```

Prebuilt binaries stand at the ready for the following platforms:
- macOS (Apple Silicon)
- Linux (x86_64)

The installer shall deposit the `anvl` binary within `~/.local/bin`. One may override this destination by setting `ANVL_INSTALL_DIR`.

### Build from Source

Should one prefer to forge the instrument with one's own hands, the following provisions are required.

#### Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)

#### Build

```sh
cargo build --release
```

#### Run

```sh
cargo run
# or
./target/release/anvl
```

## Usage

```
anvl                    Local mode (no session)
anvl -s <name>          Create and start a named session
anvl -a <name>          Attach to an existing session
anvl -l                 List sessions
anvl -r <name>          Remove a session
anvl -d                 Detach (use with -s or -a)
```

## Key Bindings

Herewith, a complete catalogue of the keyboard incantations by which one commands this instrument.

### Global

| Key | Action |
|---|---|
| `q` | Quit |
| `Tab` / `Shift+Tab` | Cycle focus between sections |
| `Esc` | Exit focused section / go back |

### Home Screen

| Key | Action |
|---|---|
| `h` `j` `k` `l` / Arrow keys | Navigate workspaces |
| `Enter` | Open selected workspace |
| `n` | New workspace |
| `D` | Delete workspace |
| `!` | Toggle attention level |
| `g` | Refresh git status |

### Workspace Screen

| Key | Action |
|---|---|
| `1` `2` / `h` `l` | Switch terminal tabs |
| `n` | New shell tab |
| `x` | Close active tab |
| `r` | Rename tab |
| `a` / `A` | Start / stop agent terminal |
| `s` / `S` | Start / stop shell terminal |
| `g` | Refresh git |
| `j` `k` / Arrow keys | Navigate file list |
| `Enter` | Show diff for selected file |
| Mouse wheel | Scroll terminal output |

## Configuration

### Environment Variables

| Variable | Description | Default |
|---|---|---|
| `ANVL_WEB_PORT` | Embedded web server port | `3001` |
| `ANVL_DISABLE_EMBEDDED_WEB` | Disable the embedded web server if set | — |
| `SHELL` | Shell used for terminal sessions | `zsh` |

### Config Paths

Anvl maintains its configuration beneath `~/.config/anvl/` (with due respect for `XDG_CONFIG_HOME`):

- `sessions.json` — the registry of sessions
- `workspaces.json` — default workspace persistence
- `workspaces.<session-name>.json` — per-session workspace state

## License

MIT
