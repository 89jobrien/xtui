# Progress: xtui

## What Works (v0.4.10)

- Workspace root discovery (walks up looking for Cargo.lock)
- 8 command sources: xtask, cargo, just, nu-script, npm, make, mise, cargo-bin
- `CommandSource` trait (port) with per-source impls in `source.rs`
- TUI with tabbed source view, command list, streaming output pane
- Args-input mode — enter args before running a command
- Pipeline: sequential command chaining state machine (`pipeline.rs`)
- Output search with match cycling (`search.rs`)
- History: JSON entries + `.log` output files in `~/.config/xtui/` (`history.rs`)
- Git status display via `Command::new("git")` (`status.rs`)
- ANSI color rendering in output pane
- Process cancel, refresh, clipboard copy (OSC52)
- xtask crate with check/test/clippy/install commands
- Tests: unit tests in every source file, `ProjectFixture` builder for synthetics
- Integration tests: `tests/integration.rs` + `tests/sources.rs`
- Proptest fuzz tests for parser robustness
- Dep view (D key): direct deps + crates.io latest version, TTL cache, local dep detection
- CI: fmt + clippy + nextest on every push/PR to main; develop→staging→main promotion
- Release: patch auto-bump (with rc suffix promotion), minor/major via cargo-rail
- Dev-state: `.ctx/dev-state.json` session metadata (session_id, hash, crate versions,
  git state, mtime); `cargo xtask dev-state [--verify]`; pre-push gate

## In Progress

- Nothing active

## Not Started

- Multiple workspace support
- Config file
- `registry.rs` wired into UI (project picker — scaffold exists, dead code)
