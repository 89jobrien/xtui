# System Patterns: xtui

## Architecture

Lib + binary crate. `lib.rs` re-exports everything; `main.rs` is the entry point.

```
CLI args -> find_workspace_root -> all_sources() -> App::new -> App::run (event loop)
```

Hexagonal: ports are traits, adapters are impls.

## Command Sources (source.rs)

Port trait `CommandSource` with 8 impls. `all_sources()` returns them in tab order:

| Tab | Source     | Detection                                  |
|-----|------------|--------------------------------------------|
| 0   | xtask      | `xtask/src/main.rs` present                |
| 1   | cargo      | `Cargo.toml` present                       |
| 2   | just       | `Justfile` present                         |
| 3   | nu-script  | `*.nu` scripts present                     |
| 4   | npm        | `package.json` scripts                     |
| 5   | make       | `Makefile` present                         |
| 6   | mise       | `.mise.toml` tasks                         |
| 7   | cargo-bin  | `~/.cargo/bin/` + subcommand schema cache  |

Each source returns `Vec<SourceCommand>` — empty vec = not applicable.

## Dep View (depview.rs + meta_cache.rs + meta_fetch.rs)

Ports/adapters for dependency metadata:

```
MetadataCache trait (port) → RedbCache adapter (redb embedded KV, TTL-based)
MetadataFetcher trait (port) → HttpMetadataFetcher adapter (ureq → crates.io JSON API)
```

App fields: `show_dep_view`, `dep_infos`, `dep_scroll`, `dep_rx`.
Toggle via D key → `toggle_dep_view()` → spawns tokio tasks per dep via `spawn_dep_fetch()`.
Results polled in event loop via `poll_dep_results()`.
UI: `draw_dep_view()` in ui.rs renders full-screen panel when `show_dep_view` true.
Local/path deps detected and labeled separately (no crates.io fetch).

## Cargo-Bin Schema Cache (bin_schema.rs)

Lazy per-binary subcommand discovery with mtime-invalidated JSON cache.

```
discover() called
  └── get_schema(dir, binary_name)
        ├── load_cached() — reads ~/.config/xtui/bin-schema/<binary>.json
        │     └── stale (mtime mismatch or version change) → cache miss
        └── probe_and_cache()
              ├── blocklist check (daemons, GUIs, profilers — 13 entries)
              ├── spawn thread: <binary> --help, recv_timeout(500ms)
              ├── parse_help_subcommands() — finds Commands:/Subcommands: section
              └── save_schema() → writes JSON cache
```

## Command Discovery (discover.rs)

XtaskSource backend: reads `xtask/src/main.rs`, parses match arms and `Some("name")`
patterns. Handles nested subcommands and bare match arms.

## Process Runner (runner.rs)

- Spawns child with piped stdout/stderr
- Two tokio tasks read stdout and stderr via LinesCodec into shared mpsc channel
- `RunningTask` exposes `poll_lines()`, `try_exit_code()`, `cancel()`
- Runner dispatches on `cmd.source` string to pick right program/args

## Pipeline (pipeline.rs)

Pure state machine for sequential command chaining. Caller handles execution.
States: Idle → Running(idx) → Done | Failed(idx).

## UI Layout (ui.rs)

- Tab bar for sources at top
- Horizontal split: ~30% command list | ~70% output pane
- Output pane auto-scrolls to bottom
- Status bar at bottom shows workspace name, state, command count
- Flash messages temporarily replace status bar text
- Args-input overlay: modal input line before run
- Dep view: full-screen overlay, replaces normal layout when active

## Key Bindings

| Key     | Action                         |
|---------|--------------------------------|
| Tab/h/l | Switch source tab              |
| j/Down  | Next command (or scroll dep view) |
| k/Up    | Previous command (or scroll dep view) |
| Enter   | Run selected (or confirm args) |
| a       | Open args-input mode           |
| D       | Toggle dep view                |
| Esc     | Cancel running / close args    |
| r       | Refresh commands               |
| c       | Copy output (OSC52)            |
| /       | Search output                  |
| n/N     | Next/prev search match         |
| q       | Quit                           |

## Dev-State (scripts/dev-state.nu)

Local session metadata file at `.ctx/dev-state.json` (gitignored). Two modes:

```
cargo xtask dev-state           # generate: write all fields, two-pass for mtime
cargo xtask dev-state --verify  # verify: recompute ground-truth fields, exit 1 if mismatch
```

Session identity: `session_id = "{project}-{pid}-{YYMMDD.HHMMSS}"` (fully derived).
`session_hash` = sha256 of `session_id + workspace_root + hostname + username` — stable
across refreshes within a session, changes on new session.

Pre-push hook order: refresh → verify → version bump + amend → refresh again.

## Conventions

- Custom base64 encoder (no external dep) for OSC52 clipboard
- `anyhow::Result` throughout for error propagation
- Sources return empty vec, never error, when not applicable to a project
- Commit types: `chore`/`refactor` for quality passes (not `quality` — git-cliff drops it)
- `xtask` is always version `0.0.0` — internal tool, not published
