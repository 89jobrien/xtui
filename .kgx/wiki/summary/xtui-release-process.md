# xtui Release Process

## Patch releases (automated)

Every push to main triggers .githooks/pre-push which:
1. Bumps Cargo.toml patch version (0.4.0 → 0.4.1)
2. Amends the outgoing commit, appending (+patch 0.4.1) to the message body
3. No tag created

## Minor / major releases

```
cargo rail release run xtui --bump minor --skip-publish
git push github main
git push github v0.X.0
```

Pushing the v* tag triggers release.yml which:
- Generates changelog via git-cliff
- Builds binary
- Creates GitHub release
- Publishes to crates.io via CARGO_REGISTRY_TOKEN secret

## CI

- ci.yml: fmt + clippy + nextest on every push/PR to main
- release.yml: fires on v* tag

## Config

cargo-rail config at .config/rail.toml.

## Known pitfalls

- Use chore/refactor commit types — git-cliff drops unknown types like 'quality'
- Verify cargo-rail tag format with --dry-run before wiring release.yml trigger
- Ensure runtime deps (just, nu, mise) are installed in CI runner for integration tests
