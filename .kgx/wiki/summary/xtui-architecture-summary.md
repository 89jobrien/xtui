# xtui Architecture Summary

xtui is a lib+binary Rust crate (v0.4.1) that discovers and runs project commands from
8 sources in a ratatui TUI. Entry flow: CLI args → find_workspace_root → all_sources() →
App::new → App::run.

## Core traits

- **CommandSource** (source.rs): port trait with 8 impls — XtaskSource, CargoSource,
  JustSource, NuScriptSource, NpmSource, MakeSource, MiseSource, CargoBinSource.
  Returns Vec<SourceCommand>; empty vec = not applicable.
- **MetadataCache** (meta_cache.rs): port for dep metadata caching. Impl: RedbCache
  (redb KV store, TTL-based).
- **MetadataFetcher** (meta_fetch.rs): port for crates.io metadata fetch. Impl:
  HttpMetadataFetcher (ureq).

## Key components

- **App** (app.rs): owns all state, event loop, keybindings, dep view toggle.
- **ui.rs**: pure rendering, no state mutation. Draws tab bar, 30/70 split, dep view panel.
- **runner.rs**: async process spawning via tokio + LinesCodec streaming.
- **pipeline.rs**: pure state machine — Idle → Running(idx) → Done | Failed(idx).
- **bin_schema.rs**: lazy per-binary subcommand discovery, mtime-invalidated JSON cache.
- **depview.rs**: collect_direct_deps via krates (normal deps only).
- **registry.rs**: project scanner stub, #[allow(dead_code)], not wired to UI.

## Dependencies

ratatui 0.30, tokio 1, redb 2, ureq 2, krates 0.17, ansi-to-tui 8, proptest (dev).

## Constraints

10k output line buffer, 50ms poll, 50 history entries/project, 100 log files/project.
