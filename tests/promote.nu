#!/usr/bin/env nu

# tests/promote.nu — fixture-based integration tests for scripts/ci-promote.nu
#
# Scenarios:
#   1. Clean develop ahead of main → promotes, rail bumps patch, tag created
#   2. Develop == main → no-op, exits 0
#   3. Main ahead of develop (diverged) → blocked, exits non-zero
#   4. --dry-run → prints plan, makes no changes

const WORKSPACE   = "/Users/joe/dev/xtui"
const FIXTURE_SRC = "/Users/joe/dev/xtui/tests/fixtures/pre-push"
const TMP_BASE    = "/tmp/xtui-promote-test"

mut results = []

def check [label: string, cond: bool]: nothing -> record {
    let status = if $cond { "PASS" } else { "FAIL" }
    print $"  ($status)  ($label)"
    { label: $label, pass: $cond }
}

# ---------------------------------------------------------------------------
# Helper: set up a fixture repo with a bare remote, develop and main branches
# ---------------------------------------------------------------------------
def make_fixture [name: string]: nothing -> record {
    let dir    = $"($TMP_BASE)/($name)"
    let remote = $"($TMP_BASE)/($name)-remote"

    rm -rf $dir $remote
    git init --bare -q $remote

    mkdir $"($dir)/scripts"
    mkdir $"($dir)/src"

    cp $"($WORKSPACE)/scripts/ci-promote.nu" $"($dir)/scripts/ci-promote.nu"
    mkdir $"($dir)/.config"
    cp $"($WORKSPACE)/.config/rail.toml"    $"($dir)/.config/rail.toml"

    # Minimal standalone Cargo.toml — no workspace members needed for promote
    "[workspace]\n\n[package]\nname = \"xtui\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
    | save $"($dir)/Cargo.toml"
    "fn main() {}" | save $"($dir)/src/main.rs"

    git -C $dir init -q
    git -C $dir remote add origin $remote
    git -C $dir config user.name  "test"
    git -C $dir config user.email "test@test.com"
    git -C $dir config core.hooksPath /dev/null  # no hooks during fixture setup

    # Initial commit on main
    git -C $dir add . | ignore
    git -C $dir commit -q -m "chore: init v0.1.0" | ignore
    git -C $dir push -q origin $"HEAD:main" | ignore

    # develop starts from main
    git -C $dir push -q origin $"HEAD:develop" | ignore
    git -C $dir checkout -q -b develop | ignore
    git -C $dir branch -q --set-upstream-to origin/develop | ignore

    { dir: $dir, remote: $remote }
}

def run_promote [dir: string, flags: list<string> = []]: nothing -> record {
    do { nu $"($dir)/scripts/ci-promote.nu" --remote origin --repo-root $dir ...$flags } | complete
}

def remote_version [remote: string]: nothing -> string {
    # Clone bare remote into temp dir to read Cargo.toml
    let tmp = $"($TMP_BASE)/read-tmp"
    rm -rf $tmp
    git clone -q $remote $tmp
    let ver = (
        open --raw $"($tmp)/Cargo.toml"
        | lines
        | where { |l| $l | str starts-with "version = \"" }
        | first
        | str replace --regex '^version = "(.+)"$' '$1'
    )
    rm -rf $tmp
    $ver
}

def remote_tags [remote: string]: nothing -> list<string> {
    git -C $remote tag | lines | where { |l| ($l | str trim | is-not-empty) }
}

def remote_log [remote: string, n: int = 3]: nothing -> string {
    git -C $remote log --oneline $"-($n)" | str trim
}

# ---------------------------------------------------------------------------
# Scenario 1: develop ahead of main → promote + patch bump + tag
# ---------------------------------------------------------------------------
print "\n=== Scenario 1: develop ahead of main → promote ==="

let s1 = (make_fixture "s1")

# Add a commit to develop
"fn main() { println!(\"s1\"); }" | save --force $"($s1.dir)/src/main.rs"
git -C $s1.dir add . | ignore
git -C $s1.dir commit -q --no-verify -m "feat: add s1" | ignore
git -C $s1.dir push -q --no-verify origin develop | ignore

