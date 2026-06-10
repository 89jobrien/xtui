## [unreleased]

### Miscellaneous Tasks

- *(release)* Xtui v0.4.5-rc.1
## [nightly] - 2026-06-09

### Miscellaneous Tasks

- Automate minor/major releases via pre-push hook, enforce label in CI
- Automate minor/major releases via pre-push hook, enforce label in CI
- Restructure docs to xbook/ with configurable copies and glob support
- Develop→staging→main pipeline with nightly release and xtask commands
## [0.4.0] - 2026-06-09

### Documentation

- Document release process with cargo-rail, op plugin, tag fix

### Miscellaneous Tasks

- Fix release trigger to v*, disable tag.yml, clean release docs
- Remove tag.yml — rail owns tagging
- *(release)* Xtui v0.4.0
## [xtui-v0.3.0] - 2026-06-09

### Features

- *(xtui)* Add depview domain types and collect_direct_deps
- *(xtui)* Add MetadataCache port and RedbCache adapter
- *(xtui)* Add MetadataFetcher port and HttpMetadataFetcher adapter
- *(xtui)* Wire dep view background fetch into App
- *(xtui)* Render dep view panel in ui
- *(depview)* Detect and display local/path workspace deps

### Bug Fixes

- *(xtui)* Dep view shows normal deps only; renders full-screen
- *(ci)* Use open --raw for Cargo.toml in pre-push hook
- *(ci)* Install just on CI runner for integration tests

### Documentation

- *(xtui)* Add DEPLOYMENT.md with release process
- *(xtui)* Fix stale keybindings, nu-script->nu, add bin_schema to README
- *(xtui)* Add CLAUDE.md for Claude Code guidance
- *(xtui)* Add dep view keybinding to README, ARCHITECTURE, CLAUDE.md, status bar

### Miscellaneous Tasks

- Add CI and release workflows with git-cliff changelog
- Fix release trigger to match cargo-rail tag format (xtui-v*), drop publish job
- Auto-tag on Cargo.toml version bump, publish in release workflow
- Pre-push patch bump hook, simplified release tag workflow
- Pre-push hook labels +patch, +minor, +major correctly
- Update changelog
- Ignore .config/ dir
- *(release)* Xtui v0.3.0
## [0.2.0] - 2026-06-09

### Miscellaneous Tasks

- *(xtui)* Bump to 0.2.0
## [0.1.0] - 2026-06-09

### Features

- *(xtui)* Initial implementation of cargo-xtask TUI dashboard
- *(xtui)* Add deps and module stubs for v1 plan
- *(xtui)* Implement 6 core modules for v1
- *(xtui)* Implement 6 remaining command sources
- *(xtui)* Integrate all modules into app, UI, and runner
- *(xtui)* Discover nested xtask subcommands and clean up dead code
- *(xtui)* Handle bare match arms in dispatch functions
- *(xtui)* Add cargo-bin source tab and args-input mode
- *(xtui)* Add bin-schema cache and krates-backed cargo binary discovery

### Bug Fixes

- *(xtui)* Resolve clippy collapsible_if warnings
- *(xtui)* Clippy allow for const_is_empty; improve MakeSource regex and description extraction

### Other

- *(xtui)* Raise rustqual score from 46.5% to 95.7%

### Documentation

- *(xtui)* Mark all plan docs as done for v0.1.0
- *(xtui)* Add crates.io badge and cargo install section
- *(xtui)* Add ARCHITECTURE.md and CONTRIBUTING.md from memory bank

### Testing

- *(xtui)* Add end-to-end integration tests for v1 modules
- *(xtui)* Add testing infrastructure and deepen coverage

### Miscellaneous Tasks

- *(xtui)* Add crates.io metadata (repository, readme, keywords, categories)
