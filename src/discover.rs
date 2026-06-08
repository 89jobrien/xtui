use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct XtaskCommand {
    pub name: String,
    pub description: Option<String>,
}

pub fn parse_source(source: &str) -> Vec<XtaskCommand> {
    let desc_map = extract_descriptions(source);

    // Find all functions and their match arms with Some("name") patterns.
    // We detect which top-level commands delegate to sub-dispatch functions
    // by looking for functions called from the main match (e.g. cmd_test)
    // that themselves contain match blocks with Some("subcommand").
    // Match both Some("name") and bare "name" => in match arms
    let some_re = Regex::new(r#"Some\("([^"]+)"\)"#).unwrap();
    let bare_re = Regex::new(r#"(?m)^\s+"([a-z][a-z0-9_-]*)""#).unwrap();

    // Split source into function blocks (fn name ... { ... })
    let fn_re = Regex::new(r"(?m)^fn\s+(\w+)").unwrap();
    let fn_starts: Vec<(usize, &str)> = fn_re
        .captures_iter(source)
        .map(|c| (c.get(0).unwrap().start(), c.get(1).unwrap().as_str()))
        .collect();

    // Build map: function_name -> list of match arm string values in that function
    let mut fn_commands: HashMap<&str, Vec<String>> = HashMap::new();
    for (i, &(start, name)) in fn_starts.iter().enumerate() {
        let end = fn_starts
            .get(i + 1)
            .map(|(s, _)| *s)
            .unwrap_or(source.len());
        let body = &source[start..end];
        // Collect Some("...") patterns
        let mut cmds: Vec<String> = some_re
            .captures_iter(body)
            .map(|c| c[1].to_string())
            .collect();
        // For dispatch_ functions, also collect bare "name" => patterns
        if name.starts_with("dispatch_") && cmds.is_empty() {
            cmds = bare_re
                .captures_iter(body)
                .map(|c| c[1].to_string())
                .collect();
        }
        fn_commands.insert(name, cmds);
    }

    // Identify the main function's commands
    let main_cmds = fn_commands.get("main").cloned().unwrap_or_default();

    // For each main command, check if there's a dispatch function (cmd_<name> or dispatch_<name>)
    // that has its own subcommands
    let mut commands = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for name in &main_cmds {
        let dispatch_fn = format!("dispatch_{name}");
        let cmd_fn = format!("cmd_{name}");

        // Check dispatch function first, then cmd function for sub-commands
        let sub_cmds = fn_commands
            .get(dispatch_fn.as_str())
            .or_else(|| fn_commands.get(cmd_fn.as_str()));

        if let Some(subs) = sub_cmds {
            if !subs.is_empty() {
                // This is a group command — emit subcommands as "group sub"
                for sub in subs {
                    let full_name = format!("{name} {sub}");
                    if seen.insert(full_name.clone()) {
                        let desc = desc_map
                            .get(&full_name)
                            .or_else(|| desc_map.get(sub.as_str()))
                            .cloned();
                        commands.push(XtaskCommand {
                            name: full_name,
                            description: desc,
                        });
                    }
                }
                continue;
            }
        }

        // Top-level command (no sub-dispatch)
        if seen.insert(name.clone()) {
            let desc = desc_map.get(name.as_str()).cloned();
            commands.push(XtaskCommand {
                name: name.clone(),
                description: desc,
            });
        }
    }

    commands
}

/// Extract descriptions from help/usage eprintln!/println! lines.
///
/// Matches patterns like:
///   eprintln!("  name         Description text");
///   eprintln!("  name sub     Description text");
fn extract_descriptions(source: &str) -> HashMap<String, String> {
    let help_re =
        Regex::new(r#"(?:println!|eprintln!)\(\s*"\\?\s{2,}(\S+(?:\s+\S+)?)\s{2,}(.+?)\\?"\s*\)"#)
            .unwrap();
    let mut map = HashMap::new();
    for caps in help_re.captures_iter(source) {
        let key = caps[1].trim().to_string();
        let desc = caps[2].trim().to_string();
        map.insert(key, desc);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source_flat() {
        let source = r#"
fn main() {
    match cmd.as_deref() {
        Some("verify") => verify(),
        Some("lint") => lint(),
        _ => usage(),
    }
}
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name, "verify");
        assert_eq!(cmds[1].name, "lint");
    }

    #[test]
    fn test_parse_source_with_descriptions() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("check") => cargo(&["check"]),
        Some("test") => cargo(&["test"]),
        _ => {
            eprintln!("    check        Run cargo check");
            eprintln!("    test         Run cargo test");
        }
    }
}
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].name, "check");
        assert_eq!(cmds[0].description, Some("Run cargo check".to_string()));
    }

    #[test]
    fn test_parse_nested_subcommands() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("test") => cmd_test(),
        Some("verify") => verify(),
        _ => help(),
    }
}

