# AGENTS.md — xtui

## What This Is

TUI for discovering and running project commands from 8 sources (xtask, cargo,
just, nu, npm, make, mise, cargo-bin). Rust 2024 edition, ratatui-based.

## Build & Test

```sh
cargo check                   # type check
cargo clippy                  # lint
cargo test                    # unit + fixture integration tests
cargo test -- --ignored       # real-repo tests (local only)
cargo xtask check             # workspace check
cargo xtask test              # workspace test
cargo xtask clippy            # workspace clippy -D warnings
cargo xtask install           # install to ~/.cargo/bin
```

## Architecture

- `src/source.rs` — `CommandSource` trait (port) with 8 implementations
- `src/discover.rs` — xtask main.rs parser, used by `XtaskSource`
- `src/app.rs` — `App` struct owns all state, runs the event loop
- `src/ui.rs` — pure rendering functions, no state mutation
- `src/runner.rs` — async process spawning via tokio, streams stdout/stderr
- `src/pipeline.rs` — sequential command chaining state machine
- `src/search.rs` — output search with match cycling
- `src/history.rs` — JSON history + `.log` output files in `~/.config/xtui/`
- `src/status.rs` — git status via `Command::new("git")`
- `src/registry.rs` — project scanner/cache (not wired into UI yet)

## Testing

- Inline `#[cfg(test)]` modules in every source file
- `tests/common/mod.rs` — `ProjectFixture` builder for synthetic projects
- `tests/integration.rs` — cross-module discovery + `#[ignore]` real-repo tests
- `tests/sources.rs` — per-source integration tests with fixtures
- `proptest` used in `discover.rs` for parser fuzzing

## Key Conventions

- Sources return `Vec<SourceCommand>` — empty vec means "not applicable"
- Runner dispatches on `cmd.source` string to pick the right program/args
- Pipeline is a pure state machine — caller handles actual execution
- History caps at 50 entries, logs cap at 100 files per project
- `registry.rs` is `#[allow(dead_code)]` — reserved for future project picker
