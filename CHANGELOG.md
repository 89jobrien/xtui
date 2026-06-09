## [unreleased]

### Miscellaneous Tasks

- Automate minor/major releases via pre-push hook, enforce label in CI
- Restructure docs to xbook/ with configurable copies and glob support
- Develop→staging→main pipeline with nightly release and xtask commands

## [0.4.0] - 2026-06-09

### Documentation

- Document release process with cargo-rail, op plugin, tag fix

### Miscellaneous Tasks

- Fix release trigger to v\*, disable tag.yml, clean release docs
- Remove tag.yml — rail owns tagging
- _(release)_ Xtui v0.4.0

## [xtui-v0.3.0] - 2026-06-09

### Features

- _(xtui)_ Add depview domain types and collect_direct_deps
- _(xtui)_ Add MetadataCache port and RedbCache adapter
- _(xtui)_ Add MetadataFetcher port and HttpMetadataFetcher adapter
- _(xtui)_ Wire dep view background fetch into App
- _(xtui)_ Render dep view panel in ui
- _(depview)_ Detect and display local/path workspace deps

### Bug Fixes

- _(xtui)_ Dep view shows normal deps only; renders full-screen
- _(ci)_ Use open --raw for Cargo.toml in pre-push hook
- _(ci)_ Install just on CI runner for integration tests

### Documentation

- _(xtui)_ Add DEPLOYMENT.md with release process
- _(xtui)_ Fix stale keybindings, nu-script->nu, add bin_schema to README
- _(xtui)_ Add CLAUDE.md for Claude Code guidance
- _(xtui)_ Add dep view keybinding to README, ARCHITECTURE, CLAUDE.md, status bar

### Miscellaneous Tasks

- Add CI and release workflows with git-cliff changelog
- Fix release trigger to match cargo-rail tag format (xtui-v\*), drop publish job
- Auto-tag on Cargo.toml version bump, publish in release workflow
- Pre-push patch bump hook, simplified release tag workflow
- Pre-push hook labels +patch, +minor, +major correctly
- Update changelog
- Ignore .config/ dir
- _(release)_ Xtui v0.3.0

## [0.2.0] - 2026-06-09

### Miscellaneous Tasks

- _(xtui)_ Bump to 0.2.0

## [0.1.0] - 2026-06-09

### Features

- _(xtui)_ Initial implementation of cargo-xtask TUI dashboard
- _(xtui)_ Add deps and module stubs for v1 plan
- _(xtui)_ Implement 6 core modules for v1
- _(xtui)_ Implement 6 remaining command sources
- _(xtui)_ Integrate all modules into app, UI, and runner
- _(xtui)_ Discover nested xtask subcommands and clean up dead code
- _(xtui)_ Handle bare match arms in dispatch functions
- _(xtui)_ Add cargo-bin source tab and args-input mode
- _(xtui)_ Add bin-schema cache and krates-backed cargo binary discovery

### Bug Fixes

- _(xtui)_ Resolve clippy collapsible_if warnings
- _(xtui)_ Clippy allow for const_is_empty; improve MakeSource regex and description extraction

### Other

- _(xtui)_ Raise rustqual score from 46.5% to 95.7%

### Documentation

- _(xtui)_ Mark all plan docs as done for v0.1.0
- _(xtui)_ Add crates.io badge and cargo install section
- _(xtui)_ Add ARCHITECTURE.md and CONTRIBUTING.md from memory bank

### Testing

- _(xtui)_ Add end-to-end integration tests for v1 modules
- _(xtui)_ Add testing infrastructure and deepen coverage

### Miscellaneous Tasks

- _(xtui)_ Add crates.io metadata (repository, readme, keywords, categories)
