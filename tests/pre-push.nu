#!/usr/bin/env nu

# tests/pre-push.nu — fixture-based integration tests for .githooks/pre-push
#
# Scenarios:
#   1. Unlabelled commit  → rail bumps patch, commit relabelled, push succeeds
#   2. Already-labelled, SHA matches remote → push proceeds, no re-bump
#   3. Already-labelled, SHA diverged from remote → force-push reconciliation

const WORKSPACE   = "/Users/joe/dev/xtui"
const FIXTURE_SRC = "/Users/joe/dev/xtui/tests/fixtures/pre-push"
const TMP_BASE    = "/tmp/xtui-pre-push-test"

mut results = []

def check [label: string, cond: bool]: nothing -> record {
    let status = if $cond { "PASS" } else { "FAIL" }
    print $"  ($status)  ($label)"
    { label: $label, pass: $cond }
}

# ---------------------------------------------------------------------------
# Build the fixture xtask binary once (shared across all scenarios)
# ---------------------------------------------------------------------------
print "\n=== Build fixture xtask ==="
mkdir $"($TMP_BASE)/bin"

let build = (do { cargo build --manifest-path $"($FIXTURE_SRC)/xtask/Cargo.toml" --target-dir $"($TMP_BASE)/target" --quiet } | complete)
if $build.exit_code != 0 {
    print "ERROR: fixture xtask build failed — aborting"
    print $build.stderr
    exit 1
}

let xtask_bin = $"($TMP_BASE)/bin/xtask"
cp $"($TMP_BASE)/target/debug/xtask" $xtask_bin
print $"  xtask binary → ($xtask_bin)"

# ---------------------------------------------------------------------------
# Helper: spin up a fresh fixture repo + bare remote
# ---------------------------------------------------------------------------
def make_fixture [name: string]: nothing -> string {
    let dir    = $"($TMP_BASE)/($name)"
    let remote = $"($TMP_BASE)/($name)-remote"

    rm -rf $dir $remote
    git init --bare -q $remote

    mkdir $"($dir)/scripts"
    mkdir $"($dir)/.githooks"
    mkdir $"($dir)/.config"
    mkdir $"($dir)/.cargo"
    mkdir $"($dir)/.ctx"
    mkdir $"($dir)/src"
    mkdir $"($dir)/xtask/src"

    # Copy live sources — tests always reflect the current hook and xstate script
    cp $"($WORKSPACE)/scripts/xstate.nu"     $"($dir)/scripts/xstate.nu"
    cp $"($WORKSPACE)/.githooks/pre-push"    $"($dir)/.githooks/pre-push"
    cp $"($WORKSPACE)/.config/rail.toml"     $"($dir)/.config/rail.toml"
    cp $"($FIXTURE_SRC)/.cargo/config.toml"  $"($dir)/.cargo/config.toml"
    cp $"($FIXTURE_SRC)/xtask/Cargo.toml"    $"($dir)/xtask/Cargo.toml"
    cp $"($FIXTURE_SRC)/xtask/src/main.rs"   $"($dir)/xtask/src/main.rs"

    # Single-crate Cargo.toml (no workspace) — matches what xstate.nu and rail expect
    cp $"($FIXTURE_SRC)/Cargo.toml" $"($dir)/Cargo.toml"
    "fn main() {}" | save $"($dir)/src/main.rs"

    git -C $dir init -q
    git -C $dir config core.hooksPath .githooks
    git -C $dir remote add origin $remote

    $dir
}

def do_push [dir: string, branch: string = "main"]: nothing -> record {
    do { ^git -C $dir push origin $branch } | complete
}

def read_app_version [dir: string]: nothing -> string {
    open --raw $"($dir)/Cargo.toml"
    | lines
    | where { |l| $l | str starts-with "version = \"" }
    | first
    | str replace --regex '^version = "(.+)"$' '$1'
}

# ---------------------------------------------------------------------------
# Scenario 1: unlabelled commit → rail patch bump applied
# ---------------------------------------------------------------------------
print "\n=== Scenario 1: unlabelled commit → rail patch bump ==="

let s1 = (make_fixture "s1")
git -C $s1 add . | ignore
git -C $s1 commit -q -m "chore: init\n\n(+patch 0.1.0)" | ignore
(do_push $s1) | ignore

"fn main() { println!(\"s1\"); }" | save --force $"($s1)/src/main.rs"
git -C $s1 add . | ignore
git -C $s1 commit -q -m "feat: add greeting" | ignore

let s1_before_sha = (git -C $s1 rev-parse --short HEAD | str trim)
let s1_result     = (do_push $s1)
let s1_after_sha  = (git -C $s1 rev-parse --short HEAD | str trim)
let s1_after_msg  = (git -C $s1 log -1 --format=%B | str trim)
let s1_after_ver  = (read_app_version $s1)