let s1_develop_sha = (git -C $s1.remote rev-parse --short develop | str trim)
let s1_result      = (run_promote $s1.dir)
let s1_main_sha    = (git -C $s1.remote rev-parse --short main | str trim)
let s1_tags        = (remote_tags $s1.remote)
let s1_version     = (remote_version $s1.remote)
let s1_log         = (remote_log $s1.remote)


$results = ($results | append (check "s1: promote exit 0"          ($s1_result.exit_code == 0)))
$results = ($results | append (check "s1: version bumped to 0.1.1" ($s1_version == "0.1.1")))
$results = ($results | append (check "s1: tag v0.1.1 created"      ($s1_tags | any { |t| $t == "v0.1.1" })))
$results = ($results | append (check "s1: main has release commit"  ($s1_log | str contains "chore(release)")))

# ---------------------------------------------------------------------------
# Scenario 2: develop == main → no-op
# ---------------------------------------------------------------------------
print "\n=== Scenario 2: develop == main → no-op ==="

let s2 = (make_fixture "s2")

let s2_main_before = (git -C $s2.remote rev-parse --short main | str trim)
let s2_result      = (run_promote $s2.dir)
let s2_main_after  = (git -C $s2.remote rev-parse --short main | str trim)

$results = ($results | append (check "s2: exit 0"          ($s2_result.exit_code == 0)))
$results = ($results | append (check "s2: main unchanged"  ($s2_main_before == $s2_main_after)))

# ---------------------------------------------------------------------------
# Scenario 3: main ahead of develop → blocked
# ---------------------------------------------------------------------------
print "\n=== Scenario 3: main ahead of develop (diverged) → blocked ==="

let s3 = (make_fixture "s3")

# Advance main independently (simulate a direct commit to main)
let s3_tmp = $"($TMP_BASE)/s3-main-work"
git clone -q $s3.remote $s3_tmp
git -C $s3_tmp config user.name "test" | ignore
git -C $s3_tmp config user.email "test@test.com" | ignore
git -C $s3_tmp checkout -q main | ignore
"fn main() { println!(\"mainonly\"); }" | save --force $"($s3_tmp)/src/main.rs"
git -C $s3_tmp config core.hooksPath /dev/null | ignore
git -C $s3_tmp add . | ignore
git -C $s3_tmp commit -q --no-verify -m "fix: main-only hotfix" | ignore
git -C $s3_tmp push -q --no-verify origin main | ignore
rm -rf $s3_tmp

# Add commit to develop too (creating divergence)
"fn main() { println!(\"devonly\"); }" | save --force $"($s3.dir)/src/main.rs"
git -C $s3.dir add . | ignore
git -C $s3.dir commit -q --no-verify -m "feat: dev-only" | ignore
git -C $s3.dir push -q --no-verify origin develop | ignore

let s3_result = (run_promote $s3.dir)

$results = ($results | append (check "s3: blocked (exit non-zero)" ($s3_result.exit_code != 0)))
$results = ($results | append (check "s3: error mentions diverged"  (($s3_result.stdout + $s3_result.stderr) | str contains "diverged")))

# ---------------------------------------------------------------------------
# Scenario 4: --dry-run → no changes
# ---------------------------------------------------------------------------
print "\n=== Scenario 4: --dry-run → no changes ==="

let s4 = (make_fixture "s4")

"fn main() { println!(\"s4\"); }" | save --force $"($s4.dir)/src/main.rs"
git -C $s4.dir add . | ignore
git -C $s4.dir commit -q --no-verify -m "feat: add s4" | ignore
git -C $s4.dir push -q --no-verify origin develop | ignore

let s4_main_before = (git -C $s4.remote rev-parse --short main | str trim)
let s4_result      = (run_promote $s4.dir ["--dry-run"])
let s4_main_after  = (git -C $s4.remote rev-parse --short main | str trim)

$results = ($results | append (check "s4: dry-run exit 0"       ($s4_result.exit_code == 0)))
$results = ($results | append (check "s4: main unchanged"        ($s4_main_before == $s4_main_after)))
$results = ($results | append (check "s4: prints dry-run plan"   (($s4_result.stdout + $s4_result.stderr) | str contains "[dry-run]")))

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
let passed = ($results | where pass == true  | length)
let failed = ($results | where pass == false | length)

print $"\n--- Results: ($passed) passed, ($failed) failed ---"
if $failed > 0 { exit 1 }
