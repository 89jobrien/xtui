# Active Context: xtui

## Current Focus

Branch: `develop`. Post-rc.1 state. CI pipeline restructured to develop→staging→main.
No active feature development. Working on infra/config hygiene.

## Recently Completed (since last memory-bank update)

- v0.4.5-rc.1 tagged on develop (`6d63169`)
- develop→staging→main CI promotion pipeline (`6a04351`)
- xtask commands added to CI: check/test/clippy/install/docs
- xbook/ restructured with configurable copies and glob support (`934e1ca`)
- `.github/.DS_Store` removed from git index (was tracked despite .gitignore entry)
- `xbook/**` added to `rail.toml` [change-detection] infrastructure paths (was triggering
  build+test on every book edit)

## Recent Decisions

- `ratatui 0.30` (maintained fork of tui-rs)
- `CommandSource` trait as port — each source is a separate impl
- OSC52 for clipboard instead of platform-specific crate
- Custom base64 to avoid adding a dependency for one function
- depview uses hexagonal ports (MetadataCache, MetadataFetcher) for testability
- Normal deps only in dep view (not dev/build deps) — krates filter
- CI uses `cargo rail run --profile ci --merge-base` — no explicit profile block needed,
  `--merge-base` flag is the change-detection mechanism

## Open Questions / Gaps

- `registry.rs` still dead code — project picker not wired to UI
- No config file support
- No multiple workspace support

## Dirty files (session 2026-06-09)

- `.gitignore` (modified)
- `Cargo.lock` (modified)
- `src/app.rs`, `src/bin_schema.rs`, `src/depview.rs`, `src/history.rs`,
  `src/meta_cache.rs`, `src/source.rs`, `src/ui.rs` (modified — uncommitted work)
- `xbook/SUMMARY.md`, `xbook/book.toml`, `xbook/xREADME.md` (modified)
- `xtask/Cargo.toml`, `xtask/src/main.rs` (modified)