fn cmd_test() {
    match suite.as_deref() {
        Some(s) => dispatch_test(s),
        None => {
            eprintln!("  unit              unit tests");
            eprintln!("  integration       integration tests");
        }
    }
}

fn dispatch_test(suite: &str) {
    match suite {
        Some("unit") => test_unit(),
        Some("integration") => test_integration(),
        other => bail!("unknown"),
    }
}
        "#;
        let cmds = parse_source(source);
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"test unit"), "got: {names:?}");
        assert!(names.contains(&"test integration"), "got: {names:?}");
        assert!(names.contains(&"verify"), "got: {names:?}");
        // "test" itself should NOT appear as a standalone command
        assert!(!names.contains(&"test"), "got: {names:?}");
    }

    #[test]
    fn test_nested_descriptions_from_dispatch() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("check") => cmd_check(),
        _ => {}
    }
}

fn cmd_check() {
    match sub.as_deref() {
        Some(s) => dispatch_check(s),
        None => {
            eprintln!("  stale-names        audit old names");
            eprintln!("  no-unwrap          scan for unwrap");
        }
    }
}

fn dispatch_check(sub: &str) {
    match sub {
        Some("stale-names") => check_stale(),
        Some("no-unwrap") => check_unwrap(),
        _ => {}
    }
}
        "#;
        let cmds = parse_source(source);
        let stale = cmds.iter().find(|c| c.name == "check stale-names").unwrap();
        assert_eq!(stale.description.as_deref(), Some("audit old names"));
    }

    #[test]
    fn test_bare_match_arms_in_dispatch() {
        // Minibox-style: dispatch_test uses bare "unit" => not Some("unit")
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("test") => cmd_test(),
        Some("verify") => verify(),
        _ => {}
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
        "e2e" => test_e2e(),
        "system-suite" | "e2e-suite" => test_system(),
        other => bail!("unknown"),
    }
}
        "#;
        let cmds = parse_source(source);
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"test unit"), "got: {names:?}");
        assert!(names.contains(&"test e2e"), "got: {names:?}");
        assert!(names.contains(&"test system-suite"), "got: {names:?}");
        assert!(names.contains(&"verify"), "got: {names:?}");
        assert!(
            !names.contains(&"test"),
            "group cmd should be expanded: {names:?}"
        );
    }

    #[test]
    fn test_no_duplicates() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("build") => build(),
        Some("build") => build(),
        _ => {}
    }
}
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn test_parse_source_empty_input() {
        let cmds = parse_source("");
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_parse_source_no_match_block() {
        let source = "fn main() {\n    println!(\"hello\");\n}\n";
        let cmds = parse_source(source);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_descriptions_with_escaped_quotes() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("check") => check(),
        _ => {
            eprintln!("    check        Run \"cargo check\"");
        }
    }
}
        "#;
        let cmds = parse_source(source);
        assert_eq!(cmds.len(), 1);
        // Description parsing may or may not capture escaped quotes —
        // the important thing is it doesn't panic
    }

    #[test]
    fn test_descriptions_multiword_command_key() {
        let source = r#"
fn main() {
    match task.as_deref() {
        Some("test") => cmd_test(),
        _ => {}
    }
}

fn cmd_test() {
    match sub.as_deref() {
        Some(s) => dispatch_test(s),
        None => {
            eprintln!("  test unit     Run unit tests");
        }
    }
}

fn dispatch_test(sub: &str) {
    match sub {
        Some("unit") => test_unit(),
        _ => {}
    }
}
        "#;
        let cmds = parse_source(source);
        let unit = cmds.iter().find(|c| c.name == "test unit");
        assert!(unit.is_some(), "missing 'test unit' in {cmds:?}");
        assert_eq!(unit.unwrap().description.as_deref(), Some("Run unit tests"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_source_never_panics(input in any::<String>()) {
            let _ = parse_source(&input);
        }

        #[test]
        fn parsed_commands_have_nonempty_names(input in any::<String>()) {
            let cmds = parse_source(&input);
            for cmd in &cmds {
                prop_assert!(!cmd.name.is_empty());
            }
        }

        #[test]
        fn parse_source_is_idempotent(input in any::<String>()) {
            let first = parse_source(&input);
            let second = parse_source(&input);
            prop_assert_eq!(first.len(), second.len());
            for (a, b) in first.iter().zip(second.iter()) {
                prop_assert_eq!(&a.name, &b.name);
                prop_assert_eq!(&a.description, &b.description);
            }
        }
    }
}
