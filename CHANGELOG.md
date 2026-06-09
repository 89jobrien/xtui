## [unreleased]

### Documentation

- *(xtui)* Add DEPLOYMENT.md with release process
- *(xtui)* Fix stale keybindings, nu-script->nu, add bin_schema to README
- *(xtui)* Add CLAUDE.md for Claude Code guidance
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