print $"  debug s1: exit=($s1_result.exit_code) sha=($s1_after_sha) ver=($s1_after_ver)"
print $"  debug s1: msg=($s1_after_msg | str substring 0..60)"
$results = ($results | append (check "s1: push exit 0"               ($s1_result.exit_code == 0)))
$results = ($results | append (check "s1: SHA changed (rail commit)"  ($s1_after_sha != $s1_before_sha)))
$results = ($results | append (check "s1: commit labelled"            (($s1_after_msg | str contains "(+patch ") or ($s1_after_msg | str contains "chore(release)"))))
$results = ($results | append (check "s1: version bumped to 0.1.1"   ($s1_after_ver == "0.1.1")))

# ---------------------------------------------------------------------------
# Scenario 2: already-labelled commit, remote in sync → no re-bump
# ---------------------------------------------------------------------------
print "\n=== Scenario 2: already-labelled, remote in sync → no re-bump ==="

let s2 = (make_fixture "s2")
git -C $s2 add . | ignore
git -C $s2 commit -q -m "chore: init\n\n(+patch 0.1.0)" | ignore
(do_push $s2) | ignore

"fn main() { println!(\"s2\"); }" | save --force $"($s2)/src/main.rs"
git -C $s2 add . | ignore
git -C $s2 commit -q -m "feat: add s2" | ignore
(do_push $s2) | ignore   # first push: rail bumps, remote now == local

let s2_sha_synced = (git -C $s2 rev-parse --short HEAD | str trim)
let s2_remote_sha = (
    git -C $"($TMP_BASE)/s2-remote" rev-parse --short HEAD | str trim
)

print $"  debug s2: local=($s2_sha_synced) remote=($s2_remote_sha)"
let s2_local_log  = (git -C $s2 log --oneline -3 | str trim | lines | str join " | ")
let s2_remote_log = (git -C $"($TMP_BASE)/s2-remote" log --oneline -3 | str trim | lines | str join " | ")
print $"  debug s2 local log:  ($s2_local_log)"
print $"  debug s2 remote log: ($s2_remote_log)"
$results = ($results | append (check "s2: remote == local after first push" ($s2_sha_synced == $s2_remote_sha)))

let s2_result2        = (do_push $s2)
let s2_sha_after_push = (git -C $s2 rev-parse --short HEAD | str trim)

$results = ($results | append (check "s2: second push exit 0" ($s2_result2.exit_code == 0)))
$results = ($results | append (check "s2: SHA unchanged"       ($s2_sha_after_push == $s2_sha_synced)))

# ---------------------------------------------------------------------------
# Scenario 3: already-labelled, SHA diverged from remote → force-push
# ---------------------------------------------------------------------------
print "\n=== Scenario 3: already-labelled, SHA diverged → force-push reconciliation ==="

let s3 = (make_fixture "s3")
git -C $s3 add . | ignore
git -C $s3 commit -q -m "chore: init\n\n(+patch 0.1.0)" | ignore
(do_push $s3) | ignore

"fn main() { println!(\"s3\"); }" | save --force $"($s3)/src/main.rs"
git -C $s3 add . | ignore
git -C $s3 commit -q -m "feat: add s3" | ignore

# Push unamended commit to remote directly (bypassing hook)
do { git -C $s3 push origin main --no-verify } | complete | ignore

# Amend locally — simulates the hook having already relabelled on a previous run
git -C $s3 commit --amend -q -m "feat: add s3\n\n(+patch 0.1.1)" | ignore
let s3_amended_sha = (git -C $s3 rev-parse --short HEAD | str trim)

# Write xstate.json reflecting the amended local state as source of truth
{
    commit: $s3_amended_sha,
    branch: "main",
    version: "0.1.1",
    last_pushed_version: "0.1.0",
    session_id: "test",
    session_hash: "test",
    timestamp: "2026-01-01T00:00:00+0000",
    claude_session_pid: 0,
    project: "app",
    workspace_root: $s3,
    hostname: "test",
    username: "test",
    commit_msg: "feat: add s3",
    dirty: false,
    dirty_files: [],
    crate_versions: { app: "0.1.1" },
    mtime: 0,
} | to json --indent 2 | save --force $"($s3)/.ctx/xstate.json"

let s3_result     = (do_push $s3)
print $"  debug s3: exit=($s3_result.exit_code)"
print $"  debug s3: stderr=($s3_result.stderr | str substring 0..200)"
let s3_remote_sha = (
    git -C $"($TMP_BASE)/s3-remote" rev-parse --short HEAD | str trim
)

$results = ($results | append (check "s3: push exit 0"                   ($s3_result.exit_code == 0)))
$results = ($results | append (check "s3: remote reconciled to local SHA" ($s3_remote_sha == $s3_amended_sha)))

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
let passed = ($results | where pass == true  | length)
let failed = ($results | where pass == false | length)

print $"\n--- Results: ($passed) passed, ($failed) failed ---"
if $failed > 0 { exit 1 }
