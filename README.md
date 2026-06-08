# xtui

A terminal UI for discovering and running project commands. Point it at any
project directory and it finds runnable commands from 7 sources, organizes
them into tabs, and lets you run them with live streaming output.

## Sources

| Source | Detects                            | Runs via                      |
| ------ | ---------------------------------- | ----------------------------- |
| xtask  | `xtask/src/main.rs` match arms     | `cargo run -p xtask -- <cmd>` |
| cargo  | `Cargo.toml` (+ `[[bin]]` targets) | `cargo <cmd>`                 |
| just   | `Justfile` / `justfile`            | `just <recipe>`               |
| nu     | `scripts/*.nu`                     | `nu scripts/<name>.nu`        |
| npm    | `package.json` scripts             | `npm run <script>`            |
| make   | `Makefile` targets                 | `make <target>`               |
| mise   | `mise.toml` / `.mise.toml` tasks   | `mise run <task>`             |

## Usage

```sh
# Run in current directory
xtui

# Run against a specific project
xtui /path/to/project
xtui --path /path/to/project
```

## Keybindings

| Key                 | Action                                        |
| ------------------- | --------------------------------------------- |
| `j` / `k` / arrows  | Navigate commands or scroll output            |
| `Tab` / `Shift+Tab` | Cycle source tabs                             |
| `1`-`9`             | Jump to tab by index                          |
| `Enter`             | Run selected command                          |
| `o`                 | Focus output pane                             |
| `Esc`               | Cancel running command / exit output focus    |
| `Ctrl+C`            | Cancel running command or quit                |
| `/`                 | Search output                                 |
| `n` / `N`           | Next / previous search match                  |
| `s`                 | Toggle git status tab                         |
| `r`                 | Refresh commands                              |
| `c`                 | Copy output to clipboard (OSC 52)             |
| `P`                 | Run all commands in current tab as a pipeline |
| `q`                 | Quit                                          |

## Features

- **Tabbed sources**: each source gets its own tab with discovered commands
- **Live output**: stdout/stderr streamed in real time with ANSI color support
- **Git status bar**: branch, dirty/clean, ahead/behind, recent commits
- **Output search**: incremental case-insensitive search with match cycling
- **Pipelines**: run all commands in a tab sequentially, stop on failure
- **History**: last 50 runs per project saved to `~/.config/xtui/history/`
- **Output logs**: command output persisted as `.log` files (max 100 per project)

## Building

```sh
cargo build --release

# Or install to ~/.cargo/bin
cargo xtask install
```

## Testing

```sh
# Unit + integration tests (fixture-based)
cargo test

# Include real-repo tests (requires local repos)
cargo test -- --ignored
```

## Project Structure

```
src/
  main.rs       Entry point, workspace resolution
  app.rs        App state, event loop, key handling
  ui.rs         Ratatui rendering (layout, tabs, output, status)
  source.rs     CommandSource trait + 7 implementations
  discover.rs   Xtask main.rs parser (regex-based)
  runner.rs     Async process spawning and output streaming
  pipeline.rs   Sequential command chaining state machine
  search.rs     Output search with match tracking
  history.rs    Run history and output log persistence
  status.rs     Git status collection
  registry.rs   Project discovery and cache (unused in v1)
xtask/
  src/main.rs   Dev tasks: check, test, clippy, install
tests/
  common/       Shared fixture builder
  integration.rs  Cross-module integration tests
  sources.rs    Per-source integration tests
```

## License

MIT OR Apache-2.0
