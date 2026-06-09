# Dep View Feature

Toggled with D key. Shows direct workspace dependencies with crates.io latest version
and versions-behind count, cached globally in redb.

## Ports/Adapters

- MetadataCache (port) → RedbCache (adapter, redb, TTL-based)
- MetadataFetcher (port) → HttpMetadataFetcher (adapter, ureq → crates.io)

## App integration

Fields added to App: show_dep_view, dep_infos, dep_scroll, dep_rx.
Methods: toggle_dep_view(), spawn_dep_fetch(), poll_dep_results().
Tokio tasks spawned per dep; cache hit/miss checked first.

## Rendering

draw_dep_view() in ui.rs renders full-screen panel when show_dep_view is true.
Color-coded by DepFetchState: Loading / Ready / Error.
j/k scroll bindings scoped to dep view.
Local/path deps detected and labeled separately (no crates.io fetch).

## Normal deps only

depview uses krates with filter to include only normal dependencies, not dev/build deps.

## Known mistake

Initial implementation (a2850e9) included dev/build deps and had broken full-screen
layout; fixed immediately in 80a0d34. Always visually verify dep view before marking done.
