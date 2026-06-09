# Architecture

## Overview

xtui is a lib + binary crate. `lib.rs` re-exports all modules; `main.rs` is the entry point.

```
CLI args -> find_workspace_root -> all_sources() -> App::new -> App::run (event loop)
```

## Modules

| Module          | Concern                                                       |
| --------------- | ------------------------------------------------------------- |
| `main.rs`       | CLI entry, workspace root discovery (walks up for Cargo.lock) |
| `app.rs`        | App state, event loop, keybindings, args-input mode           |
| `ui.rs`         | ratatui rendering — pure, no state mutation                   |
| `source.rs`     | `CommandSource` trait + 8 source implementations              |
| `discover.rs`   | xtask `main.rs` parser (regex, match arm extraction)          |
| `bin_schema.rs` | Cargo-bin subcommand cache (JSON, mtime-invalidated)          |
| `runner.rs`     | Child process spawning, stdout/stderr streaming via mpsc      |
| `pipeline.rs`   | Sequential command chaining state machine                     |
| `search.rs`     | Output search with match cycling                              |
| `history.rs`    | JSON history + `.log` files in `~/.config/xtui/`              |
| `status.rs`     | Git status via `Command::new("git")`                          |
| `registry.rs`   | Project scanner/cache (not wired into UI — reserved)          |

Each module owns exactly one concern. No cross-module state mutation.

## Command Sources

`CommandSource` is a port trait in `source.rs`:

```rust
pub trait CommandSource: Send + Sync {
    fn name(&self) -> &str;
    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>>;
}
```

`discover()` returns an empty vec when the source is not applicable — it never errors for
"not found". `all_sources()` returns implementations in tab order:

| Tab | Source    | Detection file             | Runs via                      |
| --- | --------- | -------------------------- | ----------------------------- |
| 0   | xtask     | `xtask/src/main.rs`        | `cargo run -p xtask -- <cmd>` |
| 1   | cargo     | `Cargo.toml`               | `cargo <cmd>`                 |
| 2   | just      | `Justfile` / `justfile`    | `just <recipe>`               |
| 3   | nu-script | `scripts/*.nu`             | `nu scripts/<name>.nu`        |
| 4   | npm       | `package.json` scripts     | `npm run <script>`            |
| 5   | make      | `Makefile`                 | `make <target>`               |
| 6   | mise      | `mise.toml` / `.mise.toml` | `mise run <task>`             |
| 7   | cargo-bin | `~/.cargo/bin/`            | `<binary> [subcmd]`           |

Empty tabs are hidden in the UI.

## Cargo-Bin Schema Cache

`bin_schema.rs` provides lazy per-binary subcommand discovery with mtime-invalidated JSON
cache at `~/.config/xtui/bin-schema/<binary>.json`.

```
CargoBinSource::discover()
  └── get_schema(dir, binary_name)
        ├── load_cached()  reads JSON, checks mtime + schema version
        │     └── stale → cache miss
        └── probe_and_cache()
              ├── blocklist check (daemons, GUIs, profilers — 13 entries)
              ├── spawn thread: <binary> --help, recv_timeout(500ms)
              ├── parse_help_subcommands()  finds Commands:/Subcommands: section
              └── save_schema()  writes JSON cache
```

Emits `"<bin> <subcmd>"` names when subcommands are found; bare binary name otherwise.
The runner's `cmd.name.split_whitespace()` dispatch handles both cases transparently.

## Process Runner

`runner.rs` spawns a child process with piped stdout/stderr. Two tokio tasks read each
stream via `LinesCodec` into a shared mpsc channel. `RunningTask` exposes `poll_lines()`,
`try_exit_code()`, and `cancel()`. The runner dispatches on `cmd.source` to pick the
right program and args.

## Pipeline State Machine

`pipeline.rs` is a pure state machine for sequential command chaining:

```
Idle -> Running(idx) -> Done
                     -> Failed(idx)
```

The caller handles actual execution — pipeline only tracks state.

## UI Layout

```
+------------------------------------------+
|  [xtask] [cargo] [just] [make] ...       |  <- tab bar
+-------------+----------------------------+
|             |                            |
|  Commands   |  Output (ANSI, streaming)  |
|  (~30%)     |  (~70%)                    |
|             |                            |
+-------------+----------------------------+
|  workspace · state · N commands          |  <- status bar / flash messages
+------------------------------------------+
```

Output pane auto-scrolls to bottom; focusable for manual scroll. Flash messages replace
the status bar for 2 seconds. Args-input mode overlays a modal prompt before run.

## Key Design Decisions

- **OSC52 clipboard** — avoids a platform clipboard dependency; works in any terminal
  that supports OSC52. Custom base64 encoder avoids adding a dep for one function.
- **`anyhow::Result` throughout** — no custom error types. Consider `thiserror` only if
  error variants need matching at call sites.
- **Buffer caps** — 10k output lines (drains oldest 1k on overflow), 1024-line channel
  buffer, 50ms poll interval.
- **History caps** — 50 entries per project, 100 log files per project.
