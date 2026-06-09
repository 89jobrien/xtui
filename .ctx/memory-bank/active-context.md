# Active Context: xtui

## Current Focus

Post-depview, post-release-automation state. All 6 dep-view tasks complete. CI and
release workflows stable. No active development tasks.

## Recently Completed (since last memory-bank update)

- Dep view (v0.3.0): `depview.rs`, `meta_cache.rs`, `meta_fetch.rs` — full dep panel with
  crates.io metadata, TTL cache (redb), local/path dep detection, j/k scroll, color states
- CI workflows: `ci.yml` (fmt+clippy+nextest), `release.yml` (v* tag → GitHub release + publish)
- Release automation: pre-push hook auto-bumps patch, cargo-rail for minor/major
- Pre-push hook enforces +patch/+minor/+major labels in CI
- v0.4.0 / v0.4.1 tagged and released

## Recent Decisions

- `ratatui 0.30` (maintained fork of tui-rs)
- `CommandSource` trait as port — each source is a separate impl
- OSC52 for clipboard instead of platform-specific crate
- Custom base64 to avoid adding a dependency for one function
- depview uses hexagonal ports (MetadataCache, MetadataFetcher) for testability
- Normal deps only in dep view (not dev/build deps) — krates filter

## Open Questions / Gaps

- `registry.rs` still dead code — project picker not wired to UI
- No config file support
- No multiple workspace support

## Dirty files (session start 2026-06-09)

- `.ctx/HANDOFF.xtui.xtui.yaml` (modified)
- `CHANGELOG.md` (modified)
