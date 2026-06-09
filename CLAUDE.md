# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- `src/bin_schema.rs` — cargo-bin subcommand cache (JSON, mtime-invalidated)
- `src/app.rs` — `App` struct owns all state, runs the event loop
- `src/ui.rs` — pure rendering functions, no state mutation
- `src/runner.rs` — async process spawning via tokio, streams stdout/stderr
- `src/pipeline.rs` — sequential command chaining state machine
- `src/search.rs` — output search with match cycling
- `src/history.rs` — JSON history + `.log` output files in `~/.config/xtui/`
- `src/status.rs` — git status via `Command::new("git")`
- `src/depview.rs` — dep graph domain types; `collect_direct_deps` via krates (normal deps only)
- `src/meta_cache.rs` — `MetadataCache` port + `RedbCache` adapter (redb, TTL-based)
- `src/meta_fetch.rs` — `MetadataFetcher` port + `HttpMetadataFetcher` adapter (ureq)
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
- Use `chore` or `refactor` commit types for quality passes — not `quality` (git-cliff drops unknown types)

## Release

Releases are managed via `cargo-rail` + CI. Config: `.config/rail.toml`.

### Patch bump (automated)

Every push to `main` triggers `.githooks/pre-push`, which:
- Bumps `Cargo.toml` patch version (`0.2.1` → `0.2.2`)
- Amends the outgoing commit, appending `(+patch 0.2.2)` to the message body
- No tag is created — patch releases are informational only

### Minor / major release

```nu
# 1. Run cargo-rail to bump version, generate changelog, commit, and create tag
cargo rail release run xtui --bump minor   # or --bump major

# 2. cargo-rail creates tag as bare v0.3.0 (known bug — tag_format ignored)
#    Delete wrong tag and create correct one:
git tag -d v0.3.0
git tag xtui-v0.3.0

# 3. Publish to crates.io via 1Password plugin (must be run in nu):
op plugin run -- cargo publish

# 4. Push commit and tag:
git push github main
git push github xtui-v0.3.0
```

Pushing `xtui-v*` tag triggers `release.yml` → git-cliff release notes + binary attached
to GitHub release.

### Known cargo-rail issue

`tag_format = "{crate}-{prefix}{version}"` in `.config/rail.toml` is ignored — rail
always creates bare `v{version}` tags. Manually delete and recreate as `xtui-v{version}`
after every rail release run.

### CI workflows

- `.github/workflows/ci.yml` — fmt + clippy + nextest on every push/PR to `main`
- `.github/workflows/tag.yml` — creates `xtui-v{version}` tag when `Cargo.toml` version
  has patch == 0 (minor/major releases pushed without rail)
- `.github/workflows/release.yml` — fires on `xtui-v*` tag; generates changelog,
  builds binary, creates GitHub release
