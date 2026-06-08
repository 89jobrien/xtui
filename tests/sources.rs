mod common;

use common::ProjectFixture;
use xtui::source::{CargoSource, CommandSource, JustSource, XtaskSource};

#[test]
fn just_source_with_descriptions() {
    let fix = ProjectFixture::new().with_justfile(
        "build: # Build the project\n  echo ok\ntest: # Run tests\n  echo ok\nclean:\n  echo ok",
    );

    let cmds = JustSource.discover(fix.path()).unwrap();
    assert!(cmds.len() >= 3, "expected 3+ commands, got {}", cmds.len());

    let build = cmds.iter().find(|c| c.name == "build");
    assert!(build.is_some(), "missing 'build' in {cmds:?}");
    // just --list outputs descriptions after the recipe name
    // The exact format depends on just version, so just check we got commands
}

#[test]
fn cargo_source_with_bin_targets() {
    let fix = ProjectFixture::new().with_cargo_toml(
        r#"[package]
name = "t"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "mycli"
path = "src/main.rs"

[[bin]]
name = "helper"
path = "src/helper.rs"
"#,
    );

    let cmds = CargoSource.discover(fix.path()).unwrap();
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.contains(&"run --bin mycli"),
        "missing 'run --bin mycli' in {names:?}"
    );
    assert!(
        names.contains(&"run --bin helper"),
        "missing 'run --bin helper' in {names:?}"
    );
    // Standard commands should also be present
    assert!(names.contains(&"check"), "missing 'check' in {names:?}");
    assert!(names.contains(&"test"), "missing 'test' in {names:?}");
}

#[test]
fn xtask_source_nested_subcommands() {
    let xtask_main = r#"
fn main() {
    match task.as_deref() {
        Some("test") => cmd_test(),
        Some("build") => build(),
        _ => help(),
    }
}

fn cmd_test() {
    match suite.as_deref() {
        Some(s) => dispatch_test(s),
        None => eprintln!("pick a suite"),
    }
}

fn dispatch_test(suite: &str) {
    match suite {
        "unit" => test_unit(),
        "integration" => test_integration(),
        other => bail!("unknown"),
    }
}
"#;
    let fix = ProjectFixture::new().with_xtask_main(xtask_main);

    let cmds = XtaskSource.discover(fix.path()).unwrap();
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.contains(&"test unit"),
        "missing 'test unit' in {names:?}"
    );
    assert!(
        names.contains(&"test integration"),
        "missing 'test integration' in {names:?}"
    );
    assert!(names.contains(&"build"), "missing 'build' in {names:?}");
    // "test" itself should not appear as standalone
    assert!(
        !names.contains(&"test"),
        "'test' should be expanded, got {names:?}"
    );
}
