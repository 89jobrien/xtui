mod common;

use common::ProjectFixture;
use std::path::PathBuf;
use xtui::source::all_sources;

#[test]
fn all_sources_discover_fixture() {
    let fix = ProjectFixture::new()
        .with_cargo_toml("[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"")
        .with_justfile("build:\n  echo ok\ntest:\n  echo ok")
        .with_package_json(r#"{"scripts":{"dev":"echo","lint":"echo"}}"#)
        .with_makefile("build:\n\techo\nclean:\n\trm -f x")
        .with_mise_toml("[tasks.ci]\nrun = \"echo\"")
        .with_nu_script("check", "# check");

    let sources = all_sources();
    let found: Vec<&str> = sources
        .iter()
        .filter(|s| !s.discover(fix.path()).unwrap().is_empty())
        .map(|s| s.name())
        .collect();

    assert!(found.contains(&"cargo"), "missing cargo in {found:?}");
    assert!(found.contains(&"just"), "missing just in {found:?}");
    assert!(found.contains(&"npm"), "missing npm in {found:?}");
    assert!(found.contains(&"make"), "missing make in {found:?}");
    assert!(found.contains(&"mise"), "missing mise in {found:?}");
    assert!(found.contains(&"nu"), "missing nu in {found:?}");
    assert!(
        found.contains(&"cargo-bin"),
        "missing cargo-bin in {found:?}"
    );
    assert_eq!(found.len(), 7);
}

#[test]
fn mixed_project_some_sources_missing() {
    let fix = ProjectFixture::new()
        .with_cargo_toml("[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"")
        .with_justfile("build:\n  echo ok");

    let sources = all_sources();
    let found: Vec<&str> = sources
        .iter()
        .filter(|s| !s.discover(fix.path()).unwrap().is_empty())
        .map(|s| s.name())
        .collect();
    assert!(found.contains(&"cargo"), "missing cargo in {found:?}");
    assert!(found.contains(&"just"), "missing just in {found:?}");
    // cargo-bin is global — present whenever ~/.cargo/bin exists
    let has_cargo_bin = dirs::home_dir()
        .map(|h| h.join(".cargo/bin").is_dir())
        .unwrap_or(false);
    let project_count = 2;
    let expected = project_count + if has_cargo_bin { 1 } else { 0 };
    assert_eq!(found.len(), expected, "found: {found:?}");
}

#[test]
fn empty_project_only_global_sources() {
    // An "empty" project has no project-specific commands but the global
    // cargo-bin source may still contribute.
    let fix = ProjectFixture::new();
    let sources = all_sources();
    let found: Vec<&str> = sources
        .iter()
        .filter(|s| !s.discover(fix.path()).unwrap().is_empty())
        .map(|s| s.name())
        .collect();
    let project_sources = found.iter().filter(|&&n| n != "cargo-bin").count();
    assert_eq!(project_sources, 0, "unexpected project sources: {found:?}");
}

#[test]
#[ignore]
fn real_repo_xtui() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sources = all_sources();
    let names: Vec<&str> = sources
        .iter()
        .filter(|s| !s.discover(&root).unwrap().is_empty())
        .map(|s| s.name())
        .collect();
    assert!(names.contains(&"cargo"), "missing cargo in {names:?}");
    assert!(names.contains(&"xtask"), "missing xtask in {names:?}");
}

#[test]
#[ignore]
fn real_repo_minibox() {
    let home = std::env::var("HOME").unwrap_or_default();
    let minibox = PathBuf::from(home).join("dev/minibox");
    if !minibox.exists() {
        return;
    }
    let sources = all_sources();
    let names: Vec<&str> = sources
        .iter()
        .filter(|s| !s.discover(&minibox).unwrap().is_empty())
        .map(|s| s.name())
        .collect();
    assert!(names.contains(&"xtask"), "missing xtask in {names:?}");
}
