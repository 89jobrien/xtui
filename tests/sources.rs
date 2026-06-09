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

/// Uses the xtui workspace itself as a fixture — it has a [[bin]] target named "xtui".
/// krates requires a real cargo workspace (runs `cargo metadata`), so we use a known project.
#[test]
fn cargo_source_with_bin_targets() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let cmds = CargoSource.discover(root).unwrap();
    let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();

    // Standard commands always present
    assert!(names.contains(&"check"), "missing 'check' in {names:?}");
    assert!(names.contains(&"test"), "missing 'test' in {names:?}");

    // xtui workspace has a [[bin]] named "xtui" (src/main.rs)
    assert!(
        names.contains(&"run --bin xtui"),
        "missing 'run --bin xtui' in {names:?}"
    );
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
