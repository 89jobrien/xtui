# Tech Context: xtui

## Stack

- Rust 2024 edition, rust-version 1.91
- ratatui 0.30 (TUI framework, includes crossterm)
- tokio 1 (async runtime, full features)
- tokio-util 0.7 (LinesCodec for streaming process output)
- ansi-to-tui 8 (ANSI escape -> ratatui Text)
- regex 1 (help output parsing)
- futures-util 0.3 (StreamExt for framed reads)
- anyhow 1 (error handling)
- serde 1 + serde_json 1 (history serialization)
- toml 1 (Cargo.toml source parsing)
- dirs 6 (XDG config dir for history/logs)
- krates 0.17 (dep graph from Cargo metadata; used by depview)
- redb 2 (embedded KV store for metadata cache TTL)
- ureq 2 + json (HTTP fetcher for crates.io metadata)

## Dev Dependencies

- proptest 1 (fuzz testing for parsers)

## Build

```
cargo check               # type check
cargo clippy              # lint
cargo test                # unit + fixture integration tests
cargo test -- --ignored   # real-repo tests (local only)
cargo xtask check         # workspace check
cargo xtask test          # workspace test
cargo xtask clippy        # workspace clippy -D warnings
cargo xtask install       # install to ~/.cargo/bin
```

## CI
- `.github/workflows/ci.yml` — fmt + clippy + nextest on push to develop/staging/main + PR to main
- `.github/workflows/promote-staging.yml` — merges develop → staging on push to develop
- `.github/workflows/promote-main.yml` — merges staging → main on push to staging
- `.github/workflows/nightly.yml` — nightly release build
- `.github/workflows/release.yml` — fires on `v*` tag; git-cliff changelog,
  binary build, GitHub release, crates.io publish
- Branch pipeline: develop → staging → main (automated promotion)

## Release
- Patch: auto-bumped by `.githooks/pre-push` on every push to main
- Minor/major: `cargo rail release run xtui --bump minor --skip-publish`
  then `git push github main && git push github v0.X.0`
- Config: `.config/rail.toml`
- `xbook/**` classified as infrastructure (does not trigger build+test)
- `.githooks/`: pre-push, post-push, pre-commit, commit-msg, prepare-commit-msg

## Structure

```
src/lib.rs          pub re-exports, crate entry for tests
src/main.rs         CLI entry, workspace root discovery
src/source.rs       CommandSource trait + 8 source impls
src/discover.rs     xtask main.rs parser (XtaskSource backend)
src/bin_schema.rs   cargo-bin subcommand cache (JSON, mtime-invalidated)
src/app.rs          App state, event loop, keybindings, args-input mode
src/ui.rs           ratatui rendering (tabbed layout, ANSI output, dep view)
src/runner.rs       Process spawning, streaming output via mpsc
src/pipeline.rs     Sequential command chaining state machine
src/search.rs       Output search with match cycling
src/history.rs      JSON history + .log files in ~/.config/xtui/
src/status.rs       Git status via Command::new("git")
src/depview.rs      DepInfo, DepFetchState, collect_direct_deps via krates
src/meta_cache.rs   MetadataCache port + RedbCache adapter (redb, TTL)
src/meta_fetch.rs   MetadataFetcher port + HttpMetadataFetcher adapter (ureq)
src/registry.rs     Project scanner/cache (#[allow(dead_code)], not wired to UI)
xtask/src/main.rs   xtask commands: check, test, clippy, install
tests/common/mod.rs ProjectFixture builder
tests/integration.rs cross-module discovery + #[ignore] real-repo
tests/sources.rs    per-source integration tests with fixtures
```

## Constraints

- History caps at 50 entries; logs cap at 100 files per project
- Output buffer capped at 10k lines (drains oldest 1k on overflow)
- Channel buffer: 1024 lines
- Poll interval: 50ms
