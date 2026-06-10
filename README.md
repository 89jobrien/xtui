# xtui

[![crates.io](https://img.shields.io/crates/v/xtui.svg)](https://crates.io/crates/xtui)

A terminal UI for discovering and running project commands. Point it at any
project directory and it finds runnable commands from 8 sources, organizes
them into tabs, and lets you run them with live streaming output.

## Sources

| Source    | Detects                            | Runs via                      |
| --------- | ---------------------------------- | ----------------------------- |
| xtask     | `xtask/src/main.rs` match arms     | `cargo run -p xtask -- <cmd>` |
| cargo     | `Cargo.toml` (+ `[[bin]]` targets) | `cargo <cmd>`                 |
| just      | `Justfile` / `justfile`            | `just <recipe>`               |
| nu        | `scripts/*.nu`                     | `nu scripts/<name>.nu`        |
| npm       | `package.json` scripts             | `npm run <script>`            |
| make      | `Makefile` targets                 | `make <target>`               |
| mise      | `mise.toml` / `.mise.toml` tasks   | `mise run <task>`             |
| cargo-bin | `~/.cargo/bin/` executables        | `<binary>`                    |

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
| `a`                 | Open args-input mode (enter args before run)  |
| `o`                 | Focus output pane                             |
| `g` / `G`           | Scroll output to top / bottom (output focus)  |
| `Esc`               | Cancel running command / exit output focus    |
| `Ctrl+C`            | Cancel running command or quit                |
| `/`                 | Search output                                 |
| `n` / `N`           | Next / previous search match                  |
| `s`                 | Toggle git status tab                         |
| `D`                 | Toggle dependency graph view                  |
| `r`                 | Refresh commands                              |
| `c`                 | Copy output to clipboard (OSC 52)             |
| `P`                 | Run all commands in current tab as a pipeline |
| `q`                 | Quit                                          |

## Features

- **Tabbed sources**: each source gets its own tab with discovered commands
- **Live output**: stdout/stderr streamed in real time with ANSI color support
- **Git status bar**: branch, dirty/clean, ahead/behind, recent commits
- **Output search**: incremental case-insensitive search with match cycling
- **Dep view**: toggle with `D` to see direct workspace dependencies with crates.io latest version
  and versions-behind count, cached globally in redb
- **Pipelines**: run all commands in a tab sequentially, stop on failure
- **History**: last 50 runs per project saved to `~/.config/xtui/history/`
- **Output logs**: command output persisted as `.log` files (max 100 per project)

## Install

```sh
cargo install xtui
```

## Building from source

```sh
cargo build --release

# Or install to ~/.cargo/bin
cargo xtask install
```

## Dev Tasks

```sh
cargo xtask check            # cargo check --workspace
cargo xtask test             # cargo test --workspace
cargo xtask clippy           # clippy -D warnings
cargo xtask install          # install to ~/.cargo/bin
cargo xtask docs             # build mdbook → xbook/dist/
cargo xtask book             # build and serve mdbook (opens browser)
cargo xtask graph            # render cargo-rail dep graph as text tree
cargo xtask promote-staging  # FF-merge develop → staging and push
cargo xtask promote-main     # FF-merge staging → main and push
cargo xtask nightly          # build release binary and upsert nightly tag
cargo xtask xstate        # refresh .ctx/xstate.json (session metadata)
cargo xtask xstate --verify  # verify xstate matches HEAD (exit 1 if stale)
```

## Testing

```sh
# Unit + integration tests (fixture-based)
cargo test

# Include real-repo tests (requires local repos)
cargo test -- --ignored
```

## Docs

```sh
# Build only → xbook/dist/
cargo xtask docs

# Build and serve (opens browser)
cargo xtask book
```

Reads [`xbook/copies.toml`](xbook/copies.toml) to copy source files (README,
CLAUDE.md, design plans, memory bank, knowledge graph wiki) into `xbook/`, then
builds an [mdbook](https://rust-lang.github.io/mdBook/) at `xbook/dist/`. The
source files stay in their canonical locations in the repo — `xbook/` holds only
generated copies and built output (both gitignored). Requires `mdbook`
(`cargo install mdbook`).

## Project Structure

```
src/
  main.rs         Entry point, workspace resolution
  app.rs          App state, event loop, key handling
  ui.rs           Ratatui rendering (layout, tabs, output, status)
  source.rs       CommandSource trait + 8 implementations
  discover.rs     Xtask main.rs parser (regex-based)
  bin_schema.rs   Cargo-bin subcommand cache (JSON, mtime-invalidated)
  runner.rs       Async process spawning and output streaming
  pipeline.rs     Sequential command chaining state machine
  search.rs       Output search with match tracking
  history.rs      Run history and output log persistence
  status.rs       Git status collection
  depview.rs      Dep graph domain types and workspace dep collection
  meta_cache.rs   Metadata cache port + RedbCache adapter
  meta_fetch.rs   MetadataFetcher port + HttpMetadataFetcher adapter
  registry.rs     Project discovery and cache (unused in v1)
xtask/
  src/main.rs   Dev tasks: check, test, clippy, install, docs, graph, promote-*, nightly, xstate
scripts/
  xstate.nu  Generate/verify .ctx/xstate.json
  session-init.nu  SessionStart hook entry point (reads Claude session_id from stdin)
.claude/
  settings.json  Project-level Claude Code hooks (SessionStart → xstate)
.ctx/
  xstate.json  Local session metadata: session_id, hash, crate versions, git state (gitignored)
xbook/
  book.toml     mdbook config (src = ".", build-dir = "dist")
  copies.toml   Declares which files to copy into xbook/ (supports globs)
  SUMMARY.md    mdbook table of contents (tracked)
  intro.md      Book introduction page (tracked)
  */            Generated copies of source files (gitignored)
  dist/         Built HTML output (gitignored)
tests/
  common/       Shared fixture builder
  integration.rs  Cross-module integration tests
  sources.rs    Per-source integration tests
```

## License

MIT OR Apache-2.0
