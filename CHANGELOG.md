## [unreleased]

### Features

- _(xtui)_ Add cargo-bin source tab and args-input mode
- _(xtui)_ Add bin-schema cache and krates-backed cargo binary discovery

### Other

- _(xtui)_ Raise rustqual score from 46.5% to 95.7%

### Documentation

- _(xtui)_ Mark all plan docs as done for v0.1.0

## [0.1.0] - 2026-06-08

### Features

- _(xtui)_ Initial implementation of cargo-xtask TUI dashboard
- _(xtui)_ Add deps and module stubs for v1 plan
- _(xtui)_ Implement 6 core modules for v1
- _(xtui)_ Implement 6 remaining command sources
- _(xtui)_ Integrate all modules into app, UI, and runner
- _(xtui)_ Discover nested xtask subcommands and clean up dead code
- _(xtui)_ Handle bare match arms in dispatch functions

### Bug Fixes

- _(xtui)_ Resolve clippy collapsible_if warnings

### Testing

- _(xtui)_ Add end-to-end integration tests for v1 modules
- _(xtui)_ Add testing infrastructure and deepen coverage
