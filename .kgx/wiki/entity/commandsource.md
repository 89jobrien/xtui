# CommandSource Trait

Port trait defined in source.rs. Enables discovery of runnable commands from any backend.

```rust
pub trait CommandSource: Send + Sync {
    fn name(&self) -> &str;
    fn discover(&self, project: &Path) -> Result<Vec<SourceCommand>>;
}
```

## Contract

- Returns empty Vec when source is not applicable — never errors for 'not found'
- all_sources() returns 8 implementations in fixed tab order

## Implementations

| Impl | Detection | Runner |
|---|---|---|
| XtaskSource | xtask/src/main.rs | cargo run -p xtask -- |
| CargoSource | Cargo.toml | cargo |
| JustSource | Justfile/justfile | just |
| NuScriptSource | scripts/*.nu | nu scripts/<name>.nu |
| NpmSource | package.json scripts | npm run |
| MakeSource | Makefile | make |
| MiseSource | .mise.toml | mise run |
| CargoBinSource | ~/.cargo/bin/ | <binary> [subcmd] |

## Related

- XtaskSource uses discover.rs for xtask main.rs parsing
- CargoBinSource uses bin_schema.rs for lazy subcommand discovery
- App calls all_sources() to build the tab list
