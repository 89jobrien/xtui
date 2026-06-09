# Deployment

## Prerequisites

- 1Password CLI (`op`) authenticated
- GitHub PAT in 1Password item `bnzbcwrhetc6kzywmsnkpj2qja`, field `GITHUB_TOKEN`
- `cargo` logged in to crates.io (`cargo login`)
- `nu` available on PATH

## Release Process

### 1. Commit all changes

Ensure the working tree is clean and all changes are committed and pushed to `main`.

### 2. Bump the version

Update `version` in `Cargo.toml`:

```sh
# Edit Cargo.toml: version = "x.y.z"
git add Cargo.toml
git commit -m "chore(xtui): bump to x.y.z"
```

### 3. Tag and push

```sh
git tag vx.y.z
git push github main
git push github vx.y.z
```

### 4. Create the GitHub release

Write a release script to `/tmp/gh-release.nu`:

````nu
#!/usr/bin/env nu

let token = (op item get bnzbcwrhetc6kzywmsnkpj2qja --account my.1password.com --field GITHUB_TOKEN --reveal)

let body = {
    tag_name: "vx.y.z",
    name: "vx.y.z",
    body: "## What's new\n\n- item one\n- item two\n\n## Install\n\n```sh\ncargo install xtui\n```",
    draft: false,
    prerelease: false
}

let result = (http post
    --content-type application/json
    --headers [Authorization $"token ($token)"]
    https://api.github.com/repos/89jobrien/xtui/releases
    $body)

print $result.html_url
````

Run it:

```sh
nu /tmp/gh-release.nu
```

### 5. Publish to crates.io

```sh
cargo publish --allow-dirty
```

The `--allow-dirty` flag is needed if untracked files (e.g. `AGENTS.md`, `test.md`) are
present in the working tree. They are not included in the published crate.

## Notes

- Do not use `gh release create` — it requires interactive IO that is unavailable in this
  environment. Use the GitHub API via the nu script above.
- The `v0.1.0` release was created automatically by an earlier `gh` attempt that returned
  an error but succeeded silently. Verify releases exist with:
  ```sh
  curl -s -H "Authorization: token $TOKEN" \
    https://api.github.com/repos/89jobrien/xtui/releases | jq '[.[] | {name, tag_name}]'
  ```
- crates.io requires waiting ~30 seconds after publish before the new version is
  discoverable via `cargo install`.
